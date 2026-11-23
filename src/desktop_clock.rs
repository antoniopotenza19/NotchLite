//! Optional native desktop clock. Uses the shell's existing tick, cached GDI
//! fonts and its own window state; no renderer, timer or worker is created.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use windows::core::{w, Error, Result};
use windows::Win32::Foundation::{
    GetLastError, SetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS,
    E_FAIL, E_INVALIDARG, HWND, LPARAM, LRESULT, RECT, SYSTEMTIME, WPARAM,
};
use windows::Win32::Globalization::{GetDateFormatEx, DATE_LONGDATE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::gfx::Color;
use crate::theme;

pub const SETTING_KEY: &str = "desktop_clock";

thread_local! {
    static CLOCK: RefCell<Option<ClockWindow>> = const { RefCell::new(None) };
    static SHUTDOWN_PENDING: Cell<bool> = const { Cell::new(false) };
}

pub fn enabled(map: &BTreeMap<String, String>) -> bool {
    map.get(SETTING_KEY).is_some_and(|value| value == "1")
}

/// Create hidden, or refresh the existing window for a palette change.
pub fn sync(wanted: bool) -> Result<()> {
    if !wanted {
        shutdown();
        return Ok(());
    }
    let result = CLOCK.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(clock) = slot.as_ref() {
            clock.invalidate()
        } else {
            *slot = Some(ClockWindow::create()?);
            Ok(())
        }
    });
    drain_pending_shutdown();
    result
}

pub fn tick(now: &SYSTEMTIME) -> Result<()> {
    let result = CLOCK.with(|cell| {
        let mut slot = cell.borrow_mut();
        match slot.as_mut() {
            Some(clock) => clock.tick(now),
            None => Ok(()),
        }
    });
    if result.is_err() {
        // The next shell tick sees None and does not repeat the same failure.
        shutdown();
    }
    drain_pending_shutdown();
    result
}

/// A User32 callback may request shutdown while sync/tick holds the manager.
pub fn shutdown() {
    SHUTDOWN_PENDING.with(|pending| pending.set(true));
    drain_pending_shutdown();
}

fn drain_pending_shutdown() {
    if !SHUTDOWN_PENDING.with(Cell::get) {
        return;
    }
    let removed = CLOCK.with(|cell| cell.try_borrow_mut().map(|mut slot| slot.take()));
    if let Ok(clock) = removed {
        SHUTDOWN_PENDING.with(|pending| pending.set(false));
        // DestroyWindow may dispatch callbacks; the manager is no longer borrowed.
        drop(clock);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MinuteKey {
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
}

impl From<&SYSTEMTIME> for MinuteKey {
    fn from(now: &SYSTEMTIME) -> Self {
        Self {
            year: now.wYear, month: now.wMonth, day: now.wDay,
            hour: now.wHour, minute: now.wMinute,
        }
    }
}

fn clock_text(now: &SYSTEMTIME) -> String {
    format!("{:02}:{:02}", now.wHour, now.wMinute)
}

fn desktop_class(class: &str) -> bool {
    matches!(class, "Progman" | "WorkerW")
}

fn scaled(dip: i32, dpi: u32) -> i32 {
    ((i64::from(dip) * i64::from(dpi.max(1))) / 96).clamp(1, i64::from(i32::MAX)) as i32
}

/// Return None for an empty/inverted work area. All arithmetic stays wide until
/// the final clamped coordinates, including negative monitor coordinates.
fn widget_rect(work: RECT, dpi: u32) -> Option<RECT> {
    let available_w = i64::from(work.right) - i64::from(work.left);
    let available_h = i64::from(work.bottom) - i64::from(work.top);
    if available_w <= 0 || available_h <= 0 {
        return None;
    }
    let width = i64::from(scaled(360, dpi)).min(available_w);
    let height = i64::from(scaled(160, dpi)).min(available_h);
    let left = i64::from(work.left) + (available_w - width) / 2;
    // On a work area shorter than the desired offset + height, fit inside it.
    let top = i64::from(work.top) + i64::from(scaled(160, dpi)).min(available_h - height);
    Some(RECT {
        left: left as i32, top: top as i32,
        right: (left + width) as i32, bottom: (top + height) as i32,
    })
}

fn work_area() -> Result<RECT> {
    let mut work = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA, 0, Some((&mut work as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
    }
    Ok(work)
}

fn desktop_is_foreground() -> bool {
    let mut class = [0u16; 64];
    let length = unsafe { GetClassNameW(GetForegroundWindow(), &mut class) };
    if length <= 0 {
        return false;
    }
    desktop_class(&String::from_utf16_lossy(&class[..length as usize]))
}

struct Fonts {
    time: HFONT,
    date: HFONT,
}

impl Fonts {
    fn create(dpi: u32) -> Result<Self> {
        let font = |size: i32| unsafe {
            CreateFontW(
                -scaled(size, dpi), 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32, w!("Segoe UI"),
            )
        };
        let time = font(64);
        if time.is_invalid() {
            return Err(Error::new(E_FAIL, "Could not create desktop clock font"));
        }
        let date = font(16);
        if date.is_invalid() {
            unsafe { let _ = DeleteObject(HGDIOBJ(time.0)); }
            return Err(Error::new(E_FAIL, "Could not create desktop date font"));
        }
        Ok(Self { time, date })
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.time.0));
            let _ = DeleteObject(HGDIOBJ(self.date.0));
        }
    }
}

struct PaintData {
    fonts: Fonts,
    time: [u16; 5],
    date: [u16; 128],
    date_len: usize,
}

struct WindowState {
    destroyed: Cell<bool>,
    paint: RefCell<PaintData>,
}

fn text_for(now: &SYSTEMTIME) -> ([u16; 5], [u16; 128], usize) {
    let mut time = [0u16; 5];
    for (dest, unit) in time.iter_mut().zip(clock_text(now).encode_utf16()) {
        *dest = unit;
    }
    let mut date = [0u16; 128];
    let count = unsafe {
        GetDateFormatEx(
            None, DATE_LONGDATE, Some(now as *const SYSTEMTIME),
            None, Some(&mut date), None,
        )
    };
    let date_len = if count > 0 && count as usize <= date.len() {
        count as usize - 1
    } else {
        let fallback = format!("{:02}/{:02}/{:04}", now.wDay, now.wMonth, now.wYear);
        let mut len = 0;
        for (dest, unit) in date.iter_mut().zip(fallback.encode_utf16()) {
            *dest = unit;
            len += 1;
        }
        len
    };
    (time, date, date_len)
}

struct ClockWindow {
    hwnd: HWND,
    // Box allocation stays stable while Win32 borrows its pointer in USERDATA.
    state: Box<WindowState>,
    rect: RECT,
    dpi: u32,
    minute: Option<MinuteKey>,
    visible: bool,
}

impl ClockWindow {
    fn create() -> Result<Self> {
        let work = work_area()?;
        let rect = widget_rect(work, 96)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Empty desktop work area"))?;
        let now = unsafe { GetLocalTime() };
        let (time, date, date_len) = text_for(&now);
        let mut clock = Self {
            hwnd: HWND::default(),
            state: Box::new(WindowState {
                destroyed: Cell::new(false),
                paint: RefCell::new(PaintData { fonts: Fonts::create(96)?, time, date, date_len }),
            }),
            rect, dpi: 96, minute: None, visible: false,
        };
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("NotchLiteDesktopClock"),
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                return Err(Error::from_win32());
            }
            clock.hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
                w!("NotchLiteDesktopClock"), w!("NotchLite desktop clock"), WS_POPUP,
                rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top,
                None, None, instance,
                Some((&*clock.state as *const WindowState).cast()),
            )?;
            SetLayeredWindowAttributes(clock.hwnd, COLORREF(0), 255, LWA_ALPHA)?;
        }
        clock.refresh_layout(true)?;
        Ok(clock)
    }

    fn invalidate(&self) -> Result<()> {
        if self.state.destroyed.get() {
            return Err(Error::new(E_FAIL, "Desktop clock window was destroyed"));
        }
        unsafe { InvalidateRect(self.hwnd, None, false).ok() }
    }

    fn refresh_layout(&mut self, force: bool) -> Result<bool> {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            return Err(Error::from_win32());
        }
        let rect = widget_rect(work_area()?, dpi)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Empty desktop work area"))?;
        let changed = force || dpi != self.dpi || rect != self.rect;
        if !changed {
            return Ok(false);
        }
        if dpi != self.dpi {
            let fonts = Fonts::create(dpi)?;
            let old = {
                let mut paint = self.state.paint.try_borrow_mut()
                    .map_err(|_| Error::new(E_FAIL, "Desktop clock paint is busy"))?;
                std::mem::replace(&mut paint.fonts, fonts)
            };
            drop(old);
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        unsafe {
            SetWindowPos(
                self.hwnd, HWND::default(), rect.left, rect.top, width, height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
            let radius = scaled(24, dpi).min(width).min(height);
            let region = CreateRoundRectRgn(0, 0, width, height, radius, radius);
            if region.is_invalid() {
                return Err(Error::new(E_FAIL, "Could not create desktop clock region"));
            }
            if SetWindowRgn(self.hwnd, region, false) == 0 {
                let _ = DeleteObject(HGDIOBJ(region.0));
                return Err(Error::new(E_FAIL, "Could not apply desktop clock region"));
            }
            // The system owns a region after a successful SetWindowRgn.
        }
        self.dpi = dpi;
        self.rect = rect;
        Ok(true)
    }

    fn tick(&mut self, now: &SYSTEMTIME) -> Result<()> {
        // IsWindow alone cannot establish ownership after an HWND is recycled.
        if self.state.destroyed.get() {
            return Err(Error::new(E_FAIL, "Desktop clock window was destroyed"));
        }
        if !unsafe { IsWindow(self.hwnd) }.as_bool() {
            return Err(Error::new(E_FAIL, "Desktop clock window is no longer available"));
        }
        let layout_changed = self.refresh_layout(false)?;
        let minute = MinuteKey::from(now);
        let text_changed = self.minute != Some(minute);
        if text_changed {
            let (time, date, date_len) = text_for(now);
            {
                let mut paint = self.state.paint.try_borrow_mut()
                    .map_err(|_| Error::new(E_FAIL, "Desktop clock paint is busy"))?;
                paint.time = time;
                paint.date = date;
                paint.date_len = date_len;
            }
            self.minute = Some(minute);
        }
        let visible = desktop_is_foreground();
        let newly_visible = visible && !self.visible;
        if visible != self.visible {
            unsafe { let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE }); }
            self.visible = visible;
        }
        if visible && (text_changed || newly_visible || layout_changed) {
            self.invalidate()?;
        }
        Ok(())
    }
}

impl Drop for ClockWindow {
    fn drop(&mut self) {
        if !self.state.destroyed.get() && !self.hwnd.is_invalid() {
            // WM_NCDESTROY marks dead and clears the borrowed pointer. Fonts and Box are
            // still alive until this DestroyWindow call has completed.
            unsafe { let _ = DestroyWindow(self.hwnd); }
        }
    }
}

fn color_ref(color: Color, background: Color) -> COLORREF {
    let channel = |front: f32, back: f32| {
        ((front * color.a + back * (1.0 - color.a)).clamp(0.0, 1.0) * 255.0).round() as u32
    };
    COLORREF(channel(color.r, background.r)
        | (channel(color.g, background.g) << 8)
        | (channel(color.b, background.b) << 16))
}

fn paint(hwnd: HWND, dc: HDC, state: &WindowState) {
    let Ok(data) = state.paint.try_borrow() else { return };
    let mut client = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return;
    }
    let background = theme::bg();
    unsafe {
        let previous_brush_color = SetDCBrushColor(dc, color_ref(background, background));
        let brush = GetStockObject(DC_BRUSH);
        FillRect(dc, &client, HBRUSH(brush.0));
        let previous_mode = SetBkMode(dc, TRANSPARENT);
        let previous_color = SetTextColor(dc, color_ref(theme::text(), background));
        let previous_font = SelectObject(dc, HGDIOBJ(data.fonts.time.0));
        if !previous_font.is_invalid() {
            let height = client.bottom.saturating_sub(client.top);
            let mut time_rect = RECT { top: client.top + height / 12, bottom: client.top + height.saturating_mul(3) / 4, ..client };
            let mut date_rect = RECT { top: time_rect.bottom, bottom: client.bottom - height / 12, ..client };
            let flags = DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS;
            let mut time = data.time;
            let mut date = data.date;
            DrawTextW(dc, &mut time, &mut time_rect, flags);
            if !SelectObject(dc, HGDIOBJ(data.fonts.date.0)).is_invalid() {
                SetTextColor(dc, color_ref(theme::text_dim(), background));
                let date_len = data.date_len.min(date.len());
                DrawTextW(dc, &mut date[..date_len], &mut date_rect, flags);
            }
            SelectObject(dc, previous_font);
        }
        SetTextColor(dc, previous_color);
        SetBkMode(dc, BACKGROUND_MODE(previous_mode as u32));
        SetDCBrushColor(dc, previous_brush_color);
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_NCCREATE => {
                let Some(create) = (lparam.0 as *const CREATESTRUCTW).as_ref() else { return LRESULT(0) };
                if create.lpCreateParams.is_null() {
                    return LRESULT(0);
                }
                SetLastError(ERROR_SUCCESS);
                let previous = SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                if previous == 0 && GetLastError() != ERROR_SUCCESS {
                    return LRESULT(0);
                }
                return LRESULT(1);
            }
            WM_NCDESTROY => {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
                if let Some(state) = state.as_ref() {
                    state.destroyed.set(true);
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                // USERDATA is borrowed; ClockWindow owns and releases its Box.
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
                if !dc.is_invalid() {
                    if let Some(state) = state.as_ref() {
                        paint(hwnd, dc, state);
                    }
                }
                let _ = EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_NCHITTEST => return LRESULT(HTTRANSPARENT as isize),
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            _ => {}
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> SYSTEMTIME {
        SYSTEMTIME { wYear: 2026, wMonth: 11, wDay: 23, wHour: 9, wMinute: 5, ..Default::default() }
    }

    #[test]
    fn shutdown_defers_during_a_manager_borrow_and_is_idempotent() {
        std::thread::spawn(|| {
            CLOCK.with(|cell| {
                assert!(cell.borrow().is_none());
                let held = cell.borrow_mut();
                shutdown();
                assert!(SHUTDOWN_PENDING.with(Cell::get));
                assert!(held.is_none());
            });
            drain_pending_shutdown();
            assert!(!SHUTDOWN_PENDING.with(Cell::get));
            CLOCK.with(|cell| assert!(cell.borrow().is_none()));
            shutdown();
            shutdown();
            assert!(!SHUTDOWN_PENDING.with(Cell::get));
            CLOCK.with(|cell| assert!(cell.borrow().is_none()));
        }).join().unwrap();
    }

    #[test]
    fn widget_requires_the_exact_opt_in_value() {
        let mut map = BTreeMap::new();
        assert!(!enabled(&map));
        for value in ["0", "true", "yes", " 1", "1 "] {
            map.insert(SETTING_KEY.to_string(), value.to_string());
            assert!(!enabled(&map));
        }
        map.insert(SETTING_KEY.to_string(), "1".to_string());
        assert!(enabled(&map));
    }

    #[test]
    fn time_is_24_hour_and_seconds_do_not_change_the_redraw_key() {
        let mut value = now();
        assert_eq!(clock_text(&value), "09:05");
        let key = MinuteKey::from(&value);
        value.wSecond = 59;
        value.wMilliseconds = 999;
        assert_eq!(MinuteKey::from(&value), key);
        value.wHour = 23;
        value.wMinute = 59;
        assert_eq!(clock_text(&value), "23:59");
        value.wHour = 0;
        value.wMinute = 0;
        assert_eq!(clock_text(&value), "00:00");
    }

    #[test]
    fn the_redraw_key_includes_the_calendar_date() {
        let value = now();
        let original = MinuteKey::from(&value);
        for changed in [
            SYSTEMTIME { wMinute: 6, ..value },
            SYSTEMTIME { wHour: 10, ..value },
            SYSTEMTIME { wDay: 24, ..value },
            SYSTEMTIME { wMonth: 12, ..value },
            SYSTEMTIME { wYear: 2027, ..value },
        ] {
            assert_ne!(MinuteKey::from(&changed), original);
        }
    }

    #[test]
    fn foreground_policy_is_limited_to_desktop_classes() {
        assert!(desktop_class("Progman"));
        assert!(desktop_class("WorkerW"));
        for other in ["", "workerw", "Shell_TrayWnd", "NotchLiteWindow", "Chrome_WidgetWin_1"] {
            assert!(!desktop_class(other));
        }
    }

    #[test]
    fn geometry_fits_small_negative_and_high_dpi_work_areas() {
        for (work, dpi) in [
            (RECT { left: 0, top: 0, right: 1920, bottom: 1040 }, 96),
            (RECT { left: -1920, top: -100, right: 0, bottom: 940 }, 144),
            (RECT { left: -20, top: -30, right: 70, bottom: 40 }, 192),
            (RECT { left: i32::MIN, top: i32::MIN, right: i32::MAX, bottom: i32::MAX }, u32::MAX),
        ] {
            let rect = widget_rect(work, dpi).unwrap();
            assert!(rect.left >= work.left && rect.top >= work.top);
            assert!(rect.right <= work.right && rect.bottom <= work.bottom);
            assert!(rect.right > rect.left && rect.bottom > rect.top);
            assert!(i64::from(rect.right) - i64::from(rect.left) <= i64::from(i32::MAX));
        }
        let normal = widget_rect(RECT { left: 0, top: 40, right: 1920, bottom: 1040 }, 96).unwrap();
        assert_eq!((normal.left, normal.top, normal.right - normal.left, normal.bottom - normal.top), (780, 200, 360, 160));
        assert!(widget_rect(RECT { left: 5, top: 0, right: 5, bottom: 10 }, 96).is_none());
        assert!(widget_rect(RECT { left: 0, top: 9, right: 10, bottom: 0 }, 96).is_none());
    }
}
