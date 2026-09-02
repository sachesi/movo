use crate::client::models::MediaType;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// ponytail: process-wide lock; add OS file locking if multiple app instances must share progress.
static HISTORY_WRITE: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchHistoryEntry {
    pub media_id: i64,
    pub title: String,
    pub orig_title: Option<String>,
    pub url: String,
    pub poster_url: Option<String>,
    pub media_type: MediaType,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub episode_title: Option<String>,
    pub translator_id: Option<i64>,
    pub translator_name: Option<String>,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub updated_at: DateTime<Utc>,
}

impl WatchHistoryEntry {
    pub fn progress_fraction(&self) -> f64 {
        if self.duration_secs > 0.0 {
            (self.position_secs / self.duration_secs).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn formatted_position(&self) -> String {
        let cur = Self::format_time(self.position_secs);
        if self.duration_secs > 0.0 {
            let total = Self::format_time(self.duration_secs);
            format!("{} / {}", cur, total)
        } else {
            cur
        }
    }

    fn format_time(secs: f64) -> String {
        let total_secs = secs as u64;
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatchHistory {
    pub entries: Vec<WatchHistoryEntry>,
}

impl WatchHistory {
    pub fn file_path(user_id: &str) -> PathBuf {
        let account = format!("user-{}", hex::encode(user_id));
        if let Some(proj_dirs) = ProjectDirs::from("org", "gnome", "Movo") {
            let data_dir = proj_dirs.data_dir().join("accounts").join(account);
            let _ = fs::create_dir_all(&data_dir);
            data_dir.join("watch_history.json")
        } else {
            PathBuf::from("accounts")
                .join(account)
                .join("watch_history.json")
        }
    }

    pub fn load(user_id: &str) -> Self {
        let path = Self::file_path(user_id);
        super::load_json::<WatchHistory>(&path).unwrap_or_default()
    }

    pub fn save(&self, user_id: &str) -> Result<(), String> {
        super::save_json(&Self::file_path(user_id), self)
    }

    pub fn update_entry(&mut self, user_id: &str, entry: WatchHistoryEntry) -> Result<(), String> {
        // Remove existing entry for same media and season/episode if exists
        self.entries.retain(|e| {
            !(e.media_id == entry.media_id
                && e.season == entry.season
                && e.episode == entry.episode)
        });
        // Insert at beginning (most recently watched first)
        self.entries.insert(0, entry);
        // Keep last 100 entries
        if self.entries.len() > 100 {
            self.entries.truncate(100);
        }
        self.save(user_id)
    }

    pub fn update_entry_for(user_id: &str, entry: WatchHistoryEntry) -> Result<(), String> {
        let _write = HISTORY_WRITE
            .lock()
            .map_err(|_| "Watch history is unavailable".to_string())?;
        Self::load(user_id).update_entry(user_id, entry)
    }

    pub fn remove_media_for(user_id: &str, media_id: i64) -> Result<(), String> {
        let _write = HISTORY_WRITE
            .lock()
            .map_err(|_| "Watch history is unavailable".to_string())?;
        Self::load(user_id).remove_media(user_id, media_id)
    }

    pub fn reconcile_media(
        user_id: &str,
        media_ids: &std::collections::HashSet<i64>,
        protect_since: DateTime<Utc>,
    ) -> Result<Self, String> {
        let _write = HISTORY_WRITE
            .lock()
            .map_err(|_| "Watch history is unavailable".to_string())?;
        let mut history = Self::load(user_id);
        history.retain_media_since(user_id, media_ids, protect_since)?;
        Ok(history)
    }

    pub fn get_entry(
        &self,
        media_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Option<&WatchHistoryEntry> {
        self.entries
            .iter()
            .find(|e| e.media_id == media_id && e.season == season && e.episode == episode)
    }

    pub fn remove_entry(
        &mut self,
        user_id: &str,
        media_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), String> {
        self.entries
            .retain(|e| !(e.media_id == media_id && e.season == season && e.episode == episode));
        self.save(user_id)
    }

    pub fn remove_media(&mut self, user_id: &str, media_id: i64) -> Result<(), String> {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| entry.media_id != media_id);
        if self.entries.len() == previous_len {
            Ok(())
        } else {
            self.save(user_id)
        }
    }

    pub fn retain_media_since(
        &mut self,
        user_id: &str,
        media_ids: &std::collections::HashSet<i64>,
        protect_since: DateTime<Utc>,
    ) -> Result<(), String> {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| {
            media_ids.contains(&entry.media_id) || entry.updated_at >= protect_since
        });
        if self.entries.len() == previous_len {
            Ok(())
        } else {
            self.save(user_id)
        }
    }

    pub fn clear(&mut self, user_id: &str) -> Result<(), String> {
        self.entries.clear();
        self.save(user_id)
    }
}
