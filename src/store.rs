//! Modified for NotchLite: isolated storage, with no migration of other apps' data.
//! Plain-text persistence under %APPDATA%\NotchLite (settings and lists) and
//! %LOCALAPPDATA%\NotchLite (caches and the file shelf).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn dir_from_env(var: &str) -> PathBuf {
    let base = std::env::var_os(var).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("NotchLite");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Roaming data: settings, todo list, calendar events.
pub fn data_dir() -> PathBuf {
    dir_from_env("APPDATA")
}

/// Machine-local data: caches and the shelf folder.
pub fn local_dir() -> PathBuf {
    dir_from_env("LOCALAPPDATA")
}

pub fn read_text(name: &str) -> Option<String> {
    fs::read_to_string(data_dir().join(name)).ok()
}

/// Writes through a temporary file so a crash never leaves a half-written file.
pub fn write_text(name: &str, contents: &str) -> io::Result<()> {
    let path = data_dir().join(name);
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, &path)
}

/// Reads `key=value` lines; blank lines and lines starting with `#` are ignored.
pub fn read_kv(name: &str) -> BTreeMap<String, String> {
    try_read_kv(name).unwrap_or_default()
}

/// Settings reloads must distinguish I/O errors from a valid empty file.
pub fn try_read_kv(name: &str) -> io::Result<BTreeMap<String, String>> {
    read_kv_at(&data_dir().join(name))
}

fn read_kv_at(path: &Path) -> io::Result<BTreeMap<String, String>> {
    let text = fs::read_to_string(path)?;
    Ok(text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect())
}

pub fn write_kv(name: &str, map: &BTreeMap<String, String>) -> io::Result<()> {
    let body: String = map.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    write_text(name, &body)
}

pub fn read_lines(name: &str) -> Vec<String> {
    read_text(name).unwrap_or_default().lines().map(str::to_string).collect()
}

pub fn write_lines(name: &str, lines: &[String]) -> io::Result<()> {
    let mut body = lines.join("\n");
    body.push('\n');
    write_text(name, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("notchlite-kv-{}-{stamp}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn read_failures_are_not_a_successful_empty_configuration() {
        let fixture = Fixture::new();
        let file = fixture.0.join("settings.txt");
        assert_eq!(read_kv_at(&file).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert!(read_kv_at(&fixture.0).is_err());
        fs::write(&file, [0xFF]).unwrap();
        assert_eq!(read_kv_at(&file).unwrap_err().kind(), io::ErrorKind::InvalidData);
        fs::write(&file, "").unwrap();
        assert!(read_kv_at(&file).unwrap().is_empty());
    }

    #[test]
    fn strict_reader_keeps_the_existing_key_value_format() {
        let fixture = Fixture::new();
        let file = fixture.0.join("settings.txt");
        fs::write(&file, "# comment\n\n minimal = 1\nappearance=midnight\ncustom=keep=exactly|this\ninvalid-line\n").unwrap();
        assert_eq!(read_kv_at(&file).unwrap(), BTreeMap::from([
            ("minimal".to_string(), "1".to_string()),
            ("appearance".to_string(), "midnight".to_string()),
            ("custom".to_string(), "keep=exactly|this".to_string()),
        ]));
    }
}
