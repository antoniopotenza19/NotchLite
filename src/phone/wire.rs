//! One HTTP/1.1 request per pinned TLS connection; no browser/CORS endpoint.

use super::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpStream;

const MAX_HEADERS: usize = 16 * 1024;
const MAX_JSON: u64 = 8 * 1024;
const HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(120);

struct Socket { inner: TcpStream, shared: Arc<Shared>, deadline: Instant, cancel: Option<Arc<AtomicBool>> }

impl Socket {
    fn check(&self) -> io::Result<()> {
        if self.shared.stopped.load(Ordering::Acquire) || self.cancel.as_ref().is_some_and(|flag| flag.load(Ordering::Acquire)) {
            // read_exact/write_all retry Interrupted indefinitely.
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "stopped"));
        }
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "deadline"));
        }
        Ok(())
    }
}

impl Read for Socket {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        loop {
            self.check()?;
            match self.inner.read(data) {
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => continue,
                result => return result,
            }
        }
    }
}
impl Write for Socket {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        loop {
            self.check()?;
            match self.inner.write(data) {
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => continue,
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> { self.check()?; self.inner.flush() }
}

type Tls = rustls::StreamOwned<rustls::ServerConnection, Socket>;

struct Request { method: String, path: String, auth: String, length: u64, json: bool }

fn parse_headers(bytes: &[u8], host: &str) -> Result<Request, &'static str> {
    if bytes.len() > MAX_HEADERS { return Err("headers_too_large"); }
    let mut headers = [httparse::EMPTY_HEADER; 48];
    let mut parsed = httparse::Request::new(&mut headers);
    let end = parsed.parse(bytes).map_err(|_| "invalid_http")?;
    if end != httparse::Status::Complete(bytes.len()) || parsed.version != Some(1) { return Err("invalid_http"); }
    let method = parsed.method.ok_or("invalid_http")?;
    let path = parsed.path.ok_or("invalid_http")?;
    if path.len() > 256 || !path.starts_with("/v1/") || !path.is_ascii()
        || path.bytes().any(|b| matches!(b, b'%'|b'?'|b'#'|b'\\')) { return Err("invalid_path"); }
    let mut names = std::collections::HashSet::new();
    let mut length = None;
    let mut actual_host = None;
    let mut auth = String::new();
    let mut json_type = false;
    for header in parsed.headers.iter() {
        let name = header.name.to_ascii_lowercase();
        if !names.insert(name.clone()) { return Err("duplicate_header"); }
        let value = std::str::from_utf8(header.value).map_err(|_| "invalid_header")?.trim();
        match name.as_str() {
            "host" => actual_host = Some(value),
            "content-length" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) { return Err("invalid_length"); }
                length = Some(value.parse::<u64>().map_err(|_| "invalid_length")?);
            }
            "authorization" => auth = value.strip_prefix("Bearer ").ok_or("invalid_auth")?.to_string(),
            "content-type" => json_type = value == "application/json",
            "transfer-encoding" | "expect" | "origin" | "range" => return Err("unsupported_header"),
            _ => {}
        }
    }
    if actual_host != Some(host) { return Err("invalid_host"); }
    if matches!(method, "POST" | "PUT") && length.is_none() { return Err("length_required"); }
    let length = length.unwrap_or(0);
    if length > MAX_FILE_SIZE { return Err("body_too_large"); }
    if !matches!(method, "POST"|"PUT") && length != 0 { return Err("unexpected_body"); }
    Ok(Request { method: method.into(), path: path.into(), auth, length, json: json_type })
}

pub(super) fn serve(socket: TcpStream, config: Arc<rustls::ServerConfig>, shared: &Arc<Shared>) {
    if socket.set_read_timeout(Some(Duration::from_millis(200))).is_err()
        || socket.set_write_timeout(Some(Duration::from_millis(200))).is_err() { return; }
    let Ok(connection) = rustls::ServerConnection::new(config) else { return };
    let mut stream = Tls::new(connection, Socket { inner: socket, shared: Arc::clone(shared), deadline: Instant::now() + HEADER_TIMEOUT, cancel: None });
    let mut bytes = Vec::with_capacity(1024);
    let mut unit = [0u8; 1];
    loop {
        if bytes.len() >= MAX_HEADERS { let _ = error(&mut stream, 431, "headers_too_large"); return; }
        if stream.read_exact(&mut unit).is_err() { return; }
        bytes.push(unit[0]);
        if bytes.ends_with(b"\r\n\r\n") { break; }
    }
    let host = shared.state().address.clone();
    let request = match parse_headers(&bytes, &host) {
        Ok(request) => request,
        Err(code) => { let _ = error(&mut stream, 400, code); return; }
    };
    // The initial handshake/header has its own absolute 10s deadline. Body
    // transfer gets one fixed deadline, never renewed by trickling bytes.
    stream.sock.deadline = Instant::now() + TRANSFER_TIMEOUT;
    let _ = route(&request, &mut stream, shared);
    stream.conn.send_close_notify();
    let _ = stream.flush();
    let _ = stream.sock.inner.shutdown(Shutdown::Both);
}

fn body_json(request: &Request, stream: &mut impl Read) -> Result<Value, &'static str> {
    if !request.json { return Err("json_required"); }
    if request.length > MAX_JSON { return Err("json_too_large"); }
    let mut body = vec![0; request.length as usize];
    stream.read_exact(&mut body).map_err(|_| "incomplete_body")?;
    serde_json::from_slice(&body).map_err(|_| "invalid_json")
}

fn pair(shared: &Shared, value: Value) -> Result<Value, &'static str> {
    let secret = value.get("secret").and_then(Value::as_str).ok_or("invalid_pair")?;
    let name = value.get("name").and_then(Value::as_str).ok_or("invalid_pair")?;
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) { return Err("invalid_device_name"); }
    let mut state = shared.state();
    expire(&mut state, Instant::now());
    if shared.stopped.load(Ordering::Acquire) || state.pair.is_some() || state.secret.is_empty()
        || !token_eq(&state.secret, secret) { return Err("invalid_or_expired_secret"); }
    // Generate all output before committing the one-use transition.
    let id = random_hex(16).map_err(|_| "random_unavailable")?;
    let poll = random_hex(32).map_err(|_| "random_unavailable")?;
    let code_bytes = random_hex(4).map_err(|_| "random_unavailable")?;
    let code = format!("{:06}", u32::from_str_radix(&code_bytes, 16).map_err(|_| "random_unavailable")? % 1_000_000);
    state.secret.clear(); state.qr.clear();
    let deadline = state.secret_deadline;
    state.pair = Some(PairAttempt { request: PairRequest { id: id.clone(), name: name.into(), code: code.clone() },
        poll_token: poll.clone(), status: "pending", deadline });
    Ok(json!({"id":id,"pollToken":poll,"code":code}))
}

fn poll_pair(shared: &Shared, id: &str, token: &str) -> Result<Value, &'static str> {
    let mut state = shared.state();
    expire(&mut state, Instant::now());
    if shared.stopped.load(Ordering::Acquire) { return Err("stopped"); }
    let pair = state.pair.as_ref().filter(|p| p.request.id == id && token_eq(&p.poll_token, token)).ok_or("unauthorized")?;
    if pair.status == "approved" {
        let token = state.token.as_ref().ok_or("stopped")?;
        Ok(json!({"status":"approved","token":token}))
    } else { Ok(json!({"status":pair.status})) }
}

fn transfer_json(item: &Transfer) -> Value {
    json!({"id":item.id,"name":item.name,"size":item.size,"transferred":item.transferred,"status":item.status.as_str()})
}

fn incoming_offer(shared: &Shared, value: Value) -> Result<Value, &'static str> {
    let name = value.get("name").and_then(Value::as_str).ok_or("invalid_offer")?;
    validate_name(name)?;
    let size = value.get("size").and_then(Value::as_u64).filter(|&s| s <= MAX_FILE_SIZE).ok_or("invalid_size")?;
    let mut state = shared.state();
    if shared.stopped.load(Ordering::Acquire) || state.token.is_none() { return Err("unauthorized"); }
    if state.incoming.as_ref().is_some_and(|i| !i.transfer.status.terminal()) { return Err("incoming_busy"); }
    let id = random_hex(16).map_err(|_| "random_unavailable")?;
    state.incoming = Some(Incoming { transfer: Transfer { id: id.clone(), name: name.into(), size, transferred: 0, status: TransferStatus::Pending }, cancel: Arc::new(AtomicBool::new(false)) });
    Ok(json!({"id":id}))
}

fn route(request: &Request, stream: &mut Tls, shared: &Shared) -> io::Result<()> {
    let path: Vec<&str> = request.path.split('/').collect();
    if request.method == "POST" && request.path == "/v1/pair" {
        return match body_json(request, stream).and_then(|body| pair(shared, body)) {
            Ok(body) => json_response(stream, 200, &body), Err(code) => error(stream, 400, code),
        };
    }
    if request.method == "GET" && path.len() == 4 && path[2] == "pair" {
        return match poll_pair(shared, path[3], &request.auth) {
            Ok(body) => json_response(stream, 200, &body), Err(code) => error(stream, 401, code),
        };
    }
    if !shared.authorized(&request.auth) { return error(stream, 401, "unauthorized"); }
    match (request.method.as_str(), path.as_slice()) {
        ("GET", ["", "v1", "state"]) => {
            let state = shared.state();
            if shared.stopped.load(Ordering::Acquire) || state.token.is_none() { return error(stream, 401, "unauthorized"); }
            let body = json!({"paired":true,
                "incoming":state.incoming.as_ref().map(|i| transfer_json(&i.transfer)),
                "outgoing":state.outgoing.as_ref().map(|i| transfer_json(&i.transfer))});
            drop(state);
            json_response(stream, 200, &body)
        }
        ("POST", ["", "v1", "offers"]) => match body_json(request, stream).and_then(|body| incoming_offer(shared, body)) {
            Ok(body) => json_response(stream, 200, &body), Err(code) => error(stream, 400, code),
        },
        ("PUT", ["", "v1", "incoming", id]) => receive(request, stream, shared, id),
        ("GET", ["", "v1", "outgoing", id]) => send(stream, shared, id),
        ("POST", ["", "v1", "outgoing", id, "done"]) => {
            if request.length != 0 && !body_json(request, stream).ok()
                .and_then(|v| v.as_object().map(|object| object.is_empty())).unwrap_or(false) {
                return error(stream, 400, "unexpected_body");
            }
            let mut state = shared.state();
            if !state.outgoing.as_ref().is_some_and(|v| v.transfer.id == *id && v.transfer.status == TransferStatus::Complete) { return error(stream, 409, "not_complete"); }
            state.outgoing = None;
            drop(state);
            json_response(stream, 200, &json!({"ok":true}))
        }
        ("DELETE", ["", "v1", "transfers", id]) => {
            let mut state = shared.state();
            let mut found = false;
            if let Some(item) = state.incoming.as_mut().filter(|v| v.transfer.id == *id) {
                if item.transfer.status != TransferStatus::Complete {
                    item.cancel.store(true, Ordering::Release); item.transfer.status = TransferStatus::Cancelled;
                }
                found = true;
            }
            if let Some(item) = state.outgoing.as_mut().filter(|v| v.transfer.id == *id) {
                item.cancel.store(true, Ordering::Release); item.transfer.status = TransferStatus::Cancelled;
                found = true;
            }
            drop(state);
            if found { json_response(stream, 200, &json!({"ok":true})) } else { error(stream, 404, "unknown_transfer") }
        }
        _ => error(stream, 404, "not_found"),
    }
}

struct PartialFile { directory: PathBuf, path: PathBuf, guard: Option<File>, owned: bool, committed: bool }

impl PartialFile {
    fn create(root: &ReceiveRoot, id: &str) -> io::Result<(Self, File)> {
        let directory = root.path.join(format!("notchlite-{id}"));
        fs::create_dir(&directory)?;
        let guard = match directory_guard(&directory) {
            Ok(guard) => guard,
            Err(error) => { let _ = fs::remove_dir(&directory); return Err(error); }
        };
        let path = directory.join("payload.part");
        let mut partial = Self { directory, path, guard: Some(guard), owned: false, committed: false };
        let file = OpenOptions::new().write(true).create_new(true).share_mode(0).open(&partial.path)?;
        partial.owned = true;
        Ok((partial, file))
    }
    fn commit(&mut self, name: &str) -> io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        validate_name(name).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "name"))?;
        let destination = self.directory.join(name);
        let source: Vec<u16> = self.path.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
        // No REPLACE_EXISTING or COPY_ALLOWED: same-directory atomic publish,
        // existing entries (including reparse points) win instead of overwrite.
        unsafe { MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(destination.as_ptr()), MOVEFILE_WRITE_THROUGH) }
            .map_err(io::Error::other)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for PartialFile {
    fn drop(&mut self) {
        if self.owned && !self.committed { let _ = fs::remove_file(&self.path); }
        drop(self.guard.take());
        if !self.committed { let _ = fs::remove_dir(&self.directory); }
    }
}

fn cancelled(shared: &Shared, cancel: &AtomicBool) -> io::Result<()> {
    if shared.stopped.load(Ordering::Acquire) || cancel.load(Ordering::Acquire) {
        Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"))
    } else { Ok(()) }
}

fn copy_exact(reader: &mut impl Read, writer: &mut impl Write, mut remaining: u64,
              mut check: impl FnMut(u64) -> io::Result<()>) -> io::Result<()> {
    let mut buffer = [0u8; 64 * 1024];
    let mut copied = 0;
    while remaining != 0 {
        check(copied)?;
        let want = remaining.min(buffer.len() as u64) as usize;
        let count = reader.read(&mut buffer[..want])?;
        if count == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "incomplete body")); }
        check(copied)?;
        writer.write_all(&buffer[..count])?;
        remaining -= count as u64;
        copied += count as u64;
    }
    check(copied)
}

fn receive(request: &Request, stream: &mut Tls, shared: &Shared, id: &str) -> io::Result<()> {
    let (transfer, cancel) = {
        let mut state = shared.state();
        let Some(incoming) = state.incoming.as_mut().filter(|v| v.transfer.id == id && v.transfer.status == TransferStatus::Accepted) else {
            drop(state); return error(stream, 409, "not_accepted");
        };
        if incoming.transfer.size != request.length { drop(state); return error(stream, 400, "size_mismatch"); }
        // Claim under the mutex before file creation: concurrent PUT cannot
        // open the same destination or duplicate an accepted transfer.
        incoming.transfer.status = TransferStatus::Receiving;
        (incoming.transfer.clone(), Arc::clone(&incoming.cancel))
    };
    let result = (|| {
        cancelled(shared, &cancel)?;
        stream.sock.cancel = Some(Arc::clone(&cancel));
        let (mut partial, mut file) = PartialFile::create(&shared.root, id)?;
        copy_exact(stream, &mut file, request.length, |count| {
            cancelled(shared, &cancel)?;
            if let Some(item) = shared.state().incoming.as_mut().filter(|i| i.transfer.id == id) { item.transfer.transferred = count; }
            Ok(())
        })?;
        file.sync_all()?;
        drop(file);
        // Serialize cancel and the decision to publish. Stop forbids a new
        // publish, but cannot undo a MoveFileEx syscall already in progress.
        let mut state = shared.state();
        cancelled(shared, &cancel)?;
        let item = state.incoming.as_mut().filter(|i| i.transfer.id == id && i.transfer.status == TransferStatus::Receiving)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Interrupted, "obsolete transfer"))?;
        partial.commit(&transfer.name)?;
        item.transfer.status = TransferStatus::Complete;
        Ok::<(), io::Error>(())
    })();
    match result {
        Ok(()) => json_response(stream, 200, &json!({"ok":true})),
        Err(_) => { shared.fail_transfer(id); error(stream, 400, "transfer_failed") }
    }
}

fn send(stream: &mut Tls, shared: &Shared, id: &str) -> io::Result<()> {
    let (transfer, cancel, mut file) = {
        let mut state = shared.state();
        let Some(item) = state.outgoing.as_mut().filter(|i| i.transfer.id == id && matches!(i.transfer.status, TransferStatus::Accepted | TransferStatus::Complete)) else {
            drop(state); return error(stream, 409, "not_offered");
        };
        let file = item.file.try_clone()?;
        item.transfer.status = TransferStatus::Sending;
        (item.transfer.clone(), Arc::clone(&item.cancel), file)
    };
    let result = (|| {
        cancelled(shared, &cancel)?;
        stream.sock.cancel = Some(Arc::clone(&cancel));
        file.seek(SeekFrom::Start(0))?;
        header(stream, 200, "application/octet-stream", transfer.size)?;
        copy_exact(&mut file, stream, transfer.size, |count| {
            cancelled(shared, &cancel)?;
            if let Some(item) = shared.state().outgoing.as_mut().filter(|i| i.transfer.id == id) { item.transfer.transferred = count; }
            Ok(())
        })?;
        stream.flush()?;
        let mut state = shared.state();
        cancelled(shared, &cancel)?;
        let item = state.outgoing.as_mut().filter(|i| i.transfer.id == id && i.transfer.status == TransferStatus::Sending)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Interrupted, "obsolete transfer"))?;
        item.transfer.status = TransferStatus::Complete;
        Ok::<(), io::Error>(())
    })();
    if result.is_err() { shared.fail_transfer(id); }
    result
}

fn header(stream: &mut impl Write, status: u16, kind: &str, length: u64) -> io::Result<()> {
    let reason = match status { 200 => "OK", 400 => "Bad Request", 401 => "Unauthorized", 404 => "Not Found", 409 => "Conflict", 431 => "Request Header Fields Too Large", _ => "Error" };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {length}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n")
}
fn json_response(stream: &mut impl Write, status: u16, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    header(stream, status, "application/json", body.len() as u64)?;
    stream.write_all(&body)
}
fn error(stream: &mut impl Write, status: u16, code: &str) -> io::Result<()> { json_response(stream, status, &json!({"error":code})) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("notchlite-phone-test-{}", random_hex(16).unwrap()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir(&self.0); } }

    fn call_with_cert(service: &PhoneService, cert: Vec<u8>, method: &str, path: &str,
                      auth: &str, body: Option<&[u8]>) -> Result<(u16, Vec<u8>), String> {
        let mut stream = client_stream(service, cert)?;
        let address = service.snapshot().address;
        let payload = body.unwrap_or(b"");
        let auth = if auth.is_empty() { String::new() } else { format!("Authorization: Bearer {auth}\r\n") };
        let header = format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\n{auth}Content-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n", payload.len());
        stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(payload).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).map_err(|e| e.to_string())?;
        let boundary = response.windows(4).position(|w| w == b"\r\n\r\n").ok_or("missing response")?;
        let status = std::str::from_utf8(&response[..boundary]).map_err(|e| e.to_string())?
            .split_whitespace().nth(1).ok_or("missing status")?.parse().map_err(|_| "invalid status")?;
        Ok((status, response[boundary + 4..].to_vec()))
    }

    fn client_stream(service: &PhoneService, cert: Vec<u8>) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, String> {
        use rustls::pki_types::{CertificateDer, ServerName};
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from(cert)).map_err(|e| e.to_string())?;
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions().map_err(|e| e.to_string())?
            .with_root_certificates(roots).with_no_client_auth();
        let connection = rustls::ClientConnection::new(Arc::new(config), ServerName::try_from("127.0.0.1").unwrap()).map_err(|e| e.to_string())?;
        let address = service.snapshot().address;
        let socket = TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_secs(3)).map_err(|e| e.to_string())?;
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        socket.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        Ok(rustls::StreamOwned::new(connection, socket))
    }

    fn call(service: &PhoneService, method: &str, path: &str, auth: &str, body: Option<&[u8]>) -> (u16, Vec<u8>) {
        call_with_cert(service, service.certificate.clone(), method, path, auth, body).unwrap()
    }
    fn decode(response: &(u16, Vec<u8>)) -> Value { serde_json::from_slice(&response.1).unwrap() }
    fn pair_request(service: &PhoneService) -> Value {
        let secret = service.shared.state().secret.clone();
        let request = serde_json::to_vec(&json!({"secret":secret,"name":"Synthetic phone"})).unwrap();
        let response = call(service, "POST", "/v1/pair", "", Some(&request));
        assert_eq!(response.0, 200);
        decode(&response)
    }
    fn paired(service: &PhoneService) -> String {
        let pending = pair_request(service);
        let id = pending["id"].as_str().unwrap();
        service.approve_pair(id).unwrap();
        let response = call(service, "GET", &format!("/v1/pair/{id}"), pending["pollToken"].as_str().unwrap(), None);
        assert_eq!(response.0, 200);
        decode(&response)["token"].as_str().unwrap().to_string()
    }

    #[test]
    fn loopback_tls_pairing_requires_exact_identity_and_pc_consent() {
        let fixture = Fixture::new();
        let mut service = PhoneService::start_loopback(fixture.0.clone()).unwrap();
        assert!(service.snapshot().pairing_uri.contains(&format!("pin={}", hex(digest::digest(&digest::SHA256, &service.certificate).as_ref()))));
        let wrong = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        assert!(call_with_cert(&service, wrong.cert.der().as_ref().to_vec(), "POST", "/v1/pair", "", Some(b"{}")).is_err());
        assert!(service.snapshot().pending_pair.is_none());
        assert_eq!(call(&service, "GET", "/v1/state", &"f".repeat(64), None).0, 401);
        let pending = pair_request(&service);
        let id = pending["id"].as_str().unwrap();
        let poll = pending["pollToken"].as_str().unwrap();
        let ui = service.snapshot().pending_pair.unwrap();
        assert_eq!(ui.id, id);
        assert_eq!(ui.code, pending["code"].as_str().unwrap());
        assert!(!service.snapshot().pairing_available);
        let response = decode(&call(&service, "GET", &format!("/v1/pair/{id}"), poll, None));
        assert_eq!(response["status"], "pending");
        assert!(response.get("token").is_none());
        assert_eq!(call(&service, "GET", "/v1/state", poll, None).0, 401);
        assert!(service.approve_pair("stale-request").is_err());
        service.approve_pair(id).unwrap();
        let approval = decode(&call(&service, "GET", &format!("/v1/pair/{id}"), poll, None));
        let token = approval["token"].as_str().unwrap();
        assert!(service.snapshot().pending_pair.is_none());
        assert_eq!(call(&service, "GET", "/v1/state", token, None).0, 200);
        assert_eq!(call(&service, "POST", "/v1/pair", "", Some(b"{\"secret\":\"a\",\"name\":\"again\"}")).0, 400);
        let address = service.snapshot().address.parse().unwrap();
        let start = Instant::now();
        service.stop();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(!service.shared.authorized(token));
        assert!(TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_err());
    }

    #[test]
    fn loopback_pair_expiry_and_denial_never_issue_session_tokens() {
        for denied in [false, true] {
            let fixture = Fixture::new();
            let mut service = PhoneService::start_loopback(fixture.0.clone()).unwrap();
            let pending = pair_request(&service);
            let id = pending["id"].as_str().unwrap();
            if denied { service.deny_pair(id).unwrap(); }
            else { service.shared.state().pair.as_mut().unwrap().deadline = Instant::now(); }
            assert!(service.approve_pair(id).is_err());
            let result = decode(&call(&service, "GET", &format!("/v1/pair/{id}"), pending["pollToken"].as_str().unwrap(), None));
            assert_eq!(result["status"], if denied { "denied" } else { "expired" });
            assert!(result.get("token").is_none());
            service.stop();
        }
    }

    #[test]
    fn loopback_approved_upload_preserves_exact_bytes_and_cancel_blocks_write() {
        let fixture = Fixture::new();
        let mut service = PhoneService::start_loopback(fixture.0.clone()).unwrap();
        let token = paired(&service);
        let offer = call(&service, "POST", "/v1/offers", &token, Some(b"{\"name\":\"example.bin\",\"size\":5}"));
        let id = decode(&offer)["id"].as_str().unwrap().to_string();
        let target = format!("/v1/incoming/{id}");
        assert_eq!(call(&service, "PUT", &target, &token, Some(b"hello")).0, 409);
        assert!(!fixture.0.join(format!("notchlite-{id}")).exists());
        service.approve_incoming(&id).unwrap();
        assert_eq!(call(&service, "PUT", &target, &token, Some(b"short!")).0, 400);
        assert_eq!(call(&service, "PUT", &target, &token, Some(b"hello")).0, 200);
        let directory = fixture.0.join(format!("notchlite-{id}"));
        assert_eq!(fs::read(directory.join("example.bin")).unwrap(), b"hello");
        assert!(!directory.join("payload.part").exists());
        let incoming = service.snapshot().incoming.unwrap();
        assert_eq!(incoming.status, TransferStatus::Complete);
        assert_eq!(incoming.transferred, 5);
        let offer = decode(&call(&service, "POST", "/v1/offers", &token, Some(b"{\"name\":\"cancel.bin\",\"size\":1}")));
        let cancelled_id = offer["id"].as_str().unwrap();
        service.approve_incoming(cancelled_id).unwrap();
        assert_eq!(call(&service, "DELETE", &format!("/v1/transfers/{cancelled_id}"), &token, None).0, 200);
        assert_eq!(call(&service, "PUT", &format!("/v1/incoming/{cancelled_id}"), &token, Some(b"x")).0, 409);
        assert!(!fixture.0.join(format!("notchlite-{cancelled_id}")).exists());
        service.stop(); drop(service);
        fs::remove_file(directory.join("example.bin")).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn loopback_outgoing_retries_same_handle_until_client_acknowledges() {
        let fixture = Fixture::new();
        let source = fixture.0.join("outgoing.txt");
        fs::write(&source, b"actual file bytes").unwrap();
        let mut service = PhoneService::start_loopback(fixture.0.clone()).unwrap();
        let token = paired(&service);
        service.offer_file(source.clone()).unwrap();
        let transfer = service.snapshot().outgoing.unwrap();
        let endpoint = format!("/v1/outgoing/{}", transfer.id);
        for _ in 0..2 {
            let result = call(&service, "GET", &endpoint, &token, None);
            assert_eq!(result.0, 200);
            assert_eq!(result.1, b"actual file bytes");
        }
        assert_eq!(service.snapshot().outgoing.unwrap().transferred, 17);
        assert_eq!(call(&service, "POST", &format!("{endpoint}/done"), &token, Some(b"{}")).0, 200);
        assert!(service.snapshot().outgoing.is_none());
        assert_eq!(call(&service, "GET", &endpoint, &token, None).0, 409);
        service.stop(); drop(service);
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn staging_publish_never_overwrites_and_cleanup_removes_only_owned_partial() {
        let fixture = Fixture::new();
        let root = ReceiveRoot::prepare(&fixture.0).unwrap();
        let (mut partial, mut file) = PartialFile::create(&root, "synthetic-transfer").unwrap();
        file.write_all(b"new bytes").unwrap(); drop(file);
        let directory = partial.directory.clone();
        let destination = directory.join("keep.txt");
        fs::write(&destination, b"existing bytes").unwrap();
        assert!(partial.commit("keep.txt").is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"existing bytes");
        drop(partial);
        assert!(!directory.join("payload.part").exists());
        assert_eq!(fs::read(&destination).unwrap(), b"existing bytes");
        let foreign_path = directory.join("foreign.bin");
        fs::write(&foreign_path, b"not owned").unwrap();
        drop(PartialFile { directory: directory.clone(), path: foreign_path.clone(), guard: None, owned: false, committed: false });
        assert_eq!(fs::read(&foreign_path).unwrap(), b"not owned");
        fs::remove_file(foreign_path).unwrap();
        fs::remove_file(destination).unwrap();
        fs::remove_dir(directory).unwrap();
        drop(root);
    }

    #[test]
    fn loopback_cancel_interrupts_a_partial_upload_and_stop_closes_idle_handshake() {
        let fixture = Fixture::new();
        let mut service = PhoneService::start_loopback(fixture.0.clone()).unwrap();
        let token = paired(&service);
        let offer = decode(&call(&service, "POST", "/v1/offers", &token, Some(b"{\"name\":\"partial.bin\",\"size\":65536}")));
        let id = offer["id"].as_str().unwrap();
        service.approve_incoming(id).unwrap();
        let mut stream = client_stream(&service, service.certificate.clone()).unwrap();
        write!(stream, "PUT /v1/incoming/{id} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Length: 65536\r\n\r\n", service.snapshot().address).unwrap();
        stream.write_all(&[7u8; 64]).unwrap(); stream.flush().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while service.snapshot().incoming.as_ref().unwrap().transferred != 64 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(service.snapshot().incoming.unwrap().transferred, 64);
        service.cancel(id).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !service.shared.sockets.lock().unwrap().is_empty() && Instant::now() < deadline { thread::sleep(Duration::from_millis(10)); }
        assert!(service.shared.sockets.lock().unwrap().is_empty());
        assert!(!fixture.0.join(format!("notchlite-{id}")).exists());
        drop(stream);
        let address = service.snapshot().address;
        let mut idle = TcpStream::connect(&address).unwrap();
        idle.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while service.shared.sockets.lock().unwrap().is_empty() && Instant::now() < deadline { thread::sleep(Duration::from_millis(10)); }
        assert!(!service.shared.sockets.lock().unwrap().is_empty());
        let started = Instant::now();
        service.stop();
        assert!(started.elapsed() < Duration::from_secs(2));
        let mut byte = [0];
        assert!(matches!(idle.read(&mut byte), Ok(0) | Err(_)));
    }

    #[test]
    fn framing_rejects_ambiguous_lengths_and_browser_requests() {
        let good = "POST /v1/pair HTTP/1.1\r\nHost: 192.168.1.2:5\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n";
        assert_eq!(parse_headers(good.as_bytes(), "192.168.1.2:5").unwrap().length, 2);
        for bad in [
            good.replace("Content-Length: 2", "Content-Length: 2\r\ncontent-length: 2"),
            good.replace("Content-Length: 2", "Content-Length: 2,2"),
            good.replace("Content-Length: 2", "Content-Length: +2"),
            good.replace("Content-Length: 2", "Content-Length: 2\r\nTransfer-Encoding: chunked"),
            good.replace("Content-Length: 2", "Content-Length: 2\r\nOrigin: https://example.invalid"),
            good.replace("Content-Length: 2\r\n", ""),
            good.replace("192.168.1.2:5", "example.invalid"),
            good.replace("/v1/pair", "/v1/%70air"),
        ] { assert!(parse_headers(bad.as_bytes(), "192.168.1.2:5").is_err(), "{bad}"); }
    }

    #[test]
    fn json_limits_and_exact_stream_lengths_use_bounded_buffers() {
        let request = Request { method: "POST".into(), path: "/v1/pair".into(), auth: String::new(), length: 2, json: true };
        assert!(body_json(&request, &mut Cursor::new(b"{}")).is_ok());
        assert!(body_json(&Request { length: MAX_JSON + 1, ..request }, &mut Cursor::new(b"{}")).is_err());
        let mut output = Vec::new();
        copy_exact(&mut Cursor::new(b"abcEXTRA"), &mut output, 3, |_| Ok(())).unwrap();
        assert_eq!(output, b"abc");
        assert!(copy_exact(&mut Cursor::new(b"ab"), &mut Vec::new(), 3, |_| Ok(())).is_err());
        assert!(copy_exact(&mut Cursor::new(b"abc"), &mut Vec::new(), 3,
            |_| Err(io::Error::new(io::ErrorKind::Interrupted, "cancel"))).is_err());
    }
}
