use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Bumped whenever a stored value needs rewriting on load.
const SETTINGS_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Schema version of the file this was read from; 0 means pre-versioning.
    #[serde(default)]
    pub version: u32,
    pub default_quality: String,
    pub external_player: String,
    pub user_id: Option<String>,
    #[serde(default)]
    pub sort_voices: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_initial_view")]
    pub initial_view: String,
    /// Continue with the next episode automatically when playback ends.
    #[serde(default)]
    pub auto_next_episode: bool,
    /// Show the quality picker before every playback instead of using the saved choice.
    #[serde(default)]
    pub ask_quality_before_play: bool,
    /// Lower-cased country names whose titles are left out of listings.
    #[serde(default)]
    pub hidden_countries: Vec<String>,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_initial_view() -> String {
    "home".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            default_quality: "1080p".to_string(),
            external_player: "mpv".to_string(),
            user_id: None,
            sort_voices: false,
            theme: default_theme(),
            initial_view: default_initial_view(),
            auto_next_episode: false,
            ask_quality_before_play: false,
            hidden_countries: Vec::new(),
        }
    }
}

impl AppSettings {
    fn config_path() -> PathBuf {
        if let Some(proj_dirs) = super::project_dirs() {
            let config_dir = proj_dirs.config_dir();
            let _ = fs::create_dir_all(config_dir);
            config_dir.join("settings.json")
        } else {
            PathBuf::from("settings.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Some(mut settings) = super::load_json::<AppSettings>(&path) {
            if settings.migrate() {
                let _ = settings.save();
            }
            return settings;
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
    }

    /// Rewrite values carried over from an older schema. Returns whether
    /// anything changed, so the caller can persist the result.
    fn migrate(&mut self) -> bool {
        if self.version >= SETTINGS_VERSION {
            return false;
        }
        if self.version < 1 {
            // Before version 1 the player was a launcher command such as
            // `xdg-open`, which cannot stream or report progress.
            self.external_player = Self::default().external_player;
        }
        if self.version < 2 && self.initial_view == "catalog" {
            // Version 1 wrote its "catalog" default to disk on first run, so
            // the new Home default would never reach an existing install.
            self.initial_view = default_initial_view();
        }
        self.version = SETTINGS_VERSION;
        true
    }

    pub fn save(&self) -> Result<(), String> {
        super::save_json(&Self::config_path(), self)
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn old_settings_keep_new_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"default_quality":"1080p","external_player":"mpv","preferred_translators":[],"user_id":null}"#,
        )
        .unwrap();

        assert!(!settings.sort_voices);
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.initial_view, "home");
        assert_eq!(settings.external_player, "mpv");
    }

    #[test]
    fn a_pre_version_launcher_command_is_replaced() {
        let mut settings: AppSettings = serde_json::from_str(
            r#"{"default_quality":"1080p","external_player":"xdg-open","user_id":null}"#,
        )
        .unwrap();

        assert!(settings.migrate());
        assert_eq!(settings.external_player, "mpv");
        assert!(!settings.migrate());
    }

    #[test]
    fn a_version_one_catalog_start_moves_to_home_once() {
        let mut settings: AppSettings = serde_json::from_str(
            r#"{"version":1,"default_quality":"1080p","external_player":"mpv","user_id":null,"initial_view":"catalog"}"#,
        )
        .unwrap();

        assert!(settings.migrate());
        assert_eq!(settings.initial_view, "home");
        assert_eq!(settings.version, 2);
        assert!(!settings.migrate());

        let mut chosen: AppSettings = serde_json::from_str(
            r#"{"version":1,"default_quality":"1080p","external_player":"mpv","user_id":null,"initial_view":"search"}"#,
        )
        .unwrap();
        assert!(chosen.migrate());
        assert_eq!(chosen.initial_view, "search");
    }
}
