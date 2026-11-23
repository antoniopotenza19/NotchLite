//! Temporary, independently owned GDI QR window; no second GPU renderer/timer.
use std::cell::Cell;

use qrcodegen::{QrCode, QrCodeEcc};
use windows::core::{w, Error, Result};
use windows::Win32::Foundation::{COLORREF, E_FAIL, ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, SetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::*;

struct State {
    destroyed: Cell<bool>,
    code: QrCode,
    font: HFONT,
}

impl Drop for State {
    fn drop(&mut self) { unsafe { let _ = DeleteObject(HGDIOBJ(self.font.0)); } }
}

pub struct QrWindow {
    hwnd: HWND,
    // USERDATA borrows this stable box; WM_NCDESTROY clears it before Drop.
    state: Box<State>,
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF { COLORREF(u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16) }

/// Integral QR modules and the mandatory four-module quiet zone.
fn qr_geometry(width: i32, height: i32, modules: i32) -> Option<(i32, i32, i32)> {
    if modules <= 0 { return None; }
    let side = modules.checked_add(8)?;
    let scale = (width.min(height) / side).max(0);
    if scale == 0 { return None; }
    Some(((width - side * scale) / 2, (height - side * scale) / 2, scale))
}

impl QrWindow {
    pub fn open(owner: HWND, payload: &str) -> Result<Self> {
        let code = QrCode::encode_text(payload, QrCodeEcc::Medium).map_err(|_| Error::new(E_FAIL, "QR payload too long"))?;
        let dpi = unsafe { GetDpiForWindow(owner) }.max(96);
        let scale = |dip: i32| ((i64::from(dip) * i64::from(dpi)) / 96) as i32;
        let font = unsafe {
            CreateFontW(-scale(14), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32, w!("Segoe UI"))
        };
        if font.is_invalid() { return Err(Error::new(E_FAIL, "Cannot create QR font")); }
        let mut window = Self { hwnd: HWND::default(), state: Box::new(State { destroyed: Cell::new(false), code, font }) };
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc), hInstance: instance.into(), lpszClassName: w!("NotchLitePairQR"),
                hCursor: LoadCursorW(None, IDC_ARROW)?, style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS { return Err(Error::from_win32()); }
            let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
            let mut rect = RECT { left: 0, top: 0, right: scale(380), bottom: scale(458) };
            AdjustWindowRectExForDpi(&mut rect, style, false, WS_EX_TOOLWINDOW, dpi)?;
            window.hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, w!("NotchLitePairQR"), w!("NotchLite - Pair your phone"),
                style, CW_USEDEFAULT, CW_USEDEFAULT, rect.right - rect.left, rect.bottom - rect.top,
                owner, None, instance, Some((&*window.state as *const State).cast()))?;
            let _ = ShowWindow(window.hwnd, SW_SHOWNORMAL);
        }
        Ok(window)
    }

    pub fn is_closed(&self) -> bool { self.state.destroyed.get() }
}

impl Drop for QrWindow {
    fn drop(&mut self) {
        if !self.state.destroyed.get() && !self.hwnd.is_invalid() {
            unsafe { let _ = DestroyWindow(self.hwnd); }
        }
    }
}

unsafe fn paint(hwnd: HWND, state: &State) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    if dc.is_invalid() { let _ = EndPaint(hwnd, &ps); return; }
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    let width = client.right.max(0);
    let height = client.bottom.max(0);
    let bg = CreateSolidBrush(rgb(24, 29, 38));
    if !bg.is_invalid() { FillRect(dc, &client, bg); let _ = DeleteObject(HGDIOBJ(bg.0)); }
    let old_font = SelectObject(dc, HGDIOBJ(state.font.0));
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, rgb(244, 247, 252));
    let dip = |value: i32| ((i64::from(value) * i64::from(width)) / 380) as i32;
    let text = |value: &str, top: i32, bottom: i32| {
        let mut value: Vec<u16> = value.encode_utf16().collect();
        let mut bounds = RECT { left: dip(12), top, right: width - dip(12), bottom };
        DrawTextW(dc, &mut value, &mut bounds, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
    };
    text("NotchLite Companion", dip(10), dip(40));
    text("Android + iPhone / same local Wi-Fi", dip(40), dip(66));
    let top = dip(78);
    let qr_height = (height - top - dip(74)).max(0);
    if let Some((left, y, scale)) = qr_geometry(width, qr_height, state.code.size()) {
        let y = top + y;
        let side = (state.code.size() + 8) * scale;
        let white = CreateSolidBrush(rgb(255, 255, 255));
        let black = CreateSolidBrush(rgb(0, 0, 0));
        if !white.is_invalid() && !black.is_invalid() {
            FillRect(dc, &RECT { left, top: y, right: left + side, bottom: y + side }, white);
            for row in 0..state.code.size() {
                for col in 0..state.code.size() {
                    if state.code.get_module(col, row) {
                        let x = left + (col + 4) * scale;
                        let y = y + (row + 4) * scale;
                        FillRect(dc, &RECT { left: x, top: y, right: x + scale, bottom: y + scale }, black);
                    }
                }
            }
        }
        if !white.is_invalid() { let _ = DeleteObject(HGDIOBJ(white.0)); }
        if !black.is_invalid() { let _ = DeleteObject(HGDIOBJ(black.0)); }
    }
    SetTextColor(dc, rgb(161, 173, 190));
    text("Scan in the companion app. QR expires in 2 minutes.", height - dip(60), height - dip(35));
    text("Compare codes on PC and phone before allowing.", height - dip(35), height - dip(10));
    SelectObject(dc, old_font);
    let _ = EndPaint(hwnd, &ps);
}

unsafe extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetLastError(ERROR_SUCCESS);
        if SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize) == 0 && GetLastError() != ERROR_SUCCESS {
            return LRESULT(0);
        }
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if let Some(state) = pointer.as_ref() {
        match message {
            WM_PAINT => { paint(hwnd, state); return LRESULT(0); }
            WM_ERASEBKGND => return LRESULT(1),
            WM_CLOSE => { let _ = DestroyWindow(hwnd); return LRESULT(0); }
            WM_KEYDOWN if wparam.0 == 0x1B => { let _ = DestroyWindow(hwnd); return LRESULT(0); }
            WM_DPICHANGED => {
                let rect = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(hwnd, None, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top, SWP_NOZORDER | SWP_NOACTIVATE);
                return LRESULT(0);
            }
            WM_NCDESTROY => { state.destroyed.set(true); SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qr_preserves_integer_modules_and_four_module_border() {
        for size in [21, 57, 69] {
            for width in [320, 380, 760] {
                let (x, y, scale) = qr_geometry(width, width, size).unwrap();
                assert!(scale >= 1); assert_eq!(x, y);
                assert!(x >= 0); assert!(x + (size + 8) * scale <= width);
            }
        }
        assert!(qr_geometry(10, 10, 21).is_none());
        assert!(qr_geometry(100, 100, 0).is_none());
    }
}
