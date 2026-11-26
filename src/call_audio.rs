//! Original NotchLite code. Metadata-only call candidates.
//! Subscription-only release: no PCM acquisition, microphone, STT or network.

mod windows;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const AUDIO_DISABLED_REASON: &str = "Audio AI is disabled in subscription-only mode; no audio is captured";
const MAX_TARGETS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioTarget {
    pub key: u64,
    pub label: String,
    pub pid: u32,
    pub process_started: u64,
    pub session_id: String,
    pub broad_capture: bool,
}
// Stable controller vocabulary for a future, separately authorized provider.
// No path in this release constructs a CaptureStatus or an AudioChunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapturePhase { Starting, Listening, Stopping }
#[derive(Clone, Debug)]
pub struct CaptureStatus {
    pub generation: u64,
    pub key: u64,
    pub label: String,
    pub pid: u32,
    pub state: CapturePhase,
}
#[derive(Clone, Debug, Default)]
pub struct AudioSnapshot {
    pub candidates: Vec<AudioTarget>,
    pub suggested: Option<u64>,
    pub capture: Option<CaptureStatus>,
    pub last_error: Option<String>,
}
#[derive(Debug)]
pub struct AudioChunk { pub generation: u64, pub pcm: Vec<i16>, pub sample_rate: u32 }

#[derive(Default)]
struct State { snapshot: AudioSnapshot, next_key: u64, declined: HashSet<u64> }
impl State {
    fn catalog(&mut self, mut targets: Vec<AudioTarget>, communication_ids: &HashSet<String>) -> Result<(), String> {
        let mut seen = HashSet::new();
        targets.retain(|target| seen.insert((target.pid, target.process_started, target.session_id.clone())));
        targets.truncate(MAX_TARGETS);
        for target in &mut targets {
            target.key = match self.snapshot.candidates.iter().find(|old| same_identity(old, target)) {
                Some(old) => old.key,
                None => { self.next_key = self.next_key.checked_add(1).ok_or("Audio target keys exhausted")?; self.next_key }
            };
        }
        self.declined.retain(|key| targets.iter().any(|t| t.key == *key));
        self.snapshot.suggested = targets.iter().find(|t| communication_ids.contains(&t.session_id)
            && !self.declined.contains(&t.key)).map(|t| t.key);
        self.snapshot.candidates = targets;
        self.snapshot.capture = None;
        Ok(())
    }
    fn decline(&mut self, key: u64) {
        if self.snapshot.candidates.iter().any(|t| t.key == key) { self.declined.insert(key); }
        if self.snapshot.suggested == Some(key) { self.snapshot.suggested = None; }
    }
}
fn same_identity(a: &AudioTarget, b: &AudioTarget) -> bool {
    a.pid == b.pid && a.process_started == b.process_started && a.session_id == b.session_id
}
struct Shared { state: Mutex<State>, stop: AtomicBool, wake: windows::Wake }
impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, State> { self.state.lock().unwrap_or_else(|e| e.into_inner()) }
}
pub struct AudioMonitor { shared: Arc<Shared>, worker: Option<JoinHandle<()>> }
impl AudioMonitor {
    /// Session metadata only: never opens a PCM stream or reads audio samples.
    pub fn start() -> Result<Self, String> {
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), stop: AtomicBool::new(false), wake: windows::Wake::new()? });
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new().name("call-metadata".into()).spawn(move || windows::run(worker_shared, ready_tx))
            .map_err(|_| "Cannot start audio metadata worker")?;
        let monitor = Self { shared, worker: Some(worker) };
        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(monitor), Ok(Err(error)) => Err(error),
            Err(_) => Err("Audio metadata initialization did not complete".into()),
        }
    }
    pub fn snapshot(&self) -> AudioSnapshot { self.shared.state().snapshot.clone() }
    pub fn confirm(&self, _key: u64) -> Result<(), String> { Err(AUDIO_DISABLED_REASON.into()) }
    pub fn decline(&self, key: u64) { self.shared.state().decline(key); }
    pub fn stop_capture(&self) { self.shared.state().snapshot.capture = None; }
    pub fn poll_audio(&self) -> Vec<AudioChunk> { Vec::new() }
}
impl Drop for AudioMonitor {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release); self.shared.wake.signal();
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_millis(900);
            while !worker.is_finished() && Instant::now() < deadline { thread::sleep(Duration::from_millis(5)); }
            if worker.is_finished() { let _ = worker.join(); }
            // An OS COM call may outlive this UI wait. Worker/callback Arcs own
            // their state until unregister/release; never free borrowed memory.
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn target(pid: u32, started: u64, session: &str) -> AudioTarget {
        AudioTarget { key: 0, label: "Synthetic".into(), pid, process_started: started, session_id: session.into(), broad_capture: false }
    }
    #[test]
    fn metadata_never_creates_capture_and_pid_reuse_gets_a_new_key() {
        let mut s = State::default();
        s.catalog(vec![target(5, 10, "a")], &HashSet::from(["a".into()])).unwrap();
        let old = s.snapshot.suggested.unwrap(); assert!(s.snapshot.capture.is_none());
        s.catalog(vec![target(5, 11, "a")], &HashSet::new()).unwrap();
        assert_ne!(s.snapshot.candidates[0].key, old); assert!(s.snapshot.suggested.is_none());
        s.catalog(vec![], &HashSet::new()).unwrap(); assert!(s.snapshot.capture.is_none());
    }
    #[test]
    fn declined_candidates_stay_declined_only_for_that_identity() {
        let mut s = State::default(); let ids = HashSet::from(["a".into()]);
        s.catalog(vec![target(5, 10, "a")], &ids).unwrap(); let key = s.snapshot.suggested.unwrap();
        s.decline(key); s.catalog(vec![target(5, 10, "a")], &ids).unwrap(); assert!(s.snapshot.suggested.is_none());
        s.catalog(vec![target(5, 11, "a")], &ids).unwrap(); assert!(s.snapshot.suggested.is_some());
        s.decline(u64::MAX); assert!(s.declined.is_empty());
    }
    #[test]
    fn unknown_and_music_sessions_are_not_suggested() {
        let mut s = State::default();
        s.catalog(vec![target(5, 10, "music")], &HashSet::from(["another-instance".into()])).unwrap();
        assert!(s.snapshot.suggested.is_none()); assert!(s.snapshot.capture.is_none());
    }
}
