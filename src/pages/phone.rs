//! Opt-in foreground controls. Construction and drawing do no network or disk I/O.
use std::net::Ipv4Addr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::gfx::{Align, DrawCtx, Point, Rect, TextStyle, Weight};
use crate::pages::{Host, Mouse, Page};
use crate::phone::{self, PhoneService, PhoneSnapshot, TransferStatus};
use crate::phone_qr::QrWindow;
use crate::{store, theme};

pub struct PhonePage {
    service: Option<PhoneService>,
    snapshot: Option<PhoneSnapshot>,
    qr: Option<QrWindow>,
    addresses: Vec<Ipv4Addr>,
    address_index: usize,
    message: String,
    hover: Option<usize>,
}

fn buttons(area: Rect, count: usize) -> [Rect; 3] {
    let gap = 8.0;
    let width = ((area.w - gap * (count.saturating_sub(1)) as f32) / count.max(1) as f32).max(0.0);
    std::array::from_fn(|i| Rect::new(area.x + i as f32 * (width + gap), area.y + 43.0, width, 25.0))
}

impl PhonePage {
    pub fn new() -> Self {
        Self {
            service: None, snapshot: None, qr: None, addresses: Vec::new(),
            address_index: 0, message: "Android + iPhone companion · same Wi-Fi".into(), hover: None,
        }
    }

    fn labels(&self) -> &'static [&'static str] {
        match self.snapshot.as_ref() {
            None => &["Choose network", "Start QR pairing"],
            Some(state) if state.pending_pair.is_some() => &["Codes match: allow", "Reject", "Stop"],
            Some(state) if state.incoming.as_ref().is_some_and(|t| t.status == TransferStatus::Pending) =>
                &["Accept file", "Reject file", "Disconnect"],
            Some(state) if state.paired => &["Received folder", "Cancel transfer", "Disconnect"],
            Some(state) if state.pairing_available => &["Show QR", "Stop"],
            Some(_) => &["New QR", "Stop"],
        }
    }

    fn hit(&self, p: Point, area: Rect) -> Option<usize> {
        buttons(area, self.labels().len()).iter().take(self.labels().len()).position(|r| r.contains(p))
    }

    fn stop(&mut self) {
        self.qr = None;
        // Drop closes listener, invalidates credentials and joins bounded workers.
        self.service = None;
        self.snapshot = None;
        self.message = "Disconnected · no phone listener running".into();
    }

    fn refresh(&mut self) {
        if let Some(service) = &self.service {
            let state = service.snapshot();
            if state.paired || !state.pairing_available {
                self.qr = None;
            }
            self.snapshot = Some(state);
        }
    }

    fn scan(&mut self) -> Result<(), String> {
        self.addresses = phone::private_ipv4_addresses()?;
        self.address_index = 0;
        if self.addresses.is_empty() {
            return Err("No private IPv4 network. Connect PC and phone to the same Wi-Fi.".into());
        }
        self.message = format!("Network: {} · click Choose network to cycle", self.addresses[0]);
        Ok(())
    }

    fn start(&mut self, host: &Host) -> Result<(), String> {
        if self.addresses.is_empty() {
            self.scan()?;
            if self.addresses.len() > 1 {
                self.message = "Multiple networks: choose the Wi-Fi IPv4, then Start.".into();
                return Ok(());
            }
        }
        let ip = *self.addresses.get(self.address_index).ok_or("Choose a network first")?;
        let service = PhoneService::start(ip, store::local_dir().join("Received"))?;
        let state = service.snapshot();
        // If QR creation fails, dropping the local service leaves no open listener.
        let qr = QrWindow::open(host.hwnd, &state.pairing_uri).map_err(|e| format!("Cannot show QR: {e}"))?;
        self.qr = Some(qr);
        self.snapshot = Some(state);
        self.service = Some(service);
        self.message = "Scan in NotchLite Companion · QR expires in 2 minutes".into();
        Ok(())
    }

    fn click(&mut self, index: usize, host: &Host) -> Result<(), String> {
        let Some(state) = self.snapshot.as_ref() else {
            if index == 0 {
                if self.addresses.is_empty() {
                    self.scan()?;
                } else {
                    self.address_index = (self.address_index + 1) % self.addresses.len();
                    self.message = format!("Network: {}", self.addresses[self.address_index]);
                }
            } else {
                self.start(host)?;
            }
            return Ok(());
        };
        if let Some(pair) = &state.pending_pair {
            let id = pair.id.clone();
            match index {
                0 => self.service.as_ref().unwrap().approve_pair(&id)?,
                1 => self.service.as_ref().unwrap().deny_pair(&id)?,
                _ => self.stop(),
            }
        } else if let Some(transfer) = state.incoming.as_ref().filter(|t| t.status == TransferStatus::Pending) {
            let id = transfer.id.clone();
            match index {
                0 => self.service.as_ref().unwrap().approve_incoming(&id)?,
                1 => self.service.as_ref().unwrap().deny_incoming(&id)?,
                _ => self.stop(),
            }
        } else if state.paired {
            match index {
                0 => {
                    let folder: Vec<u16> = store::local_dir().join("Received").as_os_str().encode_wide().chain(Some(0)).collect();
                    let result = unsafe { ShellExecuteW(host.hwnd, None, PCWSTR(folder.as_ptr()), None, None, SW_SHOWNORMAL) };
                    if result.0 as usize <= 32 { return Err("Could not open received folder".into()); }
                }
                1 => {
                    let id = state.incoming.as_ref().filter(|t| !matches!(t.status, TransferStatus::Complete | TransferStatus::Cancelled))
                        .or(state.outgoing.as_ref()).map(|t| t.id.clone());
                    if let Some(id) = id { self.service.as_ref().unwrap().cancel(&id)?; }
                }
                _ => self.stop(),
            }
        } else if index == 0 {
            if state.pairing_available {
                self.qr = Some(QrWindow::open(host.hwnd, &state.pairing_uri).map_err(|e| e.to_string())?);
            } else {
                self.stop();
                self.start(host)?;
            }
        } else {
            self.stop();
        }
        self.refresh();
        Ok(())
    }
}

impl Page for PhonePage {
    fn title(&self) -> String { "Phone · local session".into() }

    fn tick(&mut self) -> bool {
        if self.qr.as_ref().is_some_and(QrWindow::is_closed) {
            self.stop();
            return true;
        }
        if self.service.is_some() {
            self.refresh();
            return true;
        }
        false
    }

    fn draw(&mut self, ctx: &mut DrawCtx, area: Rect) {
        let (title, detail) = match self.snapshot.as_ref() {
            None => ("Phone link is off".to_string(), "Enable a temporary local session".to_string()),
            Some(s) if s.pending_pair.is_some() => {
                let p = s.pending_pair.as_ref().unwrap();
                (format!("Compare code: {}", p.code), format!("{} · approve only if the phone shows this code", p.name))
            }
            Some(s) if s.paired => {
                let incoming = s.incoming.as_ref().filter(|t| !matches!(t.status, TransferStatus::Complete | TransferStatus::Cancelled));
                let detail = if let Some(t) = incoming {
                    format!("{} · {}/{} bytes · {:?}", t.name, t.transferred, t.size, t.status)
                } else if let Some(t) = &s.outgoing {
                    format!("To phone: {} · {}/{} bytes · {:?}", t.name, t.transferred, t.size, t.status)
                } else if let Some(t) = &s.incoming {
                    format!("{} · {:?} · drop one file here to send", t.name, t.status)
                } else { "Drop one file here to send · max 256 MiB".to_string() };
                ("Phone authorized · encrypted LAN".to_string(), detail)
            }
            Some(s) => (if s.pairing_available { "Waiting for QR scan" } else { "QR used or expired · start a new session" }.to_string(), s.address.clone()),
        };
        ctx.push_clip(area);
        ctx.text(&title, Rect::new(area.x, area.y, area.w, 20.0), TextStyle::new(13.0, Weight::Semibold, theme::text()));
        ctx.text(&detail, Rect::new(area.x, area.y + 21.0, area.w, 18.0), TextStyle::new(10.5, Weight::Regular, theme::text_dim()));
        for (i, (rect, label)) in buttons(area, self.labels().len()).into_iter().zip(self.labels()).enumerate() {
            ctx.fill_rect(rect, 8.0, if self.hover == Some(i) { theme::card_hover() } else { theme::card() });
            ctx.text(label, rect, TextStyle::new(11.0, Weight::Medium, theme::text()).align(Align::Center));
        }
        let message = self.snapshot.as_ref().and_then(|s| s.last_error.as_deref()).unwrap_or(&self.message);
        ctx.text(message, Rect::new(area.x, area.y + 71.0, area.w, 17.0), TextStyle::new(10.0, Weight::Regular, theme::text_dim()));
        ctx.pop_clip();
    }

    fn on_mouse(&mut self, ev: Mouse, area: Rect, host: &mut Host) -> bool {
        match ev {
            Mouse::Move(p) => { let hover = self.hit(p, area); let changed = hover != self.hover; self.hover = hover; changed }
            Mouse::Leave => self.hover.take().is_some(),
            Mouse::Down(p) => {
                if let Some(index) = self.hit(p, area) {
                    if let Err(error) = self.click(index, host) { self.message = error; }
                    true
                } else { false }
            }
            _ => false,
        }
    }

    fn keep_open(&self) -> bool { self.snapshot.as_ref().is_some_and(|s| s.pending_pair.is_some() || s.incoming.as_ref().is_some_and(|t| t.status == TransferStatus::Pending)) }
    fn accepts_files(&self) -> bool { self.snapshot.as_ref().is_some_and(|s| s.paired) }
    fn on_drop_files(&mut self, paths: &[PathBuf]) -> bool {
        if paths.len() != 1 {
            self.message = "Choose one file at a time".into();
        } else if let Some(service) = &self.service {
            match service.offer_file(paths[0].clone()) {
                Ok(()) => self.message = "File offered · accept and save on phone".into(),
                Err(error) => self.message = error,
            }
            self.refresh();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_page_is_inert_until_explicit_start() {
        let page = PhonePage::new();
        assert!(page.service.is_none()); assert!(page.qr.is_none());
        assert!(page.addresses.is_empty()); assert!(!page.accepts_files());
        assert!(!page.keep_open()); assert_eq!(page.labels().len(), 2);
    }
    #[test]
    fn controls_fit_the_compact_body() {
        for count in [2, 3] {
            let area = Rect::new(0.0, 0.0, 392.0, 88.0);
            for button in buttons(area, count).into_iter().take(count) {
                assert!(button.x >= area.x && button.right() <= area.right() + 0.01);
                assert!(button.y >= area.y && button.bottom() <= area.bottom());
                assert!(button.w > 100.0);
            }
        }
    }
}
