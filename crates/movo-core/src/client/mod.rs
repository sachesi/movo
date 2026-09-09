pub mod anubis;
pub mod auth;
pub mod catalog;
pub mod countries;
pub mod details;
pub mod models;
pub mod search;
pub mod session;
pub mod stream;

use crate::error::{ClientError, ErrorKind};
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

/// Runs `work` on the blocking thread pool. The keyring and file operations
/// this wraps are synchronous, and calling them straight from an async method
/// would stall every other task on the runtime for as long as they take.
async fn blocking<T: Send + 'static>(
    description: &'static str,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ClientError> {
    tokio::task::spawn_blocking(work).await.map_err(|_| {
        ClientError::new(
            ErrorKind::Other,
            format!("{description} stopped unexpectedly"),
        )
    })
}

fn history_entry_matches(
    entry: &ServerHistoryEntry,
    post_id: i64,
    season: Option<i64>,
    episode: Option<i64>,
) -> bool {
    if entry.media_id() != Some(post_id) {
        return false;
    }
    let (Some(season), Some(episode)) = (season, episode) else {
        return true;
    };
    // The row has to agree wherever it states a season or an episode.
    let (row_season, row_episode) = entry.labelled_position();
    row_season.is_none_or(|at| at == season) && row_episode.is_none_or(|at| at == episode)
}

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
    pub error: ClientError,
    /// Whether the stored session is worthless from here on. A session the
    /// provider turned down is; one whose check never reached the provider is
    /// not, and discarding it would sign the account out over a lost network.
    pub is_rejected: bool,
}

impl RestoreError {
    fn rejected(error: impl Into<ClientError>) -> Self {
        Self {
            error: error.into(),
            is_rejected: true,
        }
    }

    fn retryable(error: impl Into<ClientError>) -> Self {
        Self {
            error: error.into(),
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
    pub fn export_session(&self) -> Result<String, ClientError> {
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
    ) -> Result<Vec<MediaItem>, ClientError> {
        let items = catalog::fetch_catalog(&self.session, category, filter, page).await?;
        Ok(self.shown(items))
    }

    pub async fn search_full(
        &self,
        query: &str,
        page: usize,
    ) -> Result<Vec<MediaItem>, ClientError> {
        let items = search::search_full(&self.session, query, page).await?;
        Ok(self.shown(items))
    }

    pub async fn search_suggestions(&self, query: &str) -> Result<Vec<String>, ClientError> {
        Ok(search::suggestions(&self.session, query).await?)
    }

    pub async fn home(&self) -> Result<Vec<HomeSection>, ClientError> {
        let sections = search::home(&self.session).await?;
        Ok(sections
            .into_iter()
            .map(|section| HomeSection {
                items: self.shown(section.items),
                ..section
            })
            .collect())
    }

    pub async fn search_filters(&self) -> Result<Vec<SearchFilter>, ClientError> {
        Ok(search::filters(&self.session).await?)
    }

    pub async fn fetch_collections(&self, page: usize) -> Result<Vec<Collection>, ClientError> {
        Ok(search::collections(&self.session, page).await?)
    }

    pub async fn fetch_path(&self, path: &str, page: usize) -> Result<Vec<MediaItem>, ClientError> {
        let items = search::path(&self.session, path, page).await?;
        Ok(self.shown(items))
    }

    pub async fn fetch_details(&self, url: &str) -> Result<MediaDetails, ClientError> {
        let mut details = details::fetch_details(&self.session, url).await?;
        details.related = self.shown(details.related);
        Ok(details)
    }

    pub async fn fetch_actor(&self, url: &str) -> Result<ActorDetails, ClientError> {
        let mut actor = details::fetch_actor(&self.session, url).await?;
        actor.films = self.shown(actor.films);
        Ok(actor)
    }

    pub async fn fetch_comments(
        &self,
        post_id: i64,
        page: usize,
    ) -> Result<CommentsPage, ClientError> {
        Ok(details::fetch_comments(&self.session, post_id, page).await?)
    }

    pub async fn fetch_trailer(&self, post_id: i64) -> Result<Option<String>, ClientError> {
        Ok(details::fetch_trailer(&self.session, post_id).await?)
    }

    pub async fn post_rating(&self, post_id: i64, rating: u8) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        Ok(details::post_rating(&self.session, post_id, rating).await?)
    }

    pub async fn like_comment(&self, id: &str) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        Ok(details::like_comment(&self.session, id).await?)
    }

    pub async fn account_data(&self) -> Result<AccountData, ClientError> {
        self.ensure_signed_in()?;
        let (notifications, premium_days) = auth::fetch_notifications(&self.session).await?;
        Ok(AccountData {
            notifications,
            premium_days,
        })
    }

    pub async fn toggle_schedule_watched(&self, id: &str) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        Ok(auth::toggle_schedule_watched(&self.session, id).await?)
    }

    /// The episodes of a voice-over, carrying the watched state the title's
    /// schedule keeps for them.
    pub async fn fetch_episodes(
        &self,
        post_id: i64,
        translator_id: i64,
        schedules: &[models::ScheduleGroup],
    ) -> Result<Vec<models::Season>, ClientError> {
        let mut seasons = details::fetch_episodes(&self.session, post_id, translator_id).await?;
        details::mark_scheduled(&mut seasons, schedules);
        Ok(seasons)
    }

    pub async fn fetch_movie_stream(
        &self,
        post_id: i64,
        translator: &Translator,
    ) -> Result<StreamBundle, ClientError> {
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
    ) -> Result<StreamBundle, ClientError> {
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

    pub async fn login(
        &self,
        email_or_login: &str,
        password: &str,
    ) -> Result<UserProfile, ClientError> {
        let mut profile = auth::login(&self.session, email_or_login, password).await?;
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        settings.user_id = Some(profile.user_id.clone());
        let saved = settings.clone();
        let save_ok = blocking("Saving settings", move || saved.save())
            .await
            .is_ok_and(|result| result.is_ok());
        if !save_ok {
            profile.is_session_persistent = false;
        }
        self.set_account(Some(profile.clone()));
        Ok(profile)
    }

    pub async fn restore_session(&self) -> Result<Option<UserProfile>, ClientError> {
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        if let Some(user_id) = settings.user_id.clone() {
            let session = self.session.clone();
            let lookup_id = user_id.clone();
            let restored = blocking("Restoring the session", move || {
                session.restore_session(&lookup_id)
            })
            .await?;
            match restored {
                Ok(true) => return self.verify_restored_session(&mut settings, false).await,
                Ok(false) => {}
                Err(error) if !RezkaSession::cookie_file_path().exists() => return Err(error),
                Err(_) => {}
            }
            let session = self.session.clone();
            let legacy_id = user_id.clone();
            let legacy = blocking("Restoring the session", move || {
                session.load_legacy_session(Some(&legacy_id))
            })
            .await??;
            if legacy.is_some() {
                return self.verify_restored_session(&mut settings, true).await;
            }
            settings.user_id = None;
            let saved = settings.clone();
            blocking("Saving settings", move || saved.save()).await??;
            self.set_account(None);
            return Ok(None);
        }

        let session = self.session.clone();
        let Some(user_id) = blocking("Restoring the session", move || {
            session.load_legacy_session(None)
        })
        .await??
        else {
            return Ok(None);
        };
        settings.user_id = Some(user_id);
        let saved = settings.clone();
        blocking("Saving settings", move || saved.save()).await??;
        self.verify_restored_session(&mut settings, true).await
    }

    async fn verify_restored_session(
        &self,
        settings: &mut crate::storage::settings::AppSettings,
        legacy: bool,
    ) -> Result<Option<UserProfile>, ClientError> {
        match auth::check_profile(&self.session).await {
            Ok(Some(mut profile)) => {
                if legacy {
                    let session = self.session.clone();
                    let user_id = profile.user_id.clone();
                    profile.is_session_persistent = blocking("Persisting the session", move || {
                        session.persist_session(&user_id)
                    })
                    .await
                    .is_ok_and(|result| result.is_ok());
                    blocking(
                        "Removing the legacy session",
                        RezkaSession::remove_legacy_cookie_file,
                    )
                    .await??;
                }
                self.set_account(Some(profile.clone()));
                Ok(Some(profile))
            }
            Ok(None) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let session = self.session.clone();
                let _ = blocking("Clearing the session", move || {
                    session.clear_session(user_id.as_deref())
                })
                .await;
                let saved = settings.clone();
                blocking("Saving settings", move || saved.save()).await??;
                Ok(None)
            }
            Err(error) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let session = self.session.clone();
                let _ = blocking("Clearing the session", move || {
                    session.clear_session(user_id.as_deref())
                })
                .await;
                let saved = settings.clone();
                blocking("Saving settings", move || saved.save()).await??;
                Err(ClientError::from(error))
            }
        }
    }

    pub async fn logout(&self) -> Result<(), ClientError> {
        let user_id = self
            .account
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .user
            .as_ref()
            .map(|user| user.user_id.clone());
        let session = self.session.clone();
        let logout_id = user_id.clone();
        let result = blocking("Signing out", move || {
            auth::logout(&session, logout_id.as_deref())
        })
        .await
        .and_then(|inner| inner.map_err(ClientError::from));
        self.set_account(None);
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        settings.user_id = None;
        let saved = settings.clone();
        let settings_cleared = blocking("Saving settings", move || saved.save())
            .await
            .is_ok_and(|inner| inner.is_ok());
        match (result, settings_cleared) {
            (Ok(()), true) => Ok(()),
            (Err(error), true) => Err(error),
            (Ok(()), false) => Err(ClientError::from(
                "Account selection could not be cleared".to_string(),
            )),
            (Err(_), false) => Err(ClientError::from(
                "Stored session and account selection could not be fully cleared".to_string(),
            )),
        }
    }

    pub async fn fetch_favorites_categories(
        &self,
    ) -> Result<Vec<FavoritesCollection>, ClientError> {
        self.ensure_signed_in()?;
        Ok(auth::fetch_favorites_categories(&self.session).await?)
    }

    pub async fn fetch_favorites_page(
        &self,
        cat_id: Option<i64>,
        page: usize,
    ) -> Result<Vec<MediaItem>, ClientError> {
        self.ensure_signed_in()?;
        Ok(auth::fetch_favorites_page(&self.session, cat_id, page).await?)
    }

    pub async fn set_favorite(
        &self,
        details_url: &str,
        post_id: i64,
        cat_id: i64,
        favorite: bool,
    ) -> Result<(), ClientError> {
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
            Err(ClientError::from(
                "The account did not confirm the favorites update".to_string(),
            ))
        }
    }

    /// The account's history rows, newest first.
    pub async fn fetch_history(&self) -> Result<Vec<ServerHistoryEntry>, ClientError> {
        self.ensure_signed_in()?;
        Ok(auth::fetch_history(&self.session).await?)
    }

    pub async fn sync_history(&self) -> Result<SyncedHistory, ClientError> {
        let user_id = self
            .user()
            .map(|user| user.user_id.clone())
            .ok_or_else(|| "Authentication required".to_string())?;
        let entries = self.fetch_history().await?;
        let media_ids = entries
            .iter()
            .filter_map(ServerHistoryEntry::media_id)
            .collect::<HashSet<_>>();
        let protect_since = chrono::Utc::now() - chrono::Duration::minutes(5);
        let local = blocking("Syncing history", move || {
            WatchHistory::reconcile_media(&user_id, &media_ids, protect_since)
        })
        .await??;
        Ok(SyncedHistory { entries, local })
    }

    pub async fn save_watch(
        &self,
        post_id: i64,
        translator_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        if post_id <= 0 || translator_id <= 0 {
            return Err(ClientError::from(
                "Cannot sync history without valid media and voice-over IDs".to_string(),
            ));
        }
        auth::save_watch(&self.session, post_id, translator_id, season, episode).await?;
        self.wait_for_history_entry(post_id, season, episode)
            .await
            .map(|_| ())
            .map_err(ClientError::from)
    }

    pub async fn remove_history(&self, id: &str) -> Result<(), ClientError> {
        self.remove_history_with_media(id).await.map(|_| ())
    }

    pub async fn remove_history_with_media(&self, id: &str) -> Result<i64, ClientError> {
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
        // Best-effort: the caller re-reads the list right after, and the provider
        // needs a moment to drop the row. A slow confirmation is not a failed
        // removal, and reporting one would leave the local copy behind.
        let _ = self.wait_for_history_absence(id).await;
        blocking("Removing the history item", move || {
            WatchHistory::remove_media_for(&user_id, media_id)
        })
        .await??;
        Ok(media_id)
    }

    pub async fn set_history_watched(&self, id: &str, watched: bool) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        let current = auth::fetch_history(&self.session).await?;
        let entry = current
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "History item was not found".to_string())?;
        if entry.is_watched == watched {
            return Ok(());
        }
        let toggle_error = auth::toggle_history_watched(&self.session, id).await.err();
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
        } else if let Some(error) = toggle_error {
            Err(ClientError::from(error))
        } else {
            Err(ClientError::from(
                "The account did not confirm the watched state".to_string(),
            ))
        }
    }

    /// Records that the title at `url` was watched to the end: the history
    /// row is flagged, and for an episode so is its schedule row.
    pub async fn mark_watched(
        &self,
        url: &str,
        post_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), ClientError> {
        self.ensure_signed_in()?;
        let entry = self
            .wait_for_history_entry(post_id, season, episode)
            .await?;
        if !entry.is_watched {
            auth::toggle_history_watched(&self.session, &entry.id).await?;
        }
        let confirmed = auth::fetch_history(&self.session)
            .await?
            .into_iter()
            .find(|candidate| candidate.id == entry.id)
            .ok_or_else(|| "History item disappeared while marking watched".to_string())?;
        if !confirmed.is_watched {
            return Err(ClientError::from(
                "The account did not confirm the watched state".to_string(),
            ));
        }
        if let (Some(season), Some(episode)) = (season, episode) {
            // The provider keeps an episode's watched flag on its schedule
            // row; an episode the schedule does not list has none to set.
            let schedules = details::fetch_schedules(&self.session, url).await?;
            let Some(item) = details::schedule_item_for(&schedules, season, episode)
                .filter(|item| !item.id.is_empty())
            else {
                return Ok(());
            };
            if !item.is_watched {
                auth::toggle_schedule_watched(&self.session, &item.id).await?;
            }
            let confirmed = details::fetch_schedules(&self.session, url).await?;
            let item = details::schedule_item_for(&confirmed, season, episode)
                .ok_or_else(|| "Finished episode was not found".to_string())?;
            if !item.is_watched {
                return Err(ClientError::from(
                    "The account did not confirm the episode watched state".to_string(),
                ));
            }
        }
        Ok(())
    }

    async fn wait_for_history_entry(
        &self,
        post_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<ServerHistoryEntry, String> {
        for attempt in 0..10 {
            let entries = auth::fetch_history(&self.session).await?;
            log::debug!(
                "history readback {attempt}: {} rows, looking for {post_id} S{season:?}E{episode:?}; first rows: {:?}",
                entries.len(),
                entries
                    .iter()
                    .take(3)
                    .map(|entry| (entry.url.as_str(), entry.info.as_deref()))
                    .collect::<Vec<_>>()
            );
            if let Some(entry) = entries
                .into_iter()
                .find(|entry| history_entry_matches(entry, post_id, season, episode))
            {
                return Ok(entry);
            }
            if attempt < 9 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        Err("Saved history item was not found".to_string())
    }

    async fn wait_for_history_absence(&self, id: &str) -> Result<(), String> {
        for attempt in 0..10 {
            if !auth::fetch_history(&self.session)
                .await?
                .iter()
                .any(|entry| entry.id == id)
            {
                return Ok(());
            }
            if attempt < 9 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        Err("The account did not confirm history removal".to_string())
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

    #[test]
    fn episode_history_matching_does_not_accept_a_different_labelled_episode() {
        let mut entry = auth::parse_history(&history_row(false)).pop().unwrap();
        entry.info = Some("Season 1 - Episode 2".to_string());
        assert!(history_entry_matches(&entry, 7, Some(1), Some(2)));
        assert!(!history_entry_matches(&entry, 7, Some(1), Some(3)));
    }

    #[test]
    fn episode_history_matching_accepts_provider_label_order() {
        let mut entry = auth::parse_history(&history_row(false)).pop().unwrap();
        entry.info = Some("1 сезон, 2 серия".to_string());
        assert!(history_entry_matches(&entry, 7, Some(1), Some(2)));
        assert!(!history_entry_matches(&entry, 7, Some(1), Some(3)));

        entry.info = Some("Сезон 1, эпизод 2".to_string());
        assert!(history_entry_matches(&entry, 7, Some(1), Some(2)));
        assert!(!history_entry_matches(&entry, 7, Some(1), Some(3)));
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

        assert!(error.message.contains("did not confirm"), "{error}");
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
            let (mut confirmed, _) = listener.accept().unwrap();
            read_request(&mut confirmed);
            respond(&mut confirmed, "text/html", &history_row(true));
        });

        authenticated_test_client(&base_url)
            .mark_watched("/films/7-test.html", 7, None, None)
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
        // Named in another language than the listing, and by code.
        client.set_hidden_countries(models::parse_country_list(" Росія ,, gb"));
        let shown = client.shown(items());
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].info.as_deref(), Some("2019, США, Боевики"));
    }
}
