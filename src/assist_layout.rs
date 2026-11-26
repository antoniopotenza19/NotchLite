// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Pure secondary-monitor selection and physical-pixel overlay placement.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

impl ScreenRect {
    pub fn width(self) -> i32 { self.right.saturating_sub(self.left) }
    pub fn height(self) -> i32 { self.bottom.saturating_sub(self.top) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monitor { pub device: String, pub primary: bool, pub work: ScreenRect }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementError { MissingSecondary, ChooseSecondary, InsufficientSpace, InvalidGeometry }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayRects { pub dashboard: Option<ScreenRect>, pub call: Option<ScreenRect> }

pub fn select_secondary<'a>(monitors: &'a [Monitor], requested: Option<&str>) -> Result<&'a Monitor, PlacementError> {
    if let Some(device) = requested {
        return monitors.iter().find(|m| !m.primary && m.device == device).ok_or(PlacementError::MissingSecondary);
    }
    let mut candidates = monitors.iter().filter(|m| !m.primary);
    let selected = candidates.next().ok_or(PlacementError::MissingSecondary)?;
    if candidates.next().is_some() { return Err(PlacementError::ChooseSecondary); }
    Ok(selected)
}

pub fn overlay_rects(work: ScreenRect, dpi: u32, dashboard: bool, call: bool) -> Result<OverlayRects, PlacementError> {
    if !dashboard && !call { return Ok(OverlayRects { dashboard: None, call: None }); }
    if dpi == 0 || work.right <= work.left || work.bottom <= work.top { return Err(PlacementError::InvalidGeometry); }
    let px = |dip: i64| (dip * i64::from(dpi) + 48) / 96;
    let margin = px(16).max(1);
    let gap = px(16).max(1);
    let available_w = i64::from(work.right) - i64::from(work.left) - margin * 2;
    let available_h = i64::from(work.bottom) - i64::from(work.top) - margin * 2;
    let min_dashboard_w = px(320).max(1);
    let min_call_w = px(300).max(1);
    if available_h < if dashboard { px(520) } else { px(440) } {
        return Err(PlacementError::InsufficientSpace);
    }
    let (dashboard_w, call_w) = match (dashboard, call) {
        (true, true) => {
            if available_w < min_dashboard_w + min_call_w + gap { return Err(PlacementError::InsufficientSpace); }
            let left = px(480).min(available_w - min_call_w - gap);
            (left, px(420).min(available_w - left - gap))
        }
        (true, false) if available_w >= min_dashboard_w => (px(480).min(available_w), 0),
        (false, true) if available_w >= min_call_w => (0, px(420).min(available_w)),
        _ => return Err(PlacementError::InsufficientSpace),
    };
    let rect = |left: i64, width: i64, preferred_h: i64| -> Result<ScreenRect, PlacementError> {
        let height = px(preferred_h).min(available_h);
        if width <= 0 || height <= 0 || width > i64::from(i32::MAX) || height > i64::from(i32::MAX) { return Err(PlacementError::InvalidGeometry); }
        let top = i64::from(work.top) + margin;
        Ok(ScreenRect {
            left: left.try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            top: top.try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            right: (left + width).try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            bottom: (top + height).try_into().map_err(|_| PlacementError::InvalidGeometry)?,
        })
    };
    Ok(OverlayRects {
        dashboard: if dashboard { Some(rect(i64::from(work.left) + margin, dashboard_w, 700)?) } else { None },
        call: if call { Some(rect(i64::from(work.right) - margin - call_w, call_w, 540)?) } else { None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(device: &str, primary: bool) -> Monitor {
        Monitor { device: device.into(), primary, work: ScreenRect { left: -1920, top: -120, right: 0, bottom: 960 } }
    }
    #[test]
    fn secondary_selection_never_falls_back_to_primary_or_another_device() {
        let primary = monitor("primary", true);
        assert_eq!(select_secondary(&[primary.clone()], None), Err(PlacementError::MissingSecondary));
        let displays = [primary, monitor("left", false), monitor("right", false)];
        assert_eq!(select_secondary(&displays, None), Err(PlacementError::ChooseSecondary));
        assert_eq!(select_secondary(&displays, Some("primary")), Err(PlacementError::MissingSecondary));
        assert_eq!(select_secondary(&displays, Some("unplugged")), Err(PlacementError::MissingSecondary));
        assert_eq!(select_secondary(&displays, Some("left")).unwrap().device, "left");
        assert_eq!(select_secondary(&displays[..2], None).unwrap().device, "left");
    }
    #[test]
    fn corners_fit_negative_and_above_primary_coordinates_without_overlap() {
        for work in [
            ScreenRect { left: -1920, top: 40, right: 0, bottom: 1080 },
            ScreenRect { left: 0, top: -1440, right: 2560, bottom: -40 },
            ScreenRect { left: -1080, top: -2000, right: 0, bottom: -80 },
        ] {
            for dpi in [96, 120, 144] {
                let layout = overlay_rects(work, dpi, true, true).unwrap();
                let left = layout.dashboard.unwrap(); let right = layout.call.unwrap();
                assert!(left.left >= work.left && right.right <= work.right);
                assert!(left.right < right.left);
                assert!(left.top >= work.top && right.top == left.top);
                assert!(left.bottom <= work.bottom && right.bottom <= work.bottom);
            }
        }
    }
    #[test]
    fn disabled_and_single_panels_do_not_reserve_the_other_panel() {
        let invalid = ScreenRect { left: 5, top: 4, right: -8, bottom: -9 };
        assert_eq!(overlay_rects(invalid, 0, false, false).unwrap(), OverlayRects { dashboard: None, call: None });
        let narrow = ScreenRect { left: -380, top: 0, right: 0, bottom: 1000 };
        assert!(overlay_rects(narrow, 96, true, false).unwrap().call.is_none());
        assert!(overlay_rects(narrow, 96, false, true).unwrap().dashboard.is_none());
        assert_eq!(overlay_rects(narrow, 96, true, true), Err(PlacementError::InsufficientSpace));
    }
    #[test]
    fn invalid_dpi_small_workspaces_and_extreme_arithmetic_fail_closed() {
        let work = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert_eq!(overlay_rects(work, 0, true, true), Err(PlacementError::InvalidGeometry));
        assert_eq!(overlay_rects(ScreenRect { bottom: 300, ..work }, 96, true, true), Err(PlacementError::InsufficientSpace));
        assert_eq!(overlay_rects(ScreenRect { right: -1, ..work }, 96, true, true), Err(PlacementError::InvalidGeometry));
        assert!(overlay_rects(work, u32::MAX, true, true).is_err());
        let huge = ScreenRect { left: i32::MIN, top: i32::MIN, right: i32::MAX, bottom: i32::MAX };
        let layout = overlay_rects(huge, 192, true, true).unwrap();
        assert!(layout.dashboard.unwrap().width() > 0 && layout.call.unwrap().height() > 0);
    }
}
