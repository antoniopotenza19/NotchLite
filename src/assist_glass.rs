// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Optional documented DWM frame material. No workers, timers or alpha fading.

use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, ERROR_SUCCESS};
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::{RegGetValueW,HKEY_CURRENT_USER,RRF_RT_REG_DWORD};
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW,SPI_GETHIGHCONTRAST,SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};

#[repr(C)]
struct HighContrast {size:u32,flags:u32,scheme:*mut u16}
fn contrast_setting()->Option<bool> {
    let mut value=HighContrast{size:std::mem::size_of::<HighContrast>() as u32,flags:0,scheme:std::ptr::null_mut()};
    unsafe{SystemParametersInfoW(SPI_GETHIGHCONTRAST,value.size,Some((&mut value as *mut HighContrast).cast()),SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))}.ok()?;
    Some(value.flags&1!=0)
}
pub fn high_contrast()->bool {contrast_setting().unwrap_or(false)}

fn transparency_enabled()->bool {
    let mut value=0u32;let mut size=std::mem::size_of::<u32>() as u32;
    (unsafe{RegGetValueW(HKEY_CURRENT_USER,w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),w!("EnableTransparency"),RRF_RT_REG_DWORD,None,Some((&mut value as *mut u32).cast()),Some(&mut size))})==ERROR_SUCCESS && value==1
}
#[link(name="ntdll")]
extern "system" {fn RtlGetVersion(version:*mut OSVERSIONINFOW)->i32;}

fn supported(major:u32,build:u32,composing:bool,transparency:bool,contrast:Option<bool>,saver:Option<bool>)->bool {
    (major>10 || major==10&&build>=22621) && composing && transparency && contrast==Some(false) && saver==Some(false)
}
fn attributes<T>(hwnd:HWND,attribute:DWMWINDOWATTRIBUTE,value:&T)->bool {
    unsafe{DwmSetWindowAttribute(hwnd,attribute,(value as *const T).cast(),std::mem::size_of::<T>() as u32)}.is_ok()
}

/// True means the documented DWM requests were accepted, not visual proof.
/// The caller exposes only this frame and keeps native text on an opaque body.
pub fn apply(hwnd:HWND,dpi:u32,graphite:bool)->bool {
    if hwnd.is_invalid(){return false;}
    let mut version=OSVERSIONINFOW{dwOSVersionInfoSize:std::mem::size_of::<OSVERSIONINFOW>() as u32,..Default::default()};
    let version_ok=unsafe{RtlGetVersion(&mut version)}>=0;
    let composing=unsafe{DwmIsCompositionEnabled()}.is_ok_and(|value|value.as_bool());
    let mut power=SYSTEM_POWER_STATUS::default();
    let saver=unsafe{GetSystemPowerStatus(&mut power)}.ok().map(|_|power.SystemStatusFlag!=0);
    let enabled=graphite&&version_ok&&supported(version.dwMajorVersion,version.dwBuildNumber,composing,transparency_enabled(),contrast_setting(),saver);
    let mut accepted=false;
    if enabled {
        let _=attributes(hwnd,DWMWA_USE_IMMERSIVE_DARK_MODE,&BOOL(1));
        let _=attributes(hwnd,DWMWA_WINDOW_CORNER_PREFERENCE,&DWMWCP_ROUND);
        let px=|dip:u32|((u64::from(dip)*u64::from(dpi.max(1))+48)/96).max(1) as i32;
        let margins=MARGINS{cxLeftWidth:px(8),cxRightWidth:px(8),cyTopHeight:px(40),cyBottomHeight:px(8)};
        accepted=attributes(hwnd,DWMWA_SYSTEMBACKDROP_TYPE,&DWMSBT_TRANSIENTWINDOW)
            && unsafe{DwmExtendFrameIntoClientArea(hwnd,&margins)}.is_ok();
    }
    if !accepted {
        let _=attributes(hwnd,DWMWA_SYSTEMBACKDROP_TYPE,&DWMSBT_NONE);
        unsafe{let _=DwmExtendFrameIntoClientArea(hwnd,&MARGINS::default());}
    }
    accepted
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn glass_requires_supported_os_and_every_accessibility_power_gate() {
        assert!(supported(10,22621,true,true,Some(false),Some(false)));
        assert!(!supported(10,22000,true,true,Some(false),Some(false)));
        assert!(!supported(10,26300,false,true,Some(false),Some(false)));
        assert!(!supported(10,26300,true,false,Some(false),Some(false)));
        assert!(!supported(10,26300,true,true,Some(true),Some(false)));
        assert!(!supported(10,26300,true,true,Some(false),Some(true)));
        assert!(!supported(10,26300,true,true,None,Some(false)));
        assert!(!supported(10,26300,true,true,Some(false),None));
    }
}
