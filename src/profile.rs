//! Modified for NotchLite: publish a complete staged first-run profile without
//! overwriting an existing or concurrently created user configuration.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

// Enable only local shelf, to-do and focus. Network/system probes stay opt-in.
// An absent key in a later user-edited file retains the upstream semantics.
pub const FIRST_RUN_SETTINGS: &str = "mascot=0\nminimal=1\ndisabled=media,calendar,weather,football,system,claude,notifications\n";
static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);

pub fn initialize_at(path: &Path) -> io::Result<()> {
    initialize_with(path, |file| file.write_all(FIRST_RUN_SETTINGS.as_bytes()))
}

/// The write callback lets normal regression tests inject a failure using
/// synthetic files, without modifying user permissions or exhausting a disk.
fn initialize_with(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    if profile_exists(path)? {
        return Ok(());
    }
    let mut staged = StagedProfile::create(path)?;
    let file = staged.file.as_mut().expect("staging file is open");
    write(file)?;
    file.sync_all()?;
    drop(staged.file.take());
    match publish_without_replacing(&staged.path, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if profile_exists(path)? { Ok(()) } else { Err(error) }
        }
        Err(error) => Err(error),
    }
}

fn profile_exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "profile path must be a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

struct StagedProfile {
    path: PathBuf,
    file: Option<File>,
}

impl StagedProfile {
    fn create(destination: &Path) -> io::Result<Self> {
        let base = destination.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "profile path needs a file name"))?;
        for _ in 0..128 {
            let id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
            let mut name = base.to_os_string();
            name.push(format!(".{}.{id}.initializing.tmp", std::process::id()));
            let path = destination.with_file_name(name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, file: Some(file) }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(io::ErrorKind::AlreadyExists, "could not reserve a unique profile staging file"))
    }
}

impl Drop for StagedProfile {
    fn drop(&mut self) {
        drop(self.file.take());
        // A crash may leave staging behind; it is never read as the final
        // settings and never reused with truncate. Cleanup is best effort.
        let _ = fs::remove_file(&self.path);
    }
}

fn publish_without_replacing(source: &Path, destination: &Path) -> io::Result<()> {
    let wide = |path: &Path| -> io::Result<Vec<u16>> {
        let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in profile path"));
        }
        value.push(0);
        Ok(value)
    };
    let source = wide(source)?;
    let destination = wide(destination)?;
    // Same-directory staging keeps this on one volume. No REPLACE_EXISTING
    // and no COPY_ALLOWED: concurrent user files win, unsupported moves fail.
    unsafe {
        MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(destination.as_ptr()), MOVEFILE_WRITE_THROUGH)
    }.map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xFFFF))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("notchlite-profile-{}-{stamp}-{id}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn first_run_creates_profile_and_preserves_subsequent_user_choices() {
        let dir = scratch_dir();
        let path = dir.join("settings.txt");
        initialize_at(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), FIRST_RUN_SETTINGS);
        let user_settings = "mascot=1\nminimal=0\ndisabled=\n";
        fs::write(&path, user_settings).unwrap();
        initialize_with(&path, |_| panic!("existing profile must not be written")).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), user_settings);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn an_interrupted_write_leaves_no_final_profile_and_next_start_can_retry() {
        let dir = scratch_dir();
        let path = dir.join("settings.txt");
        let error = initialize_with(&path, |file| {
            file.write_all(b"mascot=0\n")?;
            Err(io::Error::new(io::ErrorKind::WriteZero, "synthetic interrupted write"))
        }).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WriteZero);
        assert!(!path.exists(), "a failed write must not publish partial settings");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        initialize_at(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), FIRST_RUN_SETTINGS);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn a_profile_created_during_staging_is_never_overwritten() {
        let dir = scratch_dir();
        let path = dir.join("settings.txt");
        let user_settings = "mascot=1\nminimal=0\ndisabled=weather\n";
        initialize_with(&path, |file| {
            file.write_all(FIRST_RUN_SETTINGS.as_bytes())?;
            fs::write(&path, user_settings)
        }).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), user_settings);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn a_directory_at_the_profile_path_fails_closed() {
        let dir = scratch_dir();
        let path = dir.join("settings.txt");
        fs::create_dir(&path).unwrap();
        let error = initialize_at(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
        fs::remove_dir(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
