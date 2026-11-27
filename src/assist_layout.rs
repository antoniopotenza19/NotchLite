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

/// Reserve the full native notch height, not its animated/clipped capsule.
/// Bounds are already physical screen pixels; overlay_rects adds the DPI gap.
pub fn work_below_main(work: ScreenRect, main: ScreenRect) -> Result<ScreenRect, PlacementError> {
    if work.right <= work.left || work.bottom <= work.top
        || main.right <= main.left || main.bottom <= main.top {
        return Err(PlacementError::InvalidGeometry);
    }
    let intersects = main.left < work.right && main.right > work.left
        && main.top < work.bottom && main.bottom > work.top;
    if !intersects { return Ok(work); }
    if main.bottom >= work.bottom { return Err(PlacementError::InsufficientSpace); }
    Ok(ScreenRect { top: work.top.max(main.bottom), ..work })
}

#[cfg(test)]
pub fn overlay_rects(work: ScreenRect, dpi: u32, dashboard: bool, call: bool) -> Result<OverlayRects, PlacementError> {
    overlay_rects_sized(work, dpi, dashboard, call, crate::call_panel_size::SizeDip::COMPACT)
}

pub fn overlay_rects_sized(work: ScreenRect, dpi: u32, dashboard: bool, call: bool, size: crate::call_panel_size::SizeDip) -> Result<OverlayRects, PlacementError> {
    if !dashboard && !call { return Ok(OverlayRects { dashboard: None, call: None }); }
    if dpi == 0 || !size.valid() || work.right <= work.left || work.bottom <= work.top { return Err(PlacementError::InvalidGeometry); }
    let px = |dip: i64| (dip * i64::from(dpi) + 48) / 96;
    let margin = px(16).max(1);
    let gap = px(16).max(1);
    let available_w = i64::from(work.right) - i64::from(work.left) - margin * 2;
    let available_h = i64::from(work.bottom) - i64::from(work.top) - margin * 2;
    let min_dashboard_w = px(320).max(1);
    let min_call_w = px(300).max(1);
    if available_h < if dashboard { px(520) } else { px(480) } {
        return Err(PlacementError::InsufficientSpace);
    }
    let (dashboard_w, call_w) = match (dashboard, call) {
        (true, true) => {
            if available_w < min_dashboard_w + min_call_w + gap { return Err(PlacementError::InsufficientSpace); }
            let right = px(i64::from(size.width)).min(available_w - min_dashboard_w - gap);
            (px(480).min(available_w - right - gap), right)
        }
        (true, false) if available_w >= min_dashboard_w => (px(480).min(available_w), 0),
        (false, true) if available_w >= min_call_w => (0, px(i64::from(size.width)).min(available_w)),
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
        call: if call { Some(rect(i64::from(work.right) - margin - call_w, call_w, i64::from(size.height))?) } else { None },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallControlsRects {
    /// Status, scope, provider, target dropdown, context label, context input.
    pub information: [ScreenRect; 6],
    pub resize: ScreenRect,
    /// Transcript action, AI action, ignore, stop.
    pub buttons: [ScreenRect; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallReaderRects {
    pub status: ScreenRect,
    pub labels: [ScreenRect; 2],
    pub editors: [ScreenRect; 2],
    /// Resize and Stop remain available without opening configuration.
    pub buttons: [ScreenRect; 2],
}

/// Reading view: the setup form never takes space from the two live text panes.
pub fn call_reader_rects(width: i32, height: i32, dpi: u32) -> Result<CallReaderRects, PlacementError> {
    if width <= 0 || height <= 0 || dpi == 0 { return Err(PlacementError::InvalidGeometry); }
    let px = |dip: i64| ((dip * i64::from(dpi) + 48) / 96).max(1);
    let margin=px(12); let gap=px(8); let content_width=i64::from(width)-margin*2;
    let rect=|left:i64,top:i64,w:i64,h:i64|->Result<ScreenRect,PlacementError>{
        if w<=0 || h<=0 {return Err(PlacementError::InsufficientSpace);}
        Ok(ScreenRect{left:left.try_into().map_err(|_|PlacementError::InvalidGeometry)?,top:top.try_into().map_err(|_|PlacementError::InvalidGeometry)?,
            right:(left+w).try_into().map_err(|_|PlacementError::InvalidGeometry)?,bottom:(top+h).try_into().map_err(|_|PlacementError::InvalidGeometry)?})
    };
    let status=rect(margin,margin+px(28),content_width,px(48))?;
    let mut y=i64::from(status.bottom)+gap;
    let buttons_y=i64::from(height)-margin-px(28);
    let label_height=px(20);
    let editor_space=buttons_y-y-label_height*2-gap*5;
    let first=editor_space*3/5;let second=editor_space-first;
    if first<px(48) || second<px(48) || content_width<px(180) {return Err(PlacementError::InsufficientSpace);}
    let transcript_label=rect(margin,y,content_width,label_height)?;y+=label_height+gap;
    let transcript=rect(margin,y,content_width,first)?;y+=first+gap*2;
    let suggestion_label=rect(margin,y,content_width,label_height)?;y+=label_height+gap;
    let suggestion=rect(margin,y,content_width,second)?;
    Ok(CallReaderRects{status,labels:[transcript_label,suggestion_label],editors:[transcript,suggestion],
        buttons:[rect(margin,buttons_y,px(96),px(28))?,rect(i64::from(width)-margin-px(76),buttons_y,px(76),px(28))?]})
}

/// Native client geometry shared by rendering and tests; the header occupies 28 DIP.
pub fn call_controls_rects(width: i32, height: i32, dpi: u32) -> Result<CallControlsRects, PlacementError> {
    if width <= 0 || height <= 0 || dpi == 0 { return Err(PlacementError::InvalidGeometry); }
    let px = |dip: i64| ((dip * i64::from(dpi) + 48) / 96).max(1);
    let margin = px(12); let gap = px(4);
    let content_width = i64::from(width) - margin * 2;
    let button_width = (content_width - gap) / 2;
    if button_width <= 0 { return Err(PlacementError::InsufficientSpace); }
    let rect = |left: i64, top: i64, w: i64, h: i64| -> Result<ScreenRect, PlacementError> {
        if h <= 0 { return Err(PlacementError::InsufficientSpace); }
        Ok(ScreenRect { left: left.try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            top: top.try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            right: (left + w).try_into().map_err(|_| PlacementError::InvalidGeometry)?,
            bottom: (top + h).try_into().map_err(|_| PlacementError::InvalidGeometry)? })
    };
    let mut y = margin + px(28);
    let mut information = [ScreenRect { left: 0, top: 0, right: 0, bottom: 0 }; 6];
    for (i, (control_height, advance)) in [(48, 52), (40, 44), (120, 124), (28, 32), (20, 20), (24, 28)].into_iter().enumerate() {
        information[i] = rect(margin, y, content_width, px(control_height))?;
        y += px(advance);
    }
    let buttons_y = i64::from(height) - margin - px(28);
    let mode_y = buttons_y - gap - px(28);
    let resize=rect(margin,y+px(8),px(96),px(28))?;
    if i64::from(resize.bottom)+px(8)>mode_y || content_width<px(180) {return Err(PlacementError::InsufficientSpace);}
    Ok(CallControlsRects { information, resize,
        buttons: [rect(margin, mode_y, button_width, px(28))?, rect(margin + button_width + gap, mode_y, button_width, px(28))?,
            rect(margin, buttons_y, button_width, px(28))?, rect(margin + button_width + gap, buttons_y, button_width, px(28))?] })
}

/// A reader who scrolled into the history keeps their position during updates.
pub fn scroll_is_at_end(min: i32, max: i32, page: u32, position: i32) -> bool {
    i64::from(position) >= (i64::from(max) - i64::from(page.saturating_sub(1))).max(i64::from(min))
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
    #[test]
    fn portrait_secondary_panels_reserve_the_owned_notch_band_at_each_dpi() {
        let work = ScreenRect { left: -1080, top: 0, right: 0, bottom: 1920 };
        let main = ScreenRect { left: -860, top: 0, right: -220, bottom: 190 };
        let available = work_below_main(work, main).unwrap();
        for dpi in [96, 120, 144] {
            let layout = overlay_rects(available, dpi, true, true).unwrap();
            let dashboard = layout.dashboard.unwrap(); let call = layout.call.unwrap();
            let gap = (16 * i64::from(dpi) + 48) / 96;
            assert_eq!(i64::from(dashboard.top), i64::from(main.bottom) + gap);
            assert_eq!(call.top, dashboard.top);
            assert!(dashboard.right < call.left);
            assert!(dashboard.left >= work.left && call.right <= work.right);
            assert!(dashboard.bottom <= work.bottom && call.bottom <= work.bottom);
        }
        // A hidden/clipped capsule has the same native rect and keeps the band.
        assert_eq!(work_below_main(work, main).unwrap(), available);
    }
    #[test]
    fn another_monitor_or_an_edge_touch_does_not_consume_the_top_band() {
        let work = ScreenRect { left: -1080, top: -1920, right: 0, bottom: 0 };
        for main in [
            ScreenRect { left: 0, top: -1920, right: 640, bottom: -1730 },
            ScreenRect { left: -860, top: 0, right: -220, bottom: 190 },
        ] {
            assert_eq!(work_below_main(work, main), Ok(work));
        }
        let main = ScreenRect { left: -860, top: -1920, right: -220, bottom: -1730 };
        let available = work_below_main(work, main).unwrap();
        let call = overlay_rects(available, 144, false, true).unwrap().call.unwrap();
        assert_eq!(call.top, -1730 + 24);
        assert!(call.bottom <= work.bottom);
    }
    #[test]
    fn reserving_the_notch_band_rejects_insufficient_or_invalid_geometry() {
        let work = ScreenRect { left: -1080, top: 0, right: 0, bottom: 600 };
        let main = ScreenRect { left: -860, top: 0, right: -220, bottom: 190 };
        let available = work_below_main(work, main).unwrap();
        assert_eq!(overlay_rects(available, 96, false, true), Err(PlacementError::InsufficientSpace));
        assert_eq!(work_below_main(work, ScreenRect { bottom: 600, ..main }), Err(PlacementError::InsufficientSpace));
        assert_eq!(work_below_main(work, ScreenRect { bottom: 0, ..main }), Err(PlacementError::InvalidGeometry));
        let extreme = ScreenRect { left: i32::MIN, top: i32::MIN, right: i32::MAX, bottom: i32::MAX };
        assert!(work_below_main(extreme, ScreenRect { bottom: i32::MIN + 190, ..extreme }).is_ok());
    }
    #[test]
    fn call_controls_keep_diagnostics_context_and_consent_separate_at_minimum_and_portrait_sizes() {
        for dpi in [96, 120, 144, 192] {
            let px = |dip: i64| ((dip * i64::from(dpi) + 48) / 96) as i32;
            for height in [px(480), px(480) - 2, px(720), px(720) - 2] {
                // The custom call frame has a full client; retain a conservative 2px regression case.
                let width = px(300) - 2;
                let body = call_controls_rects(width, height, dpi).unwrap();
                let mut bottom = px(12) + px(28);
                for rect in body.information {
                    assert!(rect.top >= bottom);
                    assert!(rect.left >= px(12) && rect.right <= width - px(12));
                    bottom = rect.bottom;
                }
                assert!(body.information[2].height()>=px(120));
                assert!(bottom<body.resize.top);
                assert!(body.resize.bottom<body.buttons[0].top);
                assert!(body.buttons[0].right < body.buttons[1].left);
                assert!(body.buttons[0].bottom < body.buttons[2].top);
                assert!(body.buttons[2].right < body.buttons[3].left);
                assert!(body.buttons[3].bottom <= height - px(12));
            }
        }
        assert_eq!(call_controls_rects(300, 350, 96), Err(PlacementError::InsufficientSpace));
        assert_eq!(call_controls_rects(300, 480, 0), Err(PlacementError::InvalidGeometry));
    }
    #[test]
    fn live_text_follows_only_a_view_already_at_the_end() {
        assert!(scroll_is_at_end(0, 0, 0, 0));
        assert!(scroll_is_at_end(0, 20, 30, 0));
        assert!(scroll_is_at_end(0, 100, 20, 81));
        assert!(!scroll_is_at_end(0, 100, 20, 80));
        assert!(!scroll_is_at_end(0, 100, 0, 99));
        assert!(scroll_is_at_end(i32::MIN, i32::MAX, u32::MAX, i32::MIN + 1));
        assert!(!scroll_is_at_end(i32::MIN, i32::MAX, u32::MAX, i32::MIN));
    }
    #[test]
    fn call_reader_gives_live_text_the_space_instead_of_the_hidden_setup_form() {
        for dpi in [96,120,144,192] {
            let px=|dip:i64|((dip*i64::from(dpi)+48)/96) as i32;
            for height in [px(480)-2,px(720),px(900)] {
                let width=px(300)-2;
                let reader=call_reader_rects(width,height,dpi).unwrap();
                let setup=call_controls_rects(width,height,dpi).unwrap();
                assert!(reader.editors[0].top<setup.information[2].bottom);
                assert!(reader.editors[0].height()+reader.editors[1].height()>=px(240));
                assert!(reader.status.bottom<reader.labels[0].top);
                for i in 0..2 {
                    assert!(reader.labels[i].bottom<reader.editors[i].top);
                    assert!(reader.editors[i].left>=px(12)&&reader.editors[i].right<=width-px(12));
                }
                assert!(reader.editors[0].bottom<reader.labels[1].top);
                assert!(reader.editors[1].bottom<reader.buttons[0].top);
                assert!(reader.buttons[0].right<reader.buttons[1].left);
                assert!(reader.buttons[1].bottom<=height-px(12));
            }
        }
        assert!(call_reader_rects(180,480,96).is_err());
        assert!(call_reader_rects(300,240,96).is_err());
        assert!(call_reader_rects(300,480,0).is_err());
    }
    #[test]
    fn enlarged_call_reflows_dashboard_and_keeps_top_right_anchor_at_each_dpi() {
        use crate::call_panel_size::SizeDip;
        let work = ScreenRect { left: -1080, top: 190, right: 0, bottom: 1920 };
        for dpi in [96, 120, 144] {
            let px = |dip: i64| ((dip * i64::from(dpi) + 48) / 96) as i32;
            let compact = overlay_rects_sized(work, dpi, true, true, SizeDip::COMPACT).unwrap();
            let wide = overlay_rects_sized(work, dpi, true, true, SizeDip::EXPANDED).unwrap();
            let dashboard = wide.dashboard.unwrap(); let call = wide.call.unwrap();
            assert_eq!(call.top, work.top + px(16));
            assert_eq!(call.right, work.right - px(16));
            assert_eq!(call.right, compact.call.unwrap().right);
            assert!(call.width() >= compact.call.unwrap().width());
            assert!(dashboard.width() >= px(320));
            assert!(dashboard.right + px(16) <= call.left);
            assert!(call.bottom <= work.bottom - px(16));
            // The same preference reapplied by the controller cannot snap back.
            assert_eq!(overlay_rects_sized(work, dpi, true, true, SizeDip::EXPANDED).unwrap(), wide);
            let maximum = overlay_rects_sized(work, dpi, true, true, SizeDip { width: 4096, height: 4096 }).unwrap();
            assert_eq!(maximum.call.unwrap().bottom, work.bottom - px(16));
            assert!(maximum.dashboard.unwrap().width() >= px(320));
        }
        let alone = overlay_rects_sized(work, 96, false, true, SizeDip::EXPANDED).unwrap().call.unwrap();
        assert_eq!(alone.width(), 640);
        let both = overlay_rects_sized(work, 96, true, true, SizeDip::EXPANDED).unwrap();
        assert_eq!(both.call.unwrap().width(), 640);
        assert_eq!(both.dashboard.unwrap().width(), 392);
        assert_eq!(overlay_rects_sized(work, 96, false, true, SizeDip { width: 299, height: 900 }), Err(PlacementError::InvalidGeometry));
    }
}
