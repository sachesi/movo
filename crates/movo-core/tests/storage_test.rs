use movo_core::client::models::MediaType;
use movo_core::storage::history::{WatchHistory, WatchHistoryEntry};
use movo_core::storage::settings::AppSettings;

#[test]
fn test_history_entry_and_progress() {
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

    let history = WatchHistory {
        entries: vec![entry],
    };

    let retrieved = history.get_entry(981, None, None);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Матрица");
}

#[test]
fn test_history_paths_are_account_scoped() {
    assert_ne!(
        WatchHistory::file_path("account-a"),
        WatchHistory::file_path("account-b")
    );
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
