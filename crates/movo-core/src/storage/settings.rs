use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
        Self::load_from(&Self::config_path())
    }

    fn load_from(path: &Path) -> Self {
        match super::load_json_checked::<AppSettings>(path) {
            Ok(Some(settings)) => settings,
            Ok(None) => {
                let defaults = Self::default();
                if let Err(error) = super::save_json(path, &defaults) {
                    log::warn!("{error}");
                }
                defaults
            }
            Err(error) => {
                // The next save replaces this file, so whatever is still
                // recoverable from it has to move out of the way first.
                let aside = path.with_extension("json.unreadable");
                log::warn!("{error}; keeping it as {aside:?}");
                if let Err(error) = fs::rename(path, &aside) {
                    log::warn!("failed to move {path:?} aside: {error}");
                }
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        super::save_json(&Self::config_path(), self)
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;
    use std::fs;

    #[test]
    fn an_unreadable_file_is_kept_rather_than_replaced() {
        let dir = std::env::temp_dir().join(format!("movo-settings-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, "{\"theme\": \"dark\",").unwrap();

        let settings = AppSettings::load_from(&path);

        assert_eq!(settings.theme, "system");
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(dir.join("settings.json.unreadable")).unwrap(),
            "{\"theme\": \"dark\","
        );
        let _ = fs::remove_dir_all(&dir);
    }

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
