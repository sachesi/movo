use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
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
        if let Some(settings) = super::load_json::<AppSettings>(&path) {
            return settings;
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
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
}
