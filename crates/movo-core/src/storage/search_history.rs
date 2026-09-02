use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static SEARCH_HISTORY_WRITE: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchHistory {
    pub values: Vec<String>,
}

impl SearchHistory {
    pub fn file_path(user_id: &str) -> PathBuf {
        let account = format!("user-{}", hex::encode(user_id));
        if let Some(proj_dirs) = ProjectDirs::from("org", "gnome", "Movo") {
            let data_dir = proj_dirs.data_dir().join("accounts").join(account);
            let _ = fs::create_dir_all(&data_dir);
            data_dir.join("search_history.json")
        } else {
            PathBuf::from("accounts")
                .join(account)
                .join("search_history.json")
        }
    }

    pub fn load(user_id: &str) -> Self {
        super::load_json(&Self::file_path(user_id)).unwrap_or_default()
    }

    pub fn add_for(user_id: &str, query: &str) -> Result<Vec<String>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Self::load(user_id).values);
        }
        let _write = SEARCH_HISTORY_WRITE
            .lock()
            .map_err(|_| "Search history is unavailable".to_string())?;
        let mut history = Self::load(user_id);
        history.values = with_query(history.values, query);
        super::save_json(&Self::file_path(user_id), &history)?;
        Ok(history.values)
    }

    pub fn clear_for(user_id: &str) -> Result<(), String> {
        let _write = SEARCH_HISTORY_WRITE
            .lock()
            .map_err(|_| "Search history is unavailable".to_string())?;
        super::save_json(&Self::file_path(user_id), &Self::default())
    }
}

fn with_query(mut values: Vec<String>, query: &str) -> Vec<String> {
    values.retain(|value| !value.eq_ignore_ascii_case(query));
    values.insert(0, query.to_string());
    values.truncate(20);
    values
}

#[cfg(test)]
mod tests {
    use super::with_query;

    #[test]
    fn recent_queries_deduplicate_and_cap() {
        let values = with_query(
            (0..20).map(|value| format!("Query {value}")).collect(),
            "query 4",
        );

        assert_eq!(values.first().unwrap(), "query 4");
        assert_eq!(values.len(), 20);
        assert_eq!(
            values
                .iter()
                .filter(|value| value.eq_ignore_ascii_case("query 4"))
                .count(),
            1
        );
    }
}
