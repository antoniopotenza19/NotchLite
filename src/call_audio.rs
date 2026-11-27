//! Original NotchLite code. Explicit-consent process loopback, never microphone.
//! PCM stays in a bounded memory queue; STT/network belong to a separate controller.

mod windows;
mod capture;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_TARGETS: usize = 128;
const SAMPLE_RATE: u32 = 16_000;
const CHUNK_SAMPLES: usize = 1_600;
const MAX_CHUNKS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioTarget {
    pub key: u64,
    pub label: String,
    pub pid: u32,
    pub process_started: u64,
    pub session_id: String,
    pub broad_capture: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapturePhase { Starting, Listening, Stopping }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputSignal { #[default] Waiting, Receiving, Silent, NoPackets }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputHealth { pub packets:u64, pub samples:u64, pub peak:u16, pub signal:InputSignal }
impl InputHealth {
    pub fn warning(self)->Option<&'static str> {match self.signal {
        InputSignal::NoPackets=>Some("Nessun audio ricevuto dall'app selezionata. Verifica la sorgente Teams nei controlli; Stop e nuova conferma per cambiarla."),
        InputSignal::Silent=>Some("Audio ricevuto ma silenzioso da 12 secondi. Può essere una pausa; se l'altra persona parla, verifica sorgente e uscita audio Teams."),
        _=>None,
    }}
}
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
    pub input_health: InputHealth,
    pub teams_render_sessions: usize,
}
#[derive(Debug)]
pub struct AudioChunk { pub generation: u64, pub pcm: Vec<i16>, pub sample_rate: u32 }

#[derive(Default)]
struct State {
    snapshot: AudioSnapshot, next_key: u64, declined: HashSet<u64>,
    generation: u64, pending: Option<AudioTarget>,
    pcm: VecDeque<AudioChunk>, partial: Vec<i16>,
}
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
        // Ignore belongs to this communications occurrence, not indefinitely to
        // a browser/Teams process that keeps the same active render session.
        self.declined.retain(|key| targets.iter().any(|t| t.key == *key && communication_ids.contains(&t.session_id)));
        self.snapshot.suggested = targets.iter().find(|t| communication_ids.contains(&t.session_id)
            && !self.declined.contains(&t.key)).map(|t| t.key);
        self.snapshot.candidates = targets;
        Ok(())
    }
    fn decline(&mut self, key: u64) {
        if self.snapshot.candidates.iter().any(|t| t.key == key) { self.declined.insert(key); }
        if self.snapshot.suggested == Some(key) { self.snapshot.suggested = None; }
    }
    fn begin_capture(&mut self, key: u64) -> Result<u64, String> {
        if self.snapshot.capture.is_some() { return Err("Stop the current capture before starting another".into()); }
        let target = self.snapshot.candidates.iter().find(|t| t.key == key).cloned()
            .ok_or("The selected audio session is no longer available")?;
        self.generation = self.generation.checked_add(1).ok_or("Audio generations exhausted")?;
        self.pcm.clear(); self.partial.clear(); self.snapshot.last_error = None;
        self.snapshot.input_health=InputHealth::default();
        self.snapshot.capture = Some(CaptureStatus { generation: self.generation, key,
            label: target.label.clone(), pid: target.pid, state: CapturePhase::Starting });
        self.pending = Some(target);
        Ok(self.generation)
    }
    fn request_stop(&mut self, error: Option<String>) {
        if let Some(status) = &mut self.snapshot.capture {
            if status.state != CapturePhase::Stopping { self.generation = self.generation.saturating_add(1); }
            status.state = CapturePhase::Stopping;
        }
        self.pending = None; self.pcm.clear(); self.partial.clear();
        self.snapshot.input_health=InputHealth::default();
        if let Some(error) = error { self.snapshot.last_error = Some(error); }
    }
    fn finish_capture(&mut self, generation: u64) {
        if self.snapshot.capture.as_ref().is_some_and(|s| s.generation == generation) {
            self.snapshot.capture = None; self.pending = None; self.pcm.clear(); self.partial.clear();
            self.snapshot.input_health=InputHealth::default();
        }
    }
    fn listening(&mut self, generation: u64) -> bool {
        if self.generation != generation { return false; }
        let Some(status) = &mut self.snapshot.capture else { return false; };
        if status.generation != generation || status.state != CapturePhase::Starting { return false; }
        status.state = CapturePhase::Listening; true
    }
    fn accept_pcm(&mut self, generation: u64, samples: &[i16]) -> Result<(), String> {
        if self.generation != generation || !self.snapshot.capture.as_ref()
            .is_some_and(|s| s.generation == generation && s.state == CapturePhase::Listening) { return Ok(()); }
        let chunks = (self.partial.len() + samples.len()) / CHUNK_SAMPLES;
        if self.pcm.len() + chunks > MAX_CHUNKS {
            self.request_stop(Some("Audio queue overrun: capture stopped and queued samples discarded".into()));
            return Err("Audio queue overrun".into());
        }
        for sample in samples {
            self.partial.push(*sample);
            if self.partial.len() == CHUNK_SAMPLES {
                self.pcm.push_back(AudioChunk { generation, pcm: std::mem::take(&mut self.partial), sample_rate: SAMPLE_RATE });
            }
        }
        Ok(())
    }
}
fn same_identity(a: &AudioTarget, b: &AudioTarget) -> bool {
    a.pid == b.pid && a.process_started == b.process_started && a.session_id == b.session_id
}
struct Shared { state: Mutex<State>, stop: AtomicBool, wanted_generation: AtomicU64, wake: windows::Wake }
impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, State> { self.state.lock().unwrap_or_else(|e| e.into_inner()) }
    fn request_stop(&self, error: Option<String>) {
        // Use confirm's state -> generation order. Publishing zero before this
        // lock could be overwritten by a concurrent confirm, leaving Stopping
        // with a nonzero wanted generation and no pending activation.
        let mut state = self.state();
        self.wanted_generation.store(0, Ordering::Release);
        state.request_stop(error);
        drop(state); self.wake.signal();
    }
}
pub struct AudioMonitor { shared: Arc<Shared>, worker: Option<JoinHandle<()>> }
impl AudioMonitor {
    /// Starts only the metadata monitor. Capture requires a later explicit confirm.
    pub fn start() -> Result<Self, String> {
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), stop: AtomicBool::new(false),
            wanted_generation: AtomicU64::new(0), wake: windows::Wake::new()? });
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
    /// Call only after explicit consent for this exact candidate and a ready provider.
    /// Starting and generation are visible synchronously before Ok is returned.
    pub fn confirm(&self, key: u64) -> Result<(), String> {
        if self.shared.stop.load(Ordering::Acquire) { return Err("Audio monitor is stopping".into()); }
        let mut state = self.shared.state();
        let generation = state.begin_capture(key)?;
        self.shared.wanted_generation.store(generation, Ordering::Release);
        drop(state); self.shared.wake.signal(); Ok(())
    }
    pub fn decline(&self, key: u64) {
        let stop = { let mut state = self.shared.state(); state.decline(key);
            state.snapshot.capture.as_ref().is_some_and(|s| s.key == key) };
        if stop { self.stop_capture(); }
    }
    pub fn stop_capture(&self) { self.shared.request_stop(None); }
    pub fn poll_audio(&self) -> Vec<AudioChunk> {
        let mut state = self.shared.state();
        let wanted = self.shared.wanted_generation.load(Ordering::Acquire);
        if wanted == 0 || !state.snapshot.capture.as_ref().is_some_and(|s| s.generation == wanted && s.state == CapturePhase::Listening) {
            state.pcm.clear(); return Vec::new();
        }
        state.pcm.drain(..).collect()
    }
}
impl Drop for AudioMonitor {
    fn drop(&mut self) {
        self.stop_capture();
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

/// Opt-in metadata-only check. It has no path that calls confirm or opens audio.
pub fn run_diagnostic()->Result<bool,String> {
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if !args.first().is_some_and(|arg|arg=="--call-audio-check") {return Ok(false);}
    if args.len()!=1 {return Err("Usa --call-audio-check senza altri argomenti".into());}
    let monitor=AudioMonitor::start()?;
    thread::sleep(Duration::from_millis(2200));
    let snapshot=monitor.snapshot();
    // Counts only: no process IDs, endpoint/session identities or window titles.
    println!("{}",serde_json::json!({"activeRenderSessions":snapshot.candidates.len(),
        "teamsRenderSessions":snapshot.teams_render_sessions,
        "hasSuggestedCandidate":snapshot.suggested.is_some(),"metadataError":snapshot.last_error,
        "audioCapture":false,"microphoneOpened":false,"guiOpened":false,
        "detection":"duck-events-or-active-teams-audio-hint","realCallVerified":false}));
    Ok(true)
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
    #[test]
    fn ignored_call_can_be_suggested_again_when_same_session_starts_a_new_call() {
        let mut s=State::default();
        let on=HashSet::from(["persistent-session".into()]);
        let active=target(5,10,"persistent-session");
        s.catalog(vec![active.clone()],&on).unwrap();
        let key=s.snapshot.suggested.unwrap();
        s.decline(key);
        s.catalog(vec![active.clone()],&on).unwrap();
        assert!(s.snapshot.suggested.is_none(),"Ignore remains active for the current call");
        s.catalog(vec![active.clone()],&HashSet::new()).unwrap();
        assert_eq!(s.snapshot.candidates[0].key,key,"The process/session stayed alive");
        assert!(s.snapshot.suggested.is_none()); assert!(s.declined.is_empty());
        s.catalog(vec![active],&on).unwrap();
        assert_eq!(s.snapshot.suggested,Some(key),"A new communications occurrence is suggested again");
        assert!(s.snapshot.capture.is_none(),"A new call never starts capture automatically");
    }
    #[test]
    fn explicit_start_publishes_generation_and_stop_invalidates_pcm() {
        let mut s = State::default(); s.catalog(vec![target(5, 10, "a")], &HashSet::new()).unwrap();
        let key = s.snapshot.candidates[0].key;
        assert!(s.begin_capture(key + 1).is_err()); assert!(s.snapshot.capture.is_none());
        let generation = s.begin_capture(key).unwrap(); assert_eq!(s.snapshot.capture.as_ref().unwrap().state, CapturePhase::Starting);
        assert!(s.begin_capture(key).is_err()); assert!(s.listening(generation));
        s.accept_pcm(generation, &vec![17; CHUNK_SAMPLES]).unwrap(); assert_eq!(s.pcm.len(), 1);
        s.request_stop(None); assert!(s.pcm.is_empty()); assert!(s.partial.is_empty());
        assert_eq!(s.snapshot.capture.as_ref().unwrap().state, CapturePhase::Stopping);
        s.accept_pcm(generation, &vec![99; CHUNK_SAMPLES]).unwrap(); assert!(s.pcm.is_empty());
        s.finish_capture(generation); let next = s.begin_capture(key).unwrap(); assert_ne!(next, generation);
        s.finish_capture(generation); assert_eq!(s.snapshot.capture.as_ref().unwrap().generation, next);
    }
    #[test]
    fn pcm_is_chunked_and_overrun_fails_closed_without_stale_backlog() {
        let mut s = State::default(); s.catalog(vec![target(5, 10, "a")], &HashSet::new()).unwrap();
        let generation = s.begin_capture(s.snapshot.candidates[0].key).unwrap(); assert!(s.listening(generation));
        s.accept_pcm(generation, &[1, 2]).unwrap(); s.accept_pcm(generation, &vec![3; CHUNK_SAMPLES - 2]).unwrap();
        assert_eq!(s.pcm.front().unwrap().pcm[..3], [1, 2, 3]); assert_eq!(s.pcm.front().unwrap().sample_rate, 16000);
        for _ in 1..MAX_CHUNKS { s.accept_pcm(generation, &vec![0; CHUNK_SAMPLES]).unwrap(); }
        assert!(s.accept_pcm(generation, &vec![4; CHUNK_SAMPLES]).is_err());
        assert!(s.pcm.is_empty()); assert!(s.partial.is_empty());
        assert_eq!(s.snapshot.capture.as_ref().unwrap().state, CapturePhase::Stopping);
    }
    #[test]
    fn concurrent_stop_waits_for_confirm_state_before_revoking_generation() {
        // Kernel event only; no COM, audio endpoint or capture is initialized.
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), stop: AtomicBool::new(false),
            wanted_generation: AtomicU64::new(9), wake: windows::Wake::new().unwrap() });
        let mut guard = shared.state();
        guard.catalog(vec![target(5,10,"synthetic")], &HashSet::new()).unwrap();
        let (entered_tx,entered_rx)=std::sync::mpsc::sync_channel(1);
        let (done_tx,done_rx)=std::sync::mpsc::sync_channel(1);
        let other=Arc::clone(&shared);
        let worker=thread::spawn(move|| { entered_tx.send(()).unwrap(); other.request_stop(None); done_tx.send(()).unwrap(); });
        entered_rx.recv().unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(20)).is_err());
        let before=shared.wanted_generation.load(Ordering::Acquire);
        let key=guard.snapshot.candidates[0].key;
        let generation=guard.begin_capture(key).unwrap();
        shared.wanted_generation.store(generation,Ordering::Release);
        drop(guard);
        done_rx.recv_timeout(Duration::from_secs(2)).unwrap(); worker.join().unwrap();
        assert_eq!(before,9,"Stop must not publish outside the confirm state lock");
        assert_eq!(shared.wanted_generation.load(Ordering::Acquire),0);
        let state=shared.state(); assert!(state.pending.is_none());
        assert_eq!(state.snapshot.capture.as_ref().unwrap().state,CapturePhase::Stopping);
    }
}
