//! Original Core Audio catalog and owner MTA for explicitly confirmed loopback.
use super::*;
use std::collections::HashMap;
use std::sync::mpsc::{self, SyncSender};
use ::windows::core::{implement, Interface, PCWSTR, PWSTR, GUID, HRESULT};
use ::windows::Win32::Foundation::{CloseHandle, BOOL, FILETIME, HANDLE, WAIT_TIMEOUT};
use ::windows::Win32::Media::Audio::*;
use ::windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
use ::windows::Win32::System::Threading::*;

const MAX_DEVICES: u32 = 16;
const MAX_ID_UNITS: usize = 1024;
const MAX_EVENTS: usize = 32;

pub(super) struct Wake(HANDLE);
// Kernel events support cross-thread SetEvent; there is one CloseHandle owner.
unsafe impl Send for Wake {}
unsafe impl Sync for Wake {}
impl Wake {
    pub(super) fn new() -> Result<Self, String> { unsafe { CreateEventW(None, false, false, None).map(Self).map_err(|e| e.to_string()) } }
    pub(super) fn signal(&self) { unsafe { let _ = SetEvent(self.0); } }
    pub(super) fn handle(&self) -> HANDLE { self.0 }
    fn wait(&self, ms: u32) { unsafe { let _ = WaitForSingleObject(self.0, ms); } }
}
impl Drop for Wake { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }

enum DuckEvent { Begin(String), End(String) }
struct Signals { shared: Arc<Shared>, dirty: AtomicBool, overflow: AtomicBool, events: SyncSender<DuckEvent> }
impl Signals {
    fn changed(&self) { self.dirty.store(true, Ordering::Release); self.shared.wake.signal(); }
    fn duck(&self, id: &PCWSTR, active: bool) {
        match unsafe { copy_wide(*id) } {
            Some(id) => {
                let event = if active { DuckEvent::Begin(id) } else { DuckEvent::End(id) };
                if self.events.try_send(event).is_err() { self.overflow.store(true, Ordering::Release); }
            }
            None => self.overflow.store(true, Ordering::Release),
        }
        self.changed();
    }
}

#[implement(IAudioVolumeDuckNotification)]
struct DuckCallback { signals: Arc<Signals> }
impl IAudioVolumeDuckNotification_Impl for DuckCallback_Impl {
    fn OnVolumeDuckNotification(&self, id: &PCWSTR, _: u32) -> ::windows::core::Result<()> { self.signals.duck(id, true); Ok(()) }
    fn OnVolumeUnduckNotification(&self, id: &PCWSTR) -> ::windows::core::Result<()> { self.signals.duck(id, false); Ok(()) }
}
#[implement(IAudioSessionNotification)]
struct CreatedCallback { signals: Arc<Signals> }
impl IAudioSessionNotification_Impl for CreatedCallback_Impl {
    fn OnSessionCreated(&self, _: Option<&IAudioSessionControl>) -> ::windows::core::Result<()> { self.signals.changed(); Ok(()) }
}
#[implement(IAudioSessionEvents)]
struct SessionCallback { signals: Arc<Signals> }
impl IAudioSessionEvents_Impl for SessionCallback_Impl {
    fn OnDisplayNameChanged(&self, _: &PCWSTR, _: *const GUID) -> ::windows::core::Result<()> { Ok(()) }
    fn OnIconPathChanged(&self, _: &PCWSTR, _: *const GUID) -> ::windows::core::Result<()> { Ok(()) }
    fn OnSimpleVolumeChanged(&self, _: f32, _: BOOL, _: *const GUID) -> ::windows::core::Result<()> { Ok(()) }
    fn OnChannelVolumeChanged(&self, _: u32, _: *const f32, _: u32, _: *const GUID) -> ::windows::core::Result<()> { Ok(()) }
    fn OnGroupingParamChanged(&self, _: *const GUID, _: *const GUID) -> ::windows::core::Result<()> { Ok(()) }
    fn OnStateChanged(&self, _: AudioSessionState) -> ::windows::core::Result<()> { self.signals.changed(); Ok(()) }
    fn OnSessionDisconnected(&self, _: AudioSessionDisconnectReason) -> ::windows::core::Result<()> { self.signals.changed(); Ok(()) }
}

// OS-returned strings are bounded before copying. Callback strings are borrowed;
// GetId and GetSessionInstanceIdentifier instead return CoTaskMem-owned strings.
unsafe fn copy_wide(value: PCWSTR) -> Option<String> {
    if value.is_null() { return None; }
    for n in 0..MAX_ID_UNITS {
        if *value.as_ptr().add(n) == 0 {
            return String::from_utf16(std::slice::from_raw_parts(value.as_ptr(), n)).ok();
        }
    }
    None
}
unsafe fn owned_wide(value: PWSTR) -> Option<String> {
    let result = copy_wide(PCWSTR(value.as_ptr()));
    CoTaskMemFree(Some(value.as_ptr().cast()));
    result
}

pub(super) struct Process { handle: HANDLE, pub(super) started: u64, label: String, broad: bool }
impl Process {
    pub(super) fn open(pid: u32) -> Option<Self> {
        if pid == 0 { return None; }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid).ok()? };
        let mut result = Self { handle, started: 0, label: format!("Process {pid}"), broad: true };
        let (mut created, mut exited, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user).ok()?; }
        result.started = u64::from(created.dwHighDateTime) << 32 | u64::from(created.dwLowDateTime);
        let mut name = [0u16; 1024]; let mut len = name.len() as u32;
        if unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), PWSTR(name.as_mut_ptr()), &mut len) }.is_ok() {
            let path = String::from_utf16_lossy(&name[..len as usize]);
            result.label = path.rsplit(['\\', '/']).next().unwrap_or("Application").to_string();
            let lower = result.label.to_ascii_lowercase();
            result.broad = ["chrome", "msedge", "firefox", "opera", "brave", "webview", "browser", "electron"]
                .iter().any(|part| lower.contains(part));
        }
        if result.alive() { Some(result) } else { None }
    }
    pub(super) fn alive(&self) -> bool { unsafe { WaitForSingleObject(self.handle, 0) == WAIT_TIMEOUT } }
}
impl Drop for Process { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.handle); } } }

struct Session { control: IAudioSessionControl, callback: IAudioSessionEvents, process: Option<Process>, pid: u32 }
impl Session {
    fn new(control: IAudioSessionControl, signals: Arc<Signals>) -> Option<Self> {
        let ext = control.cast::<IAudioSessionControl2>().ok()?;
        if unsafe { ext.IsSystemSoundsSession() } != HRESULT(1) { return None; } // S_FALSE means not system sounds.
        let mut pid = 0;
        // Generated Result<u32> discards AUDCLNT_S_NO_SINGLE_PROCESS's successful
        // HRESULT. Preserve it here and exclude ambiguous multi-process sessions.
        let status = unsafe { (Interface::vtable(&ext).GetProcessId)(Interface::as_raw(&ext), &mut pid) };
        if status != HRESULT(0) { return None; }
        let process = Process::open(pid);
        let callback: IAudioSessionEvents = SessionCallback { signals }.into();
        unsafe { control.RegisterAudioSessionNotification(&callback).ok()?; }
        Some(Self { control, callback, process, pid })
    }
    fn target(&self, id: &str) -> Option<AudioTarget> {
        let process = self.process.as_ref()?;
        if !process.alive() || unsafe { self.control.GetState() }.ok()? != AudioSessionStateActive { return None; }
        Some(AudioTarget { key: 0, label: process.label.clone(), pid: self.pid,
            process_started: process.started, session_id: id.to_string(), broad_capture: process.broad })
    }
}
impl Drop for Session { fn drop(&mut self) { unsafe { let _ = self.control.UnregisterAudioSessionNotification(&self.callback); } } }

struct Device { manager: IAudioSessionManager2, duck: IAudioVolumeDuckNotification, created: IAudioSessionNotification, sessions: HashMap<String, Session> }
impl Device {
    fn new(device: &IMMDevice, signals: &Arc<Signals>) -> ::windows::core::Result<Self> {
        let manager = unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)? };
        let duck: IAudioVolumeDuckNotification = DuckCallback { signals: Arc::clone(signals) }.into();
        let created: IAudioSessionNotification = CreatedCallback { signals: Arc::clone(signals) }.into();
        unsafe { manager.RegisterDuckNotification(PCWSTR::null(), &duck)?; }
        if let Err(error) = unsafe { manager.RegisterSessionNotification(&created) } {
            unsafe { let _ = manager.UnregisterDuckNotification(&duck); } return Err(error);
        }
        Ok(Self { manager, duck, created, sessions: HashMap::new() })
    }
    fn refresh(&mut self, signals: &Arc<Signals>, remaining: &mut usize, registered: &mut usize) -> ::windows::core::Result<Vec<AudioTarget>> {
        let list = unsafe { self.manager.GetSessionEnumerator()? };
        // GetCount after registration is required to enable new-session events.
        let count = unsafe { list.GetCount()? }.clamp(0, MAX_TARGETS as i32);
        let mut alive = HashSet::new(); let mut result = Vec::new();
        for i in 0..count {
            if *remaining == 0 { break; }
            let control = match unsafe { list.GetSession(i) } { Ok(c) => c, Err(_) => continue };
            let ext = match control.cast::<IAudioSessionControl2>() { Ok(c) => c, Err(_) => continue };
            let id = match unsafe { ext.GetSessionInstanceIdentifier().ok().and_then(|s| owned_wide(s)) } { Some(s) => s, None => continue };
            if !self.sessions.contains_key(&id) && *registered < MAX_TARGETS {
                if let Some(session) = Session::new(control, Arc::clone(signals)) {
                    self.sessions.insert(id.clone(), session); *registered += 1;
                }
            }
            if let Some(session) = self.sessions.get(&id) {
                *remaining -= 1; alive.insert(id.clone());
                if let Some(target) = session.target(&id) { result.push(target); }
            }
        }
        let before = self.sessions.len();
        self.sessions.retain(|id, _| alive.contains(id));
        *registered -= before - self.sessions.len();
        Ok(result)
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        self.sessions.clear();
        unsafe {
            let _ = self.manager.UnregisterSessionNotification(&self.created);
            let _ = self.manager.UnregisterDuckNotification(&self.duck);
        }
    }
}
struct Com;
impl Drop for Com { fn drop(&mut self) { unsafe { CoUninitialize(); } } }

pub(super) fn run(shared: Arc<Shared>, ready: SyncSender<Result<(), String>>) {
    let result = (|| {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?; }
        let _com = Com;
        let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel(MAX_EVENTS);
        let signals = Arc::new(Signals { shared: Arc::clone(&shared), dirty: AtomicBool::new(true), overflow: AtomicBool::new(false), events: tx });
        let mut devices: HashMap<String, Device> = HashMap::new(); let mut communication_ids = HashMap::new();
        let mut capture: Option<capture::Capture> = None;
        let _ = ready.try_send(Ok(()));
        let mut last = Instant::now() - Duration::from_secs(5);
        while !shared.stop.load(Ordering::Acquire) {
            // Bound work as well as storage: callbacks may keep refilling this
            // queue, so a continuously busy producer must not starve stop/refresh.
            for _ in 0..MAX_EVENTS {
                let Ok(event) = rx.try_recv() else { break; };
                match event {
                    DuckEvent::Begin(id) if communication_ids.len() < MAX_TARGETS => { communication_ids.insert(id, Instant::now()); }
                    DuckEvent::End(id) => {
                        communication_ids.remove(&id);
                        if capture.as_ref().is_some_and(|c| c.target.session_id == id) {
                            shared.request_stop(Some("The selected communications session ended".into()));
                        }
                    }
                    _ => {}
                }
            }
            if signals.overflow.swap(false, Ordering::AcqRel) {
                communication_ids.clear();
                shared.request_stop(Some("Audio metadata events overflowed; capture stopped".into()));
            }
            if signals.dirty.swap(false, Ordering::AcqRel) || last.elapsed() >= Duration::from_secs(2) {
                last = Instant::now();
                match refresh(&enumerator, &mut devices, &signals) {
                    Ok(targets) => {
                        retain_communications(&mut communication_ids, &targets, Instant::now());
                        let ids = communication_ids.keys().cloned().collect();
                        shared.state().catalog(targets, &ids)?;
                    }
                    Err(error) => {
                        devices.clear(); communication_ids.clear();
                        let mut state = shared.state(); state.catalog(Vec::new(), &HashSet::new())?;
                        state.snapshot.last_error = Some(format!("Audio metadata unavailable: {error}"));
                    }
                }
            }
            service_capture(&shared, &devices, &mut capture);
            let busy = capture.is_some() || shared.state().snapshot.capture.is_some();
            shared.wake.wait(if busy { 20 } else { 1000 });
        }
        shared.request_stop(None);
        if let Some(current) = capture.take() {
            let generation = current.generation; drop(current); shared.state().finish_capture(generation);
        }
        // Unregister and release on this MTA, never within an audio callback.
        devices.clear();
        Ok::<(), String>(())
    })();
    if let Err(error) = result { let _ = ready.try_send(Err(error.clone())); shared.state().snapshot.last_error = Some(error); }
    shared.wanted_generation.store(0, Ordering::Release);
    let mut state = shared.state(); state.snapshot.candidates.clear(); state.snapshot.suggested = None; state.snapshot.capture = None;
    state.pending = None; state.pcm.clear(); state.partial.clear();
}

fn selected_live(devices: &HashMap<String, Device>, target: &AudioTarget) -> bool {
    devices.values().any(|device| device.sessions.get(&target.session_id)
        .and_then(|session| session.target(&target.session_id))
        .is_some_and(|current| same_identity(&current, target)))
}

fn service_capture(shared: &Arc<Shared>, devices: &HashMap<String, Device>, active: &mut Option<capture::Capture>) {
    let wanted = shared.wanted_generation.load(Ordering::Acquire);
    if active.as_ref().is_some_and(|capture| capture.generation != wanted) {
        if let Some(capture) = active.take() {
            let generation = capture.generation; drop(capture); shared.state().finish_capture(generation);
        }
    }
    if wanted == 0 {
        let generation = shared.state().snapshot.capture.as_ref().map(|s| s.generation);
        if let Some(generation) = generation { shared.state().finish_capture(generation); }
        return;
    }
    if active.is_none() {
        let pending = shared.state().pending.take();
        if let Some(target) = pending {
            let result = if selected_live(devices, &target) {
                capture::Capture::begin(shared, target, wanted)
            } else { Err("The selected audio session ended or changed before capture".into()) };
            match result {
                Ok(capture) => *active = Some(capture),
                Err(error) => { shared.request_stop(Some(error)); shared.state().finish_capture(wanted); return; }
            }
        }
    }
    if let Some(capture) = active.as_mut() {
        let result = if selected_live(devices, &capture.target) { capture.poll(shared) }
            else { Err("The selected audio session ended or changed; capture stopped".into()) };
        if let Err(error) = result { shared.request_stop(Some(error)); }
    }
    if shared.wanted_generation.load(Ordering::Acquire) != wanted {
        if let Some(capture) = active.take() {
            let generation = capture.generation; drop(capture); shared.state().finish_capture(generation);
        }
    }
}

fn refresh(enumerator: &IMMDeviceEnumerator, devices: &mut HashMap<String, Device>, signals: &Arc<Signals>) -> Result<Vec<AudioTarget>, String> {
    // eRender endpoint metadata only: no capture endpoint or audio client opened.
    let list = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }.map_err(|e| e.to_string())?;
    let count = unsafe { list.GetCount() }.map_err(|e| e.to_string())?.min(MAX_DEVICES);
    let mut live = HashSet::new(); let mut endpoints = Vec::new();
    for i in 0..count {
        let endpoint = unsafe { list.Item(i) }.map_err(|e| e.to_string())?;
        let id = unsafe { endpoint.GetId().ok().and_then(|s| owned_wide(s)) }.ok_or("Invalid audio device identity")?;
        live.insert(id.clone()); endpoints.push((id, endpoint));
    }
    // Retire removed endpoints before registering replacements: the registration
    // budget is a real bound, not only a bound on the published snapshot.
    devices.retain(|id, _| live.contains(id));
    let mut targets = Vec::new(); let mut remaining = MAX_TARGETS;
    let mut registered = devices.values().map(|d| d.sessions.len()).sum();
    for (id, endpoint) in endpoints {
        if signals.shared.stop.load(Ordering::Acquire) { break; }
        if !devices.contains_key(&id) {
            match Device::new(&endpoint, signals) { Ok(device) => { devices.insert(id.clone(), device); }, Err(_) => continue }
        }
        if let Some(device) = devices.get_mut(&id) { targets.extend(device.refresh(signals, &mut remaining, &mut registered).map_err(|e| e.to_string())?); }
    }
    if count != 0 && devices.is_empty() && !signals.shared.stop.load(Ordering::Acquire) {
        return Err("Core Audio session notifications are unavailable".into());
    }
    Ok(targets)
}

fn retain_communications(ids: &mut HashMap<String, Instant>, targets: &[AudioTarget], now: Instant) {
    // Duck notification can precede the stream becoming Active. Retain a bounded
    // grace period for exact-ID correlation, never infer a call from another PID.
    ids.retain(|id, seen| {
        if targets.iter().any(|t| t.session_id == *id) { *seen = now; true }
        else { now.saturating_duration_since(*seen) < Duration::from_secs(10) }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn early_duck_is_not_lost_but_unknown_ids_expire() {
        let start = Instant::now(); let mut ids = HashMap::from([("instance".into(), start)]);
        retain_communications(&mut ids, &[], start + Duration::from_secs(1)); assert_eq!(ids.len(), 1);
        let target = AudioTarget { key: 1, label: "Fake".into(), pid: 1, process_started: 1, session_id: "instance".into(), broad_capture: false };
        retain_communications(&mut ids, &[target], start + Duration::from_secs(9)); assert_eq!(ids.len(), 1);
        retain_communications(&mut ids, &[], start + Duration::from_secs(20)); assert!(ids.is_empty());
    }
}
