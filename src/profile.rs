//! NotchLite first-run profile. Existing settings always take precedence.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

// Enable the local shelf, to-do list and focus timer. Network and system probes
// remain opt-in in Settings. An absent key later retains upstream semantics.
pub const FIRST_RUN_SETTINGS: &str = "mascot=0\nminimal=1\ndisabled=media,calendar,weather,football,system,claude,notifications\n";

pub fn initialize_at(path: &Path) -> io::Result<()> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => file.write_all(FIRST_RUN_SETTINGS.as_bytes()),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn first_run_creates_profile_and_preserves_subsequent_user_choices() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("notchlite-{}-{stamp}.txt", std::process::id()));
        initialize_at(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), FIRST_RUN_SETTINGS);
        let user_settings = "mascot=1\nminimal=0\ndisabled=\n";
        fs::write(&path, user_settings).unwrap();
        initialize_at(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), user_settings);
        fs::remove_file(path).unwrap();
    }
}
