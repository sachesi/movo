pub mod cache;
pub mod history;
pub mod search_history;
pub mod seen_notifications;
pub mod settings;

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

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
        .map_err(|e| format!("Failed to write to {:?}: {}", path, e))
}
