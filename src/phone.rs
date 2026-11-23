//! Opt-in, volatile LAN file exchange. Native clients must pin the QR's DER
//! SHA-256 before sending its one-use secret; this is not a browser server.

mod wire;

use std::fs::{self, File, OpenOptions};
use std::io;
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use ring::{digest, rand::{SecureRandom, SystemRandom}};
use rustls::pki_types::PrivatePkcs8KeyDer;
use serde_json::{json, Value};

pub const MAX_FILE_SIZE: u64 = 256 * 1024 * 1024;
const PAIR_TTL: Duration = Duration::from_secs(120);
const MAX_CONNECTIONS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferStatus { Pending, Accepted, Receiving, Sending, Complete, Cancelled }

impl TransferStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending", Self::Accepted => "accepted",
            Self::Receiving => "receiving", Self::Sending => "sending",
            Self::Complete => "complete", Self::Cancelled => "cancelled",
        }
    }
    fn terminal(self) -> bool { matches!(self, Self::Complete | Self::Cancelled) }
}

#[derive(Clone, Debug)]
pub struct Transfer {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub transferred: u64,
    pub status: TransferStatus,
}

#[derive(Clone, Debug)]
pub struct PairRequest { pub id: String, pub name: String, pub code: String }

#[derive(Clone, Debug)]
pub struct PhoneSnapshot {
    pub pairing_uri: String,
    pub pairing_available: bool,
    pub address: String,
    pub pending_pair: Option<PairRequest>,
    pub paired: bool,
    pub incoming: Option<Transfer>,
    pub outgoing: Option<Transfer>,
    pub last_error: Option<String>,
}

pub struct PhoneService {
    shared: Arc<Shared>, worker: Option<JoinHandle<()>>, cached: Mutex<PhoneSnapshot>,
    #[cfg(test)] certificate: Vec<u8>,
}

struct PairAttempt {
    request: PairRequest,
    poll_token: String,
    status: &'static str,
    deadline: Instant,
}

struct Incoming { transfer: Transfer, cancel: Arc<AtomicBool> }
struct Outgoing { transfer: Transfer, file: File, cancel: Arc<AtomicBool> }

struct State {
    qr: String,
    address: String,
    secret: String,
    secret_deadline: Instant,
    pair: Option<PairAttempt>,
    token: Option<String>,
    incoming: Option<Incoming>,
    outgoing: Option<Outgoing>,
    error: Option<String>,
}

struct Shared {
    state: Mutex<State>,
    stopped: AtomicBool,
    sockets: Mutex<Vec<(u64, TcpStream)>>,
    root: ReceiveRoot,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> { self.state.lock().unwrap_or_else(|e| e.into_inner()) }
    fn authorized(&self, token: &str) -> bool {
        !self.stopped.load(Ordering::Acquire)
            && self.state().token.as_ref().is_some_and(|expected| token_eq(expected, token))
    }
    fn fail_transfer(&self, id: &str) {
        let mut state = self.state();
        if let Some(item) = state.incoming.as_mut().filter(|item| item.transfer.id == id) {
            item.cancel.store(true, Ordering::Release);
            item.transfer.status = TransferStatus::Cancelled;
        }
        if let Some(item) = state.outgoing.as_mut().filter(|item| item.transfer.id == id) {
            item.cancel.store(true, Ordering::Release);
            item.transfer.status = TransferStatus::Cancelled;
        }
        state.error = Some("transfer_failed".into());
    }
}

fn expire(state: &mut State, now: Instant) {
    if now >= state.secret_deadline { state.secret.clear(); }
    if let Some(pair) = state.pair.as_mut() {
        if pair.status == "pending" && now >= pair.deadline { pair.status = "expired"; }
    }
}

impl PhoneService {
    pub fn start(ip: Ipv4Addr, receive_dir: PathBuf) -> Result<Self, String> {
        if !ip.is_private() { return Err("Choose an RFC1918 local IPv4 address".into()); }
        if !private_ipv4_addresses()?.contains(&ip) { return Err("IPv4 address is not on an active local adapter".into()); }
        Self::start_bound(ip, receive_dir)
    }

    fn start_bound(ip: Ipv4Addr, receive_dir: PathBuf) -> Result<Self, String> {
        let root = ReceiveRoot::prepare(&receive_dir).map_err(|_| "Cannot prepare a safe receive directory")?;
        let cert = generate_identity(ip)?;
        let der = cert.cert.der().clone();
        #[cfg(test)] let certificate = der.as_ref().to_vec();
        let pin = hex(digest::digest(&digest::SHA256, der.as_ref()).as_ref());
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions().map_err(|_| "TLS configuration failed")?
            .with_no_client_auth()
            .with_single_cert(vec![der], PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into())
            .map_err(|_| "TLS identity failed")?;
        let listener = TcpListener::bind((ip, 0)).map_err(|_| "Cannot bind the selected IPv4 address")?;
        listener.set_nonblocking(true).map_err(|_| "Cannot configure listener")?;
        let port = listener.local_addr().map_err(|_| "Cannot read listener address")?.port();
        let address = format!("{ip}:{port}");
        let secret = random_hex(32)?;
        let qr = format!("notchlite://pair?v=1&host={ip}&port={port}&pin={pin}&secret={secret}");
        let shared = Arc::new(Shared {
            state: Mutex::new(State { qr, address, secret, secret_deadline: Instant::now() + PAIR_TTL,
                pair: None, token: None, incoming: None, outgoing: None, error: None }),
            stopped: AtomicBool::new(false), sockets: Mutex::new(Vec::new()), root,
        });
        let worker_shared = Arc::clone(&shared);
        let config = Arc::new(config);
        let worker = thread::Builder::new().name("phone-listener".into())
            .spawn(move || run(listener, config, worker_shared)).map_err(|_| "Cannot start phone listener")?;
        let cached = Mutex::new(snapshot_state(&shared.state(), false));
        Ok(Self { shared, worker: Some(worker), cached, #[cfg(test)] certificate })
    }

    #[cfg(test)]
    fn start_loopback(receive_dir: PathBuf) -> Result<Self, String> {
        Self::start_bound(Ipv4Addr::LOCALHOST, receive_dir)
    }

    pub fn snapshot(&self) -> PhoneSnapshot {
        let stopped = self.shared.stopped.load(Ordering::Acquire);
        if let Ok(mut state) = self.shared.state.try_lock() {
            expire(&mut state, Instant::now());
            let snapshot = snapshot_state(&state, stopped);
            *self.cached.lock().unwrap_or_else(|e| e.into_inner()) = snapshot.clone();
            snapshot
        } else {
            // A file publish may be in progress. Poll never waits on disk I/O.
            let mut snapshot = self.cached.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if stopped { snapshot.paired = false; snapshot.pairing_available = false; snapshot.pairing_uri.clear(); snapshot.pending_pair = None; }
            snapshot
        }
    }

    pub fn approve_pair(&self, id: &str) -> Result<(), String> {
        let mut state = self.shared.state.try_lock().map_err(|_| "Phone state busy; retry or Stop")?;
        expire(&mut state, Instant::now());
        if self.shared.stopped.load(Ordering::Acquire) { return Err("Service stopped".into()); }
        let pair = state.pair.as_mut().filter(|p| p.request.id == id && p.status == "pending").ok_or("Pair request is no longer pending")?;
        // Recheck the monotonic deadline at approval, not just when rendering.
        if Instant::now() >= pair.deadline { pair.status = "expired"; return Err("Pair request expired".into()); }
        let token = random_hex(32)?;
        if Instant::now() >= pair.deadline || self.shared.stopped.load(Ordering::Acquire) {
            pair.status = "expired"; return Err("Pair request expired or stopped".into());
        }
        pair.status = "approved";
        state.token = Some(token);
        Ok(())
    }

    pub fn deny_pair(&self, id: &str) -> Result<(), String> {
        let mut state = self.shared.state.try_lock().map_err(|_| "Phone state busy; retry or Stop")?;
        let pair = state.pair.as_mut().filter(|p| p.request.id == id && p.status == "pending").ok_or("Pair request is no longer pending")?;
        pair.status = "denied";
        Ok(())
    }

    pub fn approve_incoming(&self, id: &str) -> Result<(), String> {
        let mut state = self.shared.state.try_lock().map_err(|_| "Phone state busy; retry or Stop")?;
        if self.shared.stopped.load(Ordering::Acquire) || state.token.is_none() { return Err("Not paired".into()); }
        let item = state.incoming.as_mut().filter(|i| i.transfer.id == id && i.transfer.status == TransferStatus::Pending).ok_or("Offer is no longer pending")?;
        item.transfer.status = TransferStatus::Accepted;
        Ok(())
    }

    pub fn deny_incoming(&self, id: &str) -> Result<(), String> { self.cancel(id) }

    pub fn offer_file(&self, path: PathBuf) -> Result<(), String> {
        let name = path.file_name().and_then(|n| n.to_str()).ok_or("Invalid file name")?;
        validate_name(name).map_err(str::to_string)?;
        // Deny write/delete sharing, so the same handle's size/content cannot be
        // replaced while it is offered. Remote clients never supply this path.
        let file = OpenOptions::new().read(true).share_mode(1).open(&path).map_err(|_| "Cannot open selected file")?;
        let meta = file.metadata().map_err(|_| "Cannot inspect selected file")?;
        if !meta.is_file() || meta.len() > MAX_FILE_SIZE || meta.file_attributes() & 0x400 != 0 { return Err("Unsupported file or size above 256 MiB".into()); }
        let mut state = self.shared.state.try_lock().map_err(|_| "Phone state busy; retry or Stop")?;
        if self.shared.stopped.load(Ordering::Acquire) || state.token.is_none() { return Err("Not paired".into()); }
        if state.outgoing.as_ref().is_some_and(|v| !v.transfer.status.terminal()) { return Err("An outgoing file is already active".into()); }
        state.outgoing = Some(Outgoing { transfer: Transfer { id: random_hex(16)?, name: name.into(), size: meta.len(), transferred: 0, status: TransferStatus::Accepted }, file, cancel: Arc::new(AtomicBool::new(false)) });
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let mut state = self.shared.state.try_lock().map_err(|_| "Phone state busy; retry or Stop")?;
        let mut found = false;
        if let Some(item) = state.incoming.as_mut().filter(|v| v.transfer.id == id) {
            if item.transfer.status != TransferStatus::Complete {
                item.cancel.store(true, Ordering::Release); item.transfer.status = TransferStatus::Cancelled;
            }
            found = true;
        }
        if let Some(item) = state.outgoing.as_mut().filter(|v| v.transfer.id == id) {
            item.cancel.store(true, Ordering::Release); item.transfer.status = TransferStatus::Cancelled;
            found = true;
        }
        if found { Ok(()) } else { Err("Unknown transfer".into()) }
    }

    pub fn stop(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        // Authentication and all stream checks consult stopped first. A disk
        // commit may hold state briefly: never block the UI on an OS disk stall.
        if let Ok(mut state) = self.shared.state.try_lock() {
            revoke(&mut state);
        }
        if let Ok(sockets) = self.shared.sockets.lock() {
            for (_, socket) in sockets.iter() { let _ = socket.shutdown(Shutdown::Both); }
        }
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_millis(900);
            while !worker.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            if worker.is_finished() { let _ = worker.join(); }
            // shortcut: an OS-blocked disk operation may outlive this wait;
            // its owned Arc/guards remain alive and stop prevents new work.
        }
    }
}

impl Drop for PhoneService { fn drop(&mut self) { self.stop(); } }

fn revoke(state: &mut State) {
    state.secret.clear(); state.qr.clear(); state.token = None;
    if let Some(pair) = state.pair.as_mut() { pair.status = "denied"; pair.poll_token.clear(); }
    if let Some(item) = state.incoming.as_mut() { item.cancel.store(true, Ordering::Release); }
    if let Some(item) = state.outgoing.as_mut() { item.cancel.store(true, Ordering::Release); }
}

fn snapshot_state(state: &State, stopped: bool) -> PhoneSnapshot {
    let available = !stopped && !state.secret.is_empty() && Instant::now() < state.secret_deadline && state.pair.is_none();
    PhoneSnapshot {
        pairing_uri: if available { state.qr.clone() } else { String::new() }, pairing_available: available,
        address: state.address.clone(), paired: !stopped && state.token.is_some(),
        pending_pair: state.pair.as_ref().filter(|p| !stopped && p.status == "pending" && Instant::now() < p.deadline).map(|p| p.request.clone()),
        incoming: state.incoming.as_ref().map(|v| v.transfer.clone()), outgoing: state.outgoing.as_ref().map(|v| v.transfer.clone()),
        last_error: state.error.clone(),
    }
}

fn run(listener: TcpListener, config: Arc<rustls::ServerConfig>, shared: Arc<Shared>) {
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut next_id = 0u64;
    while !shared.stopped.load(Ordering::Acquire) {
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() { let _ = workers.swap_remove(index).join(); } else { index += 1; }
        }
        if workers.len() < MAX_CONNECTIONS {
            match listener.accept() {
                Ok((socket, _)) => {
                    let Ok(control) = socket.try_clone() else { continue };
                    next_id += 1;
                    let id = next_id;
                    if let Ok(mut sockets) = shared.sockets.lock() { sockets.push((id, control)); }
                    let worker_shared = Arc::clone(&shared);
                    let worker_config = Arc::clone(&config);
                    match thread::Builder::new().name("phone-request".into()).spawn(move || {
                        wire::serve(socket, worker_config, &worker_shared);
                        if let Ok(mut sockets) = worker_shared.sockets.lock() { sockets.retain(|(key, _)| *key != id); }
                    }) {
                        Ok(worker) => workers.push(worker),
                        Err(_) => {
                            if let Ok(mut sockets) = shared.sockets.lock() { sockets.retain(|(key, _)| *key != id); }
                            shared.state().error = Some("worker_unavailable".into());
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => { shared.state().error = Some("listener_failed".into()); break; }
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
    drop(listener);
    shared.stopped.store(true, Ordering::Release);
    if let Ok(sockets) = shared.sockets.lock() { for (_, socket) in sockets.iter() { let _ = socket.shutdown(Shutdown::Both); } }
    for worker in workers { let _ = worker.join(); }
    revoke(&mut shared.state());
}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn random_hex(bytes: usize) -> Result<String, String> {
    let mut data = vec![0; bytes];
    SystemRandom::new().fill(&mut data).map_err(|_| "Secure randomness unavailable")?;
    Ok(hex(&data))
}

fn generate_identity(ip: Ipv4Addr) -> Result<rcgen::CertifiedKey<rcgen::KeyPair>, String> {
    let mut params = rcgen::CertificateParams::new(vec![ip.to_string()]).map_err(|_| "Cannot create TLS parameters")?;
    let now = SystemTime::now();
    params.not_before = (now - Duration::from_secs(300)).into();
    params.not_after = (now + Duration::from_secs(24 * 60 * 60)).into();
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    let signing_key = rcgen::KeyPair::generate().map_err(|_| "Cannot generate TLS key")?;
    let cert = params.self_signed(&signing_key).map_err(|_| "Cannot sign TLS identity")?;
    Ok(rcgen::CertifiedKey { cert, signing_key })
}

#[allow(deprecated)]
fn token_eq(expected: &str, actual: &str) -> bool {
    actual.len() == 64 && actual.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        && ring::constant_time::verify_slices_are_equal(expected.as_bytes(), actual.as_bytes()).is_ok()
}

fn validate_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() || name.encode_utf16().count() > 120 || name.ends_with(['.', ' '])
        || name.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || name == "." || name == ".." { return Err("invalid_name"); }
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ').to_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || ["COM", "LPT"].iter().any(|prefix| stem.strip_prefix(prefix).is_some_and(|s| matches!(s, "1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"¹"|"²"|"³"))) { return Err("invalid_name"); }
    Ok(())
}

struct ReceiveRoot { path: PathBuf, _guards: Vec<File> }

impl ReceiveRoot {
    fn prepare(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Absolute receive path required")); }
        let mut current = PathBuf::new();
        let mut guards = Vec::new();
        for part in path.components() {
            match part {
                Component::Prefix(_) | Component::RootDir => current.push(part.as_os_str()),
                Component::Normal(name) => {
                    current.push(name);
                    if !current.try_exists()? { fs::create_dir(&current)?; }
                    guards.push(directory_guard(&current)?);
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsafe receive path")),
            }
        }
        guards.push(directory_guard(&current)?);
        Ok(Self { path: current, _guards: guards })
    }
}

fn directory_guard(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new().read(true).share_mode(3)
        .custom_flags(0x02000000 | 0x00200000).open(path)?; // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
    let meta = file.metadata()?;
    if !meta.is_dir() || meta.file_attributes() & 0x400 != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Reparse directories are not allowed"));
    }
    Ok(file)
}

/// Enumerates adapters locally; never sends a UDP probe or contacts a service.
pub fn private_ipv4_addresses() -> Result<Vec<Ipv4Addr>, String> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    // u64 storage guarantees the alignment required by adapter structures.
    let mut size = 15_000u32;
    for _ in 0..4 {
        let mut storage = vec![0u64; (size as usize + 7) / 8];
        let ptr = storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe { GetAdaptersAddresses(AF_INET.0 as u32,
            GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
            None, Some(ptr), &mut size) };
        if result == 111 && size <= 1024 * 1024 { continue; } // ERROR_BUFFER_OVERFLOW
        if result != 0 { return Err("Cannot enumerate local IPv4 adapters".into()); }
        let mut addresses = Vec::new();
        let mut adapter = ptr;
        unsafe {
            while let Some(item) = adapter.as_ref() {
                if item.OperStatus == IfOperStatusUp {
                    let mut unicast = item.FirstUnicastAddress;
                    while let Some(entry) = unicast.as_ref() {
                        if entry.Address.iSockaddrLength as usize >= std::mem::size_of::<SOCKADDR_IN>() && !entry.Address.lpSockaddr.is_null() {
                            let socket = &*entry.Address.lpSockaddr.cast::<SOCKADDR_IN>();
                            if socket.sin_family == AF_INET {
                                let octets = socket.sin_addr.S_un.S_un_b;
                                let ip = Ipv4Addr::new(octets.s_b1, octets.s_b2, octets.s_b3, octets.s_b4);
                                if ip.is_private() { addresses.push(ip); }
                            }
                        }
                        unicast = entry.Next;
                    }
                }
                adapter = item.Next;
            }
        }
        addresses.sort_unstable(); addresses.dedup();
        return Ok(addresses);
    }
    Err("Adapter enumeration changed repeatedly".into())
}

#[cfg(test)]
fn test_shared() -> Arc<Shared> {
    Arc::new(Shared {
        state: Mutex::new(State { qr: "synthetic-pairing-uri".into(), address: "127.0.0.1:1".into(),
            secret: "a".repeat(64), secret_deadline: Instant::now() + PAIR_TTL,
            pair: None, token: None, incoming: None, outgoing: None, error: None }),
        stopped: AtomicBool::new(false), sockets: Mutex::new(Vec::new()),
        root: ReceiveRoot { path: PathBuf::new(), _guards: Vec::new() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(shared: Arc<Shared>) -> PhoneService {
        let cached = Mutex::new(snapshot_state(&shared.state(), false));
        PhoneService { shared, worker: None, cached, certificate: Vec::new() }
    }

    #[test]
    fn remote_names_never_become_paths_or_windows_device_names() {
        for bad in ["", ".", "..", "a/b", "a\\b", "a:b", "name.", "name ", "NUL.txt", "COM1.foo",
            "com¹.txt", "LPT²", "lpt³.bin", "CON", "CONIN$", "AUX.any", "a\n.txt", "x?y", "a|b"] {
            assert!(validate_name(bad).is_err(), "{bad:?}");
        }
        for good in ["foto estate.jpg", "caffè.txt", "写真.png", "COM10.txt", "hello", "😀.txt"] {
            assert!(validate_name(good).is_ok(), "{good:?}");
        }
        assert!(validate_name(&"😀".repeat(61)).is_err());
    }

    #[test]
    fn bearer_comparison_rejects_case_length_and_wrong_value() {
        let token = "a".repeat(64);
        assert!(token_eq(&token, &token));
        for bad in ["a".repeat(63), "a".repeat(65), "A".repeat(64), "b".repeat(64), "g".repeat(64)] {
            assert!(!token_eq(&token, &bad));
        }
    }

    #[test]
    fn snapshot_expiry_and_stopped_state_never_claim_pairing() {
        let shared = test_shared();
        let mut ui = service(Arc::clone(&shared));
        assert!(ui.snapshot().pairing_available);
        shared.state().secret_deadline = Instant::now();
        let snapshot = ui.snapshot();
        assert!(!snapshot.pairing_available && snapshot.pairing_uri.is_empty());
        shared.state().token = Some("b".repeat(64));
        assert!(ui.snapshot().paired);
        ui.stop(); ui.stop();
        assert!(!ui.snapshot().paired);
        assert!(!shared.authorized(&"b".repeat(64)));
    }

    #[test]
    fn approval_requires_exact_live_pending_request() {
        let shared = test_shared();
        shared.state().pair = Some(PairAttempt { request: PairRequest { id: "request".into(), name: "test".into(), code: "123456".into() },
            poll_token: "c".repeat(64), status: "pending", deadline: Instant::now() + PAIR_TTL });
        let ui = service(Arc::clone(&shared));
        assert!(ui.approve_pair("wrong").is_err());
        shared.state().pair.as_mut().unwrap().deadline = Instant::now();
        assert!(ui.approve_pair("request").is_err());
        assert!(shared.state().token.is_none());
        assert!(ui.snapshot().pending_pair.is_none());
    }

    #[test]
    fn snapshot_and_stop_do_not_wait_for_the_state_mutex() {
        let shared = test_shared();
        let mut ui = service(Arc::clone(&shared));
        let held = shared.state();
        assert!(ui.snapshot().pairing_available);
        ui.stop();
        assert!(!ui.snapshot().pairing_available);
        drop(held);
    }

    #[test]
    fn ip_policy_is_rfc1918_not_generic_lan_or_loopback() {
        for ip in [Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(172, 16, 0, 1), Ipv4Addr::new(192, 168, 0, 1)] { assert!(ip.is_private()); }
        for ip in [Ipv4Addr::LOCALHOST, Ipv4Addr::UNSPECIFIED, Ipv4Addr::new(169,254,1,1), Ipv4Addr::new(8,8,8,8)] {
            assert!(!ip.is_private());
            assert!(PhoneService::start(ip, PathBuf::new()).is_err());
        }
    }
}
