mod account;
pub mod anubis;
pub mod auth;
pub mod catalog;
mod cdn;
pub mod countries;
pub mod details;
mod history_sync;
pub mod models;
pub mod search;
pub mod session;
pub mod stream;
#[cfg(test)]
mod test_support;

use crate::error::{ClientError, ErrorKind};
use crate::storage::history::WatchHistory;
use models::{
    AccountData, ActorDetails, CatalogCategory, Collection, CommentsPage, FavoritesCollection,
    HomeSection, MediaDetails, MediaItem, SearchFilter, ServerHistoryEntry, StreamBundle,
    Translator, UserProfile,
};
use session::RezkaSession;
use std::sync::{Arc, PoisonError, RwLock};

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

    /// Starts checking which stream hosts can be reached, in the background,
    /// unless that happened recently; fetching a stream does the same. Call it
    /// from within the Tokio runtime, once the app starts.
    pub fn check_stream_hosts(&self) {
        cdn::check_if_due(self.session.user_agent());
    }

    fn add_playback_headers(&self, bundle: &mut StreamBundle) {
        bundle.user_agent = self.session.user_agent().to_string();
        bundle.referer = self.session.referer().to_string();
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
    use crate::client::test_support::authenticated_test_client;
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

        client.session.invalidate_auth();

        assert!(client.user().is_none());
        assert_ne!(client.account_generation(), generation);
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
