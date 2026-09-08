pub mod cache;
pub mod history;
pub mod search_history;
pub mod seen_notifications;
pub mod settings;

use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The application's directories, or `None` where the platform has no home to
/// resolve them against. Callers fall back to a relative path.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("org", "gnome", "Movo")
}

/// Path to one of an account's data files, with the directory created.
///
/// Accounts are keyed by a hash of the user id rather than the id itself, so
/// that a provider id never becomes a directory name.
pub fn account_file(user_id: &str, name: &str) -> PathBuf {
    let account = format!("user-{}", hex::encode(user_id));
    match project_dirs() {
        Some(dirs) => {
            let data_dir = dirs.data_dir().join("accounts").join(account);
            let _ = create_private_dir(&data_dir);
            data_dir.join(name)
        }
        None => PathBuf::from("accounts").join(account).join(name),
    }
}

/// Creates `path` and its parents, readable only by the account that owns it.
///
/// What lands here is what the user watched and searched for, which is nobody
/// else's business on a shared machine.
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
    }
    #[cfg(not(unix))]
    fs::create_dir_all(path)
}

/// Opens `path` for writing, readable only by the account that owns it.
fn create_private_file(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

static SAVE_ID: AtomicU64 = AtomicU64::new(0);

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    load_json_checked(path).ok().flatten()
}

pub fn load_json_checked<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {:?}: {}", path, error))?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Failed to parse {:?}: {}", path, error))
}

pub fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = create_private_dir(parent);
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SAVE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    create_private_file(&temporary)
        .and_then(|mut file| file.write_all(content.as_bytes()))
        .and_then(|()| fs::rename(&temporary, path))
        .map_err(|e| {
            // The rename never happened, so the partial file is nobody's.
            let _ = fs::remove_file(&temporary);
            format!("Failed to write to {:?}: {}", path, e)
        })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_saved_file_is_readable_only_by_its_owner() {
        let dir = std::env::temp_dir().join(format!("movo-storage-{}", std::process::id()));
        let path = dir.join("watch_history.json");
        save_json(&path, &vec!["private"]).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "saved as {mode:o}");

        let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "directory created as {mode:o}");

        let _ = fs::remove_dir_all(&dir);
    }

    /// A half-written account file has to be reported rather than read as an
    /// empty one: silently defaulting would overwrite the rest of it on save.
    #[test]
    fn a_corrupt_file_is_reported_instead_of_defaulted() {
        let dir = std::env::temp_dir().join(format!("movo-corrupt-{}", std::process::id()));
        let path = dir.join("search_history.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "{").unwrap();

        assert!(load_json_checked::<Vec<String>>(&path).is_err());
        assert!(load_json::<Vec<String>>(&path).is_none());
        assert!(load_json_checked::<Vec<String>>(&dir.join("absent.json"))
            .unwrap()
            .is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
