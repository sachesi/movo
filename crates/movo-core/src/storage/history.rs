use crate::client::models::MediaType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

/// How close to the end a stopped playback counts as finished. A player that
/// closes on the credits should start the title over next time, not resume
/// into the last few seconds and end again at once.
const FINISHED_MARGIN_SECS: f64 = 10.0;

impl WatchHistoryEntry {
    pub fn progress_fraction(&self) -> f64 {
        if self.duration_secs > 0.0 {
            (self.position_secs / self.duration_secs).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn is_finished(&self) -> bool {
        self.duration_secs > 0.0 && self.position_secs >= self.duration_secs - FINISHED_MARGIN_SECS
    }

    /// Where playback should start when this entry is played again.
    pub fn resume_secs(&self) -> f64 {
        if self.is_finished() {
            0.0
        } else {
            self.position_secs.max(0.0)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatchHistory {
    pub entries: Vec<WatchHistoryEntry>,
}

impl WatchHistory {
    pub fn file_path(user_id: &str) -> PathBuf {
        super::account_file(user_id, "watch_history.json")
    }

    pub fn load(user_id: &str) -> Self {
        let path = Self::file_path(user_id);
        super::load_json::<WatchHistory>(&path).unwrap_or_default()
    }

    fn save(&self, user_id: &str) -> Result<(), String> {
        super::save_json(&Self::file_path(user_id), self)
    }

    fn update_entry(&mut self, user_id: &str, entry: WatchHistoryEntry) -> Result<(), String> {
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

    /// The most recently played entry of a title, whichever episode it was.
    pub fn latest_for(&self, media_id: i64) -> Option<&WatchHistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.media_id == media_id)
            .max_by_key(|entry| entry.updated_at)
    }

    fn remove_media(&mut self, user_id: &str, media_id: i64) -> Result<(), String> {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| entry.media_id != media_id);
        if self.entries.len() == previous_len {
            Ok(())
        } else {
            self.save(user_id)
        }
    }

    fn retain_media_since(
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
}

#[cfg(test)]
mod tests {
    use super::{WatchHistory, WatchHistoryEntry};
    use crate::client::models::MediaType;
    use std::collections::HashSet;

    fn entry(media_id: i64, age_minutes: i64) -> WatchHistoryEntry {
        WatchHistoryEntry {
            media_id,
            title: media_id.to_string(),
            orig_title: None,
            url: format!("https://hdrzk.org/films/{media_id}-test.html"),
            poster_url: None,
            media_type: MediaType::Movie,
            season: None,
            episode: None,
            episode_title: None,
            translator_id: None,
            translator_name: None,
            position_secs: 10.0,
            duration_secs: 100.0,
            updated_at: chrono::Utc::now() - chrono::Duration::minutes(age_minutes),
        }
    }

    #[test]
    fn reconciling_drops_entries_the_account_no_longer_lists() {
        let user_id = "storage-sync-test-account";
        let mut history = WatchHistory {
            entries: vec![entry(1, 10), entry(2, 10)],
        };

        history
            .retain_media_since(user_id, &HashSet::from([2]), chrono::Utc::now())
            .unwrap();

        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].media_id, 2);
        let _ = std::fs::remove_file(WatchHistory::file_path(user_id));
    }

    /// A watch the provider has not published yet is not a deletion.
    #[test]
    fn reconciling_keeps_entries_written_since_the_cutoff() {
        let user_id = "storage-propagation-test-account";
        let mut history = WatchHistory {
            entries: vec![entry(3, 0)],
        };

        history
            .retain_media_since(
                user_id,
                &HashSet::new(),
                chrono::Utc::now() - chrono::Duration::minutes(5),
            )
            .unwrap();

        assert_eq!(history.entries.len(), 1);
        let _ = std::fs::remove_file(WatchHistory::file_path(user_id));
    }

    #[test]
    fn a_finished_title_starts_over_and_the_latest_episode_wins() {
        let mut finished = entry(5, 0);
        finished.position_secs = 95.0;
        assert!(finished.is_finished());
        assert_eq!(finished.resume_secs(), 0.0);
        let midway = entry(5, 1);
        assert!(!midway.is_finished());
        assert_eq!(midway.resume_secs(), 10.0);

        let mut older = entry(5, 30);
        older.episode = Some(1);
        let history = WatchHistory {
            entries: vec![older, finished, midway],
        };
        assert_eq!(history.latest_for(5).unwrap().position_secs, 95.0);
        assert!(history.latest_for(6).is_none());
    }

    #[test]
    fn a_rewatched_episode_replaces_its_earlier_entry() {
        let user_id = "storage-update-test-account";
        let mut history = WatchHistory::default();

        history.update_entry(user_id, entry(981, 0)).unwrap();
        history.update_entry(user_id, entry(981, 0)).unwrap();

        assert_eq!(history.entries.len(), 1);
        assert!(history.get_entry(981, None, None).is_some());
        let _ = std::fs::remove_file(WatchHistory::file_path(user_id));
    }
}
