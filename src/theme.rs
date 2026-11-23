//! Modified for NotchLite: geometry and lightweight, shared appearance presets.

use std::sync::atomic::{AtomicU8, Ordering};
use crate::gfx::{Color, Rect};

pub const COLLAPSED_W: f32 = 150.0;
pub const COLLAPSED_H: f32 = 30.0;
pub const EXPANDED_W: f32 = 420.0;
pub const EXPANDED_H: f32 = 140.0;
/// "Minimal notch" setting: a small, empty pill.
pub const MINIMAL_W: f32 = 104.0;
pub const MINIMAL_H: f32 = 24.0;
/// Size while showing a peek (notification, timer finished, ...).
pub const PEEK_W: f32 = 360.0;
pub const PEEK_H: f32 = 64.0;
/// Bottom corner radius = BASE + GROWTH * expansion (clamped to half the size).
pub const BOTTOM_RADIUS_BASE: f32 = 14.0;
pub const BOTTOM_RADIUS_GROWTH: f32 = 18.0;
/// Concave top "ear" radius = BASE + GROWTH * expansion.
pub const EAR_RADIUS_BASE: f32 = 10.0;
pub const EAR_RADIUS_GROWTH: f32 = 12.0;
/// Size springs (stiffness 230, damping 19 — a visible bounce).
pub const SIZE_STIFFNESS: f32 = 230.0;
pub const SIZE_DAMPING: f32 = 19.0;
/// Horizontal slide distance when switching pages.
pub const PAGE_SLIDE: f32 = 90.0;

/// Window size: room for the ears and spring overshoot.
pub const WINDOW_W: f32 = 640.0;
pub const WINDOW_H: f32 = 190.0;

pub const HEADER_H: f32 = 30.0;
/// Page content area, relative to the expanded panel's top-left corner.
pub const BODY: Rect = Rect { x: 38.0, y: 32.0, w: EXPANDED_W - 76.0, h: 88.0 };
pub const ARROW_RADIUS: f32 = 9.0;
pub const ARROW_INSET: f32 = 18.0;
pub const DOTS_Y: f32 = 130.0;
pub const DOT_SIZE: f32 = 5.0;
pub const DOT_ACTIVE_W: f32 = 14.0;
pub const DOT_GAP: f32 = 5.0;

pub const PRESET_KEY: &str = "appearance";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Preset {
    Original = 0,
    Graphite = 1,
    Midnight = 2,
    Rose = 3,
}

impl Preset {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value {
            Some("graphite") => Self::Graphite,
            Some("midnight") => Self::Midnight,
            Some("rose") => Self::Rose,
            _ => Self::Original,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Graphite => "graphite",
            Self::Midnight => "midnight",
            Self::Rose => "rose",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Graphite => "Graphite",
            Self::Midnight => "Midnight",
            Self::Rose => "Rose",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Original => Self::Graphite,
            Self::Graphite => Self::Midnight,
            Self::Midnight => Self::Rose,
            Self::Rose => Self::Original,
        }
    }

    fn palette(self) -> &'static Palette {
        match self {
            Self::Original => &ORIGINAL,
            Self::Graphite => &GRAPHITE,
            Self::Midnight => &MIDNIGHT,
            Self::Rose => &ROSE,
        }
    }
}

struct Palette {
    bg: Color,
    text: Color,
    accent: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::rgba(((hex >> 16) & 255) as f32 / 255.0,
                ((hex >> 8) & 255) as f32 / 255.0,
                (hex & 255) as f32 / 255.0, 1.0)
}

const ORIGINAL: Palette = Palette {
    bg: rgb(0x000000), text: rgb(0xFFFFFF),
    accent: Color::rgba(0.20, 0.86, 0.36, 1.0),
};
const GRAPHITE: Palette = Palette {
    bg: rgb(0x181D26), text: rgb(0xF4F7FC), accent: rgb(0x6BDEEF),
};
const MIDNIGHT: Palette = Palette {
    bg: rgb(0x11182D), text: rgb(0xF3F1FF), accent: rgb(0xBBA6FF),
};
const ROSE: Palette = Palette {
    bg: rgb(0x21181F), text: rgb(0xFFF2F7), accent: rgb(0xF3A4C8),
};

// Workers may read an accent for a queued notification. No lock, allocation,
// file access or background task is needed to select a built-in palette.
static ACTIVE_PRESET: AtomicU8 = AtomicU8::new(Preset::Original as u8);

pub fn apply(preset: Preset) {
    ACTIVE_PRESET.store(preset as u8, Ordering::Relaxed);
}

fn palette() -> &'static Palette {
    match ACTIVE_PRESET.load(Ordering::Relaxed) {
        1 => Preset::Graphite.palette(),
        2 => Preset::Midnight.palette(),
        3 => Preset::Rose.palette(),
        _ => Preset::Original.palette(),
    }
}

pub fn bg() -> Color { palette().bg }
pub fn text() -> Color { palette().text }
pub fn text_dim() -> Color { text().with_alpha(0.82) }
pub fn text_faint() -> Color { text().with_alpha(0.68) }
pub fn card() -> Color { text().with_alpha(0.08) }
pub fn card_hover() -> Color { text().with_alpha(0.14) }
pub fn border() -> Color { text().with_alpha(0.32) }
pub fn accent() -> Color { palette().accent }
pub fn warn() -> Color { Color::rgba(1.0, 0.72, 0.16, 1.0) }
pub fn danger() -> Color { Color::rgba(1.0, 0.30, 0.28, 1.0) }
pub fn blue() -> Color { Color::rgba(0.30, 0.62, 1.0, 1.0) }

#[cfg(test)]
mod tests {
    use super::*;

    const PRESETS: [Preset; 4] = [Preset::Original, Preset::Graphite, Preset::Midnight, Preset::Rose];

    #[test]
    fn settings_round_trip_and_unknown_values_preserve_original_appearance() {
        for preset in PRESETS {
            assert_eq!(Preset::from_setting(Some(preset.id())), preset);
        }
        assert_eq!(Preset::from_setting(None), Preset::Original);
        assert_eq!(Preset::from_setting(Some("untrusted-or-future-theme")), Preset::Original);
    }

    #[test]
    fn cycle_visits_each_preset_once_and_returns_to_original() {
        let mut current = Preset::Original;
        for expected in PRESETS {
            assert_eq!(current, expected);
            current = current.next();
        }
        assert_eq!(current, Preset::Original);
    }

    fn over(fg: Color, bg: Color) -> Color {
        Color::rgba(fg.r * fg.a + bg.r * (1.0 - fg.a),
                    fg.g * fg.a + bg.g * (1.0 - fg.a),
                    fg.b * fg.a + bg.b * (1.0 - fg.a), 1.0)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        fn luminance(c: Color) -> f32 {
            let linear = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
        }
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn text_tokens_remain_legible_on_panel_and_hover_card() {
        for preset in PRESETS {
            let p = preset.palette();
            for background in [p.bg, over(p.text.with_alpha(0.14), p.bg)] {
                for alpha in [1.0, 0.82, 0.68] {
                    assert!(contrast(over(p.text.with_alpha(alpha), background), background) >= 4.5,
                            "{:?}, alpha {}", preset, alpha);
                }
            }
            // Selected calendar labels and checked controls use bg as ink.
            for surface in [p.accent, p.text] {
                assert!(contrast(p.bg, surface) >= 4.5, "{:?} selected control", preset);
            }
        }
    }
}
