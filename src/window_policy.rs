// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Own-window presentation policy. No windows, hooks or background workers created.
use std::cell::RefCell;
use std::collections::BTreeMap;
use windows::core::{Error, Result};
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG, HWND};
use windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled;
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const TRAY_ONLY_KEY: &str = "tray_only";
pub const EXCLUDE_CAPTURE_KEY: &str = "exclude_from_capture";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy { pub tray_only: bool, pub exclude_capture: bool }
impl Default for Policy {
    fn default() -> Self { Self { tray_only: true, exclude_capture: true } }
}
impl Policy {
    pub fn from_settings(settings: &BTreeMap<String,String>) -> Self {
        Self { tray_only: settings.get(TRAY_ONLY_KEY).is_none_or(|v| v != "0"),
            exclude_capture: settings.get(EXCLUDE_CAPTURE_KEY).is_none_or(|v| v != "0") }
    }
}
#[derive(Clone, Copy)]
enum Attempt { Off, Requested { readback: bool }, Failed }
#[derive(Default)]
struct State { policy: Policy, windows: BTreeMap<isize,Attempt>, last_error: bool }
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

pub fn current() -> Policy { STATE.with(|state| state.borrow().policy) }
/// A settings reload starts a new reporting attempt; no I/O or window changes here.
pub fn set_policy(policy: Policy) {
    STATE.with(|state| { let mut state=state.borrow_mut(); state.policy=policy; state.last_error=false; });
}
pub fn unregister(hwnd: HWND) { STATE.with(|state| { state.borrow_mut().windows.remove(&(hwnd.0 as isize)); }); }
pub fn has_failures() -> bool { STATE.with(|state| { let state=state.borrow(); state.last_error || state.windows.values().any(|v| matches!(v,Attempt::Failed)) }) }
/// Cached outcomes only: safe to read from Settings drawing.
pub fn status_text() -> String {
    STATE.with(|state| {
        let state=state.borrow();
        if state.last_error || state.windows.values().any(|v| matches!(v,Attempt::Failed)) {
            "Cattura: esclusione non applicata a una finestra".into()
        } else if !state.policy.exclude_capture { "Cattura: esclusione disattivata".into() }
        else {
            let requested=state.windows.values().filter(|v| matches!(v,Attempt::Requested { .. })).count();
            let verified=state.windows.values().filter(|v| matches!(v,Attempt::Requested { readback:true })).count();
            if requested == 0 { "Cattura: esclusione in attesa della finestra".into() }
            else if requested == verified { format!("Cattura: richiesta Windows verificata ({requested})") }
            else { format!("Cattura: richiesta accettata da Windows ({requested})") }
        }
    })
}

pub fn panel_style(explicit: bool, tray_only: bool) -> WINDOW_EX_STYLE {
    if !explicit { WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE }
    else if tray_only { WS_EX_TOOLWINDOW }
    else { WS_EX_APPWINDOW }
}
fn supported(major: u32, build: u32, composing: bool) -> bool {
    composing && (major > 10 || (major == 10 && build >= 19041))
}
#[link(name="ntdll")]
extern "system" { fn RtlGetVersion(version: *mut OSVERSIONINFOW) -> i32; }
fn metadata() -> Result<(u32,u32,bool)> {
    let mut version=OSVERSIONINFOW { dwOSVersionInfoSize:std::mem::size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
    if unsafe { RtlGetVersion(&mut version) } < 0 { return Err(Error::new(E_FAIL,"Cannot read Windows version")); }
    let composing=unsafe { DwmIsCompositionEnabled()? }.as_bool();
    Ok((version.dwMajorVersion,version.dwBuildNumber,composing))
}
/// Call on an owned top-level window before its first ShowWindow, and on reload.
/// A successful Set is an OS request, never proof of a sharing client's behavior.
pub fn apply(hwnd: HWND) -> Result<()> {
    let result=apply_inner(hwnd,current().exclude_capture);
    STATE.with(|state| {
        let mut state=state.borrow_mut();
        let attempt=match &result { Ok(value)=>*value, Err(_)=>{state.last_error=true;Attempt::Failed} };
        state.windows.insert(hwnd.0 as isize,attempt);
    });
    result.map(|_| ())
}
fn apply_inner(hwnd: HWND, exclude: bool) -> Result<Attempt> {
    if hwnd.is_invalid() { return Err(Error::new(E_INVALIDARG,"Invalid own window")); }
    if exclude {
        let (major,build,composing)=metadata()?;
        if !supported(major,build,composing) {
            return Err(Error::new(E_FAIL,"Capture exclusion requires Windows 10 2004 or later and DWM composition"));
        }
    }
    let affinity=if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    unsafe { SetWindowDisplayAffinity(hwnd,affinity)?; }
    let mut actual=0;
    // Microsoft documents Get for layered windows. Non-layered GDI/DComp
    // windows can accept Set while readback is unavailable: report this honestly.
    let readback=unsafe { GetWindowDisplayAffinity(hwnd,&mut actual) }.is_ok();
    classify(exclude,readback.then_some(actual))
}
fn classify(exclude: bool, actual: Option<u32>) -> Result<Attempt> {
    let expected=if exclude { WDA_EXCLUDEFROMCAPTURE.0 } else { WDA_NONE.0 };
    if actual.is_some_and(|actual| actual != expected) {
        return Err(Error::new(E_FAIL,"Windows did not retain the requested capture affinity"));
    }
    Ok(if exclude { Attempt::Requested { readback:actual.is_some() } } else { Attempt::Off })
}

pub fn run_diagnostic() -> std::result::Result<bool,String> {
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.first().is_none_or(|a| a != "--privacy-check") { return Ok(false); }
    if args.len()!=1 { return Err("Use --privacy-check without additional arguments".into()); }
    let (major,build,composing)=metadata().map_err(|e|e.to_string())?;
    println!("{}",serde_json::json!({"windowsMajor":major,"windowsBuild":build,"dwmComposing":composing,
        "captureExclusionSupportedByOs":supported(major,build,composing),"windowsCreated":false,
        "captureAffinitySetOnRealWindow":false,"screenSharingTested":false}));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn privacy_defaults_on_and_explicit_zero_is_respected() {
        assert_eq!(Policy::from_settings(&BTreeMap::new()),Policy::default());
        let map=BTreeMap::from([(TRAY_ONLY_KEY.into(),"0".into()),(EXCLUDE_CAPTURE_KEY.into(),"0".into())]);
        assert_eq!(Policy::from_settings(&map),Policy{tray_only:false,exclude_capture:false});
    }
    #[test]
    fn tray_mode_does_not_remove_explicit_panel_keyboard_activation() {
        let style=panel_style(true,true).0;
        assert_ne!(style & WS_EX_TOOLWINDOW.0,0); assert_eq!(style & (WS_EX_APPWINDOW.0|WS_EX_NOACTIVATE.0),0);
        assert_eq!(panel_style(true,false),WS_EX_APPWINDOW);
        assert_eq!(panel_style(false,false),WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE);
        assert_eq!(panel_style(false,true),WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE);
    }
    #[test]
    fn older_windows_and_disabled_composition_never_claim_full_exclusion() {
        assert!(!supported(10,19040,true)); assert!(!supported(10,26300,false));
        assert!(!supported(6,99999,true)); assert!(supported(10,19041,true));
    }
    #[test]
    fn accepted_set_with_unavailable_readback_is_not_a_false_rejection() {
        assert!(matches!(classify(true,None).unwrap(),Attempt::Requested{readback:false}));
        assert!(matches!(classify(true,Some(17)).unwrap(),Attempt::Requested{readback:true}));
        assert!(classify(true,Some(1)).is_err());
        assert!(classify(false,Some(17)).is_err());
        assert!(matches!(classify(false,Some(0)).unwrap(),Attempt::Off));
    }
}
