// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! Independent call-window preference; never reloads settings or restarts audio.
use std::io::{self, Read};

const FILE: &str = "call-panel-size.json";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeDip { pub width: i32, pub height: i32 }
impl SizeDip {
    pub const COMPACT: Self = Self { width: 420, height: 720 };
    pub const EXPANDED: Self = Self { width: 640, height: 900 };
    pub fn valid(self) -> bool { (300..=4096).contains(&self.width) && (480..=4096).contains(&self.height) }
    pub fn from_pixels(width: i32, height: i32, dpi: u32) -> Option<Self> {
        if width <= 0 || height <= 0 || dpi == 0 { return None; }
        let dip = |v: i32| ((i64::from(v) * 96 + i64::from(dpi) / 2) / i64::from(dpi)).clamp(1, 4096) as i32;
        Some(Self { width: dip(width).max(300), height: dip(height).max(480) })
    }
}
impl Default for SizeDip { fn default() -> Self { Self::COMPACT } }

fn parse(bytes: &[u8]) -> Option<SizeDip> {
    if bytes.len() > 1024 { return None; }
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if value.get("version")?.as_u64()? != 1 { return None; }
    let size = SizeDip { width: value.get("width_dip")?.as_i64()?.try_into().ok()?, height: value.get("height_dip")?.as_i64()?.try_into().ok()? };
    size.valid().then_some(size)
}
pub fn load() -> SizeDip {
    let result = (|| -> io::Result<Vec<u8>> {
        let file = std::fs::File::open(crate::store::data_dir().join(FILE))?;
        let mut bytes = Vec::new(); file.take(1025).read_to_end(&mut bytes)?; Ok(bytes)
    })();
    result.ok().and_then(|bytes| parse(&bytes)).unwrap_or_default()
}
pub fn save(size: SizeDip) -> io::Result<()> {
    if !size.valid() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid call panel size")); }
    crate::store::write_text(FILE, &serde_json::json!({ "version": 1, "width_dip": size.width, "height_dip": size.height }).to_string())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn saved_dimensions_require_a_supported_bounded_complete_object() {
        assert_eq!(parse(br#"{"version":1,"width_dip":640,"height_dip":900}"#), Some(SizeDip::EXPANDED));
        for bytes in [b"{}".as_slice(), br#"{"version":2,"width_dip":640,"height_dip":900}"#,
            br#"{"version":1,"width_dip":299,"height_dip":900}"#, br#"{"version":1,"width_dip":640,"height_dip":479}"#,
            br#"{"version":1,"width_dip":1e20,"height_dip":900}"#, br#"{"version":1,"width_dip":640.5,"height_dip":900}"#,
            br#"{"version":1,"width_dip":"640","height_dip":900}"#, br#"{"version":1,"width_dip":4097,"height_dip":900}"#] {
            assert!(parse(bytes).is_none());
        }
        assert!(parse(&vec![b' '; 1025]).is_none());
    }
    #[test] fn physical_size_roundtrips_at_supported_dpi_without_persisting_pixels() {
        for dpi in [96, 120, 144, 192] {
            assert_eq!(SizeDip::from_pixels(640 * dpi as i32 / 96, 900 * dpi as i32 / 96, dpi), Some(SizeDip::EXPANDED));
        }
        assert_eq!(SizeDip::from_pixels(1, 1, 192), Some(SizeDip { width: 300, height: 480 }));
        assert!(SizeDip::from_pixels(640, 900, 0).is_none());
    }
}
