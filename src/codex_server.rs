// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Explicit, bounded launcher for the official persistent local Codex daemon.
//! Closing the panel never stops a daemon or its conversations.
use std::io::Read;
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::{Foundation::HANDLE, System::Pipes::PeekNamedPipe};

pub struct StartWorker {
    result: mpsc::Receiver<Result<PathBuf, String>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl StartWorker {
    pub fn start(executable: Option<PathBuf>) -> Result<Self, String> {
        let (sender, result) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancel);
        let worker = thread::Builder::new().name("codex-server-start".into()).spawn(move || {
            let result = start_daemon(executable, &stop);
            let _ = sender.try_send(result);
        }).map_err(|_| "Impossibile avviare il controllo del server Codex".to_string())?;
        Ok(Self { result, cancel, worker: Some(worker) })
    }
    pub fn poll(&self) -> Option<Result<PathBuf, String>> { self.result.try_recv().ok() }
}
impl Drop for StartWorker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}

fn valid_start_reply(bytes: &[u8], expected: &Path) -> Result<(), String> {
    if bytes.len() > 65536 { return Err("Risposta del launcher Codex troppo grande".into()); }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "Il launcher Codex non ha restituito un risultato JSON valido".to_string())?;
    if !matches!(value["status"].as_str(), Some("started" | "alreadyRunning"))
        || value["backend"] != "pid"
        || value["cliVersion"].as_str().is_none_or(str::is_empty)
        || value["appServerVersion"].as_str().is_none_or(str::is_empty) {
        return Err("Risposta inattesa dal launcher Codex".into());
    }
    let socket = value["socketPath"].as_str().filter(|s| s.len() <= 2048)
        .ok_or("Socket non restituito dal launcher Codex")?;
    // Compare components only; do not probe a path supplied by stdout (UNC etc.).
    // Windows components normalize duplicate separators in official CLI output.
    if Path::new(socket) != expected { return Err("Il launcher ha restituito un socket diverso da quello locale atteso".into()); }
    Ok(())
}

fn start_daemon(executable: Option<PathBuf>, cancel: &AtomicBool) -> Result<PathBuf, String> {
    let executable = executable.map(Ok).unwrap_or_else(crate::codex_bridge::cli_executable)?;
    if !executable.is_absolute() || !executable.is_file() {
        return Err("Indica un percorso assoluto valido per il CLI Codex ufficiale".into());
    }
    let mut child = Command::new(executable).args(["app-server", "daemon", "start"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .creation_flags(0x0800_0000).spawn()
        .map_err(|_| "Impossibile eseguire il launcher Codex ufficiale".to_string())?;
    let result = (|| {
        let mut stdout = child.stdout.take().ok_or("Pipe del launcher non disponibile")?;
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut bytes = Vec::new();
        loop {
            if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
                return Err("Avvio interrotto o scaduto. Verifica il collegamento: il daemon potrebbe essere già partito; nessuna chat viene arrestata.".into());
            }
            let mut available = 0;
            let open = unsafe { PeekNamedPipe(HANDLE(stdout.as_raw_handle()), None, 0, None, Some(&mut available), None) }.is_ok();
            if open && available > 0 {
                if bytes.len().saturating_add(available as usize) > 65536 { return Err("Risposta del launcher Codex troppo grande".into()); }
                let mut buffer = [0u8; 4096];
                let amount = stdout.read(&mut buffer[..(available as usize).min(4096)])
                    .map_err(|_| "Lettura del launcher Codex interrotta".to_string())?;
                bytes.extend_from_slice(&buffer[..amount]);
                continue;
            }
            if let Some(status) = child.try_wait().map_err(|_| "Stato del launcher Codex non disponibile")? {
                if !status.success() {
                    return Err("Avvio non riuscito. Serve il pacchetto CLI ufficiale completo; il solo eseguibile incluso in Desktop può non bastare. Vedi CODEX-SHARED-SERVER.md.".into());
                }
                // Recheck the pipe after exit: the final write can race the first peek.
                let mut remaining = 0;
                if unsafe { PeekNamedPipe(HANDLE(stdout.as_raw_handle()), None, 0, None, Some(&mut remaining), None) }.is_ok() && remaining > 0 { continue; }
                let socket = crate::codex_bridge::shared_socket_path()?;
                valid_start_reply(&bytes, &socket)?;
                return Ok(socket);
            }
            // A closed pipe can precede process exit by a scheduling interval.
            thread::sleep(Duration::from_millis(20));
        }
    })();
    // Only the launcher is owned. Never issue daemon stop/restart on failure or Drop.
    if child.try_wait().ok().flatten().is_none() { let _ = child.kill(); }
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_reply_must_be_one_bounded_json_object() {
        let expected = Path::new(r"C:\Users\synthetic\control.sock");
        let reply = serde_json::json!({"status":"started","backend":"pid","cliVersion":"test","appServerVersion":"test","socketPath":r"C:\\Users\\synthetic\\control.sock"});
        assert!(valid_start_reply(reply.to_string().as_bytes(), expected).is_ok());
        assert!(valid_start_reply(reply.to_string().as_bytes(), Path::new(r"C:\other.sock")).is_err());
        for bytes in [b"".as_slice(), b"started", b"[]", b"{}\n{}"] {
            assert!(valid_start_reply(bytes, expected).is_err());
        }
        assert!(valid_start_reply(b"{}", expected).is_err());
        assert!(valid_start_reply(&vec![b' '; 65537], expected).is_err());
    }
}
