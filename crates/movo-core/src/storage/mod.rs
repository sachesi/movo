pub mod cache;
pub mod history;
pub mod search_history;
pub mod seen_notifications;
pub mod settings;

use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
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
            let _ = fs::create_dir_all(&data_dir);
            data_dir.join(name)
        }
        None => PathBuf::from("accounts").join(account).join(name),
    }
}

static SAVE_ID: AtomicU64 = AtomicU64::new(0);

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    if path.exists() {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

pub fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SAVE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temporary, content)
        .and_then(|()| fs::rename(&temporary, path))
        .map_err(|e| {
            // The rename never happened, so the partial file is nobody's.
            let _ = fs::remove_file(&temporary);
            format!("Failed to write to {:?}: {}", path, e)
        })
}
