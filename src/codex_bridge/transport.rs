// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
use super::{protocol::State, *};
use std::io::{self, Read, Write};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command as Process, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tungstenite::{client::client_with_config, handshake::HandshakeError, protocol::WebSocketConfig, Error, Message};
use windows::Win32::{Foundation::HANDLE, System::Pipes::PeekNamedPipe};

const MAX_WIRE: usize = 512 * 1024;
const WRITE_DEADLINE: u64 = 5_000;

fn canonical_local(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() { return Err("Il percorso del socket Codex deve essere assoluto".into()); }
    let raw=path.to_string_lossy();
    if raw.starts_with(r"\\?\UNC\") || raw.starts_with(r"\\")&&!raw.starts_with(r"\\?\") {
        return Err("Sono supportati soltanto socket locali sul PC".into());
    }
    let parent=path.parent().ok_or("Percorso socket non valido")?;
    let parent=std::fs::canonicalize(parent).map_err(|_| "Daemon condiviso non disponibile. Le chat di Codex Desktop su server incorporato non sono collegabili al pannello.".to_string())?;
    let normalized=parent.to_string_lossy();
    if normalized.starts_with(r"\\?\UNC\") {return Err("Sono supportati soltanto socket locali sul PC".into());}
    let normalized=normalized.strip_prefix(r"\\?\").unwrap_or(&normalized);
    if normalized.starts_with(r"\\") { return Err("Sono supportati soltanto socket locali sul PC".into()); }
    Ok(PathBuf::from(normalized).join(path.file_name().ok_or("Nome socket non valido")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct BlockOnce { blocked:bool, bytes:Vec<u8> }
    impl Read for BlockOnce {fn read(&mut self,_:&mut[u8])->io::Result<usize>{Err(io::ErrorKind::WouldBlock.into())}}
    impl Write for BlockOnce {
        fn write(&mut self,input:&[u8])->io::Result<usize>{
            if !self.blocked{self.blocked=true;return Err(io::ErrorKind::WouldBlock.into());}
            self.bytes.extend_from_slice(input);Ok(input.len())
        }
        fn flush(&mut self)->io::Result<()>{Ok(())}
    }
    #[test]
    fn would_block_keeps_one_rpc_frame_for_flush_without_resend(){
        // Our worker removes a JSON-RPC from its outbox on WouldBlock. Verify
        // tungstenite retained it so a later flush sends it exactly once.
        use tungstenite::protocol::{Role,WebSocket};
        let io=BlockOnce{blocked:false,bytes:Vec::new()};
        let config=WebSocketConfig::default().write_buffer_size(0);
        let mut client=WebSocket::from_raw_socket(io,Role::Client,Some(config));
        let result=client.write(Message::Text("{\"id\":42,\"result\":{}}".into()));
        assert!(matches!(result,Err(ref e) if would_block(e)));
        client.flush().unwrap();let bytes=client.get_ref().bytes.clone();
        let mut server=WebSocket::from_raw_socket(io::Cursor::new(bytes),Role::Server,None);
        assert_eq!(server.read().unwrap().into_text().unwrap(),"{\"id\":42,\"result\":{}}");
        assert!(server.read().is_err());
    }
    #[test]
    fn remote_and_relative_socket_paths_are_rejected_before_metadata(){
        assert!(canonical_local(Path::new("relative.sock")).is_err());
        assert!(canonical_local(Path::new(r"\\untrusted.invalid\share\socket")).is_err());
        assert!(canonical_local(Path::new(r"\\?\UNC\untrusted.invalid\share\socket")).is_err());
    }
}
pub(super) fn discover_socket(override_path: Option<PathBuf>) -> Result<PathBuf, String> {
    let candidate=if let Some(path)=override_path {path} else {
        let home=std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| std::env::var_os("USERPROFILE").map(|p|PathBuf::from(p).join(".codex"))).ok_or("Cartella Codex non disponibile")?;
        home.join("app-server-control").join("app-server-control.sock")
    };
    let socket=canonical_local(&candidate)?;
    if socket.to_string_lossy().len()>=108 {return Err("Percorso del socket Codex troppo lungo per AF_UNIX su Windows".into());}
    if std::fs::symlink_metadata(&socket).is_err(){return Err("Daemon Codex condiviso non disponibile: nessun socket locale. Il pannello non avvia un secondo server.".into());}
    Ok(socket)
}
pub(super) fn discover_cli() -> Result<PathBuf,String> {
    // Prefer a complete official package when present: the Desktop bundle's
    // single executable supports proxy, but cannot install/start the daemon.
    let codex_home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(||
        std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join(".codex")));
    if let Some(home) = codex_home.filter(|p| p.is_absolute()) {
        for relative in ["packages/app-server-daemon/current/bin/codex.exe", "packages/standalone/current/bin/codex.exe"] {
            let candidate = home.join(relative);
            if candidate.is_file() { return Ok(candidate); }
        }
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if !directory.is_absolute(){continue;}
        let exe=directory.join("codex.exe");
        if exe.is_file(){return Ok(exe);}
    }
    if let Some(local)=std::env::var_os("LOCALAPPDATA") {
        let alias=PathBuf::from(local).join("Microsoft").join("WindowsApps").join("codex.exe");
        if alias.is_absolute()&&alias.is_file(){return Ok(alias);}
    }
    Err("Codex CLI non trovato nel PATH. È necessario il CLI ufficiale con app-server proxy.".into())
}

struct Progress { queued:AtomicUsize, changed:AtomicU64, began:Instant }
impl Progress { fn now(&self)->u64{self.began.elapsed().as_millis().min(u64::MAX as u128) as u64} }
struct ProxyIo { stdout:ChildStdout, writer:mpsc::SyncSender<Vec<u8>>, progress:Arc<Progress> }
impl Read for ProxyIo {
    fn read(&mut self, output:&mut[u8])->io::Result<usize>{
        if output.is_empty(){return Ok(0);}
        let mut available=0u32;
        // Peek avoids blocking the worker while it must service commands and stop deadlines.
        unsafe{PeekNamedPipe(HANDLE(self.stdout.as_raw_handle()),None,0,None,Some(&mut available),None)}
            .map_err(|_|io::Error::new(io::ErrorKind::UnexpectedEof,"Proxy Codex chiuso"))?;
        if available==0{return Err(io::ErrorKind::WouldBlock.into());}
        let size=output.len().min(available as usize);self.stdout.read(&mut output[..size])
    }
}
impl Write for ProxyIo {
    fn write(&mut self, input:&[u8])->io::Result<usize>{
        if input.is_empty(){return Ok(0);}
        let count=input.len().min(4096);
        if self.progress.queued.fetch_add(1,Ordering::AcqRel)==0{self.progress.changed.store(self.progress.now(),Ordering::Release);}
        match self.writer.try_send(input[..count].to_vec()){
            Ok(())=>Ok(count),
            Err(mpsc::TrySendError::Full(_))=>{self.progress.queued.fetch_sub(1,Ordering::AcqRel);Err(io::ErrorKind::WouldBlock.into())}
            Err(mpsc::TrySendError::Disconnected(_))=>{self.progress.queued.fetch_sub(1,Ordering::AcqRel);Err(io::ErrorKind::BrokenPipe.into())}
        }
    }
    fn flush(&mut self)->io::Result<()>{Ok(())}
}
fn would_block(error:&Error)->bool{matches!(error,Error::Io(e) if e.kind()==io::ErrorKind::WouldBlock)}
fn publish(shared:&Arc<Mutex<Snapshot>>,state:&State){*shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=state.snapshot.clone();}
fn kill(shared:&Arc<Mutex<Option<Child>>>){if let Some(child)=shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_mut(){let _=child.kill();}}

pub(super) fn run(executable:PathBuf,socket:PathBuf,commands:mpsc::Receiver<Command>,shared:Arc<Mutex<Snapshot>>,stop:Arc<AtomicBool>,child:Arc<Mutex<Option<Child>>>) {
    let result=run_inner(executable,socket,commands,&shared,&stop,&child);
    stop.store(true,Ordering::Release);kill(&child);
    if let Some(mut process)=child.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take(){let _=process.wait();}
    if let Err(reason)=result {
        let mut snapshot=shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.connection=Connection::Unavailable(reason);snapshot.threads.clear();snapshot.quota=None;snapshot.revision=snapshot.revision.wrapping_add(1);
    }
}
fn run_inner(executable:PathBuf,socket:PathBuf,commands:mpsc::Receiver<Command>,shared:&Arc<Mutex<Snapshot>>,stop:&Arc<AtomicBool>,child:&Arc<Mutex<Option<Child>>>)->Result<(),String>{
    let mut process=Process::new(executable).args(["app-server","proxy","--sock"]).arg(socket)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).creation_flags(0x0800_0000)
        .spawn().map_err(|_|"Impossibile avviare il proxy ufficiale Codex".to_string())?;
    let stdout=process.stdout.take().ok_or("Pipe Codex non disponibile")?;
    let mut stdin=process.stdin.take().ok_or("Pipe Codex non disponibile")?;
    *child.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=Some(process);
    let progress=Arc::new(Progress{queued:AtomicUsize::new(0),changed:AtomicU64::new(0),began:Instant::now()});
    let (writer,receiver)=mpsc::sync_channel::<Vec<u8>>(4);
    let writer_thread={let progress=Arc::clone(&progress);let stop=Arc::clone(stop);thread::Builder::new().name("codex-proxy-write".into()).spawn(move||{
        while let Ok(bytes)=receiver.recv(){if stop.load(Ordering::Acquire){break;}let result=stdin.write_all(&bytes);progress.queued.fetch_sub(1,Ordering::AcqRel);progress.changed.store(progress.now(),Ordering::Release);if result.is_err(){break;}}
    }).map_err(|_|"Impossibile avviare la pipe Codex".to_string())?};
    let result=(||{
        let io=ProxyIo{stdout,writer,progress:Arc::clone(&progress)};
        let config=WebSocketConfig::default().read_buffer_size(4096).write_buffer_size(0).max_write_buffer_size(65536).max_message_size(Some(MAX_WIRE)).max_frame_size(Some(MAX_WIRE));
        let mut handshake=client_with_config("ws://localhost/",io,Some(config));
        let began=Instant::now();
        let mut websocket=loop{
            if stop.load(Ordering::Acquire){return Ok(());}
            if began.elapsed()>Duration::from_secs(5){return Err("Il server Codex non risponde al collegamento locale".into());}
            match handshake{
                Ok((socket,_))=>break socket,
                Err(HandshakeError::Interrupted(mid))=>{thread::sleep(Duration::from_millis(20));handshake=mid.handshake();}
                Err(HandshakeError::Failure(_))=>return Err("Socket Codex non compatibile con il protocollo WebSocket ufficiale".into()),
            }
        };
        let initial=shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        let mut state=State::new(initial);state.begin();
        let mut metadata=Instant::now();let mut quota=Instant::now();let mut received=Instant::now();let mut revision=u64::MAX;
        while !stop.load(Ordering::Acquire){
            if progress.queued.load(Ordering::Acquire)>0 && progress.now().saturating_sub(progress.changed.load(Ordering::Acquire))>WRITE_DEADLINE{return Err("Scrittura al server Codex scaduta".into());}
            if received.elapsed()>Duration::from_secs(45){return Err("Server Codex non raggiungibile: ricollega il pannello".into());}
            for _ in 0..16{match commands.try_recv(){Ok(command)=>state.command(command),Err(_)=>break,}}
            if metadata.elapsed()>=Duration::from_secs(5){let read_quota=quota.elapsed()>=Duration::from_secs(15);state.refresh(read_quota);metadata=Instant::now();if read_quota{quota=Instant::now();}}
            for _ in 0..16{
                let Some(value)=state.outbox.pop_front() else{break;};
                let encoded=serde_json::to_string(&value).map_err(|_|"Richiesta Codex non valida")?;
                if encoded.len()>32768{state.error("Richiesta troppo grande per il pannello");continue;}
                match websocket.write(Message::Text(encoded.into())){
                    Ok(())=>{},Err(Error::WriteBufferFull(_))=>{state.outbox.push_front(value);break;}
                    Err(e)if would_block(&e)=>break,
                    Err(_)=>return Err("Collegamento Codex interrotto durante l'invio".into()),
                }
            }
            if let Err(e)=websocket.flush(){if !would_block(&e){return Err("Collegamento Codex chiuso".into());}}
            for _ in 0..32{match websocket.read(){
                Ok(Message::Text(text))=>{received=Instant::now();let value:serde_json::Value=serde_json::from_str(text.as_ref()).map_err(|_|"Risposta Codex non valida")?;state.message(value);}
                Ok(Message::Close(_))=>return Err("Server Codex disconnesso".into()),
                Ok(Message::Ping(_))|Ok(Message::Pong(_))=>{received=Instant::now();},
                Ok(_)=>{},Err(e)if would_block(&e)=>break,
                Err(Error::Capacity(_))=>return Err("Evento Codex oltre il limite del pannello (512 KiB)".into()),
                Err(_)=>return Err("Collegamento Codex non disponibile".into()),
            }}
            if revision!=state.snapshot.revision{revision=state.snapshot.revision;publish(shared,&state);}
            thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    })();
    // Closing the child releases a stalled write; no shared daemon is stopped.
    stop.store(true,Ordering::Release);kill(child);let _=writer_thread.join();result
}
