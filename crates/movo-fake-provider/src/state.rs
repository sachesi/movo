//! What the provider remembers: accounts, sessions and each account's writes.

use std::collections::{HashMap, HashSet};

pub const SEASONS: i64 = 2;
pub const EPISODES_PER_SEASON: i64 = 6;
pub const FAVORITE_CATEGORIES: [(i64, &str); 3] = [(1, "Смотрю"), (2, "На потом"), (3, "Любимое")];

pub const USER_ID: &str = "42";
pub const LOGIN: &str = "tester";
pub const PASSWORD: &str = "secret";
const SESSION_TOKEN: &str = "fake-session-token";

#[derive(Debug, Clone)]
pub struct Account {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub premium_days: u32,
}

#[derive(Debug, Clone)]
pub struct HistoryRow {
    pub id: String,
    pub post_id: i64,
    pub position: Option<(i64, i64)>,
    pub watched: bool,
    pub date: String,
}

#[derive(Debug)]
pub struct State {
    pub account: Account,
    pub favorites: HashMap<i64, Vec<i64>>,
    pub history: Vec<HistoryRow>,
    pub watched_schedule: HashSet<String>,
    pub rated: HashMap<i64, u8>,
    pub liked_comments: HashSet<String>,
    pub sessions_valid: bool,
    pub tracked_series: Vec<i64>,
}

impl State {
    pub fn seeded() -> Self {
        let mut state = Self {
            account: Account {
                user_id: USER_ID.to_string(),
                username: "Тестер".to_string(),
                email: "tester@example.invalid".to_string(),
                premium_days: 30,
            },
            favorites: HashMap::from([(1, vec![1, 2, 3]), (2, vec![4]), (3, Vec::new())]),
            history: Vec::new(),
            watched_schedule: HashSet::new(),
            rated: HashMap::new(),
            liked_comments: HashSet::new(),
            sessions_valid: true,
            tracked_series: vec![1, 5, 9],
        };
        state.save_watch(9, Some((2, 1)));
        state.save_watch(4, None);
        state.save_watch(5, Some((1, 3)));
        state
    }

    pub fn accepts_credentials(login: &str, password: &str) -> bool {
        login == LOGIN && password == PASSWORD
    }

    pub fn session_cookies(&self) -> [String; 2] {
        [
            format!("dle_user_id={}; Path=/; HttpOnly", self.account.user_id),
            format!("dle_password={SESSION_TOKEN}; Path=/; HttpOnly"),
        ]
    }

    pub fn account_for(&self, cookies: &HashMap<String, String>) -> Option<&Account> {
        let signed_in = self.sessions_valid
            && cookies.get("dle_user_id").map(String::as_str) == Some(USER_ID)
            && cookies.get("dle_password").map(String::as_str) == Some(SESSION_TOKEN);
        signed_in.then_some(&self.account)
    }

    /// A title has one row; saving it again moves it to the top.
    pub fn save_watch(&mut self, post_id: i64, position: Option<(i64, i64)>) {
        let watched = self
            .history
            .iter()
            .find(|row| row.post_id == post_id)
            .is_some_and(|row| row.watched);
        self.history.retain(|row| row.post_id != post_id);
        self.history.insert(
            0,
            HistoryRow {
                id: format!("h{post_id}"),
                post_id,
                position,
                watched,
                date: "1 января 2024".to_string(),
            },
        );
    }

    /// Flips membership, as the site's endpoint does.
    pub fn toggle_favorite(&mut self, category: i64, post_id: i64) -> bool {
        let Some(titles) = self.favorites.get_mut(&category) else {
            return false;
        };
        match titles.iter().position(|id| *id == post_id) {
            Some(at) => {
                titles.remove(at);
            }
            None => titles.push(post_id),
        }
        true
    }

    pub fn categories_of(&self, post_id: i64) -> Vec<i64> {
        let mut categories: Vec<i64> = self
            .favorites
            .iter()
            .filter(|(_, titles)| titles.contains(&post_id))
            .map(|(category, _)| *category)
            .collect();
        categories.sort_unstable();
        categories
    }
}
