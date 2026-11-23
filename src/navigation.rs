//! Stack-only header navigation geometry. The shell retains its legacy
//! navigation when the icon pill and a readable status area cannot both fit.

use crate::gfx::{Point, Rect};

const HEADER_H: f32 = 30.0;
const MARGIN: f32 = 14.0;
const STATUS_MIN_W: f32 = 64.0;
const STATUS_GAP: f32 = 12.0;
const TAB_SIZE: f32 = 24.0;
const TAB_GAP: f32 = 2.0;
const PADDING: f32 = 3.0;
const MAX_TABS: usize = 12;

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub pill: Rect,
    pub status: Rect,
    page_count: usize,
}

impl Layout {
    pub fn new(panel: Rect, page_count: usize) -> Option<Self> {
        if page_count == 0 || page_count > MAX_TABS || panel.h < HEADER_H
            || [panel.x, panel.y, panel.w, panel.h, panel.right(), panel.bottom()]
                .iter().any(|value| !value.is_finite())
        {
            return None;
        }
        let count = page_count as f32;
        let pill_w = count * TAB_SIZE + (count - 1.0) * TAB_GAP + 2.0 * PADDING;
        if panel.w < MARGIN * 2.0 + STATUS_MIN_W + STATUS_GAP + pill_w {
            return None;
        }
        let pill = Rect::new(panel.right() - MARGIN - pill_w, panel.y, pill_w, HEADER_H);
        let status = Rect::new(
            panel.x + MARGIN, panel.y,
            pill.x - STATUS_GAP - panel.x - MARGIN, HEADER_H,
        );
        Some(Self { pill, status, page_count })
    }

    pub fn tab(&self, index: usize) -> Option<Rect> {
        if index >= self.page_count {
            return None;
        }
        Some(Rect::new(
            self.pill.x + PADDING + index as f32 * (TAB_SIZE + TAB_GAP),
            self.pill.y + PADDING, TAB_SIZE, TAB_SIZE,
        ))
    }

    /// Rect::contains is half-open: padding, gaps and the trailing edge do not
    /// select the nearest tab or intercept the status area.
    pub fn hit(&self, point: Point) -> Option<usize> {
        (0..self.page_count).find(|&index| self.tab(index).is_some_and(|rect| rect.contains(point)))
    }
}

/// Only the current window's Ctrl+Tab navigation; no hook or global shortcut.
pub fn key_step(vk: u16, ctrl: bool, shift: bool) -> Option<isize> {
    if ctrl && vk == 0x09 {
        Some(if shift { -1 } else { 1 })
    } else {
        None
    }
}

/// Segoe Fluent Icons glyphs. This mapping adds no pages or integrations.
pub fn glyph(feature_id: &str) -> &'static str {
    match feature_id {
        "media" => "\u{E8D6}",         // MusicInfo
        "shelf" => "\u{E8B7}",         // Folder
        "todo" => "\u{E73E}",          // CheckMark
        "calendar" => "\u{E787}",      // Calendar
        "focus" => "\u{E916}",         // Stopwatch
        "weather" => "\u{E9BD}",       // Sunny
        "football" => "\u{E7C1}",      // Flag
        "system" => "\u{E770}",        // System
        "claude" => "\u{E8F2}",        // Message
        "notifications" => "\u{E7E7}", // ActionCenter
        "phone" => "\u{E8EA}",         // CellPhone
        "settings" => "\u{E713}",      // Settings
        _ => "\u{E71D}",               // AllApps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect::new(80.0, 12.0, 420.0, 140.0)
    }

    #[test]
    fn supported_counts_keep_tabs_inside_the_header_and_reserve_status_space() {
        for count in [1, 4, 11, 12] {
            let panel = panel();
            let layout = Layout::new(panel, count).unwrap();
            assert_eq!(layout.pill.right(), panel.right() - MARGIN);
            assert_eq!(layout.pill.h, HEADER_H);
            assert!(layout.status.w >= STATUS_MIN_W);
            assert_eq!(layout.status.x, panel.x + MARGIN);
            assert_eq!(layout.status.right() + STATUS_GAP, layout.pill.x);
            for index in 0..count {
                let tab = layout.tab(index).unwrap();
                assert_eq!((tab.w, tab.h), (TAB_SIZE, TAB_SIZE));
                assert!(tab.x >= layout.pill.x + PADDING);
                assert!(tab.right() <= layout.pill.right() - PADDING);
                assert!(tab.y >= panel.y && tab.bottom() <= panel.y + HEADER_H);
            }
            assert!(layout.tab(count).is_none());
            assert!(layout.tab(usize::MAX).is_none());
        }
    }

    #[test]
    fn twelve_tabs_fit_exactly_and_unsupported_layouts_use_the_fallback() {
        let layout = Layout::new(panel(), 12).unwrap();
        assert_eq!(layout.pill.w, 316.0);
        assert_eq!(layout.status.w, 64.0);
        assert!(Layout::new(Rect { w: 419.0, ..panel() }, 12).is_none());
        assert!(Layout::new(panel(), 0).is_none());
        assert!(Layout::new(Rect { w: 1000.0, ..panel() }, 13).is_none());
        assert!(Layout::new(panel(), usize::MAX).is_none());
        assert!(Layout::new(Rect { w: 0.0, ..panel() }, 1).is_none());
        assert!(Layout::new(Rect { h: 29.0, ..panel() }, 1).is_none());
        assert!(Layout::new(Rect::new(-50.0, -20.0, 134.0, 30.0), 1).is_some());
        assert!(Layout::new(Rect { w: 133.0, ..panel() }, 1).is_none());
    }

    #[test]
    fn hit_testing_selects_every_tab_and_never_the_gaps_or_padding() {
        for count in [1, 4, 11, 12] {
            let layout = Layout::new(panel(), count).unwrap();
            for index in 0..count {
                let tab = layout.tab(index).unwrap();
                assert_eq!(layout.hit(tab.center()), Some(index));
                assert_eq!(layout.hit(Point { x: tab.x, y: tab.y }), Some(index));
                assert_eq!(layout.hit(Point { x: tab.right(), y: tab.center().y }), None);
                assert_eq!(layout.hit(Point { x: tab.center().x, y: tab.bottom() }), None);
                if index + 1 < count {
                    let next = layout.tab(index + 1).unwrap();
                    assert_eq!(layout.hit(Point { x: (tab.right() + next.x) / 2.0, y: tab.center().y }), None);
                }
            }
            assert_eq!(layout.hit(layout.status.center()), None);
            assert_eq!(layout.hit(Point { x: layout.pill.x + 1.0, y: layout.pill.center().y }), None);
            assert_eq!(layout.hit(Point { x: layout.pill.center().x, y: layout.pill.y + 1.0 }), None);
            assert_eq!(layout.hit(Point { x: layout.pill.right() - 1.0, y: layout.pill.center().y }), None);
            assert_eq!(layout.hit(Point { x: layout.pill.right() + 1.0, y: layout.pill.center().y }), None);
            assert_eq!(layout.hit(Point { x: f32::NAN, y: layout.pill.y }), None);
        }
    }

    #[test]
    fn invalid_or_overflowing_coordinates_are_not_a_layout() {
        for invalid in [
            Rect { x: f32::NAN, ..panel() },
            Rect { w: f32::INFINITY, ..panel() },
            Rect { x: f32::MAX, w: f32::MAX, ..panel() },
            Rect { w: -10.0, ..panel() },
        ] {
            assert!(Layout::new(invalid, 1).is_none());
        }
    }

    #[test]
    fn shortcuts_require_ctrl_tab_and_shift_only_reverses_that_command() {
        assert_eq!(key_step(0x09, true, false), Some(1));
        assert_eq!(key_step(0x09, true, true), Some(-1));
        assert_eq!(key_step(0x09, false, false), None);
        assert_eq!(key_step(0x09, false, true), None);
        for other in [0, 0x1B, 0x25, 0x27, 0x41] {
            assert_eq!(key_step(other, true, false), None);
            assert_eq!(key_step(other, true, true), None);
        }
    }

    #[test]
    fn feature_glyphs_are_static_native_icons() {
        for feature in [
            "media", "shelf", "todo", "calendar", "focus", "weather",
            "football", "system", "claude", "notifications", "phone", "settings",
        ] {
            let icon = glyph(feature);
            assert_eq!(icon.chars().count(), 1);
            assert!(('\u{E000}'..='\u{F8FF}').contains(&icon.chars().next().unwrap()));
        }
        assert_eq!(glyph("media"), "\u{E8D6}");
        assert_eq!(glyph("settings"), "\u{E713}");
        assert_eq!(glyph("phone"), "\u{E8EA}");
        assert_eq!(glyph(""), glyph("unknown-feature"));
    }
}
