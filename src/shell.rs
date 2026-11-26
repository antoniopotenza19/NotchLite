// Modified for NotchLite: isolated identity, opt-in mascot timer, 16 ms transitions,
// reentrant shutdown, Unicode input, page-edit lifecycle, fullscreen rendering suspension
// and explicit opening of the real panel from the executable or tray menu.
//! The notch window: geometry and animation, header, page navigation, input
//! routing, file drops and the tray icon.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{RegisterDragDrop, RevokeDragDrop};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::*;
use windows::Win32::System::SystemInformation::{GetLocalTime, GetTickCount};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::anim::Spring;
use crate::desktop_clock;
use crate::droptarget::{self, DropData, DropHandler};
use crate::gfx::{Align, DrawCtx, Gfx, Point, Rect, TextStyle, Weight};
use crate::mascot::Mascot;
use crate::navigation;
use crate::pages::{self, Host, Mouse, Page};
use crate::store;
use crate::theme;
use crate::worker::{self, Peek, ShellCmd, Waker, WM_APP_CMD, WM_APP_WAKE};

const WM_APP_TRAY: u32 = WM_APP + 2;
const WM_APP_OPEN: u32 = WM_APP + 4;
const WM_MOUSELEAVE: u32 = 0x02A3;
const TIMER_ANIM: usize = 1;
const TIMER_TICK: usize = 2;
const TIMER_COLLAPSE: usize = 3;
const TIMER_MASCOT: usize = 4;
const TIMER_PEEK: usize = 5;
const MENU_STARTUP: usize = 1;
const MENU_EXIT: usize = 2;
const MENU_MASCOT: usize = 3;
const MENU_MINIMAL: usize = 4;
const MENU_OPEN: usize = 5;
const SETTINGS_FILE: &str = "settings.txt";
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("NotchLite");

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

struct Battery {
    percent: Option<u8>,
    plugged: bool,
}

/// WM_CHAR carries UTF-16 code units, not Unicode scalar values.
#[derive(Default)]
struct Utf16Input {
    high_surrogate: Option<u16>,
}

impl Utf16Input {
    fn push(&mut self, unit: u16) -> Option<char> {
        match unit {
            0xD800..=0xDBFF => {
                self.high_surrogate = Some(unit);
                None
            }
            0xDC00..=0xDFFF => self.high_surrogate.take().and_then(|high| {
                let scalar = 0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                char::from_u32(scalar)
            }),
            _ => {
                self.high_surrogate = None;
                char::from_u32(u32::from(unit))
            }
        }
    }
}

fn read_battery() -> Battery {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe {
        let _ = GetSystemPowerStatus(&mut s);
    }
    let has_battery = s.BatteryFlag != 128 && s.BatteryFlag != 255;
    Battery {
        percent: (has_battery && s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
        plugged: s.ACLineStatus == 1,
    }
}

fn report_clock_error() {
    worker::peek(Peek {
        icon: "\u{E7BA}".to_string(),
        emoji: false,
        accent: theme::warn(),
        title: "Desktop clock unavailable".to_string(),
        body: "The notch is still available. Retry in Settings.".to_string(),
        seconds: 6.0,
    });
}

struct App {
    hwnd: HWND,
    gfx: Gfx,
    scale: f32,
    /// 0 = collapsed, 1 = expanded panel (drives the expanded content fade).
    expand: Spring,
    /// Notch size in DIPs; targets come from `update_targets`.
    width: Spring,
    height: Spring,
    /// 0..1 fade of the peek content.
    peek_t: Spring,
    page_shift: Spring,
    last_frame: Instant,
    animating: bool,
    /// Current TIMER_ANIM period: 16 ms while springs move, ~30 fps for ambient
    /// animation only (audio bars, mascot).
    anim_interval: u32,
    /// Last window region applied, to skip redundant SetWindowRgn calls.
    last_region: (i32, i32, i32),
    /// True while hidden because a fullscreen app (game, video, slideshow) is in front.
    hidden_for_fullscreen: bool,
    /// An explicit user request keeps the real panel open until Escape/collapse.
    explicit_panel: bool,
    hovering: bool,
    keyboard: bool,
    input_utf16: Utf16Input,
    pages: Vec<Box<dyn Page>>,
    /// Feature id of each entry in `pages` (same order).
    page_ids: Vec<&'static str>,
    current: usize,
    nav_hover: Option<usize>,
    battery: Battery,
    minute: u16,
    mascot: Mascot,
    mascot_enabled: bool,
    mascot_last: Instant,
    waker: Waker,
    minimal: bool,
    disabled: Vec<String>,
    phone_enabled: bool,
    assistant: crate::assist::Assistant,
    /// Index of the page whose live activity the collapsed notch shows.
    activity: Option<usize>,
    peeks: VecDeque<Peek>,
    /// The peek currently on screen (None once its time is up).
    peek: Option<Peek>,
    /// Last shown peek, kept for drawing while it fades out.
    peek_display: Option<Peek>,
    /// True while an OLE drag (files or a link) is over the notch.
    dragging: bool,
    /// True while a page asks to stay in view (`Page::keep_open`, e.g. a live match).
    pinned: bool,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

impl App {
    /// Panel rectangle in window DIPs for the current expansion.
    fn panel(&self) -> Rect {
        let (w, h) = (self.width.value.max(1.0), self.height.value.max(1.0));
        Rect::new((theme::WINDOW_W - w) / 2.0, 0.0, w, h)
    }

    /// Height-based expansion (0 collapsed .. 1 expanded) used for the corner radii.
    fn height_t(&self) -> f32 {
        ((self.height.value - theme::COLLAPSED_H) / (theme::EXPANDED_H - theme::COLLAPSED_H)).max(0.0)
    }

    /// Picks the notch size for the current state and animates toward it.
    fn update_targets(&mut self) {
        let (w, h) = if self.expand.target > 0.5 {
            (theme::EXPANDED_W, theme::EXPANDED_H)
        } else if self.peek.is_some() {
            (theme::PEEK_W, theme::PEEK_H)
        } else {
            let (bw, bh) = if self.minimal {
                (theme::MINIMAL_W, theme::MINIMAL_H)
            } else {
                (theme::COLLAPSED_W, theme::COLLAPSED_H)
            };
            match self.activity.and_then(|i| self.pages.get(i)).and_then(|p| p.activity()) {
                Some(a) => (a.width.max(bw), theme::COLLAPSED_H),
                None => (bw, bh),
            }
        };
        let peek = if self.peek.is_some() && self.expand.target < 0.5 { 1.0 } else { 0.0 };
        if self.width.target != w || self.height.target != h || self.peek_t.target != peek {
            self.width.target = w;
            self.height.target = h;
            self.peek_t.target = peek;
            self.start_animation();
        }
    }

    /// Re-evaluates which page's live activity to show.
    fn refresh_activity(&mut self) {
        self.activity = self
            .pages
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.activity().map(|a| (i, a.priority)))
            .max_by_key(|&(_, priority)| priority)
            .map(|(i, _)| i);
        self.update_pin();
        self.update_targets();
        if self.activity_animating() {
            self.start_animation();
        }
    }

    /// Follows `Page::keep_open`: switch to that page and stay expanded while it
    /// asks, then collapse unless the user is still on the notch.
    fn update_pin(&mut self) {
        let want = self.pages.iter().position(|p| p.keep_open());
        match (want, self.pinned) {
            (Some(i), false) => {
                self.pinned = true;
                // Don't pull the page away from under someone typing.
                if !self.keyboard {
                    self.go_to(i);
                }
                self.set_expanded(true);
            }
            (None, true) => {
                self.pinned = false;
                // Peeks that queued up while expanded are stale (e.g. goals already
                // seen on the scoreboard); keep only the latest, such as "Full time".
                while self.peeks.len() > 1 {
                    self.peeks.pop_front();
                }
                if !self.explicit_panel && !self.hovering && !self.keyboard && !self.dragging && !self.cursor_inside() {
                    self.set_expanded(false);
                }
            }
            _ => {}
        }
    }

    fn activity_animating(&self) -> bool {
        self.expand.target < 0.5 && self.activity.is_some_and(|i| self.pages[i].activity_animating())
    }

    fn show_next_peek(&mut self) {
        if self.peek.is_some() || self.expand.target > 0.5 {
            return;
        }
        if let Some(p) = self.peeks.pop_front() {
            let ms = (p.seconds.clamp(1.0, 30.0) * 1000.0) as u32;
            self.peek_display = Some(p.clone());
            self.peek = Some(p);
            unsafe {
                SetTimer(self.hwnd, TIMER_PEEK, ms, None);
            }
            self.update_targets();
        }
    }

    fn on_peek_timer(&mut self) {
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_PEEK);
        }
        self.peek = None;
        self.update_targets();
        self.show_next_peek();
    }

    fn on_commands(&mut self) {
        for cmd in worker::drain_commands() {
            match cmd {
                ShellCmd::Peek(p) => self.peeks.push_back(p),
                ShellCmd::ReloadSettings => { let _ = self.reload_settings(); }
            }
        }
        self.show_next_peek();
    }

    /// Applies settings.txt: mascot, minimal mode and the enabled features.
    fn reload_settings(&mut self) -> std::io::Result<()> {
        // Keep the complete active snapshot if the file cannot be read.
        let settings = store::try_read_kv(SETTINGS_FILE)?;
        theme::apply(theme::Preset::from_setting(settings.get(theme::PRESET_KEY).map(String::as_str)));
        if let Err(error) = self.assistant.sync(&settings, self.hwnd) {
            worker::peek(Peek { icon: "\u{E946}".into(), emoji: false, accent: theme::warn(),
                title: "Assistant panels".into(), body: error, seconds: 5.0 });
        }
        if desktop_clock::sync(desktop_clock::enabled(&settings)).is_err() {
            report_clock_error();
        }
        self.mascot_enabled = settings.get("mascot").map_or(true, |v| v != "0");
        self.sync_mascot_timer();
        self.minimal = settings.get("minimal").is_some_and(|v| v == "1");
        let disabled: Vec<String> = settings
            .get(pages::DISABLED_KEY)
            .map(|v| v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();
        let phone_enabled = pages::phone_enabled(&settings);
        if disabled != self.disabled || phone_enabled != self.phone_enabled || self.pages.is_empty() {
            let current_id = self.page_ids.get(self.current).copied();
            self.disabled = disabled;
            self.phone_enabled = phone_enabled;
            let off = self.disabled.clone();
            let old: Vec<(&'static str, Box<dyn Page>)> = self.page_ids.drain(..).zip(self.pages.drain(..)).collect();
            let (ids, built): (Vec<_>, Vec<_>) =
                pages::build(self.waker, |id| !off.iter().any(|x| x == id), phone_enabled, old).into_iter().unzip();
            self.page_ids = ids;
            self.pages = built;
            // Stay on the same page if it still exists.
            self.current = current_id
                .and_then(|id| self.page_ids.iter().position(|x| *x == id))
                .unwrap_or(0)
                .min(self.pages.len() - 1);
            self.keyboard = false;
            self.nav_hover = None;
        }
        self.refresh_activity();
        self.update_targets();
        if !self.animating {
            self.redraw();
        }
        Ok(())
    }

    fn set_setting(&mut self, key: &str, value: &str) {
        let Ok(mut settings) = store::try_read_kv(SETTINGS_FILE) else { return };
        settings.insert(key.to_string(), value.to_string());
        if store::write_kv(SETTINGS_FILE, &settings).is_ok() {
            let _ = self.reload_settings();
        }
    }

    /// The fully expanded panel, used to lay out expanded content.
    fn expanded_panel(&self) -> Rect {
        Rect::new((theme::WINDOW_W - theme::EXPANDED_W) / 2.0, 0.0, theme::EXPANDED_W, theme::EXPANDED_H)
    }

    fn body(&self) -> Rect {
        let p = self.expanded_panel();
        theme::BODY.offset(p.x, p.y)
    }

    fn is_expanded(&self) -> bool {
        self.expand.target > 0.5 && self.expand.value > 0.85
    }

    fn update_region(&mut self) {
        let p = self.panel();
        let (_, ear) = corner_radii(self.height_t());
        let s = self.scale;
        let left = ((p.x - ear - 1.0) * s).floor() as i32;
        let right = ((p.right() + ear + 1.0) * s).ceil() as i32;
        let bottom = ((p.bottom() + 1.0) * s).ceil() as i32;
        // SetWindowRgn makes DWM recompose the window; skip it when nothing changed
        // (ambient animations redraw every frame without resizing).
        if (left, right, bottom) == self.last_region {
            return;
        }
        self.last_region = (left, right, bottom);
        unsafe {
            let rgn = CreateRectRgn(left, 0, right, bottom);
            SetWindowRgn(self.hwnd, rgn, true);
        }
    }

    /// Runs the animation timer at `interval_ms` (re-arming it if the period changes).
    fn set_anim_interval(&mut self, interval_ms: u32) {
        if self.hidden_for_fullscreen {
            return;
        }
        if self.anim_interval != interval_ms {
            self.anim_interval = interval_ms;
            unsafe {
                SetTimer(self.hwnd, TIMER_ANIM, interval_ms, None);
            }
        }
    }

    fn stop_animation(&mut self) {
        self.animating = false;
        self.anim_interval = 0;
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_ANIM);
        }
    }

    /// Windows can drop a topmost window below normal windows (e.g. after an app
    /// leaves fullscreen or the shell restarts). Re-assert our place every tick,
    /// and step aside while a fullscreen app is in front.
    fn guard_z_order(&mut self) {
        let fullscreen = !self.explicit_panel && unsafe {
            matches!(
                SHQueryUserNotificationState(),
                Ok(QUNS_BUSY) | Ok(QUNS_RUNNING_D3D_FULL_SCREEN) | Ok(QUNS_PRESENTATION_MODE)
            )
        };
        if fullscreen != self.hidden_for_fullscreen {
            self.hidden_for_fullscreen = fullscreen;
            self.sync_mascot_timer();
            if fullscreen {
                self.stop_animation();
            } else {
                // Functional state kept advancing while hidden. Resume from
                // now, without treating the hidden interval as an animation frame.
                self.mascot_last = Instant::now();
                self.start_animation();
            }
            unsafe {
                let _ = ShowWindow(self.hwnd, if fullscreen { SW_HIDE } else { SW_SHOWNOACTIVATE });
            }
        }
        if !fullscreen {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
            }
        }
    }

    fn start_animation(&mut self) {
        if self.hidden_for_fullscreen {
            return;
        }
        if !self.animating {
            self.animating = true;
            self.last_frame = Instant::now();
        }
        self.set_anim_interval(16);
    }

    fn set_expanded(&mut self, expanded: bool) {
        if !expanded {
            self.nav_hover = None;
        }
        if !expanded && self.explicit_panel {
            self.finish_page_edit();
            self.explicit_panel = false;
            self.guard_z_order();
            self.apply_panel_style();
        }
        let target = if expanded { 1.0 } else { 0.0 };
        if self.expand.target != target {
            self.expand.target = target;
            self.start_animation();
        }
        self.update_targets();
        if !expanded {
            self.show_next_peek();
        }
    }

    /// Expose an explicitly opened panel to taskbar/Alt-Tab, retaining the
    /// composition window style. Returning to the capsule restores tool-window behavior.
    fn apply_panel_style(&self) {
        unsafe {
            let old = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32;
            let mode_mask = WS_EX_APPWINDOW.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
            let mode = if self.explicit_panel {
                WS_EX_APPWINDOW.0
            } else {
                WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0
            };
            let style = (old & !mode_mask) | mode;
            if style != old {
                // The shell updates taskbar membership when the window is shown again.
                let _ = ShowWindow(self.hwnd, SW_HIDE);
                SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style as isize);
                let _ = SetWindowPos(
                    self.hwnd, HWND::default(), 0, 0, 0, 0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                if !self.hidden_for_fullscreen {
                    let _ = ShowWindow(self.hwnd, if self.explicit_panel { SW_SHOWNORMAL } else { SW_SHOWNOACTIVATE });
                }
            }
        }
    }

    fn open_panel(&mut self) {
        self.explicit_panel = true;
        self.guard_z_order();
        self.apply_panel_style();
        self.set_expanded(true);
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(self.hwnd);
        }
        self.redraw();
    }

    fn go_to(&mut self, index: usize) {
        self.nav_hover = None;
        if index == self.current || index >= self.pages.len() {
            return;
        }
        // Commit the outgoing editor before keyboard input is routed elsewhere.
        self.finish_page_edit();
        let direction = if index > self.current { 1.0 } else { -1.0 };
        self.current = index;
        self.page_shift.snap(direction);
        self.page_shift.target = 0.0;
        self.start_animation();
    }

    fn step_animation(&mut self) {
        // KillTimer does not remove WM_TIMER messages already in the queue.
        if self.hidden_for_fullscreen {
            self.stop_animation();
            return;
        }
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(1.0 / 30.0);
        self.last_frame = now;
        let a = self.expand.step(dt) | self.width.step(dt) | self.height.step(dt) | self.peek_t.step(dt);
        let b = self.page_shift.step(dt);
        let c = self.step_mascot();
        let d = self.activity_animating();
        self.update_region();
        self.redraw();
        if !a && !b && !c && !d {
            self.stop_animation();
        } else if !a && !b {
            // Only ambient animation left: 30 fps is plenty and saves CPU.
            self.set_anim_interval(33);
        }
    }

    fn redraw(&mut self) {
        // Worker results, timer state and edits still update while hidden;
        // none of those paths should decode/draw/present an invisible frame.
        if self.hidden_for_fullscreen {
            return;
        }
        let panel = self.panel();
        let t = self.expand.value;
        let ht = self.height_t();
        let peek_t = self.peek_t.value;
        let activity = self.activity;
        let minimal = self.minimal;
        let peek = self.peek_display.as_ref();
        let expanded_panel = self.expanded_panel();
        let body = self.body();
        let page_shift = self.page_shift.value;
        let battery = &self.battery;
        let current = self.current;
        let page_count = self.pages.len();
        let title_index = self.nav_hover.filter(|&i| i < page_count).unwrap_or(current);
        let title = self.pages[title_index].title();
        let nav_hover = self.nav_hover;
        let page_ids = &self.page_ids;
        let mascot = &self.mascot;
        let pages = &mut self.pages;

        let _ = self.gfx.frame(|ctx| {
            let (bottom, ear) = corner_radii(ht);
            ctx.fill_notch(panel, bottom, ear, theme::bg());

            let st = unsafe { GetLocalTime() };
            let clock = format!("{:02}:{:02}", st.wHour, st.wMinute);

            let collapsed_alpha = (1.0 - t / 0.45).clamp(0.0, 1.0) * (1.0 - peek_t / 0.45).clamp(0.0, 1.0);
            if collapsed_alpha > 0.01 {
                ctx.push_clip(panel);
                ctx.opacity = collapsed_alpha;
                if let Some(i) = activity {
                    pages[i].draw_activity(ctx, panel);
                } else {
                    mascot.draw(ctx, panel);
                    if !minimal {
                        ctx.opacity = collapsed_alpha * (1.0 - mascot.visibility());
                        draw_battery(ctx, Point { x: panel.x + 18.0, y: panel.h / 2.0 }, battery);
                        ctx.text(&clock, panel, TextStyle::new(13.5, Weight::Bold, theme::text()).align(Align::Center));
                    }
                }
                ctx.pop_clip();
            }

            if let (Some(p), true) = (peek, peek_t > 0.4 && t < 0.5) {
                ctx.push_clip(panel);
                ctx.opacity = ((peek_t - 0.4) / 0.6).clamp(0.0, 1.0) * (1.0 - t * 2.0).clamp(0.0, 1.0);
                draw_peek(ctx, panel, p);
                ctx.pop_clip();
            }

            if t > 0.55 {
                ctx.push_clip(panel);
                let alpha = ((t - 0.55) / 0.45).clamp(0.0, 1.0);
                ctx.opacity = alpha;
                let p = expanded_panel;
                if let Some(nav) = navigation::Layout::new(p, page_count) {
                    let cy = nav.status.center().y;
                    draw_battery(ctx, Point { x: nav.status.x + 8.0, y: cy }, battery);
                    ctx.text(&clock, Rect::new(nav.status.x + 22.0, p.y, 42.0, theme::HEADER_H),
                             TextStyle::new(11.5, Weight::Semibold, theme::text()));
                    let date = format!("{} {} {}", DAYS[st.wDayOfWeek as usize % 7], st.wDay,
                                       MONTHS[(st.wMonth as usize).saturating_sub(1) % 12]);
                    let style = TextStyle::new(10.5, Weight::Regular, theme::text_dim());
                    let date_w = ctx.measure_text(&date, &style);
                    if nav.status.w >= date_w + 72.0 {
                        ctx.text(&date, Rect::new(nav.status.x + 72.0, p.y, date_w, theme::HEADER_H), style);
                    }
                    ctx.fill_rect(nav.pill, 12.0, theme::card());
                    for (i, id) in page_ids.iter().enumerate() {
                        let Some(tab) = nav.tab(i) else { continue };
                        if i == current || nav_hover == Some(i) {
                            ctx.fill_rect(tab, 8.0, theme::card_hover());
                        }
                        let color = if i == current { theme::accent() } else { theme::text_dim() };
                        ctx.text(navigation::glyph(id), tab, TextStyle::icon(11.5, color));
                        if i == current {
                            ctx.fill_rect(Rect::new(tab.x + 8.0, tab.bottom() - 3.0, tab.w - 16.0, 2.0),
                                          1.0, theme::accent());
                        }
                    }
                    // The current/hovered section stays named even with an icon-only header.
                    ctx.text(&title, Rect::new(p.x + 12.0, p.y + 122.0, p.w - 24.0, 16.0),
                             TextStyle::new(10.5, Weight::Regular, theme::text_dim()).align(Align::Center));
                } else {
                    let header = Rect::new(p.x, 0.0, p.w, theme::HEADER_H);

                    ctx.text(&title, Rect::new(p.x + 14.0, 0.0, 110.0, theme::HEADER_H), TextStyle::new(14.5, Weight::Bold, theme::text()));

                    let date = format!(
                        "{} {} {} {}",
                        DAYS[st.wDayOfWeek as usize % 7],
                        st.wDay,
                        MONTHS[(st.wMonth as usize).saturating_sub(1) % 12],
                        st.wYear
                    );
                    let date_style = TextStyle::new(12.5, Weight::Regular, theme::text());
                    let clock_style = TextStyle::new(13.5, Weight::Bold, theme::text());
                    let date_w = ctx.measure_text(&date, &date_style);
                    let clock_w = ctx.measure_text(&clock, &clock_style);
                    let group_w = 16.0 + 8.0 + date_w + 10.0 + clock_w;
                    let mut x = header.center().x - group_w / 2.0;
                    draw_battery(ctx, Point { x: x + 8.0, y: header.h / 2.0 }, battery);
                    x += 24.0;
                    ctx.text(&date, Rect::new(x, 0.0, date_w + 2.0, header.h), date_style);
                    x += date_w + 10.0;
                    ctx.text(&clock, Rect::new(x, 0.0, clock_w + 2.0, header.h), clock_style);

                    // Arrows.
                    let cy = body.center().y;
                    for (cx, glyph) in [(p.x + theme::ARROW_INSET, "\u{E76B}"), (p.right() - theme::ARROW_INSET, "\u{E76C}")] {
                        let c = Point { x: cx, y: cy };
                        ctx.fill_circle(c, theme::ARROW_RADIUS, theme::card_hover());
                        let r = Rect::new(cx - 10.0, cy - 10.0, 20.0, 20.0);
                        ctx.text(glyph, r, TextStyle::icon(9.0, theme::text()));
                    }

                    // Page dots.
                    let total_w = (page_count as f32 - 1.0) * (theme::DOT_SIZE + theme::DOT_GAP) + theme::DOT_ACTIVE_W;
                    let mut dx = p.center().x - total_w / 2.0;
                    for i in 0..page_count {
                        let (w, color) = if i == current {
                            (theme::DOT_ACTIVE_W, theme::text())
                        } else {
                            (theme::DOT_SIZE, theme::text_faint())
                        };
                        let r = Rect::new(dx, theme::DOTS_Y - theme::DOT_SIZE / 2.0, w, theme::DOT_SIZE);
                        ctx.fill_rect(r, theme::DOT_SIZE / 2.0, color);
                        dx += w + theme::DOT_GAP;
                    }

                }

                // Current page, sliding in during page changes.
                ctx.opacity = alpha * (1.0 - page_shift.abs()).clamp(0.0, 1.0);
                let area = body.offset(page_shift * theme::PAGE_SLIDE, 0.0);
                ctx.push_clip(body);
                pages[current].draw(ctx, area);
                ctx.pop_clip();
                ctx.pop_clip();
            }
        });
    }

    fn arrow_hit(&self, pt: Point) -> Option<isize> {
        if navigation::Layout::new(self.expanded_panel(), self.pages.len()).is_some() {
            return None;
        }
        let p = self.expanded_panel();
        let cy = self.body().center().y;
        let hit = |cx: f32| (pt.x - cx).hypot(pt.y - cy) <= theme::ARROW_RADIUS + 6.0;
        if hit(p.x + theme::ARROW_INSET) {
            Some(-1)
        } else if hit(p.right() - theme::ARROW_INSET) {
            Some(1)
        } else {
            None
        }
    }

    fn dot_hit(&self, pt: Point) -> Option<usize> {
        if navigation::Layout::new(self.expanded_panel(), self.pages.len()).is_some() {
            return None;
        }
        if (pt.y - theme::DOTS_Y).abs() > 7.0 {
            return None;
        }
        let n = self.pages.len();
        let total_w = (n as f32 - 1.0) * (theme::DOT_SIZE + theme::DOT_GAP) + theme::DOT_ACTIVE_W;
        let mut x = self.expanded_panel().center().x - total_w / 2.0;
        for i in 0..n {
            let w = if i == self.current { theme::DOT_ACTIVE_W } else { theme::DOT_SIZE };
            if pt.x >= x - theme::DOT_GAP / 2.0 && pt.x < x + w + theme::DOT_GAP / 2.0 {
                return Some(i);
            }
            x += w + theme::DOT_GAP;
        }
        None
    }

    fn page_mouse(&mut self, ev: Mouse) -> bool {
        let area = self.body();
        let mut host = Host { hwnd: self.hwnd, want_keyboard: false };
        let redraw = self.pages[self.current].on_mouse(ev, area, &mut host);
        if host.want_keyboard && !self.keyboard {
            self.keyboard = true;
            self.input_utf16 = Utf16Input::default();
            unsafe {
                let _ = SetForegroundWindow(self.hwnd);
                let _ = SetFocus(self.hwnd);
            }
        } else if self.keyboard && !self.pages[self.current].editing() {
            // The page finished editing through a click (e.g. committed by clicking elsewhere).
            self.keyboard = false;
        }
        redraw
    }

    fn track_leave(&self) {
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: self.hwnd,
            dwHoverTime: 0,
        };
        unsafe {
            let _ = TrackMouseEvent(&mut tme);
        }
    }

    fn cursor_inside(&self) -> bool {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() || !ScreenToClient(self.hwnd, &mut pt).as_bool() {
                return false;
            }
        }
        self.panel().contains(Point { x: pt.x as f32 / self.scale, y: pt.y as f32 / self.scale })
    }

    fn on_mouse_move(&mut self, pt: Point) {
        if !self.hovering {
            self.hovering = true;
            self.track_leave();
            unsafe {
                let _ = KillTimer(self.hwnd, TIMER_COLLAPSE);
            }
            self.set_expanded(true);
        }
        if self.is_expanded() {
            let hit = navigation::Layout::new(self.expanded_panel(), self.pages.len()).and_then(|nav| nav.hit(pt));
            let hover_changed = hit != self.nav_hover;
            self.nav_hover = hit;
            let redraw = self.page_mouse(Mouse::Move(pt));
            if (redraw || hover_changed) && !self.animating {
                self.redraw();
            }
        }
    }

    /// Windows also posts WM_MOUSELEAVE on clicks and window-region changes
    /// while the cursor is still inside, so collapsing waits briefly and then
    /// re-checks the real cursor position.
    fn on_mouse_leave(&mut self) {
        self.hovering = false;
        let nav_changed = self.nav_hover.take().is_some();
        let redraw = self.page_mouse(Mouse::Leave) || nav_changed;
        unsafe {
            SetTimer(self.hwnd, TIMER_COLLAPSE, 250, None);
        }
        if redraw && !self.animating {
            self.redraw();
        }
    }

    fn on_collapse_timer(&mut self) {
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_COLLAPSE);
        }
        if self.cursor_inside() {
            self.hovering = true;
            self.track_leave();
        } else if !self.explicit_panel && !self.keyboard && !self.dragging && !self.pinned && !self.pages[self.current].editing() {
            self.set_expanded(false);
        }
    }

    fn on_button(&mut self, pt: Point, down: bool, double: bool) {
        if !self.is_expanded() {
            return;
        }
        if down {
            if let Some(i) = navigation::Layout::new(self.expanded_panel(), self.pages.len()).and_then(|nav| nav.hit(pt)) {
                self.nav_hover = None;
                self.go_to(i);
                return;
            }
            if let Some(dir) = self.arrow_hit(pt) {
                let n = self.pages.len() as isize;
                let next = (self.current as isize + dir).rem_euclid(n) as usize;
                self.go_to(next);
                return;
            }
            if let Some(i) = self.dot_hit(pt) {
                self.go_to(i);
                return;
            }
        }
        let ev = match (down, double) {
            (true, true) => Mouse::DoubleClick(pt),
            (true, false) => Mouse::Down(pt),
            (false, _) => Mouse::Up(pt),
        };
        if self.page_mouse(ev) && !self.animating {
            self.redraw();
        }
    }

    fn on_drag_enter(&mut self) {
        self.dragging = true;
        if let Some(i) = self.drop_page() {
            self.go_to(i);
        }
        self.set_expanded(true);
    }

    fn on_drag_leave(&mut self) {
        self.dragging = false;
        unsafe {
            SetTimer(self.hwnd, TIMER_COLLAPSE, 250, None);
        }
    }

    fn on_drop_data(&mut self, data: DropData) {
        self.dragging = false;
        match data {
            DropData::Files(paths) => self.on_drop(paths),
            DropData::Url { url, title } => {
                // Links become .url shortcuts in the local data folder, then go on the shelf.
                let dir = store::local_dir().join("Links");
                let _ = std::fs::create_dir_all(&dir);
                if let Ok(path) = droptarget::save_url_shortcut(&dir, &url, title.as_deref()) {
                    self.on_drop(vec![path]);
                }
            }
        }
    }

    fn on_drop(&mut self, paths: Vec<PathBuf>) {
        if let Some(i) = self.drop_page() {
            self.go_to(i);
            self.pages[i].on_drop_files(&paths);
            self.set_expanded(true);
            self.redraw();
        }
    }

    /// Respect an explicitly selected receiver before falling back to Shelf.
    fn drop_page(&self) -> Option<usize> {
        if self.pages.get(self.current).is_some_and(|page| page.accepts_files()) {
            Some(self.current)
        } else {
            self.pages.iter().position(|page| page.accepts_files())
        }
    }

    fn on_tick(&mut self) {
        self.assistant.tick();
        self.guard_z_order();
        let st = unsafe { GetLocalTime() };
        if desktop_clock::tick(&st).is_err() {
            report_clock_error();
        }
        let battery = read_battery();
        let mut redraw = st.wMinute != self.minute
            || battery.percent != self.battery.percent
            || battery.plugged != self.battery.plugged;
        self.minute = st.wMinute;
        self.battery = battery;
        for page in &mut self.pages {
            redraw |= page.tick();
        }
        self.refresh_activity();
        if redraw && !self.animating {
            self.redraw();
        }
    }

    fn on_wake(&mut self) {
        self.assistant.actions();
        let mut redraw = false;
        for page in &mut self.pages {
            redraw |= page.poll();
        }
        self.refresh_activity();
        if redraw && !self.animating {
            self.redraw();
        }
    }

    fn finish_page_edit(&mut self) {
        self.keyboard = false;
        self.input_utf16 = Utf16Input::default();
        self.pages[self.current].on_focus_lost();
    }

    fn end_keyboard(&mut self) {
        if !self.keyboard {
            return;
        }
        self.finish_page_edit();
        if !self.explicit_panel && !self.hovering && !self.pinned {
            self.set_expanded(false);
        }
        if !self.animating {
            self.redraw();
        }
    }

    fn on_char(&mut self, ch: char) {
        if self.keyboard && self.pages[self.current].on_char(ch) && !self.animating {
            self.redraw();
        }
    }

    fn on_key(&mut self, vk: u16) {
        if !self.keyboard && !self.pages[self.current].editing() && self.is_expanded() {
            let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
            let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
            if let Some(step) = navigation::key_step(vk, ctrl, shift) {
                let next = (self.current as isize + step).rem_euclid(self.pages.len() as isize) as usize;
                self.nav_hover = None;
                self.go_to(next);
                return;
            }
        }
        // Escape first cancels an active editor; a subsequent Escape returns
        // an explicitly opened panel to its normal low-distraction capsule.
        if vk == VK_ESCAPE.0 && self.explicit_panel && !self.keyboard {
            self.set_expanded(false);
            return;
        }
        if !self.keyboard {
            return;
        }
        if vk == VK_V.0 && unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0 {
            if let Some(text) = droptarget::clipboard_text() {
                let mut redraw = false;
                for ch in text.chars().filter(|ch| !ch.is_control()).take(120) {
                    redraw |= self.pages[self.current].on_char(ch);
                }
                if redraw && !self.animating {
                    self.redraw();
                }
            }
            return;
        }
        let redraw = self.pages[self.current].on_key(vk);
        if !self.pages[self.current].editing() {
            self.end_keyboard();
        } else if redraw && !self.animating {
            self.redraw();
        }
    }

    /// Collapsed, nothing else on screen, mascot enabled and battery not low.
    fn mascot_eligible(&self) -> bool {
        let battery_ok = self.battery.plugged || self.battery.percent.map_or(true, |p| p > 20);
        self.mascot_enabled
            && self.expand.target < 0.5
            && self.expand.value < 0.05
            && !self.keyboard
            && battery_ok
            && self.activity.is_none()
            && self.peek.is_none()
    }

    /// Advances the mascot to now; returns true while it needs animation frames.
    fn step_mascot(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.mascot_last).as_secs_f32();
        self.mascot_last = now;
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        let idle_ms = unsafe {
            if GetLastInputInfo(&mut info).as_bool() {
                GetTickCount().wrapping_sub(info.dwTime)
            } else {
                0
            }
        };
        let mut cursor = Point::default();
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_ok() && ScreenToClient(self.hwnd, &mut pt).as_bool() {
                let c = self.panel().center();
                cursor = Point { x: pt.x as f32 / self.scale - c.x, y: pt.y as f32 / self.scale - c.y };
            }
        }
        let eligible = self.mascot_eligible();
        self.mascot.update(dt, idle_ms, eligible, cursor)
    }

    /// Arms the idle poll only while the optional mascot is enabled and visible.
    fn sync_mascot_timer(&self) {
        unsafe {
            if self.mascot_enabled && !self.hidden_for_fullscreen {
                SetTimer(self.hwnd, TIMER_MASCOT, 500, None);
            } else {
                let _ = KillTimer(self.hwnd, TIMER_MASCOT);
            }
        }
    }

    /// Polls idle time twice a second while no animation is running, so the
    /// mascot can appear after 15 s of inactivity.
    fn on_mascot_timer(&mut self) {
        if self.mascot_enabled && !self.hidden_for_fullscreen && !self.animating && self.step_mascot() {
            self.start_animation();
        }
    }

    fn show_tray_menu(&mut self) {
        unsafe {
            let Ok(menu) = CreatePopupMenu() else { return };
            let _ = AppendMenuW(menu, MF_STRING, MENU_OPEN, w!("Open panel"));
            let startup_flags = if startup_enabled() { MF_STRING | MF_CHECKED } else { MF_STRING };
            let _ = AppendMenuW(menu, startup_flags, MENU_STARTUP, w!("Start with Windows"));
            let mascot_flags = if self.mascot_enabled { MF_STRING | MF_CHECKED } else { MF_STRING };
            let _ = AppendMenuW(menu, mascot_flags, MENU_MASCOT, w!("Mascot"));
            let minimal_flags = if self.minimal { MF_STRING | MF_CHECKED } else { MF_STRING };
            let _ = AppendMenuW(menu, minimal_flags, MENU_MINIMAL, w!("Minimal notch"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT, w!("Exit"));
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, self.hwnd, None);
            let _ = DestroyMenu(menu);
            match cmd.0 as usize {
                MENU_OPEN => self.open_panel(),
                MENU_STARTUP => set_startup(!startup_enabled()),
                MENU_MASCOT => {
                    let value = if self.mascot_enabled { "0" } else { "1" };
                    self.set_setting("mascot", value);
                    self.on_mascot_timer();
                }
                MENU_MINIMAL => {
                    let value = if self.minimal { "0" } else { "1" };
                    self.set_setting("minimal", value);
                }
                MENU_EXIT => {
                    let _ = DestroyWindow(self.hwnd);
                }
                _ => {}
            }
        }
    }
}

/// (bottom corner radius, top ear radius) for an expansion `t` (may overshoot 0..1).
fn corner_radii(t: f32) -> (f32, f32) {
    let t = t.max(0.0);
    (
        theme::BOTTOM_RADIUS_BASE + theme::BOTTOM_RADIUS_GROWTH * t,
        theme::EAR_RADIUS_BASE + theme::EAR_RADIUS_GROWTH * t,
    )
}

/// Routes OLE drag-and-drop callbacks (UI thread) to the shell. They arrive
/// from the message loop, outside any App borrow, except when our own shelf
/// starts a drag; those nested calls are ignored.
struct ShellDrop;

fn with_app(f: impl FnOnce(&mut App)) {
    APP.with(|cell| {
        if let Ok(mut slot) = cell.try_borrow_mut() {
            if let Some(app) = slot.as_mut() {
                f(app);
            }
        }
    });
}

impl DropHandler for ShellDrop {
    fn drag_enter(&self) {
        with_app(App::on_drag_enter);
    }

    fn drag_leave(&self) {
        with_app(App::on_drag_leave);
    }

    fn drop(&self, data: DropData) {
        with_app(|app| app.on_drop_data(data));
    }
}

/// Peek layout: accent icon bubble on the left, title and body on the right.
fn draw_peek(ctx: &mut DrawCtx, panel: Rect, p: &Peek) {
    let c = Point { x: panel.x + 34.0, y: panel.h / 2.0 + 2.0 };
    ctx.fill_circle(c, 17.0, p.accent.with_alpha(0.22));
    let icon = Rect::new(c.x - 14.0, c.y - 14.0, 28.0, 28.0);
    if p.emoji {
        ctx.text(&p.icon, icon, TextStyle::emoji(17.0));
    } else {
        ctx.text(&p.icon, icon, TextStyle::icon(15.0, p.accent));
    }
    let x = panel.x + 62.0;
    let w = (panel.right() - 18.0 - x).max(0.0);
    let mid = panel.h / 2.0 + 2.0;
    ctx.text(&p.title, Rect::new(x, mid - 18.0, w, 18.0), TextStyle::new(13.5, Weight::Bold, theme::text()));
    ctx.text(&p.body, Rect::new(x, mid, w, 18.0), TextStyle::new(12.0, Weight::Regular, theme::text_dim()));
}

fn draw_battery(ctx: &mut DrawCtx, center: Point, battery: &Battery) {
    let fraction = battery.percent.map_or(1.0, |p| p as f32 / 100.0);
    let color = match battery.percent {
        Some(p) if p <= 15 && !battery.plugged => theme::danger(),
        Some(p) if p <= 30 && !battery.plugged => theme::warn(),
        _ => theme::accent(),
    };
    ctx.stroke_circle(center, 6.5, color.with_alpha(0.25), 2.0);
    ctx.arc(center, 6.5, fraction, color, 2.0);
    if battery.plugged {
        let r = Rect::new(center.x - 6.0, center.y - 6.0, 12.0, 12.0);
        ctx.text("\u{E945}", r, TextStyle::icon(7.5, color));
    }
}

fn startup_enabled() -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ, None, None, None).is_ok() }
}

fn set_startup(enable: bool) {
    unsafe {
        if enable {
            let Ok(exe) = std::env::current_exe() else { return };
            let value: Vec<u16> = format!("\"{}\" --background", exe.display()).encode_utf16().chain(Some(0)).collect();
            let _ = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                RUN_VALUE,
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                (value.len() * 2) as u32,
            );
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
        }
    }
}

/// The Ada icon at the system's small-icon size, built from embedded PNGs so it
/// also works in debug builds (release builds additionally embed `ada.ico`).
fn app_icon() -> HICON {
    unsafe {
        let size = GetSystemMetrics(SM_CXSMICON);
        let png: &[u8] = if size <= 16 {
            include_bytes!("../assets/icon/ada_16.png")
        } else {
            include_bytes!("../assets/icon/ada_32.png")
        };
        CreateIconFromResourceEx(png, true, 0x0003_0000, size, size, LR_DEFAULTCOLOR)
            .or_else(|_| LoadIconW(None, IDI_APPLICATION))
            .unwrap_or_default()
    }
}

fn add_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_APP_TRAY,
            hIcon: app_icon(),
            ..Default::default()
        };
        for (dst, src) in nid.szTip.iter_mut().zip("NotchLite".encode_utf16()) {
            *dst = src;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn dropped_paths(drop: HDROP) -> Vec<PathBuf> {
    unsafe {
        let count = DragQueryFileW(drop, u32::MAX, None);
        let mut paths = Vec::with_capacity(count as usize);
        for i in 0..count {
            let len = DragQueryFileW(drop, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            DragQueryFileW(drop, i, Some(&mut buf));
            paths.push(PathBuf::from(String::from_utf16_lossy(&buf[..len])));
        }
        DragFinish(drop);
        paths
    }
}

fn point_from_lparam(lparam: LPARAM, scale: f32) -> Point {
    let x = (lparam.0 & 0xFFFF) as i16 as f32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
    Point { x: x / scale, y: y / scale }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // DestroyWindow sends WM_DESTROY synchronously, including when Exit is
    // selected while APP is already borrowed by this window procedure.
    // Shutdown must not depend on acquiring that RefCell borrow again.
    if msg == WM_DESTROY {
        crate::assist_overlay::shutdown_all();
        desktop_clock::shutdown();
        unsafe {
            let _ = RevokeDragDrop(hwnd);
        }
        remove_tray_icon(hwnd);
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    }
    let handled = APP.with(|cell| {
        // Win32 re-enters the window procedure from calls like SetWindowRgn and
        // TrackPopupMenu; nested messages fall through to DefWindowProcW.
        let Ok(mut slot) = cell.try_borrow_mut() else { return None };
        let app = slot.as_mut()?;
        match msg {
            WM_MOUSEMOVE => app.on_mouse_move(point_from_lparam(lparam, app.scale)),
            WM_MOUSELEAVE => app.on_mouse_leave(),
            WM_LBUTTONDOWN => app.on_button(point_from_lparam(lparam, app.scale), true, false),
            WM_LBUTTONDBLCLK => app.on_button(point_from_lparam(lparam, app.scale), true, true),
            WM_LBUTTONUP => app.on_button(point_from_lparam(lparam, app.scale), false, false),
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32 / WHEEL_DELTA as f32;
                let mut pt = POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 };
                unsafe {
                    let _ = ScreenToClient(hwnd, &mut pt);
                }
                let pt = Point { x: pt.x as f32 / app.scale, y: pt.y as f32 / app.scale };
                if app.is_expanded() && app.page_mouse(Mouse::Wheel(pt, delta)) && !app.animating {
                    app.redraw();
                }
            }
            WM_CHAR => {
                if app.keyboard {
                    if let Some(ch) = app.input_utf16.push(wparam.0 as u16) {
                        app.on_char(ch);
                    }
                }
            }
            WM_KEYDOWN => app.on_key(wparam.0 as u16),
            WM_KILLFOCUS => app.end_keyboard(),
            WM_CONTEXTMENU => app.show_tray_menu(),
            WM_DROPFILES => app.on_drop(dropped_paths(HDROP(wparam.0 as _))),
            WM_TIMER if wparam.0 == TIMER_ANIM => app.step_animation(),
            WM_TIMER if wparam.0 == TIMER_TICK => app.on_tick(),
            WM_TIMER if wparam.0 == TIMER_COLLAPSE => app.on_collapse_timer(),
            WM_TIMER if wparam.0 == TIMER_MASCOT => app.on_mascot_timer(),
            WM_TIMER if wparam.0 == TIMER_PEEK => app.on_peek_timer(),
            WM_APP_CMD => app.on_commands(),
            WM_APP_WAKE => app.on_wake(),
            WM_APP_OPEN => app.open_panel(),
            WM_DISPLAYCHANGE | WM_SETTINGCHANGE => app.assistant.display_changed(),
            WM_APP_TRAY => {
                let event = (lparam.0 & 0xFFFF) as u32;
                if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                    app.show_tray_menu();
                } else if event == WM_LBUTTONUP {
                    app.set_expanded(app.expand.target < 0.5);
                }
            }
            _ => return None,
        }
        Some(LRESULT(0))
    });
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
}

/// Development aid: `NOTCHLITE_OFFSET_X` shifts the notch horizontally (e.g. to run next to another instance).
fn dev_offset_x() -> i32 {
    std::env::var("NOTCHLITE_OFFSET_X").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Ask the already running instance to open its real panel.
/// No process is started and no user content is passed in the window message.
pub fn request_open_existing() -> Result<bool> {
    unsafe {
        let Ok(hwnd) = FindWindowW(w!("NotchLiteWindow"), None) else { return Ok(false) };
        PostMessageW(hwnd, WM_APP_OPEN, WPARAM(0), LPARAM(0))?;
        Ok(true)
    }
}

pub fn run(open_panel: bool) -> Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = w!("NotchLiteWindow");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let scale = GetDpiForSystem() as f32 / 96.0;
        let px_w = (theme::WINDOW_W * scale).round() as i32;
        let px_h = (theme::WINDOW_H * scale).round() as i32;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);

        let interaction_style = if open_panel { WS_EX_APPWINDOW } else { WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE };
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_ACCEPTFILES | interaction_style,
            class,
            w!("NotchLite"),
            WS_POPUP,
            (screen_w - px_w) / 2 + dev_offset_x(),
            0,
            px_w,
            px_h,
            None,
            None,
            instance,
            None,
        )?;

        let gfx = Gfx::new(hwnd, px_w as u32, px_h as u32, scale)?;
        let waker = Waker::new(hwnd);
        let mut app = App {
            hwnd,
            gfx,
            scale,
            expand: Spring::new(if open_panel { 1.0 } else { 0.0 }, theme::SIZE_STIFFNESS, theme::SIZE_DAMPING),
            width: Spring::new(theme::COLLAPSED_W, theme::SIZE_STIFFNESS, theme::SIZE_DAMPING),
            height: Spring::new(theme::COLLAPSED_H, theme::SIZE_STIFFNESS, theme::SIZE_DAMPING),
            peek_t: Spring::new(0.0, 300.0, 30.0),
            page_shift: Spring::new(0.0, 420.0, 40.0),
            last_frame: Instant::now(),
            animating: false,
            anim_interval: 0,
            last_region: (0, 0, 0),
            hidden_for_fullscreen: false,
            explicit_panel: open_panel,
            hovering: false,
            keyboard: false,
            input_utf16: Utf16Input::default(),
            pages: Vec::new(),
            page_ids: Vec::new(),
            current: 0,
            nav_hover: None,
            battery: read_battery(),
            minute: GetLocalTime().wMinute,
            mascot: Mascot::new(),
            mascot_enabled: store::read_kv(SETTINGS_FILE).get("mascot").map_or(true, |v| v != "0"),
            mascot_last: Instant::now(),
            waker,
            minimal: false,
            disabled: Vec::new(),
            phone_enabled: false,
            assistant: crate::assist::Assistant::new(),
            activity: None,
            peeks: VecDeque::new(),
            peek: None,
            peek_display: None,
            dragging: false,
            pinned: false,
        };
        // Determine visibility before reload_settings can request the first
        // frame, so starting during fullscreen does not briefly show the notch.
        app.guard_z_order();
        app.reload_settings().map_err(|error| Error::new(E_FAIL, format!("Cannot read NotchLite settings: {error}")))?;
        app.width.snap(app.width.target);
        app.height.snap(app.height.target);
        app.update_region();
        app.redraw();
        let initially_visible = !app.hidden_for_fullscreen;
        APP.with(|cell| *cell.borrow_mut() = Some(app));

        add_tray_icon(hwnd);
        // OLE drop target: gives drag-over feedback (expand to the shelf) and
        // accepts links; WM_DROPFILES stays as a fallback if this fails.
        let _ = RegisterDragDrop(hwnd, &droptarget::create(Box::new(ShellDrop)));
        if initially_visible {
            let _ = ShowWindow(hwnd, if open_panel { SW_SHOWNORMAL } else { SW_SHOWNOACTIVATE });
            if open_panel {
                let _ = SetForegroundWindow(hwnd);
                let _ = SetFocus(hwnd);
            }
        }
        SetTimer(hwnd, TIMER_TICK, 1000, None);
        // Development aid: `NOTCHLITE_TEST_PEEK=1` shows a sample peek at startup.
        if std::env::var_os("NOTCHLITE_TEST_PEEK").is_some() {
            worker::peek(Peek {
                icon: "\u{1F44B}".to_string(),
                emoji: true,
                accent: theme::blue(),
                title: "Merhaba!".to_string(),
                body: "This is a sample peek from NotchLite".to_string(),
                seconds: 4.0,
            });
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if crate::assist_overlay::translate_message(&msg) { continue; }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        desktop_clock::shutdown();
        crate::assist_overlay::shutdown_all();
        // Release owned proxy/metadata workers outside an APP RefCell borrow.
        let removed = APP.with(|cell| cell.borrow_mut().take());
        drop(removed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Utf16Input;

    #[test]
    fn wm_char_decodes_bmp_and_supplementary_text() {
        let mut input = Utf16Input::default();
        let decoded: String = "Caffè 😀 𐐀".encode_utf16().filter_map(|unit| input.push(unit)).collect();
        assert_eq!(decoded, "Caffè 😀 𐐀");
    }

    #[test]
    fn invalid_surrogates_do_not_consume_following_text() {
        let mut input = Utf16Input::default();
        assert_eq!(input.push(0xDC00), None);
        assert_eq!(input.push(0xD83D), None);
        assert_eq!(input.push(u16::from(b'A')), Some('A'));
        assert_eq!(input.push(0xDE00), None);
        assert_eq!(input.push(0xD83D), None);
        input = Utf16Input::default(); // a new editor/focus session
        assert_eq!(input.push(0xDE00), None);
        assert_eq!(input.push(u16::from(b'B')), Some('B'));
    }
}
