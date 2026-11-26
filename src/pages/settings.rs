//! Modified for NotchLite: appearance preset selector, fresh-map writes and
//! visible save failures. Optional features remain individually configurable.

use std::collections::BTreeMap;
use std::io;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::desktop_clock;
use crate::gfx::{Align, DrawCtx, Point, Rect, TextStyle, Weight};
use crate::pages::{self, Host, Mouse, Page};
use crate::store;
use crate::theme;
use crate::worker::{self, ShellCmd, Waker};

const FILE: &str = "settings.txt";
const COLS: usize = 2;
const COL_GAP: f32 = 12.0;
const ROW_H: f32 = 22.0;
const ROW_GAP: f32 = 4.0;
const ROW_RADIUS: f32 = 6.0;
const PAD_X: f32 = 6.0;
const LABEL_GAP: f32 = 6.0;
const SWITCH_W: f32 = 26.0;
const SWITCH_H: f32 = 14.0;
const SWITCH_R: f32 = 7.0;
const KNOB_R: f32 = 5.0;
const ERROR_H: f32 = 18.0;
const PRESET_VALUE_W: f32 = 60.0;
const PRESET_ARROW_W: f32 = 12.0;

/// Preserves scroll position when `ReloadSettings` recreates `SettingsPage`.
static LAST_SCROLL: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy)]
enum Kind {
    Appearance,
    AssistantMonitor,
    Feature(&'static str),
    Flag { key: &'static str, default_on: bool },
}

struct Row {
    label: &'static str,
    kind: Kind,
}

fn is_row_on(map: &BTreeMap<String, String>, kind: Kind) -> bool {
    match kind {
        Kind::Appearance | Kind::AssistantMonitor => false,
        Kind::Feature(id) => !map
            .get(pages::DISABLED_KEY)
            .is_some_and(|v| v.split(',').any(|x| x.trim() == id)),
        Kind::Flag { key, default_on: true } => map.get(key).map_or(true, |v| v != "0"),
        Kind::Flag { key, default_on: false } => map.get(key).is_some_and(|v| v == "1"),
    }
}

fn changed_settings(mut map: BTreeMap<String, String>, kind: Kind) -> BTreeMap<String, String> {
    let next_on = !is_row_on(&map, kind);
    match kind {
        Kind::Appearance => {
            let next = theme::Preset::from_setting(map.get(theme::PRESET_KEY).map(String::as_str)).next();
            map.insert(theme::PRESET_KEY.to_string(), next.id().to_string());
        }
        Kind::AssistantMonitor => {}, // monitor enumeration belongs to the explicit click only
        Kind::Feature(id) => {
            let mut disabled: Vec<String> = map
                .get(pages::DISABLED_KEY)
                .map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect())
                .unwrap_or_default();
            if next_on {
                disabled.retain(|x| x != id);
            } else if !disabled.iter().any(|x| x == id) {
                disabled.push(id.to_string());
            }
            if disabled.is_empty() {
                map.remove(pages::DISABLED_KEY);
            } else {
                map.insert(pages::DISABLED_KEY.to_string(), disabled.join(","));
            }
        }
        Kind::Flag { key, .. } => {
            map.insert(key.to_string(), if next_on { "1" } else { "0" }.to_string());
        }
    }
    map
}

pub struct SettingsPage {
    rows: Vec<Row>,
    map: BTreeMap<String, String>,
    /// Unscrolled row rectangles from layout; scroll offset is applied on draw and hit-test.
    rects: Vec<Rect>,
    last_area: Option<Rect>,
    view_h: f32,
    scroll: f32,
    hover: Option<usize>,
    inside: bool,
    save_error: bool,
}

fn select_next_monitor(mut map: BTreeMap<String, String>, devices: &[String]) -> BTreeMap<String, String> {
    let current = map.get(crate::assist::MONITOR_KEY).map(String::as_str).unwrap_or("auto");
    let next = if current == "auto" || current.is_empty() {
        devices.first().map(String::as_str).unwrap_or("auto")
    } else {
        devices.iter().position(|device| device == current).and_then(|i| devices.get(i + 1))
            .map(String::as_str).unwrap_or("auto")
    }.to_string();
    map.insert(crate::assist::MONITOR_KEY.into(), next);
    map
}

impl SettingsPage {
    pub fn new(_waker: Waker) -> Self {
        let mut rows = Vec::with_capacity(pages::FEATURES.len() + 8);
        rows.push(Row { label: "Appearance", kind: Kind::Appearance });
        rows.push(Row { label: "Desktop clock", kind: Kind::Flag { key: desktop_clock::SETTING_KEY, default_on: false } });
        rows.push(Row { label: "Phone link (LAN)", kind: Kind::Flag { key: pages::PHONE_KEY, default_on: false } });
        rows.push(Row { label: "Codex dashboard", kind: Kind::Flag { key: crate::assist::DASHBOARD_KEY, default_on: false } });
        rows.push(Row { label: "Call assistant", kind: Kind::Flag { key: crate::assist::CALL_KEY, default_on: false } });
        rows.push(Row { label: "Assistant monitor", kind: Kind::AssistantMonitor });
        for &(id, label) in pages::FEATURES {
            rows.push(Row { label, kind: Kind::Feature(id) });
        }
        rows.push(Row { label: "Mascot", kind: Kind::Flag { key: "mascot", default_on: true } });
        rows.push(Row { label: "Minimal notch", kind: Kind::Flag { key: "minimal", default_on: false } });

        let saved = f32::from_bits(LAST_SCROLL.load(Ordering::Relaxed));
        let scroll = if saved.is_finite() { saved.max(0.0) } else { 0.0 };

        Self {
            rows,
            map: store::read_kv(FILE),
            rects: Vec::new(),
            last_area: None,
            view_h: theme::BODY.h,
            scroll,
            hover: None,
            inside: false,
            save_error: false,
        }
    }

    fn rows_area(&self, area: Rect) -> Rect {
        Rect { h: (area.h - if self.save_error { ERROR_H } else { 0.0 }).max(0.0), ..area }
    }

    fn max_scroll(&self) -> f32 {
        let grid_rows = (self.rows.len() + COLS - 1) / COLS;
        let content_h = if grid_rows == 0 {
            0.0
        } else {
            grid_rows as f32 * ROW_H + (grid_rows - 1) as f32 * ROW_GAP
        };
        (content_h - self.view_h).max(0.0)
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
        LAST_SCROLL.store(self.scroll.to_bits(), Ordering::Relaxed);
    }

    fn layout_rects(&mut self, area: Rect) {
        let col_w = ((area.w - COL_GAP * (COLS as f32 - 1.0)) / COLS as f32).max(0.0);
        self.rects.clear();
        self.rects.reserve(self.rows.len());
        for i in 0..self.rows.len() {
            let col = i % COLS;
            let row = i / COLS;
            let x = area.x + col as f32 * (col_w + COL_GAP);
            let y = area.y + row as f32 * (ROW_H + ROW_GAP);
            self.rects.push(Rect::new(x, y, col_w, ROW_H));
        }
        self.last_area = Some(area);
    }

    fn hit(&self, p: Point, area: Rect) -> Option<usize> {
        if !self.rows_area(area).contains(p) {
            return None;
        }
        self.rects.iter().position(|r| r.offset(0.0, -self.scroll).contains(p))
    }

    fn toggle(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else { return };
        let fresh = match store::try_read_kv(FILE) {
            Ok(map) => map,
            Err(_) => {
                self.save_error = true;
                return;
            }
        };
        let map = if matches!(row.kind, Kind::AssistantMonitor) {
            let devices = match crate::assist_overlay::secondary_devices() {
                Ok(devices) => devices,
                Err(_) => { self.save_error = true; return; }
            };
            select_next_monitor(fresh, &devices)
        } else { changed_settings(fresh, row.kind) };
        if self.commit_with(map, |map| store::write_kv(FILE, map)) {
            worker::send(ShellCmd::ReloadSettings);
        }
    }

    fn commit_with(
        &mut self,
        map: BTreeMap<String, String>,
        write: impl FnOnce(&BTreeMap<String, String>) -> io::Result<()>,
    ) -> bool {
        match write(&map) {
            Ok(()) => {
                self.map = map;
                self.save_error = false;
                true
            }
            Err(_) => {
                self.save_error = true;
                false
            }
        }
    }
}

impl Page for SettingsPage {
    fn title(&self) -> String {
        "Settings".to_string()
    }

    fn draw(&mut self, ctx: &mut DrawCtx, area: Rect) {
        let rows_area = self.rows_area(area);
        self.view_h = rows_area.h;
        self.clamp_scroll();
        self.layout_rects(rows_area);

        ctx.push_clip(rows_area);
        for (i, (row, &base)) in self.rows.iter().zip(self.rects.iter()).enumerate() {
            let r = base.offset(0.0, -self.scroll);
            if r.bottom() <= rows_area.y || r.y >= rows_area.bottom() {
                continue;
            }
            if self.hover == Some(i) {
                ctx.fill_rect(r, ROW_RADIUS, theme::card());
            }
            if matches!(row.kind, Kind::Appearance | Kind::AssistantMonitor) {
                let arrow = Rect::new(r.right() - PAD_X - PRESET_ARROW_W, r.y, PRESET_ARROW_W, ROW_H);
                let value = Rect::new(arrow.x - 4.0 - PRESET_VALUE_W, r.y, PRESET_VALUE_W, ROW_H);
                let label = Rect::new(r.x + PAD_X, r.y, (value.x - 4.0 - r.x - PAD_X).max(0.0), ROW_H);
                let selected = if matches!(row.kind, Kind::Appearance) {
                    theme::Preset::from_setting(self.map.get(theme::PRESET_KEY).map(String::as_str)).label().to_string()
                } else {
                    self.map.get(crate::assist::MONITOR_KEY).filter(|v| !v.is_empty() && *v != "auto")
                        .map(|v| v.trim_start_matches("\\\\.\\").to_string()).unwrap_or_else(|| "Auto".into())
                };
                ctx.stroke_rect(r, ROW_RADIUS, theme::border(), 1.0, false);
                ctx.text(row.label, label, TextStyle::new(11.5, Weight::Regular, theme::text_dim()));
                ctx.text(&selected, value, TextStyle::new(11.5, Weight::Semibold, theme::text()).align(Align::Right));
                ctx.text("\u{203A}", arrow, TextStyle::new(14.0, Weight::Medium, theme::text_dim()).align(Align::Center));
                continue;
            }
            let on = is_row_on(&self.map, row.kind);
            let track = Rect::new(
                r.right() - PAD_X - SWITCH_W,
                r.y + (ROW_H - SWITCH_H) / 2.0,
                SWITCH_W,
                SWITCH_H,
            );
            ctx.fill_rect(track, SWITCH_R, if on { theme::accent() } else { theme::card_hover() });
            let knob_x = if on { track.right() - SWITCH_R } else { track.x + SWITCH_R };
            ctx.fill_circle(Point { x: knob_x, y: track.center().y }, KNOB_R, if on { theme::bg() } else { theme::text() });

            let label_x = r.x + PAD_X;
            let label_w = (track.x - LABEL_GAP - label_x).max(0.0);
            let label_rect = Rect::new(label_x, r.y, label_w, ROW_H);
            ctx.text(row.label, label_rect, TextStyle::new(12.0, Weight::Regular, theme::text()));
        }
        ctx.pop_clip();
        if self.save_error {
            let error_area = Rect::new(area.x, rows_area.bottom(), area.w, (area.bottom() - rows_area.bottom()).max(0.0));
            ctx.text("Could not save settings. Try again.", error_area, TextStyle::new(10.5, Weight::Medium, theme::danger()));
        }
    }

    fn on_mouse(&mut self, ev: Mouse, area: Rect, _host: &mut Host) -> bool {
        let rows_area = self.rows_area(area);
        self.view_h = rows_area.h;
        if self.last_area != Some(rows_area) {
            self.clamp_scroll();
            self.layout_rects(rows_area);
        }
        match ev {
            Mouse::Move(p) => {
                let mut changed = false;
                if !self.inside {
                    self.inside = true;
                    if let Ok(fresh) = store::try_read_kv(FILE) {
                        if fresh != self.map {
                            self.map = fresh;
                            changed = true;
                        }
                    }
                }
                let hit = self.hit(p, area);
                if hit != self.hover {
                    self.hover = hit;
                    changed = true;
                }
                changed
            }
            Mouse::Leave => {
                self.inside = false;
                self.hover.take().is_some()
            }
            // A double click arrives as Down + DoubleClick; toggling on both would undo it.
            Mouse::DoubleClick(_) => false,
            Mouse::Down(p) => {
                self.hover = self.hit(p, area);
                if let Some(i) = self.hover {
                    self.toggle(i);
                    true
                } else {
                    false
                }
            }
            Mouse::Wheel(p, delta) => {
                if !rows_area.contains(p) {
                    return false;
                }
                let before = self.scroll;
                self.scroll -= delta * (ROW_H + ROW_GAP);
                self.clamp_scroll();
                self.hover = self.hit(p, area);
                self.scroll != before
            }
            Mouse::Up(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_map() -> BTreeMap<String, String> {
        BTreeMap::from([
            (desktop_clock::SETTING_KEY.to_string(), "0".to_string()),
            ("mascot".to_string(), "0".to_string()),
            ("minimal".to_string(), "1".to_string()),
            (pages::DISABLED_KEY.to_string(), "weather,media".to_string()),
            ("custom_future_option".to_string(), "keep=exactly|this".to_string()),
        ])
    }

    fn page(map: BTreeMap<String, String>) -> SettingsPage {
        SettingsPage {
            rows: vec![Row { label: "Appearance", kind: Kind::Appearance }],
            map,
            rects: Vec::new(),
            last_area: None,
            view_h: theme::BODY.h,
            scroll: 0.0,
            hover: None,
            inside: false,
            save_error: false,
        }
    }

    #[test]
    fn appearance_cycles_and_changes_no_other_preference() {
        let base = sample_map();
        let mut current = base.clone();
        for expected in ["graphite", "midnight", "rose", "original", "graphite"] {
            current = changed_settings(current, Kind::Appearance);
            let mut complete_expected = base.clone();
            complete_expected.insert(theme::PRESET_KEY.to_string(), expected.to_string());
            assert_eq!(current, complete_expected);
        }
    }

    #[test]
    fn invalid_appearance_uses_original_before_advancing() {
        let mut initial = sample_map();
        initial.insert(theme::PRESET_KEY.to_string(), "unknown-theme".to_string());
        let changed = changed_settings(initial.clone(), Kind::Appearance);
        initial.insert(theme::PRESET_KEY.to_string(), "graphite".to_string());
        assert_eq!(changed, initial);
    }

    #[test]
    fn toggles_keep_the_current_appearance_and_unrelated_keys() {
        let mut initial = sample_map();
        initial.insert(theme::PRESET_KEY.to_string(), "midnight".to_string());
        let with_weather = changed_settings(initial.clone(), Kind::Feature("weather"));
        let mut expected = initial;
        expected.insert(pages::DISABLED_KEY.to_string(), "media".to_string());
        assert_eq!(with_weather, expected);
        let with_mascot = changed_settings(with_weather, Kind::Flag { key: "mascot", default_on: true });
        expected.insert("mascot".to_string(), "1".to_string());
        assert_eq!(with_mascot, expected);
    }

    #[test]
    fn toggling_the_desktop_clock_keeps_the_preset_and_disabled_features() {
        let mut initial = sample_map();
        initial.insert(theme::PRESET_KEY.to_string(), "midnight".to_string());
        let flag = Kind::Flag { key: desktop_clock::SETTING_KEY, default_on: false };
        let enabled = changed_settings(initial.clone(), flag);
        let mut expected = initial.clone();
        expected.insert(desktop_clock::SETTING_KEY.to_string(), "1".to_string());
        assert_eq!(enabled, expected);
        assert_eq!(changed_settings(enabled, flag), initial);
    }

    #[test]
    fn a_failed_save_does_not_apply_the_selection_and_a_retry_can_succeed() {
        let initial = sample_map();
        let requested = changed_settings(initial.clone(), Kind::Appearance);
        let mut settings = page(initial.clone());
        assert!(!settings.commit_with(requested.clone(), |_| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "synthetic save failure"))
        }));
        assert_eq!(settings.map, initial);
        assert!(settings.save_error);
        assert!(settings.rows_area(theme::BODY).h < theme::BODY.h);

        let mut written = None;
        assert!(settings.commit_with(requested.clone(), |map| {
            written = Some(map.clone());
            Ok(())
        }));
        assert_eq!(written, Some(requested.clone()));
        assert_eq!(settings.map, requested);
        assert!(!settings.save_error);
    }

    #[test]
    fn phone_is_opt_in_and_other_settings_preserve_that_choice() {
        let initial = sample_map();
        let flag = Kind::Flag { key: pages::PHONE_KEY, default_on: false };
        assert!(!is_row_on(&initial, flag));
        let enabled = changed_settings(initial.clone(), flag);
        assert!(is_row_on(&enabled, flag));
        let other = changed_settings(enabled, Kind::Appearance);
        assert!(is_row_on(&other, flag));
        let disabled = changed_settings(other, flag);
        assert!(!is_row_on(&disabled, flag));
        assert_eq!(disabled.get("custom_future_option"), initial.get("custom_future_option"));
    }

    #[test]
    fn assistant_monitor_cycles_only_the_supplied_secondary_devices() {
        let initial = sample_map();
        let devices = vec!["DISPLAY2".to_string(), "DISPLAY3".to_string()];
        let one = select_next_monitor(initial.clone(), &devices);
        assert_eq!(one.get(crate::assist::MONITOR_KEY).unwrap(), "DISPLAY2");
        let two = select_next_monitor(one, &devices);
        assert_eq!(two.get(crate::assist::MONITOR_KEY).unwrap(), "DISPLAY3");
        let auto = select_next_monitor(two, &devices);
        assert_eq!(auto.get(crate::assist::MONITOR_KEY).unwrap(), "auto");
        assert_eq!(auto.get("custom_future_option"), initial.get("custom_future_option"));
        assert_eq!(select_next_monitor(auto, &[]).get(crate::assist::MONITOR_KEY).unwrap(), "auto");
    }
}
