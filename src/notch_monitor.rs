// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Main-notch monitor preference. Topology is read only on explicit change/startup.
use std::collections::BTreeMap;
use crate::assist_layout::{Monitor, ScreenRect};
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_FAIL;

pub const KEY: &str = "notch_monitor";

fn ordered(monitors: &[Monitor]) -> Vec<&Monitor> {
    let mut result: Vec<_> = monitors.iter().collect();
    result.sort_by(|a,b| b.primary.cmp(&a.primary).then_with(||a.device.cmp(&b.device)));
    result.dedup_by(|a,b| a.device==b.device);
    result
}
pub fn next(mut map: BTreeMap<String,String>, monitors: &[Monitor]) -> Option<BTreeMap<String,String>> {
    let choices=ordered(monitors);
    let selected=select(monitors,map.get(KEY).map(String::as_str))?;
    let index=choices.iter().position(|m|m.device==selected.device)?;
    map.insert(KEY.into(),choices[(index+1)%choices.len()].device.clone());
    Some(map)
}
fn select<'a>(monitors: &'a [Monitor], requested: Option<&str>) -> Option<&'a Monitor> {
    requested.and_then(|device|monitors.iter().find(|m|m.device==device))
        .or_else(||monitors.iter().find(|m|m.primary))
}
fn place(work: ScreenRect,width: i32,height: i32) -> Option<(i32,i32)> {
    let available_w=i64::from(work.right)-i64::from(work.left);
    let available_h=i64::from(work.bottom)-i64::from(work.top);
    if width<=0 || height<=0 || available_w<i64::from(width) || available_h<i64::from(height) {return None;}
    let x=i64::from(work.left)+(available_w-i64::from(width))/2;
    Some((x.try_into().ok()?,work.top))
}
pub fn position(requested: Option<&str>,width: i32,height: i32) -> Result<(i32,i32)> {
    let monitors=crate::assist_overlay::enumerate_monitors()?;
    target_position(&monitors,requested,width,height)
        .ok_or_else(||Error::new(E_FAIL,"The selected display has no room for the notch"))
}
fn target_position(monitors: &[Monitor],requested: Option<&str>,width: i32,height: i32) -> Option<(i32,i32)> {
    select(monitors,requested).and_then(|m|place(m.work,width,height))
        .or_else(||monitors.iter().find(|m|m.primary).and_then(|m|place(m.work,width,height)))
}
pub fn cycle(map: BTreeMap<String,String>) -> Result<BTreeMap<String,String>> {
    next(map,&crate::assist_overlay::enumerate_monitors()?)
        .ok_or_else(||Error::new(E_FAIL,"No display available"))
}
pub fn label(map: &BTreeMap<String,String>) -> String {
    map.get(KEY).filter(|v|!v.is_empty()).map(|v|v.trim_start_matches("\\\\.\\").to_string())
        .unwrap_or_else(||"Primary".into())
}

#[cfg(test)] mod tests {
    use super::*;
    fn monitors()->Vec<Monitor> {vec![
        Monitor{device:"second".into(),primary:false,work:ScreenRect{left:-1080,top:0,right:0,bottom:1872}},
        Monitor{device:"first".into(),primary:true,work:ScreenRect{left:0,top:40,right:1920,bottom:1080}}
    ]}
    #[test] fn main_monitor_selection_restores_saved_device_and_falls_back_without_changing_preference() {
        let displays=monitors();
        assert_eq!(select(&displays,None).unwrap().device,"first");
        assert_eq!(select(&displays,Some("second")).unwrap().device,"second");
        assert_eq!(select(&displays[..1],Some("second")).unwrap().device,"second");
        assert_eq!(select(&displays[1..],Some("second")).unwrap().device,"first");
        let map=BTreeMap::from([("external_edit".into(),"keep".into())]);
        let second=next(map.clone(),&displays).unwrap();
        assert_eq!(second[KEY],"second");assert_eq!(second["external_edit"],"keep");
        assert_eq!(next(second,&displays).unwrap()[KEY],"first");
        assert!(next(map,&[]).is_none());
    }
    #[test] fn placement_fits_portrait_negative_origins_and_top_taskbars() {
        let displays=monitors();
        assert_eq!(place(displays[0].work,640,190),Some((-860,0)));
        assert_eq!(place(displays[1].work,640,190),Some((640,40)));
        assert!(place(displays[0].work,1280,190).is_none());
        assert!(place(displays[0].work,640,2000).is_none());
        assert!(place(displays[0].work,0,190).is_none());
    }
    #[test] fn a_saved_monitor_too_small_does_not_prevent_restart_on_primary() {
        let mut displays=monitors();displays[0].work.right=-800;
        assert_eq!(target_position(&displays,Some("second"),640,190),Some((640,40)));
        assert!(target_position(&displays,Some("second"),3000,190).is_none());
    }
}
