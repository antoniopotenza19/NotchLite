// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Local Codex dashboard client. Thread creation is explicit and sends no user turn.
mod protocol;
mod transport;
pub(crate) use transport::RpcTransport;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connection { Connecting, Connected, Unavailable(String) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status { Idle, Active, WaitingInput, WaitingApproval, Unavailable }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountState { Checking, ChatGpt { plan: String }, Unavailable(String) }
impl AccountState { pub fn is_ready(&self) -> bool { matches!(self, Self::ChatGpt { .. }) } }

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestId { Number(i64), Text(String) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerKey {
    pub epoch: u64, pub request_id: RequestId, pub thread_id: String, pub turn_id: String,
}

#[derive(Clone, Debug)]
pub struct QuestionOption { pub label: String, pub description: String }
#[derive(Clone, Debug)]
pub struct Question {
    pub id: String, pub header: String, pub text: String, pub options: Vec<QuestionOption>,
    pub is_secret: bool, pub allows_other: bool,
}
#[derive(Clone, Debug)]
pub struct QuestionRequest {
    pub key: AnswerKey,
    pub item_id: String, pub questions: Vec<Question>, pub is_blocking: bool,
}
#[derive(Clone, Debug)]
pub struct Approval {
    pub key: AnswerKey, pub kind: String,
    pub label: String, pub can_deny: bool,
}
#[derive(Clone, Debug, Default)]
pub struct TokenUsage {
    pub input: i64, pub cached_input: i64, pub output: i64,
    pub reasoning_output: i64, pub total: i64, pub context_window: Option<i64>,
}
#[derive(Clone, Debug)]
pub struct QuotaWindow {
    pub bucket: String, pub label: String, pub used_percent: i64,
    pub resets_at: Option<i64>, pub window_duration_mins: Option<i64>,
}
#[derive(Clone, Debug, Default)]
pub struct Quota { pub windows: Vec<QuotaWindow> }
#[derive(Clone, Debug)]
pub struct RpcError { pub method: &'static str, pub code: Option<i64>, pub reason: &'static str }
#[derive(Clone, Debug)]
pub struct ThreadView {
    pub id: String, pub title: String, pub status: Status, pub last_message: String,
    pub subscription_eligible: bool,
    pub subscription_ready: bool, pub subscription_error: Option<RpcError>,
    pub questions: Vec<QuestionRequest>, pub approvals: Vec<Approval>,
    pub token_usage: Option<TokenUsage>, pub completed: bool,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub epoch: u64, pub revision: u64, pub connection: Connection, pub threads: Vec<ThreadView>,
    pub account: AccountState, pub creating_thread: bool, pub created_thread_id: Option<String>,
    pub quota: Option<Quota>, pub last_error: Option<String>, pub server_info: String,
    pub last_rpc_error: Option<RpcError>,
}
impl Snapshot {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self { epoch: 0, revision: 0, connection: Connection::Unavailable(reason.into()), threads: Vec::new(),
            account: AccountState::Checking, creating_thread: false, created_thread_id: None,
            quota: None, last_error: None, server_info: String::new(), last_rpc_error: None }
    }
}

pub(super) enum Command {
    Refresh,
    Answer(AnswerKey, BTreeMap<String, Vec<String>>),
    FollowUp(String, String),
    Deny(AnswerKey),
    CreateThread(String),
}

fn next_epoch() -> u64 {
    static NEXT_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// An owned CLI proxy and bounded I/O, created only when the panel requests it.
pub struct Bridge {
    snapshot: Arc<Mutex<Snapshot>>,
    commands: mpsc::SyncSender<Command>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    child: Arc<Mutex<Option<std::process::Child>>>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Bridge {
    pub fn start(socket: Option<PathBuf>) -> Result<Self, String> {
        Self::start_inner(socket, None)
    }
    pub fn start_with_cli(socket: Option<PathBuf>, executable: PathBuf) -> Result<Self, String> {
        Self::start_inner(socket, Some(executable))
    }
    fn start_inner(socket: Option<PathBuf>, executable: Option<PathBuf>) -> Result<Self, String> {
        let socket = transport::discover_socket(socket)?;
        let executable = if let Some(path) = executable {
            if !path.is_absolute() || !path.is_file() { return Err("Percorso Codex CLI non valido".into()); }
            path
        } else { transport::discover_cli()? };
        let snapshot = Arc::new(Mutex::new(Snapshot { epoch: next_epoch(), connection: Connection::Connecting,
            ..Snapshot::unavailable("") }));
        let (commands, receiver) = mpsc::sync_channel(16);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let worker = {
            let snapshot = Arc::clone(&snapshot); let stop = Arc::clone(&stop); let child = Arc::clone(&child);
            thread::Builder::new().name("codex-dashboard".into()).spawn(move || {
                transport::run(executable, socket, receiver, snapshot, stop, child);
            }).map_err(|_| "Impossibile avviare il bridge Codex".to_string())?
        };
        Ok(Self { snapshot, commands, stop, child, worker: Some(worker) })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
    pub fn refresh(&self) -> Result<(), String> { self.send(Command::Refresh) }
    pub fn create_thread(&self, cwd: &str) -> Result<(), String> {
        let snapshot=self.snapshot();
        if snapshot.connection!=Connection::Connected || !snapshot.account.is_ready(){return Err("Collega Codex con l'account ChatGPT prima di creare una chat".into());}
        if snapshot.creating_thread{return Err("Creazione chat già in corso".into());}
        if snapshot.threads.len()>=protocol::MAX_THREADS{return Err("Limite di 32 chat caricate nel pannello".into());}
        let cwd=validate_workspace(cwd)?;
        self.send(Command::CreateThread(cwd))
    }
    pub fn answer(&self, key: AnswerKey, answers: BTreeMap<String, Vec<String>>) -> Result<(), String> {
        if answers.len() > 3 || answers.values().flatten().map(String::len).sum::<usize>() > 4096
            || answers.values().any(|a| a.len() > 32) {
            return Err("Risposta troppo grande".into());
        }
        let snapshot=self.snapshot();
        if !snapshot.account.is_ready() || !snapshot.threads.iter().filter(|t|t.subscription_eligible).flat_map(|t| &t.questions).any(|q| q.key == key) {
            return Err("Domanda scaduta o appartenente a una connessione precedente".into());
        }
        self.send(Command::Answer(key, answers))
    }
    pub fn follow_up(&self, thread_id: &str, text: &str) -> Result<(), String> {
        if text.trim().is_empty() || text.len() > 4096 { return Err("Inserisci un messaggio entro 4096 byte".into()); }
        let snapshot=self.snapshot();
        if !snapshot.account.is_ready() || !snapshot.threads.iter().any(|t| t.id == thread_id && t.status == Status::Idle && t.subscription_eligible && t.subscription_ready) {
            return Err("La chat deve essere inattiva prima di inviare".into());
        }
        self.send(Command::FollowUp(thread_id.into(), text.into()))
    }
    pub fn deny_approval(&self, key: AnswerKey) -> Result<(), String> {
        let snapshot=self.snapshot();
        if !snapshot.account.is_ready() || !snapshot.threads.iter().filter(|t|t.subscription_eligible).flat_map(|t| &t.approvals).any(|a| a.key == key && a.can_deny) {
            return Err("Richiesta scaduta o non gestibile dal pannello".into());
        }
        self.send(Command::Deny(key))
    }
    fn send(&self, command: Command) -> Result<(), String> {
        self.commands.try_send(command).map_err(|_| "Bridge Codex occupato o disconnesso".into())
    }
}

pub fn cli_executable() -> Result<PathBuf, String> { transport::discover_cli() }
pub fn shared_socket_path() -> Result<PathBuf, String> { transport::discover_socket(None) }

fn local_drive_root(path: &std::path::Path) -> Result<[u16;4], String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute(){return Err("Scegli una cartella workspace assoluta locale".into());}
    match path.components().next() {
        Some(Component::Prefix(prefix))=>match prefix.kind() {
            Prefix::Disk(drive)|Prefix::VerbatimDisk(drive)=>Ok([drive as u16,b':' as u16,b'\\' as u16,0]),
            _=>Err("Il workspace deve essere su un'unità locale, senza percorsi UNC o dispositivi".into()),
        },
        _=>Err("Workspace locale non valido".into()),
    }
}
fn validate_workspace(cwd: &str) -> Result<String, String> {
    use windows::{core::PCWSTR, Win32::Storage::FileSystem::GetDriveTypeW};
    if cwd.is_empty()||cwd.len()>4096||cwd.contains('\0'){return Err("Percorso workspace non valido".into());}
    let path=std::path::Path::new(cwd);
    let root=local_drive_root(path)?;
    // Win32 drive types: removable, fixed, CD-ROM, RAM disk. Remote/unknown are excluded.
    if !matches!(unsafe{GetDriveTypeW(PCWSTR(root.as_ptr()))},2|3|5|6){return Err("Il workspace deve risiedere su un'unità locale".into());}
    let canonical=std::fs::canonicalize(path).map_err(|_|"Cartella workspace non accessibile".to_string())?;
    let root=local_drive_root(&canonical)?;
    if !matches!(unsafe{GetDriveTypeW(PCWSTR(root.as_ptr()))},2|3|5|6)||!canonical.is_dir(){return Err("Scegli una cartella workspace locale esistente".into());}
    let value=canonical.to_str().ok_or("Il percorso workspace non è rappresentabile")?;
    let value=value.strip_prefix(r"\\?\").unwrap_or(value);
    if value.len()>4096{return Err("Percorso workspace risolto troppo lungo".into());}
    Ok(value.to_owned())
}

/// Metadata only: never starts a proxy, initializes a server, or reads conversation data.
pub fn check_local(socket: Option<PathBuf>) -> Result<PathBuf, String> {
    let socket = if let Some(path)=socket { transport::discover_socket(Some(path))? } else { shared_socket_path()? };
    transport::discover_cli()?;
    Ok(socket)
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(child) = self.child.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_mut() {
            let _ = child.kill();
        }
        // Worker polls every 20ms; killing its owned proxy also releases blocked pipe I/O.
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::*;
    #[test]
    fn workspace_requires_absolute_local_disk_path_before_any_metadata_read() {
        for path in ["relative",r"C:relative",r"\relative",r"\\server.invalid\share\folder",r"\\?\UNC\server.invalid\share\folder",r"\\.\pipe\name"] {
            assert!(local_drive_root(std::path::Path::new(path)).is_err());
        }
        assert_eq!(local_drive_root(std::path::Path::new(r"C:\synthetic")).unwrap(),[67,58,92,0]);
        assert_eq!(local_drive_root(std::path::Path::new(r"\\?\C:\synthetic")).unwrap(),[67,58,92,0]);
        assert!(validate_workspace("").is_err());assert!(validate_workspace("C:\0invalid").is_err());
    }
}
