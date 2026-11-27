//! Original Core Audio catalog and owner MTA for explicitly confirmed loopback.
use super::*;
use std::collections::HashMap;
use std::sync::mpsc::{self, SyncSender};
use ::windows::core::{implement, Interface, PCWSTR, PWSTR, GUID, HRESULT};
use ::windows::Win32::Foundation::{CloseHandle, BOOL, FILETIME, HANDLE, WAIT_TIMEOUT};
use ::windows::Win32::Media::Audio::*;
use ::windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
use ::windows::Win32::System::Threading::*;
use ::windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot,Process32FirstW,Process32NextW,PROCESSENTRY32W,TH32CS_SNAPPROCESS};

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

pub(super) struct Process { handle: HANDLE, pub(super) started: u64, label: String, broad: bool, teams_hint:bool }
impl Process {
    pub(super) fn open(pid: u32) -> Option<Self> {
        let mut process=Self::open_raw(pid)?;
        // A WebView2 process may belong to Outlook, Widgets or another app.
        // Follow only live, creation-time-checked WebView2 parents to Teams.
        if is_webview(&process.label) {
            if let Some(parents)=process_parents() {
                process.teams_hint=has_teams_ancestor(pid,process.started,&parents,|ancestor|{
                    let parent=Self::open_raw(ancestor)?;
                    Some((parent.started,is_teams(&parent.label),is_webview(&parent.label)))
                });
            }
        }
        Some(process)
    }
    fn open_raw(pid:u32)->Option<Self> {
        if pid == 0 { return None; }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid).ok()? };
        let mut result = Self { handle, started: 0, label: format!("Process {pid}"), broad: true, teams_hint:false };
        let (mut created, mut exited, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user).ok()?; }
        result.started = u64::from(created.dwHighDateTime) << 32 | u64::from(created.dwLowDateTime);
        let mut name = [0u16; 1024]; let mut len = name.len() as u32;
        if unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), PWSTR(name.as_mut_ptr()), &mut len) }.is_ok() {
            let path = String::from_utf16_lossy(&name[..len as usize]);
            result.label = path.rsplit(['\\', '/']).next().unwrap_or("Application").to_string();
            let lower = result.label.to_ascii_lowercase();
            result.teams_hint=is_teams(&lower);
            result.broad = ["chrome", "msedge", "firefox", "opera", "brave", "webview", "browser", "electron"]
                .iter().any(|part| lower.contains(part));
        }
        if result.alive() { Some(result) } else { None }
    }
    pub(super) fn alive(&self) -> bool { unsafe { WaitForSingleObject(self.handle, 0) == WAIT_TIMEOUT } }
}
impl Drop for Process { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.handle); } } }

fn is_teams(label:&str)->bool {matches!(label.to_ascii_lowercase().as_str(),"ms-teams.exe"|"teams.exe"|"msteams.exe")}
fn is_webview(label:&str)->bool {label.eq_ignore_ascii_case("msedgewebview2.exe")}
fn process_parents()->Option<HashMap<u32,u32>> {
    let snapshot=unsafe{CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0).ok()?};
    struct Snapshot(HANDLE);
    impl Drop for Snapshot {fn drop(&mut self){unsafe{let _=CloseHandle(self.0);}}}
    let snapshot=Snapshot(snapshot);
    let mut row=PROCESSENTRY32W{dwSize:std::mem::size_of::<PROCESSENTRY32W>() as u32,..Default::default()};
    unsafe{Process32FirstW(snapshot.0,&mut row).ok()?;}
    let mut result=HashMap::new();
    for _ in 0..4096 {
        result.insert(row.th32ProcessID,row.th32ParentProcessID);
        if unsafe{Process32NextW(snapshot.0,&mut row)}.is_err(){break;}
    }
    Some(result)
}
fn has_teams_ancestor(mut pid:u32,mut started:u64,parents:&HashMap<u32,u32>,mut identity:impl FnMut(u32)->Option<(u64,bool,bool)>)->bool {
    let mut seen=HashSet::from([pid]);
    for _ in 0..8 {
        let Some(&parent)=parents.get(&pid) else{return false;};
        if parent==0 || !seen.insert(parent){return false;}
        let Some((parent_started,teams,webview))=identity(parent) else{return false;};
        if parent_started>started{return false;}
        if teams{return true;}
        if !webview{return false;}
        pid=parent;started=parent_started;
    }
    false
}

fn single_owner(session:&IAudioSessionControl2)->Option<u32> {
    let mut pid=0;
    // The generated Result<u32> erases the successful NO_SINGLE_PROCESS
    // HRESULT. A spanning/ambiguous session must not select a single owner.
    let status=unsafe{(Interface::vtable(session).GetProcessId)(Interface::as_raw(session),&mut pid)};
    (status==HRESULT(0) && pid!=0).then_some(pid)
}
fn owner_matches(cached:u32,held_alive:bool,current:Option<u32>)->bool {held_alive && cached!=0 && current==Some(cached)}

struct Session { control: IAudioSessionControl, extended:IAudioSessionControl2, callback: IAudioSessionEvents, process: Option<Process>, pid: u32 }
impl Session {
    fn new(control: IAudioSessionControl, signals: Arc<Signals>) -> Option<Self> {
        let ext = control.cast::<IAudioSessionControl2>().ok()?;
        if unsafe { ext.IsSystemSoundsSession() } != HRESULT(1) { return None; } // S_FALSE means not system sounds.
        let pid=single_owner(&ext)?;
        let process = Process::open(pid);
        let callback: IAudioSessionEvents = SessionCallback { signals }.into();
        unsafe { control.RegisterAudioSessionNotification(&callback).ok()?; }
        Some(Self { control, extended:ext, callback, process, pid })
    }
    fn matches_owner(&self,current:Option<u32>)->bool {owner_matches(self.pid,self.process.as_ref().is_some_and(|process|process.alive()),current)}
    fn target(&self, id: &str) -> Option<AudioTarget> {
        let process = self.process.as_ref()?;
        // GetProcessId updates session stream-switch state. Never keep using
        // a cached render owner after Windows reports a replacement/ambiguity.
        if !self.matches_owner(single_owner(&self.extended)) || unsafe { self.control.GetState() }.ok()? != AudioSessionStateActive { return None; }
        let label=if process.teams_hint {format!("Microsoft Teams · {}",process.label)}else{process.label.clone()};
        Some(AudioTarget { key: 0, label, pid: self.pid,
            process_started: process.started, session_id: id.to_string(), broad_capture: process.broad })
    }
}
impl Drop for Session { fn drop(&mut self) { unsafe { let _ = self.control.UnregisterAudioSessionNotification(&self.callback); } } }

struct Device { manager: IAudioSessionManager2, duck: Option<IAudioVolumeDuckNotification>, created: IAudioSessionNotification, sessions: HashMap<String, Session> }
impl Device {
    fn new(device: &IMMDevice, signals: &Arc<Signals>) -> ::windows::core::Result<Self> {
        let manager = unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)? };
        let duck: IAudioVolumeDuckNotification = DuckCallback { signals: Arc::clone(signals) }.into();
        let created: IAudioSessionNotification = CreatedCallback { signals: Arc::clone(signals) }.into();
        // Duck events are optional hints, not a prerequisite for cataloguing
        // render sessions or explicitly selecting a Teams/WebView2 stream.
        let duck=unsafe { manager.RegisterDuckNotification(PCWSTR::null(), &duck) }.ok().map(|_|duck);
        if let Err(error) = unsafe { manager.RegisterSessionNotification(&created) } {
            if let Some(duck)=&duck {unsafe { let _ = manager.UnregisterDuckNotification(duck); }} return Err(error);
        }
        Ok(Self { manager, duck, created, sessions: HashMap::new() })
    }
    fn refresh(&mut self, signals: &Arc<Signals>, remaining: &mut usize, registered: &mut usize, teams:&mut HashSet<String>) -> ::windows::core::Result<Vec<AudioTarget>> {
        let list = unsafe { self.manager.GetSessionEnumerator()? };
        // GetCount after registration is required to enable new-session events.
        let count = unsafe { list.GetCount()? }.clamp(0, MAX_TARGETS as i32);
        let mut alive = HashSet::new(); let mut result = Vec::new();
        for i in 0..count {
            if *remaining == 0 { break; }
            let control = match unsafe { list.GetSession(i) } { Ok(c) => c, Err(_) => continue };
            let ext = match control.cast::<IAudioSessionControl2>() { Ok(c) => c, Err(_) => continue };
            let id = match unsafe { ext.GetSessionInstanceIdentifier().ok().and_then(|s| owned_wide(s)) } { Some(s) => s, None => continue };
            let owner=single_owner(&ext);
            if self.sessions.get(&id).is_some_and(|session|!session.matches_owner(owner)) {
                // Retire before replacement registration to preserve the real
                // callback budget. A new owner is a new target identity/key;
                // an existing capture is stopped by selected_live, not rebound.
                self.sessions.remove(&id);*registered-=1;
            }
            if !self.sessions.contains_key(&id) && *registered < MAX_TARGETS {
                if let Some(session) = Session::new(control, Arc::clone(signals)) {
                    self.sessions.insert(id.clone(), session); *registered += 1;
                }
            }
            if let Some(session) = self.sessions.get(&id) {
                *remaining -= 1; alive.insert(id.clone());
                if let Some(target) = session.target(&id) {
                    if session.process.as_ref().is_some_and(|process|process.teams_hint){teams.insert(id.clone());}
                    result.push(target);
                }
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
            if let Some(duck)=&self.duck {let _ = self.manager.UnregisterDuckNotification(duck);}
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
                    Ok((mut targets,teams)) => {
                        retain_communications(&mut communication_ids, &targets, Instant::now());
                        // Windows does not replay the duck event when the
                        // monitor starts during a call. Active exact Teams
                        // render owners are a possible-call hint only; capture
                        // still requires explicit consent for this session.
                        let ids=proposal_ids(&mut targets,&communication_ids,&teams);
                        let mut state=shared.state();state.catalog(targets, &ids)?;
                        state.snapshot.teams_render_sessions=teams.len();
                    }
                    Err(error) => {
                        devices.clear(); communication_ids.clear();
                        let mut state = shared.state(); state.catalog(Vec::new(), &HashSet::new())?;
                        state.snapshot.teams_render_sessions=0;
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
    state.snapshot.input_health=InputHealth::default();
    state.snapshot.teams_render_sessions=0;
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

fn refresh(enumerator: &IMMDeviceEnumerator, devices: &mut HashMap<String, Device>, signals: &Arc<Signals>) -> Result<(Vec<AudioTarget>,HashSet<String>), String> {
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
    let mut targets = Vec::new(); let mut teams=HashSet::new();let mut remaining = MAX_TARGETS;
    let mut registered = devices.values().map(|d| d.sessions.len()).sum();
    for (id, endpoint) in endpoints {
        if signals.shared.stop.load(Ordering::Acquire) { break; }
        if !devices.contains_key(&id) {
            match Device::new(&endpoint, signals) { Ok(device) => { devices.insert(id.clone(), device); }, Err(_) => continue }
        }
        if let Some(device) = devices.get_mut(&id) { targets.extend(device.refresh(signals, &mut remaining, &mut registered,&mut teams).map_err(|e| e.to_string())?); }
    }
    if count != 0 && devices.is_empty() && !signals.shared.stop.load(Ordering::Acquire) {
        return Err("Core Audio session notifications are unavailable".into());
    }
    Ok((targets,teams))
}

fn retain_communications(ids: &mut HashMap<String, Instant>, targets: &[AudioTarget], now: Instant) {
    // Duck notification can precede the stream becoming Active. Retain a bounded
    // grace period for exact-ID correlation, never infer a call from another PID.
    ids.retain(|id, seen| {
        if targets.iter().any(|t| t.session_id == *id) { *seen = now; true }
        else { now.saturating_duration_since(*seen) < Duration::from_secs(10) }
    });
}

fn proposal_ids(targets:&mut [AudioTarget],communications:&HashMap<String,Instant>,teams:&HashSet<String>)->HashSet<String> {
    // A correlated communication event is stronger than the Teams fallback.
    targets.sort_by_key(|target|(!communications.contains_key(&target.session_id),!teams.contains(&target.session_id)));
    let mut ids:HashSet<_>=communications.keys().cloned().collect();
    ids.extend(teams.iter().cloned());
    ids
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
    #[test]
    fn teams_webview_requires_a_live_time_checked_chain_not_any_webview() {
        assert!(is_teams("MS-TEAMS.EXE"));assert!(!is_teams("ms-teamsupdate.exe"));
        assert!(!is_teams("teams-helper.exe"));assert!(!is_teams("msedgewebview2.exe"));
        assert!(!is_teams("Microsoft Teams · lookalike.exe"));
        let parents=HashMap::from([(4,3),(3,2),(2,1)]);
        let identities=HashMap::from([(3,(30,false,true)),(2,(20,true,false)),(1,(10,true,false))]);
        assert!(has_teams_ancestor(4,40,&parents,|pid|identities.get(&pid).copied()));
        assert!(!has_teams_ancestor(4,20,&parents,|pid|identities.get(&pid).copied()),"A reused/newer parent PID is not the creator");
        assert!(!has_teams_ancestor(4,40,&parents,|pid|if pid==3{None}else{identities.get(&pid).copied()}));
        assert!(!has_teams_ancestor(4,40,&parents,|pid|Some((pid as u64,false,false))),"Another WebView2 host is not Teams");
        let cycle=HashMap::from([(4,3),(3,4)]);
        assert!(!has_teams_ancestor(4,40,&cycle,|_|Some((10,false,true))));
        let deep:HashMap<_,_>=(1..=10).map(|pid|(pid,pid+1)).collect();
        assert!(!has_teams_ancestor(1,100,&deep,|pid|Some((100,pid==10,true))),"Ancestry traversal is bounded");
    }
    #[test]
    fn active_teams_can_offer_consent_without_a_new_duck_event_but_cannot_capture() {
        let mut teams=AudioTarget { key:0,label:"Microsoft Teams · msedgewebview2.exe".into(),pid:5,process_started:50,session_id:"teams-active".into(),broad_capture:true };
        let unrelated=AudioTarget {label:"msedgewebview2.exe".into(),session_id:"outlook".into(),..teams.clone()};
        let communications=HashMap::new();let mut targets=vec![unrelated,teams.clone()];
        assert!(proposal_ids(&mut targets,&communications,&HashSet::new()).is_empty(),"Display labels never grant a Teams hint");
        let ids=proposal_ids(&mut targets,&communications,&HashSet::from(["teams-active".into()]));assert_eq!(ids,HashSet::from(["teams-active".into()]));
        let mut state=State::default();state.catalog(targets,&ids).unwrap();
        assert!(state.snapshot.suggested.is_some());assert!(state.snapshot.capture.is_none());assert!(state.pending.is_none());
        let key=state.snapshot.suggested.unwrap();state.decline(key);
        teams.key=key;state.catalog(vec![teams.clone()],&ids).unwrap();assert!(state.snapshot.suggested.is_none());
        state.catalog(vec![],&HashSet::new()).unwrap();state.catalog(vec![teams],&ids).unwrap();assert!(state.snapshot.suggested.is_some());
    }
    #[test]
    fn exact_communications_event_precedes_teams_audio_hint() {
        let mut targets=vec![AudioTarget{key:0,label:"Microsoft Teams · ms-teams.exe".into(),pid:5,process_started:50,session_id:"teams".into(),broad_capture:false},
            AudioTarget{key:0,label:"Discord.exe".into(),pid:6,process_started:60,session_id:"communication".into(),broad_capture:false}];
        let ids=proposal_ids(&mut targets,&HashMap::from([("communication".into(),Instant::now())]),&HashSet::from(["teams".into()]));
        let mut state=State::default();state.catalog(targets,&ids).unwrap();
        assert_eq!(state.snapshot.candidates[0].session_id,"communication");
        assert_eq!(state.snapshot.suggested,Some(state.snapshot.candidates[0].key));
    }
    #[test]
    fn stream_owner_change_retires_old_identity_and_requires_new_consent() {
        let old=AudioTarget{key:0,label:"Teams audio".into(),pid:5,process_started:50,session_id:"same-instance".into(),broad_capture:true};
        let ids=HashSet::from([old.session_id.clone()]);let mut state=State::default();
        state.catalog(vec![old.clone()],&ids).unwrap();let old_key=state.snapshot.candidates[0].key;
        let generation=state.begin_capture(old_key).unwrap();assert!(state.listening(generation));
        state.accept_pcm(generation,&vec![17;CHUNK_SAMPLES]).unwrap();
        assert!(owner_matches(5,true,Some(5)));assert!(!owner_matches(5,true,Some(9)));
        assert!(!owner_matches(5,false,Some(5)));assert!(!owner_matches(5,true,None));
        let replacement=AudioTarget{pid:9,process_started:90,..old};
        state.catalog(vec![replacement],&ids).unwrap();let new_key=state.snapshot.candidates[0].key;
        assert_ne!(old_key,new_key,"An unchanged session-instance string does not preserve consent for a different PID");
        assert_eq!(state.snapshot.capture.as_ref().unwrap().key,old_key,"Cataloguing does not rebind the capture");
        assert!(state.begin_capture(new_key).is_err());
        // selected_live rejects the old target on this mismatch; service_capture
        // then revokes/drains it before offering the new target for confirmation.
        state.request_stop(Some("Selected audio owner changed".into()));state.finish_capture(generation);
        assert!(state.snapshot.capture.is_none());assert!(state.pcm.is_empty());assert!(state.pending.is_none());
        let next=state.begin_capture(new_key).unwrap();assert_ne!(next,generation);
        assert_eq!(state.snapshot.capture.as_ref().unwrap().key,new_key);
    }
}
