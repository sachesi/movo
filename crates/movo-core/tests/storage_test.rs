use movo_core::client::models::MediaType;
use movo_core::storage::history::{WatchHistory, WatchHistoryEntry};
use movo_core::storage::settings::AppSettings;
use std::collections::HashSet;

#[test]
fn test_history_entry_and_progress() {
    let user_id = "storage-test-account";
    let entry = WatchHistoryEntry {
        media_id: 981,
        title: "Матрица".to_string(),
        orig_title: Some("The Matrix".to_string()),
        url: "https://hdrzk.org/films/fiction/981-matrica-1999-latest.html".to_string(),
        poster_url: None,
        media_type: MediaType::Movie,
        season: None,
        episode: None,
        episode_title: None,
        translator_id: Some(56),
        translator_name: Some("Дубляж".to_string()),
        position_secs: 1800.0,
        duration_secs: 7200.0,
        updated_at: chrono::Utc::now(),
    };

    assert_eq!(entry.progress_fraction(), 0.25);
    assert_eq!(entry.formatted_position(), "30:00 / 02:00:00");

    let mut history = WatchHistory::default();
    history.update_entry(user_id, entry.clone()).unwrap();
    assert_eq!(history.entries.len(), 1);

    let retrieved = history.get_entry(981, None, None);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Матрица");

    history.remove_entry(user_id, 981, None, None).unwrap();
    assert_eq!(history.entries.len(), 0);
    let _ = std::fs::remove_file(WatchHistory::file_path(user_id));
}

#[test]
fn test_history_paths_are_account_scoped() {
    assert_ne!(
        WatchHistory::file_path("account-a"),
        WatchHistory::file_path("account-b")
    );
}

#[test]
fn test_history_reconciles_remote_deletions_by_media() {
    let user_id = "storage-sync-test-account";
    let entry = |media_id| WatchHistoryEntry {
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
        updated_at: chrono::Utc::now() - chrono::Duration::minutes(10),
    };
    let mut history = WatchHistory {
        entries: vec![entry(1), entry(2)],
    };

    history
        .retain_media_since(user_id, &HashSet::from([2]), chrono::Utc::now())
        .unwrap();
    assert_eq!(history.entries.len(), 1);
    assert_eq!(history.entries[0].media_id, 2);
    let _ = std::fs::remove_file(WatchHistory::file_path(user_id));
}

#[test]
fn test_history_keeps_recent_entries_during_remote_propagation() {
    let user_id = "storage-propagation-test-account";
    let mut history = WatchHistory {
        entries: vec![WatchHistoryEntry {
            media_id: 3,
            title: "3".to_string(),
            orig_title: None,
            url: "https://hdrzk.org/films/3-test.html".to_string(),
            poster_url: None,
            media_type: MediaType::Movie,
            season: None,
            episode: None,
            episode_title: None,
            translator_id: None,
            translator_name: None,
            position_secs: 10.0,
            duration_secs: 100.0,
            updated_at: chrono::Utc::now(),
        }],
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
fn test_settings_defaults() {
    let settings = AppSettings::default();
    assert_eq!(settings.default_quality, "1080p");
    assert_eq!(settings.external_player, "mpv");
}

#[test]
fn test_settings_ignore_legacy_external_player_switch() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value["use_external_player"] = serde_json::Value::Bool(false);

    let settings: AppSettings = serde_json::from_value(value).unwrap();
    assert_eq!(settings.external_player, "mpv");
}

#[test]
fn test_settings_ignore_removed_options() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value["official_mode"] = serde_json::Value::Bool(false);
    value["base_url"] = serde_json::Value::String("https://mirror.example".to_string());
    value["download_dir"] = serde_json::Value::String("/tmp/old-downloads".to_string());

    let settings: AppSettings = serde_json::from_value(value).unwrap();
    let saved = serde_json::to_value(settings).unwrap();
    assert!(saved.get("official_mode").is_none());
    assert!(saved.get("base_url").is_none());
    assert!(saved.get("download_dir").is_none());
}
