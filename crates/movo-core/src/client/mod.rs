pub mod anubis;
pub mod auth;
pub mod catalog;
pub mod details;
pub mod models;
pub mod search;
pub mod session;
pub mod stream;

use crate::storage::history::WatchHistory;
use models::{
    AccountData, ActorDetails, CatalogCategory, Collection, CommentsPage, FavoritesCollection,
    HomeSection, MediaDetails, MediaItem, SearchFilter, ServerHistoryEntry, StreamBundle,
    Translator, UserProfile,
};
use session::RezkaSession;
use std::collections::HashSet;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

#[derive(Debug, serde::Serialize)]
pub struct SyncedHistory {
    pub entries: Vec<ServerHistoryEntry>,
    pub local: WatchHistory,
}

#[derive(Clone)]
pub struct RezkaClient {
    session: RezkaSession,
    account: Arc<RwLock<AccountState>>,
    /// Lower-cased country names whose titles are left out of listings.
    hidden_countries: Arc<RwLock<Vec<String>>>,
}

/// Signed-in account, shared between clones so every caller observes the same
/// sign-in state without an external lock around the whole client.
#[derive(Default)]
struct AccountState {
    user: Option<UserProfile>,
    generation: u64,
}

/// Why a stored session could not be restored.
pub struct RestoreError {
    pub message: String,
    /// Whether the stored session is worthless from here on. A session the
    /// provider turned down is; one whose check never reached the provider is
    /// not, and discarding it would sign the account out over a lost network.
    pub is_rejected: bool,
}

impl RestoreError {
    fn rejected(message: String) -> Self {
        Self {
            message,
            is_rejected: true,
        }
    }

    fn retryable(message: String) -> Self {
        Self {
            message,
            is_rejected: false,
        }
    }
}

impl Default for RezkaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RezkaClient {
    pub fn export_session(&self) -> Result<String, String> {
        self.ensure_signed_in()?;
        self.session.export_session()
    }

    pub async fn import_session(&self, secret: &str) -> Result<UserProfile, RestoreError> {
        self.session
            .import_session(secret)
            .map_err(RestoreError::rejected)?;
        let profile = auth::check_profile(&self.session)
            .await
            .map_err(RestoreError::retryable)?
            .ok_or_else(|| RestoreError::rejected("Stored session has expired".to_string()))?;
        self.set_account(Some(profile.clone()));
        Ok(profile)
    }

    pub fn new() -> Self {
        Self {
            session: RezkaSession::new(),
            account: Arc::default(),
            hidden_countries: Arc::default(),
        }
    }

    /// Leave titles from these countries out of every listing from now on.
    /// Names are matched as [`MediaItem::is_from`] does, so pass them through
    /// [`models::parse_country_list`].
    pub fn set_hidden_countries(&self, countries: Vec<String>) {
        *self
            .hidden_countries
            .write()
            .unwrap_or_else(PoisonError::into_inner) = countries;
    }

    fn shown(&self, items: Vec<MediaItem>) -> Vec<MediaItem> {
        let hidden = self
            .hidden_countries
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        if hidden.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|item| !item.is_from(&hidden))
            .collect()
    }

    fn set_account(&self, user: Option<UserProfile>) {
        let mut account = self.account.write().unwrap_or_else(PoisonError::into_inner);
        account.user = user;
        account.generation = account.generation.wrapping_add(1);
    }

    pub fn session(&self) -> &RezkaSession {
        &self.session
    }

    pub fn user(&self) -> Option<UserProfile> {
        let account = self.account.read().unwrap_or_else(PoisonError::into_inner);
        account.user.clone().filter(|user| {
            self.session.authenticated_user_id().as_deref() == Some(user.user_id.as_str())
        })
    }

    pub fn account_generation(&self) -> u64 {
        let account = self.account.read().unwrap_or_else(PoisonError::into_inner);
        account.generation.wrapping_add(self.session.auth_epoch())
    }

    pub fn is_account_current(&self, generation: u64, user_id: &str) -> bool {
        self.account_generation() == generation
            && self.user().is_some_and(|user| user.user_id == user_id)
    }

    pub async fn fetch_catalog(
        &self,
        category: CatalogCategory,
        filter: Option<&str>,
        page: usize,
    ) -> Result<Vec<MediaItem>, String> {
        let items = catalog::fetch_catalog(&self.session, category, filter, page).await?;
        Ok(self.shown(items))
    }

    pub async fn search_full(&self, query: &str, page: usize) -> Result<Vec<MediaItem>, String> {
        let items = search::search_full(&self.session, query, page).await?;
        Ok(self.shown(items))
    }

    pub async fn search_suggestions(&self, query: &str) -> Result<Vec<String>, String> {
        search::suggestions(&self.session, query).await
    }

    pub async fn home(&self) -> Result<Vec<HomeSection>, String> {
        let sections = search::home(&self.session).await?;
        Ok(sections
            .into_iter()
            .map(|section| HomeSection {
                items: self.shown(section.items),
                ..section
            })
            .collect())
    }

    pub async fn search_filters(&self) -> Result<Vec<SearchFilter>, String> {
        search::filters(&self.session).await
    }

    pub async fn fetch_collections(&self, page: usize) -> Result<Vec<Collection>, String> {
        search::collections(&self.session, page).await
    }

    pub async fn fetch_path(&self, path: &str, page: usize) -> Result<Vec<MediaItem>, String> {
        let items = search::path(&self.session, path, page).await?;
        Ok(self.shown(items))
    }

    pub async fn fetch_details(&self, url: &str) -> Result<MediaDetails, String> {
        let mut details = details::fetch_details(&self.session, url).await?;
        details.related = self.shown(details.related);
        Ok(details)
    }

    pub async fn fetch_actor(&self, url: &str) -> Result<ActorDetails, String> {
        let mut actor = details::fetch_actor(&self.session, url).await?;
        actor.films = self.shown(actor.films);
        Ok(actor)
    }

    pub async fn fetch_comments(&self, post_id: i64, page: usize) -> Result<CommentsPage, String> {
        details::fetch_comments(&self.session, post_id, page).await
    }

    pub async fn fetch_trailer(&self, post_id: i64) -> Result<Option<String>, String> {
        details::fetch_trailer(&self.session, post_id).await
    }

    pub async fn post_rating(&self, post_id: i64, rating: u8) -> Result<(), String> {
        self.ensure_signed_in()?;
        details::post_rating(&self.session, post_id, rating).await
    }

    pub async fn like_comment(&self, id: &str) -> Result<(), String> {
        self.ensure_signed_in()?;
        details::like_comment(&self.session, id).await
    }

    pub async fn account_data(&self) -> Result<AccountData, String> {
        self.ensure_signed_in()?;
        let (notifications, premium_days) = auth::fetch_notifications(&self.session).await?;
        Ok(AccountData {
            notifications,
            premium_days,
        })
    }

    pub async fn toggle_schedule_watched(&self, id: &str) -> Result<(), String> {
        self.ensure_signed_in()?;
        auth::toggle_schedule_watched(&self.session, id).await
    }

    pub async fn fetch_episodes(
        &self,
        post_id: i64,
        translator_id: i64,
    ) -> Result<Vec<models::Season>, String> {
        details::fetch_episodes(&self.session, post_id, translator_id).await
    }

    pub async fn fetch_movie_stream(
        &self,
        post_id: i64,
        translator: &Translator,
    ) -> Result<StreamBundle, String> {
        let mut bundle = stream::fetch_movie_stream(&self.session, post_id, translator).await?;
        self.add_playback_headers(&mut bundle);
        Ok(bundle)
    }

    pub async fn fetch_episode_stream(
        &self,
        post_id: i64,
        translator_id: i64,
        season: i64,
        episode: i64,
    ) -> Result<StreamBundle, String> {
        let mut bundle =
            stream::fetch_episode_stream(&self.session, post_id, translator_id, season, episode)
                .await?;
        self.add_playback_headers(&mut bundle);
        Ok(bundle)
    }

    fn add_playback_headers(&self, bundle: &mut StreamBundle) {
        bundle.user_agent = self.session.user_agent().to_string();
        bundle.referer = self.session.referer().to_string();
    }

    pub async fn login(&self, email_or_login: &str, password: &str) -> Result<UserProfile, String> {
        let mut profile = auth::login(&self.session, email_or_login, password).await?;
        let mut settings = crate::storage::settings::AppSettings::load();
        settings.user_id = Some(profile.user_id.clone());
        if settings.save().is_err() {
            profile.is_session_persistent = false;
        }
        self.set_account(Some(profile.clone()));
        Ok(profile)
    }

    pub async fn restore_session(&self) -> Result<Option<UserProfile>, String> {
        let mut settings = crate::storage::settings::AppSettings::load();
        if let Some(user_id) = settings.user_id.clone() {
            match self.session.restore_session(&user_id) {
                Ok(true) => return self.verify_restored_session(&mut settings, false).await,
                Ok(false) => {}
                Err(error) if !RezkaSession::cookie_file_path().exists() => return Err(error),
                Err(_) => {}
            }
            if self.session.load_legacy_session(Some(&user_id))?.is_some() {
                return self.verify_restored_session(&mut settings, true).await;
            }
            settings.user_id = None;
            settings.save()?;
            self.set_account(None);
            return Ok(None);
        }

        let Some(user_id) = self.session.load_legacy_session(None)? else {
            return Ok(None);
        };
        settings.user_id = Some(user_id);
        settings.save()?;
        self.verify_restored_session(&mut settings, true).await
    }

    async fn verify_restored_session(
        &self,
        settings: &mut crate::storage::settings::AppSettings,
        legacy: bool,
    ) -> Result<Option<UserProfile>, String> {
        match auth::check_profile(&self.session).await {
            Ok(Some(mut profile)) => {
                if legacy {
                    profile.is_session_persistent =
                        self.session.persist_session(&profile.user_id).is_ok();
                    RezkaSession::remove_legacy_cookie_file()?;
                }
                self.set_account(Some(profile.clone()));
                Ok(Some(profile))
            }
            Ok(None) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let _ = self.session.clear_session(user_id.as_deref());
                settings.save()?;
                Ok(None)
            }
            Err(error) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let _ = self.session.clear_session(user_id.as_deref());
                settings.save()?;
                Err(error)
            }
        }
    }

    pub async fn logout(&self) -> Result<(), String> {
        let user_id = self
            .account
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .user
            .as_ref()
            .map(|user| user.user_id.clone());
        let result = auth::logout(&self.session, user_id.as_deref());
        self.set_account(None);
        let mut settings = crate::storage::settings::AppSettings::load();
        settings.user_id = None;
        let settings_cleared = settings.save().is_ok();
        match (result, settings_cleared) {
            (Ok(()), true) => Ok(()),
            (Err(error), true) => Err(error),
            (Ok(()), false) => Err("Account selection could not be cleared".to_string()),
            (Err(_), false) => {
                Err("Stored session and account selection could not be fully cleared".to_string())
            }
        }
    }

    pub async fn fetch_favorites_categories(&self) -> Result<Vec<FavoritesCollection>, String> {
        self.ensure_signed_in()?;
        auth::fetch_favorites_categories(&self.session).await
    }

    pub async fn fetch_favorites_page(
        &self,
        cat_id: Option<i64>,
        page: usize,
    ) -> Result<Vec<MediaItem>, String> {
        self.ensure_signed_in()?;
        auth::fetch_favorites_page(&self.session, cat_id, page).await
    }

    pub async fn set_favorite(
        &self,
        details_url: &str,
        post_id: i64,
        cat_id: i64,
        favorite: bool,
    ) -> Result<(), String> {
        self.ensure_signed_in()?;
        let current = self.fetch_details(details_url).await?;
        if current.favorite_category_ids.contains(&cat_id) == favorite {
            return Ok(());
        }
        auth::add_to_favorites(&self.session, post_id, cat_id).await?;
        let confirmed = self.fetch_details(details_url).await?;
        if confirmed.favorite_category_ids.contains(&cat_id) == favorite {
            Ok(())
        } else {
            Err("The account did not confirm the favorites update".to_string())
        }
    }

    async fn fetch_history(&self) -> Result<Vec<ServerHistoryEntry>, String> {
        self.ensure_signed_in()?;
        auth::fetch_history(&self.session).await
    }

    pub async fn sync_history(&self) -> Result<SyncedHistory, String> {
        let user_id = self
            .user()
            .map(|user| user.user_id.clone())
            .ok_or_else(|| "Authentication required".to_string())?;
        let entries = self.fetch_history().await?;
        let media_ids = entries
            .iter()
            .filter_map(ServerHistoryEntry::media_id)
            .collect::<HashSet<_>>();
        let local = WatchHistory::reconcile_media(
            &user_id,
            &media_ids,
            chrono::Utc::now() - chrono::Duration::minutes(5),
        )?;
        Ok(SyncedHistory { entries, local })
    }

    pub async fn save_watch(
        &self,
        post_id: i64,
        translator_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), String> {
        self.ensure_signed_in()?;
        if post_id <= 0 || translator_id <= 0 {
            return Err("Cannot sync history without valid media and voice-over IDs".to_string());
        }
        auth::save_watch(&self.session, post_id, translator_id, season, episode).await?;
        self.wait_for_history_entry(post_id).await.map(|_| ())
    }

    pub async fn remove_history(&self, id: &str) -> Result<(), String> {
        self.ensure_signed_in()?;
        let user_id = self
            .user()
            .ok_or_else(|| "Authentication required".to_string())?
            .user_id;
        let media_id = auth::fetch_history(&self.session)
            .await?
            .into_iter()
            .find(|entry| entry.id == id)
            .and_then(|entry| entry.media_id())
            .ok_or_else(|| "History item was not found".to_string())?;
        auth::remove_history(&self.session, id).await?;
        WatchHistory::remove_media_for(&user_id, media_id)
    }

    pub async fn set_history_watched(&self, id: &str, watched: bool) -> Result<(), String> {
        self.ensure_signed_in()?;
        let current = auth::fetch_history(&self.session).await?;
        let entry = current
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "History item was not found".to_string())?;
        if entry.is_watched == watched {
            return Ok(());
        }
        auth::toggle_history_watched(&self.session, id).await?;
        // The endpoint flips the flag rather than setting it, so the request is
        // never retried. Read the state back instead: a toggle whose reply was
        // lost still landed, and reporting failure for it would be wrong.
        let confirmed = auth::fetch_history(&self.session)
            .await?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "History item was not found".to_string())?;
        if confirmed.is_watched == watched {
            Ok(())
        } else {
            Err("The account did not confirm the watched state".to_string())
        }
    }

    pub async fn mark_watched(
        &self,
        post_id: i64,
        translator_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), String> {
        self.ensure_signed_in()?;
        let entry = self.wait_for_history_entry(post_id).await?;
        if !entry.is_watched {
            auth::toggle_history_watched(&self.session, &entry.id).await?;
        }
        if let (Some(season), Some(episode)) = (season, episode) {
            let seasons = self.fetch_episodes(post_id, translator_id).await?;
            let item = seasons
                .iter()
                .find(|item| item.id == season)
                .and_then(|item| item.episodes.iter().find(|item| item.id == episode))
                .ok_or_else(|| "Finished episode was not found".to_string())?;
            if !item.is_watched {
                let watch_id = item
                    .watch_id
                    .as_deref()
                    .ok_or_else(|| "Finished episode has no watched-state ID".to_string())?;
                auth::toggle_schedule_watched(&self.session, watch_id).await?;
            }
        }
        Ok(())
    }

    async fn wait_for_history_entry(&self, post_id: i64) -> Result<ServerHistoryEntry, String> {
        for attempt in 0..10 {
            if let Some(entry) = auth::fetch_history(&self.session)
                .await?
                .into_iter()
                .find(|entry| entry.media_id() == Some(post_id))
            {
                return Ok(entry);
            }
            if attempt < 9 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        Err("Saved history item was not found".to_string())
    }

    fn ensure_signed_in(&self) -> Result<(), String> {
        let account = self.account.read().unwrap_or_else(PoisonError::into_inner);
        let user = account
            .user
            .as_ref()
            .ok_or_else(|| "Authentication required".to_string())?;
        if self.session.authenticated_user_id().as_deref() != Some(user.user_id.as_str()) {
            return Err("Authenticated session is no longer valid".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..headers_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                })
                .unwrap_or(0);
            if request.len() >= headers_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8(request).unwrap()
    }

    fn respond(stream: &mut TcpStream, content_type: &str, body: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    }

    fn authenticated_test_client(base_url: &str) -> RezkaClient {
        let session = RezkaSession::new_for_test(base_url);
        session.authenticate_for_test("42");
        RezkaClient {
            session,
            hidden_countries: Arc::default(),
            account: Arc::new(RwLock::new(AccountState {
                user: Some(UserProfile {
                    user_id: "42".to_string(),
                    username: "Tester".to_string(),
                    is_logged_in: true,
                    is_vip: false,
                    email: None,
                    avatar_url: None,
                    premium_days: None,
                    is_session_persistent: false,
                }),
                generation: 0,
            })),
        }
    }

    #[tokio::test]
    async fn save_watch_accepts_provider_false_after_readback() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut save, _) = listener.accept().unwrap();
            assert!(read_request(&mut save).starts_with("POST /ajax/send_save/"));
            respond(&mut save, "application/json", r#"{"success":false}"#);
            let (mut history, _) = listener.accept().unwrap();
            assert!(read_request(&mut history).starts_with("GET /continue/"));
            respond(
                &mut history,
                "text/html",
                r#"<div class="b-videosaves__list_item"></div><div class="b-videosaves__list_item"><button class="delete" data-id="saved"></button><div class="title"><a href="/films/7-test.html">Test</a></div></div>"#,
            );
        });

        authenticated_test_client(&base_url)
            .save_watch(7, 8, None, None)
            .await
            .unwrap();
        server.join().unwrap();
    }

    #[tokio::test]
    async fn favorite_mutation_converges_to_desired_server_state() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut before, _) = listener.accept().unwrap();
            assert!(read_request(&mut before).starts_with("GET /films/7-test.html"));
            respond(
                &mut before,
                "text/html",
                r#"<h1 class="b-post__title">Test</h1><div class="hd-label-row"><input value="3"></div>"#,
            );
            let (mut toggle, _) = listener.accept().unwrap();
            assert!(read_request(&mut toggle).starts_with("POST /ajax/favorites/"));
            respond(&mut toggle, "application/json", r#"{"success":true}"#);
            let (mut after, _) = listener.accept().unwrap();
            read_request(&mut after);
            respond(
                &mut after,
                "text/html",
                r#"<h1 class="b-post__title">Test</h1><div class="hd-label-row"><input value="3" checked></div>"#,
            );
        });
        let url = format!("{base_url}/films/7-test.html");

        authenticated_test_client(&base_url)
            .set_favorite(&url, 7, 3, true)
            .await
            .unwrap();
        server.join().unwrap();
    }

    fn history_row(watched: bool) -> String {
        format!(
            r#"<div class="b-videosaves__list_item"></div><div class="b-videosaves__list_item{}"><button class="delete" data-id="saved"></button><div class="title"><a href="/films/7-test.html">Test</a></div></div>"#,
            if watched { " watched-row" } else { "" }
        )
    }

    /// The toggle endpoint is posted once and never retried, so success is
    /// decided by reading the account back rather than by the reply alone.
    #[tokio::test]
    async fn watched_mutation_is_confirmed_by_reading_the_account_back() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut before, _) = listener.accept().unwrap();
            read_request(&mut before);
            respond(&mut before, "text/html", &history_row(false));
            let (mut toggle, _) = listener.accept().unwrap();
            assert!(read_request(&mut toggle).starts_with("POST /engine/ajax/cdn_saves_view.php"));
            respond(&mut toggle, "application/json", r#"{"success":true}"#);
            let (mut after, _) = listener.accept().unwrap();
            read_request(&mut after);
            respond(&mut after, "text/html", &history_row(true));
        });

        authenticated_test_client(&base_url)
            .set_history_watched("saved", true)
            .await
            .unwrap();
        server.join().unwrap();
    }

    /// A reply claiming success that the account does not actually reflect is
    /// reported as a failure rather than posted a second time.
    #[tokio::test]
    async fn watched_mutation_reports_a_state_the_account_did_not_take() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut before, _) = listener.accept().unwrap();
            read_request(&mut before);
            respond(&mut before, "text/html", &history_row(false));
            let (mut toggle, _) = listener.accept().unwrap();
            assert!(read_request(&mut toggle).starts_with("POST /engine/ajax/cdn_saves_view.php"));
            respond(&mut toggle, "application/json", r#"{"success":true}"#);
            let (mut after, _) = listener.accept().unwrap();
            read_request(&mut after);
            respond(&mut after, "text/html", &history_row(false));
        });

        let error = authenticated_test_client(&base_url)
            .set_history_watched("saved", true)
            .await
            .unwrap_err();

        assert!(error.contains("did not confirm"), "{error}");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn mark_watched_waits_for_delayed_history_entry() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut missing, _) = listener.accept().unwrap();
            read_request(&mut missing);
            respond(
                &mut missing,
                "text/html",
                r#"<div class="b-videosaves"></div>"#,
            );

            let (mut found, _) = listener.accept().unwrap();
            read_request(&mut found);
            respond(
                &mut found,
                "text/html",
                r#"<div class="b-videosaves__list_item"></div><div class="b-videosaves__list_item"><button class="delete" data-id="saved"></button><div class="title"><a href="/films/7-test.html">Test</a></div></div>"#,
            );

            let (mut toggle, _) = listener.accept().unwrap();
            assert!(read_request(&mut toggle).starts_with("POST /engine/ajax/cdn_saves_view.php"));
            respond(&mut toggle, "application/json", r#"{"success":true}"#);
        });

        authenticated_test_client(&base_url)
            .mark_watched(7, 8, None, None)
            .await
            .unwrap();
        server.join().unwrap();
    }

    #[test]
    fn cleared_session_invalidates_user_and_account_generation() {
        let client = authenticated_test_client("https://example.com");
        client
            .account
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .generation = 7;
        let generation = client.account_generation();
        assert!(client.user().is_some());
        assert!(client.is_account_current(generation, "42"));

        client.session.invalidate_auth();

        assert!(client.user().is_none());
        assert_ne!(client.account_generation(), generation);
        assert!(!client.is_account_current(generation, "42"));
    }

    #[tokio::test]
    async fn save_watch_rejects_invalid_ids_without_a_request() {
        let client = authenticated_test_client("https://example.test");
        assert!(client.save_watch(0, 8, None, None).await.is_err());
        assert!(client.save_watch(7, 0, None, None).await.is_err());
    }

    #[tokio::test]
    async fn history_rejects_a_non_history_landing_page() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut request, _) = listener.accept().unwrap();
            assert!(read_request(&mut request).starts_with("GET /continue/"));
            respond(
                &mut request,
                "text/html",
                "<html><body>Landing page</body></html>",
            );
        });

        assert!(authenticated_test_client(&base_url)
            .fetch_history()
            .await
            .is_err());
        server.join().unwrap();
    }

    #[test]
    fn hidden_countries_drop_listings_by_their_info_line() {
        let client = RezkaClient::new();
        let item = |info: &str| MediaItem {
            id: 1,
            title: String::new(),
            orig_title: None,
            url: String::new(),
            poster_url: None,
            year: None,
            category: None,
            rating: None,
            info: Some(info.to_string()),
        };
        let items = || {
            vec![
                item("2019, США, Боевики"),
                item("2021, Россия, Драмы"),
                item("2020, Великобритания, Комедии"),
            ]
        };

        assert_eq!(client.shown(items()).len(), 3);
        client.set_hidden_countries(models::parse_country_list(" россия ,, Великобритания"));
        let shown = client.shown(items());
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].info.as_deref(), Some("2019, США, Боевики"));
    }
}
