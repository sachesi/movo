use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static SEEN_WRITE: Mutex<()> = Mutex::new(());

/// Notification targets the account holder has already opened.
///
/// The provider has no read state of its own, so unread counts are kept
/// per account on this device.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeenNotifications {
    pub urls: BTreeSet<String>,
}

impl SeenNotifications {
    pub fn file_path(user_id: &str) -> PathBuf {
        let account = format!("user-{}", hex::encode(user_id));
        if let Some(proj_dirs) = ProjectDirs::from("org", "gnome", "Movo") {
            let data_dir = proj_dirs.data_dir().join("accounts").join(account);
            let _ = fs::create_dir_all(&data_dir);
            data_dir.join("seen_notifications.json")
        } else {
            PathBuf::from("accounts")
                .join(account)
                .join("seen_notifications.json")
        }
    }

    pub fn load(user_id: &str) -> Self {
        super::load_json(&Self::file_path(user_id)).unwrap_or_default()
    }

    pub fn contains(&self, url: &str) -> bool {
        self.urls.contains(url)
    }

    /// Mark one notification target as seen, keeping only targets that are
    /// still listed so the file cannot grow without bound.
    pub fn mark_seen_for(user_id: &str, url: &str, listed: &[String]) -> Result<Self, String> {
        let _write = SEEN_WRITE
            .lock()
            .map_err(|_| "Notification state is unavailable".to_string())?;
        let mut seen = Self::load(user_id);
        seen.urls.insert(url.to_string());
        seen.urls.retain(|url| listed.contains(url));
        super::save_json(&Self::file_path(user_id), &seen)?;
        Ok(seen)
    }
}

#[cfg(test)]
mod tests {
    use super::SeenNotifications;
    use std::collections::BTreeSet;

    #[test]
    fn seen_state_only_keeps_listed_targets() {
        let mut seen = SeenNotifications {
            urls: BTreeSet::from(["a".to_string(), "gone".to_string()]),
        };
        seen.urls.insert("b".to_string());
        seen.urls
            .retain(|url| ["a".to_string(), "b".to_string()].contains(url));

        assert!(seen.contains("a"));
        assert!(seen.contains("b"));
        assert!(!seen.contains("gone"));
    }
}
