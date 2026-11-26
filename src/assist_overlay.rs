// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Two opt-in native UI windows. No provider, process, timer or capture starts here.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use windows::core::{w, Error, PCWSTR, Result};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{EM_GETFIRSTVISIBLELINE, EM_LINESCROLL, EM_SETLIMITTEXT};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::assist_layout::{self, Monitor, PlacementError, ScreenRect};
use crate::codex_bridge::AnswerKey;
use crate::{theme, worker};

const MAX_ROWS: usize = 128;
const MAX_PROMPTS: usize = 3;
const MAX_ANSWER: usize = 4096;
const MAX_FOLLOWUP: usize = 8192;
const MAX_ACTIONS: usize = 64;
const CLOSE: i32 = 100;
const CONNECTION: i32 = 101;
const USAGE: i32 = 102;
const RESOURCES: i32 = 103;
const THREADS: i32 = 104;
const STATUS: i32 = 105;
const DETAIL: i32 = 106;
const REPLY: i32 = 107;
const SEND: i32 = 108;
const OPEN_CODEX: i32 = 109;
const REFRESH: i32 = 110;
const SHOW_CALL: i32 = 111;
const WORKSPACE: i32 = 112;
const START_SERVER: i32 = 113;
const NEW_CHAT: i32 = 114;
const WORKSPACE_LABEL: i32 = 115;
const QUESTION_LABEL: i32 = 120;
const QUESTION_INPUT: i32 = 130;
const CALL_STATUS: i32 = 200;
const CALL_SCOPE: i32 = 201;
const CALL_PROVIDER: i32 = 202;
const CALL_TARGET: i32 = 203;
const CALL_TRANSCRIPT: i32 = 204;
const CALL_SUGGESTION: i32 = 205;
const CONFIRM: i32 = 206;
const DECLINE: i32 = 207;
const STOP: i32 = 208;
const CALL_CONTEXT: i32 = 209;
const CALL_CONTEXT_LABEL: i32 = 210;
const CONFIRM_AI: i32 = 211;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayConfig {
    pub codex_dashboard: bool,
    pub call_panel: bool,
    pub monitor_device: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayAvailability { Off, WaitingForSecondary, ChooseSecondary, InsufficientSpace, Ready }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatRow { pub id: String, pub title: String, pub statuslabel: String, pub attention: bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt { pub id: String, pub text: String, pub options: Vec<String> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionForm { pub key: AnswerKey, pub questions: Vec<Prompt> }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DashboardView {
    pub rows: Vec<ChatRow>, pub selected_id: Option<String>, pub detail: String,
    pub usage: String, pub resources: String, pub connection: String,
    pub question: Option<QuestionForm>, pub can_followup: bool, pub approval_notice: Option<String>,
    pub workspace: String, pub can_start_server: bool, pub can_create_thread: bool, pub can_answer: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallTarget { pub key: u64, pub label: String }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallView {
    pub visible: bool, pub status: String, pub scope: String, pub provider: String,
    pub transcript: String, pub suggestion: String, pub candidates: Vec<CallTarget>,
    pub candidate_key: Option<u64>, pub can_listen: bool, pub listening: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayAction {
    SelectThread(String), SendFollowup { thread_id: String, text: String },
    AnswerQuestion { key: AnswerKey, answers: BTreeMap<String, Vec<String>> },
    OpenCodex { thread_id: String }, Refresh, ShowCallPanel,
    StartSharedServer, CreateThread { workspace: String },
    ConfirmCall { candidate_key: u64, context: String, with_ai: bool }, DeclineCall { candidate_key: u64 }, StopCall,
    SelectCallTarget { target_key: u64 }, CloseDashboard, CloseCall,
}

thread_local! {
    static WINDOWS: RefCell<Vec<HWND>> = const { RefCell::new(Vec::new()) };
    static QUITTING: Cell<bool> = const { Cell::new(false) };
    static TOPOLOGY_DIRTY: Cell<bool> = const { Cell::new(false) };
}

/// Called from the monitor selector action, never from painting or idle polling.
pub fn secondary_devices() -> Result<Vec<String>> { Ok(secondary_device_names(enumerate_monitors()?)) }

fn secondary_device_names(monitors: Vec<Monitor>) -> Vec<String> {
    let mut devices: Vec<_> = monitors.into_iter().filter(|monitor| !monitor.primary).map(|monitor| monitor.device).collect();
    devices.sort_unstable(); devices.dedup(); devices
}

/// The coordinator consumes this before its availability guard and refreshes
/// its own worker lifecycle together with window placement.
pub fn take_topology_dirty() -> bool { TOPOLOGY_DIRTY.with(|flag| flag.replace(false)) }

/// Process-exit cleanup; independent of any App or OverlayManager borrow.
pub fn shutdown_all() {
    QUITTING.with(|flag| flag.set(true));
    let windows = WINDOWS.with(|items| std::mem::take(&mut *items.borrow_mut()));
    for hwnd in windows { unsafe { let _ = DestroyWindow(hwnd); } }
}

/// Call before TranslateMessage/DispatchMessage for native Tab/Shift+Tab routing.
pub fn translate_message(message: &MSG) -> bool {
    let windows = WINDOWS.with(|items| items.borrow().clone());
    for hwnd in windows {
        unsafe {
            if IsWindowVisible(hwnd).as_bool()
                && (message.hwnd == hwnd || IsChild(hwnd, message.hwnd).as_bool())
                && IsDialogMessageW(hwnd, message).as_bool() { return true; }
        }
    }
    false
}

struct ActionQueue { actions: RefCell<VecDeque<OverlayAction>>, wake: Cell<HWND> }
impl ActionQueue {
    fn push(&self, action: OverlayAction) -> bool {
        let Ok(mut actions) = self.actions.try_borrow_mut() else { return false };
        let critical = matches!(action, OverlayAction::StopCall | OverlayAction::CloseCall | OverlayAction::CloseDashboard);
        if matches!(action, OverlayAction::StopCall) {
            actions.retain(|a| !matches!(a, OverlayAction::ConfirmCall { .. } | OverlayAction::DeclineCall { .. } | OverlayAction::SelectCallTarget { .. } | OverlayAction::StopCall));
        }
        if actions.len() >= MAX_ACTIONS { if critical { actions.pop_back(); } else { return false; } }
        if matches!(action, OverlayAction::StopCall) { actions.push_front(action); } else { actions.push_back(action); }
        drop(actions);
        self.wake(); true
    }
    fn wake(&self) {
        let hwnd = self.wake.get();
        if !hwnd.is_invalid() { unsafe { let _ = PostMessageW(hwnd, worker::WM_APP_WAKE, WPARAM(0), LPARAM(0)); } }
    }
}

pub struct OverlayManager {
    config: OverlayConfig, dashboard: Option<OverlayWindow>, call: Option<OverlayWindow>,
    dashboard_view: DashboardView, call_view: CallView, queue: Option<Rc<ActionQueue>>,
    wake: HWND, availability: OverlayAvailability, dashboard_closed: bool,
    suppressed_call: Option<Option<u64>>, auto_device: Option<String>,
}

impl Default for OverlayManager { fn default() -> Self { Self::new() } }
impl OverlayManager {
    pub fn new() -> Self {
        Self { config: OverlayConfig::default(), dashboard: None, call: None,
            dashboard_view: DashboardView::default(), call_view: CallView::default(), queue: None,
            wake: HWND::default(), availability: OverlayAvailability::Off, dashboard_closed: false,
            suppressed_call: None, auto_device: None }
    }
    pub fn set_wake_window(&mut self, hwnd: HWND) {
        self.wake = hwnd;
        if let Some(queue) = &self.queue { queue.wake.set(hwnd); }
    }
    pub fn sync(&mut self, config: OverlayConfig) -> Result<OverlayAvailability> {
        if config.codex_dashboard != self.config.codex_dashboard { self.dashboard_closed = false; }
        if config.call_panel != self.config.call_panel { self.suppressed_call = None; }
        if config.monitor_device != self.config.monitor_device { self.auto_device = None; }
        self.config = config;
        if !self.config.codex_dashboard { self.dashboard = None; self.dashboard_view = DashboardView::default(); }
        if !self.config.call_panel {
            if self.call_view.listening { self.enqueue(OverlayAction::StopCall); }
            self.call = None; self.call_view = CallView::default();
        }
        let result = (|| {
            // Apply before placement can show an existing window again.
            for window in [&self.dashboard, &self.call].into_iter().flatten() { window.apply_capture_policy()?; }
            self.refresh_placement()?;
            for window in [&self.dashboard, &self.call].into_iter().flatten() { window.palette_changed()?; }
            Ok(self.availability)
        })();
        if result.is_err() { self.close_after_error(); }
        result
    }
    pub fn display_changed(&mut self) -> Result<OverlayAvailability> {
        // Hide before enumeration so an unplugged display cannot become a
        // request to move the panels onto Windows' automatic primary fallback.
        for window in [&self.dashboard, &self.call].into_iter().flatten() { window.hide(); }
        let result = (|| {
            for window in [&self.dashboard, &self.call].into_iter().flatten() { window.apply_capture_policy()?; }
            self.refresh_placement()?;
            Ok(self.availability)
        })();
        if result.is_err() { self.close_after_error(); }
        result
    }
    pub fn apply_dashboard(&mut self, view: DashboardView) {
        if !self.config.codex_dashboard { return; }
        self.prune_closed();
        self.dashboard_view = bounded_dashboard(view);
        if let Some(window) = &self.dashboard { window.dashboard(&self.dashboard_view); }
    }
    pub fn apply_call(&mut self, view: CallView) {
        if !self.config.call_panel { return; }
        self.prune_closed();
        self.call_view = bounded_call(view);
        if !self.call_view.visible {
            // A completed/dismissed occurrence must not suppress a later call
            // that reuses the same process and WASAPI session identity.
            self.suppressed_call = None;
            if self.call_view.listening { self.enqueue(OverlayAction::StopCall); }
            self.call = None;
        } else if self.suppressed_call != Some(self.call_view.candidate_key) && self.call.is_none() {
            // The coordinator owns availability and provider lifetimes.
            let queue = self.queue.get_or_insert_with(|| Rc::new(ActionQueue { actions: RefCell::new(VecDeque::new()), wake: Cell::new(self.wake) }));
            mark_topology_dirty(queue);
        }
        if let Some(window) = &self.call { window.call(&self.call_view); }
    }
    pub fn drain_actions(&mut self) -> Vec<OverlayAction> {
        self.prune_closed();
        let actions: Vec<_> = self.queue.as_ref().map(|queue| queue.actions.borrow_mut().drain(..).collect()).unwrap_or_default();
        if actions.iter().any(|a| matches!(a, OverlayAction::ShowCallPanel)) { self.suppressed_call = None; }
        actions
    }
    pub fn shutdown(&mut self) {
        if self.call_view.listening { self.enqueue(OverlayAction::StopCall); }
        self.dashboard = None; self.call = None;
        self.config = OverlayConfig::default(); self.dashboard_view = DashboardView::default(); self.call_view = CallView::default();
        self.auto_device = None; self.availability = OverlayAvailability::Off;
    }
    fn enqueue(&mut self, action: OverlayAction) {
        let queue = self.queue.get_or_insert_with(|| Rc::new(ActionQueue { actions: RefCell::new(VecDeque::new()), wake: Cell::new(self.wake) }));
        queue.push(action);
    }
    fn close_after_error(&mut self) {
        // The coordinator has already revoked capture before a settings/display
        // transition. Keep a Stop action as a backstop for other callers.
        if self.call_view.listening || self.call.is_some() { self.enqueue(OverlayAction::StopCall); }
        for window in [&self.dashboard, &self.call].into_iter().flatten() { window.hide(); }
        self.dashboard = None; self.call = None;
        self.dashboard_view = DashboardView::default(); self.call_view = CallView::default();
        self.availability = OverlayAvailability::Off;
    }
    fn prune_closed(&mut self) {
        if self.dashboard.as_ref().is_some_and(|w| w.state.destroyed.get()) { self.dashboard = None; self.dashboard_closed = true; }
        if self.call.as_ref().is_some_and(|w| w.state.destroyed.get()) {
            self.suppressed_call = Some(self.call_view.candidate_key); self.call = None;
        }
    }
    fn unavailable(&mut self, reason: PlacementError) {
        if self.call_view.listening || self.call.is_some() { self.enqueue(OverlayAction::StopCall); }
        self.dashboard = None; self.call = None;
        self.availability = match reason {
            PlacementError::MissingSecondary => OverlayAvailability::WaitingForSecondary,
            PlacementError::ChooseSecondary => OverlayAvailability::ChooseSecondary,
            _ => OverlayAvailability::InsufficientSpace,
        };
    }
    fn refresh_placement(&mut self) -> Result<()> {
        self.prune_closed();
        if !self.config.codex_dashboard && !self.config.call_panel { self.shutdown(); return Ok(()); }
        let monitors = match enumerate_monitors() {
            Ok(monitors) => monitors,
            Err(error) => { self.unavailable(PlacementError::MissingSecondary); return Err(error); }
        };
        let requested = self.config.monitor_device.as_deref().or(self.auto_device.as_deref());
        let monitor = match assist_layout::select_secondary(&monitors, requested) {
            Ok(monitor) => monitor.clone(), Err(error) => { self.unavailable(error); return Ok(()); }
        };
        if self.config.monitor_device.is_none() { self.auto_device = Some(monitor.device.clone()); }
        let dashboard = self.config.codex_dashboard && !self.dashboard_closed;
        let call = self.config.call_panel && self.call_view.visible && self.suppressed_call != Some(self.call_view.candidate_key);
        if !call { self.call = None; }
        if !dashboard && !call { self.availability = OverlayAvailability::Ready; return Ok(()); }
        let queue = Rc::clone(self.queue.get_or_insert_with(|| Rc::new(ActionQueue { actions: RefCell::new(VecDeque::new()), wake: Cell::new(self.wake) })));
        if dashboard && self.dashboard.is_none() { self.dashboard = Some(OverlayWindow::create(Kind::Dashboard, monitor.work, Rc::clone(&queue))?); }
        if call && self.call.is_none() { self.call = Some(OverlayWindow::create(Kind::Call, monitor.work, queue)?); }
        // A hidden window is first placed on the selected display. Query its
        // actual DPI, not the notch's primary-display/system DPI.
        let mut dpi = [&self.dashboard, &self.call].into_iter().flatten().next()
            .map(|w| unsafe { GetDpiForWindow(w.hwnd) }).unwrap_or(96);
        for _ in 0..2 {
            let layout = match assist_layout::overlay_rects(monitor.work, dpi, dashboard, call) {
                Ok(layout) => layout, Err(error) => { self.unavailable(error); return Ok(()); }
            };
            if let (Some(window), Some(rect)) = (&self.dashboard, layout.dashboard) { window.position(rect, dpi)?; }
            if let (Some(window), Some(rect)) = (&self.call, layout.call) { window.position(rect, dpi)?; }
            let actual = [&self.dashboard, &self.call].into_iter().flatten().next().map(|w| unsafe { GetDpiForWindow(w.hwnd) }).unwrap_or(dpi);
            if actual == dpi { break; }
            dpi = actual;
        }
        if let Some(window) = &self.dashboard { window.dashboard(&self.dashboard_view); window.show(); }
        if let Some(window) = &self.call { window.call(&self.call_view); window.show(); }
        self.availability = OverlayAvailability::Ready;
        Ok(())
    }
}
impl Drop for OverlayManager { fn drop(&mut self) { self.shutdown(); } }

fn enumerate_monitors() -> Result<Vec<Monitor>> {
    unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let mut info = MONITORINFOEXW::default(); info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(monitor, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool() { return false.into(); }
        let Some(items) = (data.0 as *mut Vec<Monitor>).as_mut() else { return false.into() };
        let length = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
        let work = info.monitorInfo.rcWork;
        items.push(Monitor { device: String::from_utf16_lossy(&info.szDevice[..length]), primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            work: ScreenRect { left: work.left, top: work.top, right: work.right, bottom: work.bottom } });
        true.into()
    }
    let mut monitors = Vec::new();
    unsafe { EnumDisplayMonitors(None, None, Some(collect), LPARAM((&mut monitors as *mut Vec<Monitor>) as isize)).ok()?; }
    Ok(monitors)
}

fn mark_topology_dirty(queue: &ActionQueue) {
    if !TOPOLOGY_DIRTY.with(|flag| flag.replace(true)) { queue.wake(); }
}

fn bounded(value: &str, max: usize) -> String {
    value.chars().take(max).map(|c| if c == '\0' { '\u{FFFD}' } else { c }).collect()
}
fn bounded_dashboard(mut view: DashboardView) -> DashboardView {
    let selected = view.rows.iter().find(|r| Some(&r.id) == view.selected_id.as_ref()).cloned();
    view.rows.truncate(MAX_ROWS);
    if let Some(selected) = selected { if !view.rows.iter().any(|r| r.id == selected.id) { view.rows.pop(); view.rows.push(selected); } }
    view.detail = bounded(&view.detail, 65536); view.usage = bounded(&view.usage, 4096);
    view.resources = bounded(&view.resources, 4096); view.connection = bounded(&view.connection, 2048);
    view.workspace = bounded(&view.workspace, 2048);
    for row in &mut view.rows { row.title = bounded(&row.title, 160); row.statuslabel = bounded(&row.statuslabel, 160); }
    view
}
fn bounded_call(mut view: CallView) -> CallView {
    view.status = bounded(&view.status, 2048); view.scope = bounded(&view.scope, 2048); view.provider = bounded(&view.provider, 2048);
    view.transcript = bounded(&view.transcript, 65536); view.suggestion = bounded(&view.suggestion, 32768);
    view.candidates.truncate(64); for candidate in &mut view.candidates { candidate.label = bounded(&candidate.label, 256); }
    view
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { Dashboard, Call }
enum Content { Dashboard(DashboardView), Call(CallView) }
struct PaintResources { font: HFONT, background: HBRUSH, input: HBRUSH, bg: COLORREF, fg: COLORREF, accent: COLORREF, input_bg: COLORREF }
impl PaintResources {
    fn new(dpi: u32) -> Result<Self> {
        let bg = color(theme::bg(), theme::bg()); let fg = color(theme::text(), theme::bg());
        let input_bg = color(theme::card_hover(), theme::bg()); let accent = color(theme::accent(), theme::bg());
        let font = unsafe { CreateFontW(-scale(13, dpi), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
            DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI")) };
        if font.is_invalid() { return Err(Error::new(E_FAIL, "Cannot create overlay font")); }
        let background = unsafe { CreateSolidBrush(bg) }; let input = unsafe { CreateSolidBrush(input_bg) };
        let resources = Self { font, background, input, bg, fg, accent, input_bg };
        if background.is_invalid() || input.is_invalid() { return Err(Error::new(E_FAIL, "Cannot create overlay brushes")); }
        Ok(resources)
    }
}
impl Drop for PaintResources {
    fn drop(&mut self) { unsafe {
        if !self.font.is_invalid() { let _ = DeleteObject(HGDIOBJ(self.font.0)); }
        if !self.background.is_invalid() { let _ = DeleteObject(HGDIOBJ(self.background.0)); }
        if !self.input.is_invalid() { let _ = DeleteObject(HGDIOBJ(self.input.0)); }
    } }
}
struct WindowState {
    kind: Kind, destroyed: Cell<bool>, updating: Cell<bool>, initialized: Cell<bool>, dpi: Cell<u32>, queue: Rc<ActionQueue>,
    content: RefCell<Content>, controls: RefCell<Vec<(i32, HWND)>>, paint: RefCell<PaintResources>,
}
impl WindowState {
    fn control(&self, id: i32) -> HWND { self.controls.borrow().iter().find(|(key, _)| *key == id).map(|(_, h)| *h).unwrap_or_default() }
}
struct OverlayWindow { hwnd: HWND, state: Box<WindowState> }
impl OverlayWindow {
    fn create(kind: Kind, work: ScreenRect, queue: Rc<ActionQueue>) -> Result<Self> {
        if QUITTING.with(Cell::get) { return Err(Error::new(E_FAIL, "Application is shutting down")); }
        let state = Box::new(WindowState { kind, destroyed: Cell::new(false), updating: Cell::new(true), initialized: Cell::new(false), dpi: Cell::new(96), queue,
            content: RefCell::new(if kind == Kind::Dashboard { Content::Dashboard(DashboardView::default()) } else { Content::Call(CallView::default()) }),
            controls: RefCell::new(Vec::new()), paint: RefCell::new(PaintResources::new(96)?) });
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: instance.into(), lpszClassName: w!("NotchLiteAssistOverlay"),
                hCursor: LoadCursorW(None, IDC_ARROW)?, ..Default::default() };
            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS { return Err(Error::from_win32()); }
            CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_CONTROLPARENT | WS_EX_TOPMOST, w!("NotchLiteAssistOverlay"),
                if kind == Kind::Dashboard { w!("NotchLite · Codex dashboard") } else { w!("NotchLite · Call assistance") },
                WS_POPUP | WS_BORDER | WS_CLIPCHILDREN, work.left, work.top, 2, 2, None, None, instance,
                Some((&*state as *const WindowState).cast()))?
        };
        let window = Self { hwnd, state };
        WINDOWS.with(|items| items.borrow_mut().push(hwnd));
        window.apply_capture_policy()?;
        create_controls(hwnd, &window.state)?;
        window.state.updating.set(false);
        if QUITTING.with(Cell::get) { return Err(Error::new(E_FAIL, "Application is shutting down")); }
        Ok(window)
    }
    fn hide(&self) { if !self.state.destroyed.get() { unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); } } }
    fn apply_capture_policy(&self) -> Result<()> {
        if self.state.destroyed.get() { return Err(Error::new(E_FAIL, "Overlay closed")); }
        crate::window_policy::apply(self.hwnd)
    }
    fn show(&self) { if !self.state.destroyed.get() && !QUITTING.with(Cell::get) { unsafe { let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); } } }
    fn position(&self, rect: ScreenRect, dpi: u32) -> Result<()> {
        if self.state.destroyed.get() { return Err(Error::new(E_FAIL, "Overlay closed")); }
        if dpi == 0 { return Err(Error::new(E_FAIL, "Overlay DPI is unavailable")); }
        if self.state.dpi.get() != dpi { self.update_resources(dpi)?; }
        unsafe { SetWindowPos(self.hwnd, None, rect.left, rect.top, rect.width(), rect.height(), SWP_NOZORDER | SWP_NOACTIVATE)?; }
        layout_controls(self.hwnd, &self.state);
        Ok(())
    }
    fn palette_changed(&self) -> Result<()> { self.update_resources(self.state.dpi.get()) }
    fn update_resources(&self, dpi: u32) -> Result<()> {
        let next = PaintResources::new(dpi)?;
        // Native children must stop referring to the old HFONT before it drops.
        for (_, child) in self.state.controls.borrow().iter() { unsafe { SendMessageW(*child, WM_SETFONT, WPARAM(next.font.0 as usize), LPARAM(1)); } }
        let previous = self.state.paint.replace(next); drop(previous);
        self.state.dpi.set(dpi);
        unsafe { let _ = InvalidateRect(self.hwnd, None, true); }
        Ok(())
    }
    fn dashboard(&self, view: &DashboardView) { apply_dashboard(self.hwnd, &self.state, view); }
    fn call(&self, view: &CallView) { apply_call(self.hwnd, &self.state, view); }
}
impl Drop for OverlayWindow {
    fn drop(&mut self) {
        if !self.state.destroyed.get() {
            crate::window_policy::unregister(self.hwnd);
            unsafe { let _ = DestroyWindow(self.hwnd); }
        }
        // Child windows are gone before their fonts/brushes and USERDATA Box.
    }
}

fn scale(dip: i32, dpi: u32) -> i32 { ((i64::from(dip) * i64::from(dpi) + 48) / 96).clamp(1, i64::from(i32::MAX)) as i32 }
fn color(fg: crate::gfx::Color, bg: crate::gfx::Color) -> COLORREF {
    let channel = |a: f32, b: f32| ((a * fg.a + b * (1.0 - fg.a)).clamp(0.0, 1.0) * 255.0).round() as u32;
    COLORREF(channel(fg.r, bg.r) | channel(fg.g, bg.g) << 8 | channel(fg.b, bg.b) << 16)
}
fn wide(text: &str) -> Vec<u16> { text.chars().map(|c| if c == '\0' { '\u{FFFD}' } else { c }).collect::<String>().encode_utf16().chain(Some(0)).collect() }
fn set_text(hwnd: HWND, text: &str) { if !hwnd.is_invalid() { let text = wide(text); unsafe { let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr())); } } }
fn visible(hwnd: HWND, show: bool) { if !hwnd.is_invalid() { unsafe { let _ = ShowWindow(hwnd, if show { SW_SHOWNA } else { SW_HIDE }); } } }
fn enabled(hwnd: HWND, allow: bool) { if !hwnd.is_invalid() { unsafe { let _ = EnableWindow(hwnd, allow); } } }
fn text(hwnd: HWND, max: usize) -> Option<String> {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length < 0 || length as usize > max { return None; }
    let mut value = vec![0u16; length as usize + 1];
    let length = unsafe { GetWindowTextW(hwnd, &mut value) };
    String::from_utf16(&value[..length.max(0) as usize]).ok()
}

fn create_controls(hwnd: HWND, state: &WindowState) -> Result<()> {
    let child = |id: i32, class: PCWSTR, value: &str, extra: u32, tab: bool| -> Result<()> {
        let value = wide(value);
        let style = WS_CHILD | WS_VISIBLE | WINDOW_STYLE(extra) | if tab { WS_TABSTOP } else { WINDOW_STYLE(0) };
        let control = unsafe { CreateWindowExW(WINDOW_EX_STYLE(0), class, PCWSTR(value.as_ptr()), style,
            0, 0, 1, 1, hwnd, HMENU(id as usize as *mut core::ffi::c_void), GetModuleHandleW(None)?, None)? };
        state.controls.borrow_mut().push((id, control));
        unsafe { SendMessageW(control, WM_SETFONT, WPARAM(state.paint.borrow().font.0 as usize), LPARAM(0)); }
        Ok(())
    };
    child(99, w!("STATIC"), if state.kind == Kind::Dashboard { "Codex dashboard" } else { "Call preparation" }, 0, false)?;
    child(CLOSE, w!("BUTTON"), "Close", 0, true)?;
    if state.kind == Kind::Dashboard {
        for id in [CONNECTION, USAGE, RESOURCES, STATUS] { child(id, w!("STATIC"), "", 0, false)?; }
        child(THREADS, w!("COMBOBOX"), "", (CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_VSCROLL.0 as i32) as u32, true)?;
        child(DETAIL, w!("EDIT"), "", (ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY) as u32 | WS_VSCROLL.0, true)?;
        child(REPLY, w!("EDIT"), "", (ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32 | WS_VSCROLL.0, true)?;
        child(WORKSPACE_LABEL, w!("STATIC"), "Cartella", 0, false)?;
        child(WORKSPACE, w!("EDIT"), "", ES_AUTOHSCROLL as u32, true)?;
        child(START_SERVER, w!("BUTTON"), "Avvia server locale", 0, true)?;
        child(NEW_CHAT, w!("BUTTON"), "Nuova chat", 0, true)?;
        unsafe { SendMessageW(state.control(WORKSPACE), EM_SETLIMITTEXT, WPARAM(2048), LPARAM(0)); }
        for i in 0..MAX_PROMPTS {
            child(QUESTION_LABEL + i as i32, w!("STATIC"), "", 0, false)?;
            child(QUESTION_INPUT + i as i32, w!("COMBOBOX"), "", (CBS_DROPDOWN | CBS_HASSTRINGS) as u32 | WS_VSCROLL.0, true)?;
            unsafe { SendMessageW(state.control(QUESTION_INPUT + i as i32), CB_LIMITTEXT, WPARAM(MAX_ANSWER), LPARAM(0)); }
        }
        for (id, label) in [(SEND, "Send"), (OPEN_CODEX, "Open Codex"), (REFRESH, "Refresh"), (SHOW_CALL, "Calls")] { child(id, w!("BUTTON"), label, 0, true)?; }
        unsafe { SendMessageW(state.control(REPLY), EM_SETLIMITTEXT, WPARAM(MAX_FOLLOWUP), LPARAM(0)); }
    } else {
        for id in [CALL_STATUS, CALL_SCOPE, CALL_PROVIDER] { child(id, w!("STATIC"), "", 0, false)?; }
        child(CALL_TARGET, w!("COMBOBOX"), "", (CBS_DROPDOWNLIST | CBS_HASSTRINGS) as u32 | WS_VSCROLL.0, true)?;
        child(CALL_CONTEXT_LABEL, w!("STATIC"), "Obiettivo per l'AI (facoltativo)", 0, false)?;
        child(CALL_CONTEXT, w!("EDIT"), "", ES_AUTOHSCROLL as u32, true)?;
        unsafe { SendMessageW(state.control(CALL_CONTEXT), EM_SETLIMITTEXT, WPARAM(1000), LPARAM(0)); }
        for id in [CALL_TRANSCRIPT, CALL_SUGGESTION] { child(id, w!("EDIT"), "", (ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY) as u32 | WS_VSCROLL.0, true)?; }
        for (id, label) in [(CONFIRM, "Trascrivi"), (CONFIRM_AI, "Trascrivi + AI"), (DECLINE, "Ignora"), (STOP, "Stop")] { child(id, w!("BUTTON"), label, 0, true)?; }
    }
    layout_controls(hwnd, state);
    Ok(())
}

fn valid_question(form: &QuestionForm) -> bool {
    !form.questions.is_empty() && form.questions.len() <= MAX_PROMPTS
        && form.questions.iter().all(|p| !p.id.is_empty() && p.options.len() <= 64 && p.options.iter().all(|s| s.encode_utf16().count() <= 512))
        && form.questions.iter().enumerate().all(|(i, p)| !form.questions[..i].iter().any(|previous| previous.id == p.id))
}

fn same_input_context(previous: &DashboardView, next: &DashboardView) -> bool {
    previous.selected_id == next.selected_id
        && previous.question.as_ref().map(|form| &form.key) == next.question.as_ref().map(|form| &form.key)
        && previous.question.as_ref().map(|form| form.questions.iter().map(|prompt| &prompt.id).collect::<Vec<_>>())
            == next.question.as_ref().map(|form| form.questions.iter().map(|prompt| &prompt.id).collect::<Vec<_>>())
}

fn layout_controls(hwnd: HWND, state: &WindowState) {
    let mut client = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() { return; }
    let dpi = state.dpi.get(); let px = |value| scale(value, dpi);
    let margin = px(12); let gap = px(4); let width = (client.right - margin * 2).max(1);
    let mut y = margin;
    let place = |id, x: i32, top: i32, w: i32, h: i32| {
        let child = state.control(id);
        if !child.is_invalid() { unsafe { let _ = SetWindowPos(child, None, x, top, w.max(1), h.max(1), SWP_NOZORDER | SWP_NOACTIVATE); } }
    };
    let was_updating = state.updating.replace(true);
    place(99, margin, y, (width - px(62)).max(1), px(24));
    place(CLOSE, client.right - margin - px(58), y, px(58), px(24)); y += px(28);
    if state.kind == Kind::Dashboard {
        for (id, height) in [(CONNECTION, 36), (USAGE, 32), (RESOURCES, 32)] {
            place(id, margin, y, width, px(height)); y += px(height) + gap;
        }
        place(WORKSPACE_LABEL, margin, y, px(54), px(24));
        place(WORKSPACE, margin + px(58), y, width - px(58), px(24)); y += px(28);
        let setup_w = (width - gap) / 2;
        place(START_SERVER, margin, y, setup_w, px(26));
        place(NEW_CHAT, margin + setup_w + gap, y, setup_w, px(26)); y += px(30);
        place(THREADS, margin, y, width, px(240)); y += px(28);
        place(STATUS, margin, y, width, px(18)); y += px(22);
        let content = state.content.borrow();
        let form = if let Content::Dashboard(view) = &*content { view.question.as_ref().filter(|f| valid_question(f)) } else { None };
        let questions = form.map(|f| f.questions.len()).unwrap_or(0);
        let answer_h = if questions > 0 { px(50) * questions as i32 } else { px(72) };
        let buttons_y = client.bottom - margin - px(28);
        let answer_y = buttons_y - gap - answer_h;
        place(DETAIL, margin, y, width, (answer_y - y - gap).max(px(32)));
        visible(state.control(REPLY), questions == 0);
        place(REPLY, margin, answer_y, width, answer_h);
        for i in 0..MAX_PROMPTS {
            let show = i < questions;
            visible(state.control(QUESTION_LABEL + i as i32), show);
            visible(state.control(QUESTION_INPUT + i as i32), show);
            let row_y = answer_y + px(50) * i as i32;
            place(QUESTION_LABEL + i as i32, margin, row_y, width, px(22));
            place(QUESTION_INPUT + i as i32, margin, row_y + px(24), width, px(200));
        }
        let button_w = ((width - gap * 3) / 4).max(1);
        for (i, id) in [SEND, OPEN_CODEX, REFRESH, SHOW_CALL].into_iter().enumerate() { place(id, margin + (button_w + gap) * i as i32, buttons_y, button_w, px(28)); }
    } else {
        for (id, height) in [(CALL_STATUS, 28), (CALL_SCOPE, 32), (CALL_PROVIDER, 42)] {
            place(id, margin, y, width, px(height)); y += px(height) + gap;
        }
        place(CALL_TARGET, margin, y, width, px(220)); y += px(32);
        place(CALL_CONTEXT_LABEL, margin, y, width, px(20)); y += px(20);
        place(CALL_CONTEXT, margin, y, width, px(24)); y += px(28);
        let buttons_y = client.bottom - margin - px(28);
        let mode_y = buttons_y - gap - px(28);
        let available = (mode_y - y - gap * 2).max(2);
        let first = available / 2;
        place(CALL_TRANSCRIPT, margin, y, width, first);
        place(CALL_SUGGESTION, margin, y + first + gap, width, (available - first).max(1));
        let button_w = ((width - gap) / 2).max(1);
        for (i, id) in [CONFIRM, CONFIRM_AI].into_iter().enumerate() { place(id, margin + (button_w + gap) * i as i32, mode_y, button_w, px(28)); }
        for (i, id) in [DECLINE, STOP].into_iter().enumerate() { place(id, margin + (button_w + gap) * i as i32, buttons_y, button_w, px(28)); }
    }
    state.updating.set(was_updating);
}

fn combo_items(hwnd: HWND, values: impl IntoIterator<Item = String>, selected: Option<usize>) {
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, WPARAM(0), LPARAM(0));
        for value in values { let value = wide(&value); SendMessageW(hwnd, CB_ADDSTRING, WPARAM(0), LPARAM(value.as_ptr() as isize)); }
        SendMessageW(hwnd, CB_SETCURSEL, WPARAM(selected.unwrap_or(usize::MAX)), LPARAM(0));
    }
}

fn readonly_text(hwnd: HWND, value: &str) {
    let previous = unsafe { SendMessageW(hwnd, EM_GETFIRSTVISIBLELINE, WPARAM(0), LPARAM(0)).0 };
    set_text(hwnd, value);
    if previous > 0 { unsafe { SendMessageW(hwnd, EM_LINESCROLL, WPARAM(0), LPARAM(previous)); } }
}

fn apply_dashboard(hwnd: HWND, state: &WindowState, view: &DashboardView) {
    if state.destroyed.get() { return; }
    let previous = { let content = state.content.borrow(); if let Content::Dashboard(v) = &*content { v.clone() } else { DashboardView::default() } };
    if state.initialized.get() && previous == *view { return; }
    state.updating.set(true);
    set_text(state.control(CONNECTION), &view.connection);
    // A refresh must never overwrite a path the user is editing.
    if !state.initialized.get() { set_text(state.control(WORKSPACE), &view.workspace); }
    enabled(state.control(START_SERVER), view.can_start_server);
    enabled(state.control(NEW_CHAT), view.can_create_thread);
    set_text(state.control(USAGE), &format!("Codex tokens / limits: {}", view.usage));
    set_text(state.control(RESOURCES), &format!("Process RAM / CPU (shared): {}", view.resources));
    if previous.rows != view.rows || previous.selected_id != view.selected_id {
        let selected = view.rows.iter().position(|r| Some(&r.id) == view.selected_id.as_ref());
        combo_items(state.control(THREADS), view.rows.iter().map(|r| format!("{}{} · {}", if r.attention { "● " } else { "" }, r.title, r.statuslabel)), selected);
    }
    let selected = view.rows.iter().find(|r| Some(&r.id) == view.selected_id.as_ref());
    set_text(state.control(STATUS), selected.map(|r| r.statuslabel.as_str()).unwrap_or("Select a Codex chat"));
    let mut detail = format!("{}\r\n\r\n{}", view.connection, view.detail);
    if let Some(notice) = &view.approval_notice { detail.push_str("\r\n\r\nApproval in Codex: "); detail.push_str(&bounded(notice, 4096)); }
    if let Some(form) = &view.question {
        detail.push_str("\r\n\r\nQuestions:\r\n");
        for (index, prompt) in form.questions.iter().take(MAX_PROMPTS).enumerate() { detail.push_str(&format!("{}. {}\r\n", index + 1, bounded(&prompt.text, 8192))); }
        if !valid_question(form) { detail.push_str("This request cannot be answered here. Open Codex to continue."); }
    }
    let same_context = same_input_context(&previous, view);
    if previous.connection != view.connection || previous.detail != view.detail || previous.approval_notice != view.approval_notice || previous.question != view.question { readonly_text(state.control(DETAIL), &detail); }
    let valid_form = view.question.as_ref().filter(|form| valid_question(form) && view.selected_id.as_deref() == Some(form.key.thread_id.as_str()));
    if previous.question != view.question || !same_context {
        for index in 0..MAX_PROMPTS {
            let input = state.control(QUESTION_INPUT + index as i32);
            if let Some(prompt) = valid_form.and_then(|f| f.questions.get(index)) {
                let answer = if same_context { text(input, MAX_ANSWER).unwrap_or_default() } else { String::new() };
                set_text(state.control(QUESTION_LABEL + index as i32), &format!("{}. {}", index + 1, bounded(&prompt.text, 180)));
                combo_items(input, prompt.options.iter().take(64).map(|s| bounded(s, 512)), None);
                set_text(input, &answer);
            } else { set_text(input, ""); }
        }
    }
    if !same_context { set_text(state.control(REPLY), ""); }
    let may_send = if view.question.is_some() { view.can_answer && valid_form.is_some() } else { view.can_followup && view.selected_id.is_some() && view.approval_notice.is_none() };
    for index in 0..MAX_PROMPTS { enabled(state.control(QUESTION_INPUT + index as i32), view.can_answer); }
    set_text(state.control(SEND), if view.question.is_some() { "Answer" } else { "Send" });
    enabled(state.control(SEND), may_send);
    enabled(state.control(REPLY), view.question.is_none() && may_send);
    enabled(state.control(OPEN_CODEX), view.selected_id.is_some());
    state.content.replace(Content::Dashboard(view.clone()));
    state.initialized.set(true);
    layout_controls(hwnd, state);
    state.updating.set(false);
    unsafe { let _ = InvalidateRect(hwnd, None, false); }
}

fn apply_call(hwnd: HWND, state: &WindowState, view: &CallView) {
    if state.destroyed.get() { return; }
    let previous = { let content = state.content.borrow(); if let Content::Call(v) = &*content { v.clone() } else { CallView::default() } };
    if state.initialized.get() && previous == *view { return; }
    state.updating.set(true);
    set_text(state.control(CALL_STATUS), &format!("{} · {}", if view.listening { "LISTENING" } else { "Nessuna acquisizione audio" }, view.status));
    set_text(state.control(CALL_SCOPE), &view.scope);
    set_text(state.control(CALL_PROVIDER), &view.provider);
    if previous.candidates != view.candidates || previous.candidate_key != view.candidate_key {
        combo_items(state.control(CALL_TARGET), view.candidates.iter().map(|v| v.label.clone()), view.candidates.iter().position(|v| Some(v.key) == view.candidate_key));
    }
    enabled(state.control(CALL_TARGET), !view.listening && !view.candidates.is_empty());
    enabled(state.control(CALL_CONTEXT), !view.listening);
    if previous.transcript != view.transcript { readonly_text(state.control(CALL_TRANSCRIPT), &view.transcript); }
    if previous.suggestion != view.suggestion { readonly_text(state.control(CALL_SUGGESTION), &view.suggestion); }
    enabled(state.control(CONFIRM), view.can_listen && !view.listening && view.candidate_key.is_some());
    enabled(state.control(CONFIRM_AI), view.can_listen && !view.listening && view.candidate_key.is_some());
    enabled(state.control(DECLINE), !view.listening && view.candidate_key.is_some());
    state.content.replace(Content::Call(view.clone()));
    state.initialized.set(true);
    layout_controls(hwnd, state);
    state.updating.set(false);
    unsafe { let _ = InvalidateRect(hwnd, None, false); }
}

fn call_action(view: &CallView, id: i32) -> Option<OverlayAction> {
    match id {
        CONFIRM | CONFIRM_AI if view.can_listen && !view.listening => view.candidate_key.map(|candidate_key| OverlayAction::ConfirmCall { candidate_key, context: String::new(), with_ai: id == CONFIRM_AI }),
        DECLINE if !view.listening => view.candidate_key.map(|candidate_key| OverlayAction::DeclineCall { candidate_key }),
        STOP => Some(OverlayAction::StopCall),
        _ => None,
    }
}

fn command(state: &WindowState, id: i32, notification: u32) {
    if state.updating.get() { return; }
    if id == CLOSE && notification == BN_CLICKED { close_action(state); return; }
    let content = state.content.borrow();
    match &*content {
        Content::Dashboard(view) => {
            if id == THREADS && notification == CBN_SELCHANGE {
                let index = unsafe { SendMessageW(state.control(THREADS), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 };
                if let Some(row) = usize::try_from(index).ok().and_then(|i| view.rows.get(i)) { state.queue.push(OverlayAction::SelectThread(row.id.clone())); }
                return;
            }
            if notification != BN_CLICKED { return; }
            match id {
                START_SERVER if view.can_start_server => { state.queue.push(OverlayAction::StartSharedServer); }
                NEW_CHAT if view.can_create_thread => {
                    if let Some(workspace) = text(state.control(WORKSPACE), 2048).filter(|s| !s.trim().is_empty()) {
                        state.queue.push(OverlayAction::CreateThread { workspace });
                    } else { set_text(state.control(CONNECTION), "Inserisci il percorso assoluto della cartella del progetto."); }
                }
                REFRESH => { state.queue.push(OverlayAction::Refresh); }
                SHOW_CALL => { state.queue.push(OverlayAction::ShowCallPanel); }
                OPEN_CODEX => { if let Some(thread_id) = &view.selected_id { state.queue.push(OverlayAction::OpenCodex { thread_id: thread_id.clone() }); } }
                SEND => {
                    if let Some(form) = &view.question {
                        if !view.can_answer { return; }
                        if !valid_question(form) || view.selected_id.as_deref() != Some(form.key.thread_id.as_str()) { return; }
                        let mut answers = BTreeMap::new();
                        for (index, prompt) in form.questions.iter().enumerate() {
                            let Some(answer) = text(state.control(QUESTION_INPUT + index as i32), MAX_ANSWER).filter(|s| !s.trim().is_empty()) else {
                                set_text(state.control(CONNECTION), "Answer each question, or open Codex to continue."); return;
                            };
                            answers.insert(prompt.id.clone(), vec![answer]);
                        }
                        state.queue.push(OverlayAction::AnswerQuestion { key: form.key.clone(), answers });
                    } else if view.can_followup && view.approval_notice.is_none() {
                        if let (Some(thread_id), Some(text)) = (&view.selected_id, text(state.control(REPLY), MAX_FOLLOWUP).filter(|s| !s.trim().is_empty())) {
                            state.queue.push(OverlayAction::SendFollowup { thread_id: thread_id.clone(), text });
                        }
                    }
                }
                _ => {}
            }
        }
        Content::Call(view) => {
            if id == CALL_TARGET && notification == CBN_SELCHANGE && !view.listening {
                let index = unsafe { SendMessageW(state.control(CALL_TARGET), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 };
                if let Some(target) = usize::try_from(index).ok().and_then(|i| view.candidates.get(i)) { state.queue.push(OverlayAction::SelectCallTarget { target_key: target.key }); }
            } else if notification == BN_CLICKED { if let Some(mut action) = call_action(view, id) {
                if let OverlayAction::ConfirmCall { context, .. } = &mut action {
                    let Some(value) = text(state.control(CALL_CONTEXT), 1000) else { return; };
                    *context = value;
                }
                state.queue.push(action);
            } }
        }
    }
}

fn close_action(state: &WindowState) {
    if state.kind == Kind::Call { state.queue.push(OverlayAction::StopCall); state.queue.push(OverlayAction::CloseCall); }
    else { state.queue.push(OverlayAction::CloseDashboard); }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttentionTone { Normal, Question, Complete, Message }
fn attention_tone(row: &ChatRow) -> AttentionTone {
    if !row.attention { return AttentionTone::Normal; }
    let text = row.statuslabel.to_ascii_lowercase();
    if matches!(text.as_str(), "question"|"waiting input"|"waitinginput"|"domanda"|"awaiting input"|"autorizzazione in codex") { AttentionTone::Question }
    else if matches!(text.as_str(), "complete"|"completed"|"done"|"idle"|"completata"|"terminata"|"conclusa"|"concluso") { AttentionTone::Complete }
    else { AttentionTone::Message }
}
fn attention_color(row: &ChatRow, normal: COLORREF) -> COLORREF {
    match attention_tone(row) {
        AttentionTone::Normal => normal,
        AttentionTone::Question => color(theme::warn(), theme::bg()),
        AttentionTone::Complete => COLORREF(0x005CDB33),
        AttentionTone::Message => color(theme::blue(), theme::bg()),
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let Some(create) = (lparam.0 as *const CREATESTRUCTW).as_ref() else { return LRESULT(0) };
        if create.lpCreateParams.is_null() { return LRESULT(0); }
        SetLastError(ERROR_SUCCESS);
        if SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize) == 0 && GetLastError() != ERROR_SUCCESS { return LRESULT(0); }
        return LRESULT(1);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    let Some(state) = pointer.as_ref() else { return DefWindowProcW(hwnd, message, wparam, lparam) };
    match message {
        WM_NCDESTROY => {
            crate::window_policy::unregister(hwnd);
            if state.kind == Kind::Call { state.queue.push(OverlayAction::StopCall); }
            state.destroyed.set(true); SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            WINDOWS.with(|items| items.borrow_mut().retain(|h| *h != hwnd));
        }
        WM_CLOSE => { close_action(state); let _ = DestroyWindow(hwnd); return LRESULT(0); }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32; let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
            if id == CLOSE && notification == BN_CLICKED { close_action(state); let _ = DestroyWindow(hwnd); }
            else { command(state, id, notification); }
            return LRESULT(0);
        }
        WM_SIZE => { layout_controls(hwnd, state); return LRESULT(0); }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_DPICHANGED => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            mark_topology_dirty(&state.queue); return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default(); let dc = BeginPaint(hwnd, &mut ps);
            if !dc.is_invalid() { let mut rect = RECT::default(); if GetClientRect(hwnd, &mut rect).is_ok() { FillRect(dc, &rect, state.paint.borrow().background); } }
            let _ = EndPaint(hwnd, &ps); return LRESULT(0);
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            let dc = HDC(wparam.0 as *mut core::ffi::c_void);
            let control = HWND(lparam.0 as *mut core::ffi::c_void);
            let id = GetDlgCtrlID(control);
            let resources = state.paint.borrow();
            let is_input = message == WM_CTLCOLOREDIT || message == WM_CTLCOLORLISTBOX || matches!(id, DETAIL|REPLY|CALL_TRANSCRIPT|CALL_SUGGESTION);
            let mut ink = resources.fg;
            if id == STATUS { if let Ok(content) = state.content.try_borrow() { if let Content::Dashboard(view) = &*content {
                if let Some(row) = view.rows.iter().find(|r| Some(&r.id) == view.selected_id.as_ref()) { ink = attention_color(row, ink); }
            } } }
            if id == CALL_STATUS { if let Ok(content) = state.content.try_borrow() { if let Content::Call(view) = &*content { if view.listening { ink = COLORREF(0x005858FF); } } } }
            SetTextColor(dc, ink); SetBkColor(dc, if is_input { resources.input_bg } else { resources.bg });
            return LRESULT(if is_input { resources.input.0 } else { resources.background.0 } as isize);
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_bridge::RequestId;
    fn key() -> AnswerKey { AnswerKey { epoch: 1, request_id: RequestId::Number(3), thread_id: "thread".into(), turn_id: "turn".into() } }
    #[test]
    fn disabled_manager_has_no_windows_queue_or_retained_provider_data() {
        let mut manager = OverlayManager::new();
        manager.apply_dashboard(DashboardView { detail: "unused provider data".into(), ..Default::default() });
        manager.apply_call(CallView { visible: true, ..Default::default() });
        assert!(manager.dashboard.is_none() && manager.call.is_none() && manager.queue.is_none());
        assert!(manager.dashboard_view.detail.is_empty());
        assert_eq!(manager.sync(OverlayConfig::default()).unwrap(), OverlayAvailability::Off);
        assert!(manager.drain_actions().is_empty());
    }
    #[test]
    fn stop_is_never_dropped_and_removes_queued_call_start_actions() {
        let queue = ActionQueue { actions: RefCell::new(VecDeque::new()), wake: Cell::new(HWND::default()) };
        queue.push(OverlayAction::ConfirmCall { candidate_key: 1, context: String::new(), with_ai: true });
        while queue.actions.borrow().len() < MAX_ACTIONS { queue.push(OverlayAction::Refresh); }
        assert!(!queue.push(OverlayAction::SelectThread("overflow".into())));
        assert!(queue.push(OverlayAction::StopCall));
        assert!(queue.push(OverlayAction::CloseCall));
        let actions = queue.actions.borrow();
        assert!(matches!(actions.front(), Some(OverlayAction::StopCall)));
        assert!(!actions.iter().any(|a| matches!(a, OverlayAction::ConfirmCall { .. })));
        assert!(actions.iter().any(|a| matches!(a, OverlayAction::CloseCall)));
    }
    #[test]
    fn unsupported_or_ambiguous_question_forms_cannot_emit_partial_answers() {
        let mut form = QuestionForm { key: key(), questions: vec![Prompt { id: "a".into(), text: "Question".into(), options: vec!["Choice".into()] }] };
        assert!(valid_question(&form));
        form.questions.push(form.questions[0].clone()); assert!(!valid_question(&form));
        form.questions.clear(); assert!(!valid_question(&form));
        for i in 0..4 { form.questions.push(Prompt { id: i.to_string(), text: String::new(), options: Vec::new() }); }
        assert!(!valid_question(&form));
    }
    #[test]
    fn confirmation_requires_capability_and_a_current_candidate() {
        let mut view = CallView { candidate_key: Some(42), ..Default::default() };
        assert!(call_action(&view, CONFIRM).is_none());
        view.can_listen = true;
        assert_eq!(call_action(&view, CONFIRM), Some(OverlayAction::ConfirmCall { candidate_key: 42, context: String::new(), with_ai: false }));
        assert_eq!(call_action(&view, CONFIRM_AI), Some(OverlayAction::ConfirmCall { candidate_key: 42, context: String::new(), with_ai: true }));
        view.listening = true; assert!(call_action(&view, CONFIRM).is_none());
        assert_eq!(call_action(&view, STOP), Some(OverlayAction::StopCall));
    }
    #[test]
    fn failed_window_update_closes_provider_views_and_requests_stop() {
        let mut manager = OverlayManager::new();
        manager.config.call_panel = true; manager.config.codex_dashboard = true;
        manager.availability = OverlayAvailability::Ready;
        manager.call_view = CallView { visible: true, listening: true, transcript: "Synthetic private text".into(), ..Default::default() };
        manager.dashboard_view.detail = "Synthetic chat".into();
        manager.close_after_error();
        assert_eq!(manager.availability, OverlayAvailability::Off);
        assert!(manager.call.is_none() && manager.dashboard.is_none());
        assert_eq!(manager.call_view, CallView::default());
        assert_eq!(manager.dashboard_view, DashboardView::default());
        assert!(manager.drain_actions().iter().any(|action| matches!(action, OverlayAction::StopCall)));
        assert!(manager.config.call_panel && manager.config.codex_dashboard);
    }
    #[test]
    fn an_invisible_interval_rearms_a_later_call_with_the_same_identity() {
        let mut manager = OverlayManager::new(); manager.config.call_panel = true;
        manager.suppressed_call = Some(Some(7));
        manager.apply_call(CallView { visible: false, candidate_key: Some(7), ..Default::default() });
        assert!(manager.suppressed_call.is_none());
        take_topology_dirty();
        manager.apply_call(CallView { visible: true, candidate_key: Some(7), ..Default::default() });
        assert!(take_topology_dirty());
    }
    #[test]
    fn readonly_content_is_bounded_and_keeps_the_selected_chat_visible() {
        let rows = (0..200).map(|i| ChatRow { id: i.to_string(), title: "title".into(), ..Default::default() }).collect();
        let view = bounded_dashboard(DashboardView { rows, selected_id: Some("199".into()), detail: "x".repeat(70000), ..Default::default() });
        assert_eq!(view.rows.len(), MAX_ROWS);
        assert!(view.rows.iter().any(|r| r.id == "199"));
        assert_eq!(view.detail.len(), 65536);
        assert_eq!(bounded("a\0b", 10), "a\u{FFFD}b");
    }
    #[test]
    fn monitor_choices_are_sorted_unique_and_never_primary() {
        let work = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        let monitors = [("primary", true), ("right", false), ("left", false), ("left", false)]
            .into_iter().map(|(device, primary)| Monitor { device: device.into(), primary, work }).collect();
        assert_eq!(secondary_device_names(monitors), vec!["left", "right"]);
        assert!(secondary_device_names(Vec::new()).is_empty());
    }
    #[test]
    fn dpi_topology_signal_is_consumed_once_without_monitor_enumeration() {
        TOPOLOGY_DIRTY.with(|flag| flag.set(true));
        assert!(take_topology_dirty()); assert!(!take_topology_dirty());
    }
    #[test]
    fn call_visibility_requests_coordinator_layout_and_never_rearms_a_closed_candidate() {
        let mut manager = OverlayManager::new();
        manager.config.call_panel = true;
        let view = CallView { visible: true, candidate_key: Some(7), ..Default::default() };
        manager.apply_call(view.clone());
        assert!(take_topology_dirty());
        assert!(manager.call.is_none());
        assert_eq!(manager.availability, OverlayAvailability::Off);
        manager.suppressed_call = Some(Some(7));
        for _ in 0..5 { manager.apply_call(view.clone()); assert!(!take_topology_dirty()); }
        manager.apply_call(CallView { candidate_key: Some(8), ..view });
        assert!(take_topology_dirty());
    }
    #[test]
    fn input_context_changes_only_for_thread_request_or_prompt_identity() {
        let mut previous = DashboardView { selected_id: Some("thread".into()),
            question: Some(QuestionForm { key: key(), questions: vec![Prompt { id: "p".into(), text: "Question".into(), options: Vec::new() }] }), ..Default::default() };
        let mut next = previous.clone(); next.detail = "New message".into(); next.usage = "Updated metrics".into();
        assert!(same_input_context(&previous, &next));
        next.question.as_mut().unwrap().questions[0].text = "Updated label".into();
        assert!(same_input_context(&previous, &next));
        next.question.as_mut().unwrap().key.epoch += 1;
        assert!(!same_input_context(&previous, &next));
        next = previous.clone(); next.selected_id = Some("another thread".into());
        assert!(!same_input_context(&previous, &next));
        next = previous.clone(); next.question.as_mut().unwrap().questions[0].id = "different prompt".into();
        assert!(!same_input_context(&previous, &next));
        previous.question = None; next.question = None; next.selected_id = previous.selected_id.clone();
        assert!(same_input_context(&previous, &next));
    }
    #[test]
    fn localized_attention_colors_distinguish_question_completion_and_message() {
        let mut row = ChatRow { attention: true, ..Default::default() };
        for (label, expected) in [("Domanda", AttentionTone::Question), ("Conclusa", AttentionTone::Complete), ("In corso", AttentionTone::Message)] {
            row.statuslabel = label.into(); assert_eq!(attention_tone(&row), expected);
        }
        row.attention = false; assert_eq!(attention_tone(&row), AttentionTone::Normal);
    }
}
