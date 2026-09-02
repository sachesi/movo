use directories::ProjectDirs;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;
use url::Url;

use super::anubis::{AnubisSolution, AnubisSolver};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 15; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
pub const OFFICIAL_MIRROR: &str = "https://hdrzk.org";
#[cfg(not(target_os = "android"))]
const KEYRING_SERVICE: &str = "org.gnome.Movo";
const SESSION_VERSION: u8 = 1;

/// Per-host in-flight Anubis solves: one solve at a time per host so
/// concurrent challenges can't clobber each other's verification cookie in
/// the shared jar.
static ANUBIS_INFLIGHT: OnceLock<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedCookies {
    pub cookies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HostCookies {
    hosts: HashMap<String, HashMap<String, String>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StoredCookie {
    url: String,
    header: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SessionSecret {
    version: u8,
    provider: String,
    user_id: String,
    cookies: Vec<StoredCookie>,
}

#[derive(Default)]
struct SharedCookieJar {
    state: RwLock<CookieState>,
}

#[derive(Default)]
struct CookieState {
    jar: Arc<Jar>,
    stored: Vec<StoredCookie>,
}

impl SharedCookieJar {
    fn add_cookie_str(&self, header: &str, url: &Url) {
        let mut state = self.state.write().unwrap();
        state.jar.add_cookie_str(header, url);
        Self::remember(&mut state.stored, header, url);
    }

    fn remember(stored: &mut Vec<StoredCookie>, header: &str, url: &Url) {
        let Some(identity) = cookie_identity(header, url) else {
            return;
        };
        stored.retain(|cookie| {
            Url::parse(&cookie.url)
                .ok()
                .and_then(|url| cookie_identity(&cookie.header, &url))
                != Some(identity.clone())
        });
        stored.push(StoredCookie {
            url: url.as_str().to_string(),
            header: header.to_string(),
        });
    }

    fn replace(&self, cookies: &[StoredCookie]) {
        let jar = Arc::new(Jar::default());
        let mut restored = Vec::new();
        for cookie in cookies {
            if let Ok(url) = Url::parse(&cookie.url) {
                jar.add_cookie_str(&cookie.header, &url);
                restored.push(cookie.clone());
            }
        }
        *self.state.write().unwrap() = CookieState {
            jar,
            stored: restored,
        };
    }

    fn clear(&self) {
        self.replace(&[]);
    }

    fn snapshot(&self) -> Vec<StoredCookie> {
        self.state.read().unwrap().stored.clone()
    }
}

impl CookieStore for SharedCookieJar {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let headers = headers.cloned().collect::<Vec<_>>();
        let mut state = self.state.write().unwrap();
        state.jar.set_cookies(&mut headers.iter(), url);
        for header in headers {
            if let Ok(header) = header.to_str() {
                Self::remember(&mut state.stored, header, url);
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        self.state.read().unwrap().jar.cookies(url)
    }
}

fn cookie_identity(header: &str, url: &Url) -> Option<(String, String, String)> {
    let mut parts = header.split(';');
    let name = parts.next()?.split_once('=')?.0.trim().to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    let mut domain = url.host_str()?.to_ascii_lowercase();
    let mut path = "/".to_string();
    for attribute in parts {
        if let Some((key, value)) = attribute.trim().split_once('=') {
            match key.trim().to_ascii_lowercase().as_str() {
                "domain" => domain = value.trim().trim_start_matches('.').to_ascii_lowercase(),
                "path" => path = value.trim().to_string(),
                _ => {}
            }
        }
    }
    Some((name, domain, path))
}

#[derive(Clone)]
pub struct RezkaSession {
    client: reqwest::Client,
    /// Client that does NOT follow redirects — used for the Anubis
    /// pass-challenge call, whose 302 carries the clearance Set-Cookie that
    /// would be hidden if followed.
    pass_client: reqwest::Client,
    cookie_jar: Arc<SharedCookieJar>,
    auth_epoch: Arc<AtomicU64>,
    base_url: String,
}

impl Default for RezkaSession {
    fn default() -> Self {
        Self::new()
    }
}

impl RezkaSession {
    pub fn export_session(&self) -> Result<String, String> {
        let user_id = self
            .authenticated_user_id()
            .ok_or_else(|| "Authentication required".to_string())?;
        serde_json::to_string(&SessionSecret {
            version: SESSION_VERSION,
            provider: OFFICIAL_MIRROR.to_string(),
            user_id,
            cookies: self.cookie_jar.snapshot(),
        })
        .map_err(|error| format!("Could not serialize session: {error}"))
    }

    pub fn import_session(&self, secret: &str) -> Result<String, String> {
        let secret: SessionSecret =
            serde_json::from_str(secret).map_err(|_| "Stored session is invalid".to_string())?;
        if secret.version != SESSION_VERSION
            || secret.provider != OFFICIAL_MIRROR
            || secret.user_id.is_empty()
            || secret.cookies.is_empty()
        {
            return Err("Stored session is invalid".to_string());
        }
        self.cookie_jar.replace(&secret.cookies);
        if self.authenticated_user_id().as_deref() == Some(secret.user_id.as_str()) {
            self.auth_epoch.fetch_add(1, Ordering::SeqCst);
            Ok(secret.user_id)
        } else {
            self.cookie_jar.clear();
            Err("Stored session is invalid".to_string())
        }
    }
    pub fn new() -> Self {
        Self::new_for_base(OFFICIAL_MIRROR)
    }

    #[cfg(test)]
    pub(super) fn new_for_test(base_url: &str) -> Self {
        Self::new_for_base(base_url)
    }

    #[cfg(test)]
    pub(super) fn authenticate_for_test(&self, user_id: &str) {
        let url = Url::parse(&self.base_url).unwrap();
        self.cookie_jar
            .add_cookie_str(&format!("dle_user_id={user_id}; Path=/"), &url);
        self.cookie_jar
            .add_cookie_str("dle_password=hash; Path=/", &url);
    }

    fn new_for_base(base_url: &str) -> Self {
        let cookie_jar = Arc::new(SharedCookieJar::default());
        let base = base_url.trim_end_matches('/').to_string();

        let headers = Self::default_headers();

        let client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .default_headers(headers.clone())
            .timeout(Duration::from_secs(25))
            .gzip(true)
            .brotli(true)
            .build()
            .expect("Failed to build reqwest HTTP client");

        let pass_client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .default_headers(headers)
            .timeout(Duration::from_secs(25))
            .redirect(reqwest::redirect::Policy::none())
            .gzip(true)
            .brotli(true)
            .build()
            .expect("Failed to build Anubis pass-challenge HTTP client");

        Self {
            client,
            pass_client,
            cookie_jar,
            auth_epoch: Arc::new(AtomicU64::new(0)),
            base_url: base,
        }
    }

    pub fn cookie_file_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("org", "gnome", "Movo") {
            proj_dirs.data_dir().join("session_cookies.json")
        } else {
            PathBuf::from("session_cookies.json")
        }
    }

    pub fn remove_legacy_cookie_file() -> Result<(), String> {
        let path = Self::cookie_file_path();
        if path.exists() {
            fs::remove_file(path)
                .map_err(|_| "Legacy session file could not be removed".to_string())?;
        }
        Ok(())
    }

    pub fn authenticated_user_id(&self) -> Option<String> {
        let url = Url::parse(&self.base_url).ok()?;
        let cookies = self.cookie_jar.cookies(&url)?;
        let cookies = cookies.to_str().ok()?;
        let mut user_id = None;
        let mut has_password = false;
        for cookie in cookies.split(';') {
            let Some((name, value)) = cookie.trim().split_once('=') else {
                continue;
            };
            match name.trim() {
                "dle_user_id" if !value.trim().is_empty() => {
                    user_id = Some(value.trim().to_string())
                }
                "dle_password" if !value.trim().is_empty() => has_password = true,
                _ => {}
            }
        }
        has_password.then_some(user_id).flatten()
    }

    pub(super) fn auth_epoch(&self) -> u64 {
        self.auth_epoch.load(Ordering::SeqCst)
    }

    pub(super) fn invalidate_auth(&self) {
        self.cookie_jar.clear();
        self.auth_epoch.fetch_add(1, Ordering::SeqCst);
    }

    #[cfg(not(target_os = "android"))]
    pub fn persist_session(&self, user_id: &str) -> Result<(), String> {
        if self.base_url != OFFICIAL_MIRROR {
            return Err("Only official-provider sessions can be persisted".to_string());
        }
        if self.authenticated_user_id().as_deref() != Some(user_id) {
            return Err("Cannot persist an unverified session".to_string());
        }
        let secret = SessionSecret {
            version: SESSION_VERSION,
            provider: OFFICIAL_MIRROR.to_string(),
            user_id: user_id.to_string(),
            cookies: self.cookie_jar.snapshot(),
        };
        let secret = serde_json::to_vec(&secret)
            .map_err(|_| "Failed to serialize the verified session".to_string())?;
        let entry = Self::keyring_entry(user_id)?;
        entry
            .set_secret(&secret)
            .map_err(|_| "System keyring is unavailable; session remains in memory".to_string())
    }

    #[cfg(target_os = "android")]
    pub fn persist_session(&self, _user_id: &str) -> Result<(), String> {
        Err("Android stores sessions in the platform keystore".to_string())
    }

    #[cfg(not(target_os = "android"))]
    pub fn restore_session(&self, user_id: &str) -> Result<bool, String> {
        self.cookie_jar.clear();
        let entry = Self::keyring_entry(user_id)?;
        let secret = match entry.get_secret() {
            Ok(secret) => secret,
            Err(keyring::Error::NoEntry) => return Ok(false),
            Err(_) => {
                return Err("System keyring is unavailable; session was not restored".to_string())
            }
        };
        let secret: SessionSecret = match serde_json::from_slice(&secret) {
            Ok(secret) => secret,
            Err(_) => {
                let _ = entry.delete_credential();
                return Err("Stored session is invalid".to_string());
            }
        };
        if secret.version != SESSION_VERSION
            || secret.provider != OFFICIAL_MIRROR
            || secret.user_id != user_id
        {
            let _ = entry.delete_credential();
            return Err("Stored session does not match the requested account".to_string());
        }
        self.cookie_jar.replace(&secret.cookies);
        if self.authenticated_user_id().as_deref() == Some(user_id) {
            Ok(true)
        } else {
            self.cookie_jar.clear();
            let _ = entry.delete_credential();
            Ok(false)
        }
    }

    #[cfg(target_os = "android")]
    pub fn restore_session(&self, _user_id: &str) -> Result<bool, String> {
        Ok(false)
    }

    #[cfg(not(target_os = "android"))]
    pub fn clear_session(&self, user_id: Option<&str>) -> Result<(), String> {
        self.invalidate_auth();
        let legacy_removed = Self::remove_legacy_cookie_file().is_ok();
        let keyring_removed = match user_id {
            None => true,
            Some(user_id) => Self::keyring_entry(user_id).is_ok_and(|entry| {
                matches!(
                    entry.delete_credential(),
                    Ok(()) | Err(keyring::Error::NoEntry)
                )
            }),
        };
        match (legacy_removed, keyring_removed) {
            (true, true) => Ok(()),
            (false, true) => Err("Legacy session file could not be removed".to_string()),
            (true, false) => {
                Err("System keyring is unavailable; stored session was not removed".to_string())
            }
            (false, false) => Err(
                "Stored session could not be fully removed from disk or the system keyring"
                    .to_string(),
            ),
        }
    }

    #[cfg(target_os = "android")]
    pub fn clear_session(&self, _user_id: Option<&str>) -> Result<(), String> {
        self.invalidate_auth();
        Ok(())
    }

    #[cfg(not(target_os = "android"))]
    fn keyring_entry(user_id: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, &format!("hdrzk.org:{user_id}"))
            .map_err(|_| "System keyring is unavailable".to_string())
    }

    pub fn load_legacy_session(
        &self,
        expected_user_id: Option<&str>,
    ) -> Result<Option<String>, String> {
        let path = Self::cookie_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                let _ = Self::remove_legacy_cookie_file();
                return Err("Legacy session could not be read".to_string());
            }
        };
        let url = Url::parse(&self.base_url)
            .map_err(|_| "Official provider URL is invalid".to_string())?;
        let host = url
            .host_str()
            .ok_or_else(|| "Official provider URL has no host".to_string())?;
        let persisted = Self::parse_host_cookies(&content, Some(host));
        let Some(cookies) = persisted.hosts.get(host) else {
            let _ = Self::remove_legacy_cookie_file();
            return Err("Legacy session is invalid".to_string());
        };
        let cookies = cookies
            .iter()
            .map(|(name, value)| StoredCookie {
                url: url.as_str().to_string(),
                header: format!("{name}={value}; Domain={host}; Path=/; Secure; HttpOnly"),
            })
            .collect::<Vec<_>>();
        self.cookie_jar.replace(&cookies);
        let Some(user_id) = self.authenticated_user_id() else {
            self.cookie_jar.clear();
            let _ = Self::remove_legacy_cookie_file();
            return Err("Legacy session is incomplete".to_string());
        };
        if expected_user_id.is_some_and(|expected| expected != user_id) {
            self.cookie_jar.clear();
            let _ = Self::remove_legacy_cookie_file();
            return Err("Legacy session belongs to another account".to_string());
        }
        Ok(Some(user_id))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn resolve_url(&self, value: &str) -> String {
        if value.starts_with("http://") || value.starts_with("https://") {
            value.to_string()
        } else if value.starts_with("//") {
            format!("https:{value}")
        } else {
            format!(
                "{}/{}",
                self.base_url.trim_end_matches('/'),
                value.trim_start_matches('/')
            )
        }
    }

    pub fn user_agent(&self) -> &'static str {
        DEFAULT_USER_AGENT
    }

    pub fn referer(&self) -> &str {
        &self.base_url
    }

    fn parse_host_cookies(content: &str, legacy_host: Option<&str>) -> HostCookies {
        if let Ok(hosts) = serde_json::from_str::<HostCookies>(content) {
            return hosts;
        }
        let mut hosts = HostCookies::default();
        if let (Ok(legacy), Some(host)) = (
            serde_json::from_str::<PersistedCookies>(content),
            legacy_host,
        ) {
            hosts.hosts.insert(host.to_string(), legacy.cookies);
        }
        hosts
    }

    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7"),
        );
        headers.insert("X-Hdrezka-Android-App", HeaderValue::from_static("1"));
        headers.insert(
            "X-Hdrezka-Android-App-Version",
            HeaderValue::from_static("2.2.1"),
        );
        headers
    }

    /// True if the jar already holds a non-empty, non-verification `anubis`
    /// clearance cookie for `url`'s host.
    fn has_anubis_clearance(&self, url: &str) -> bool {
        let Ok(parsed) = Url::parse(url) else {
            return false;
        };
        let Some(header_val) = self.cookie_jar.cookies(&parsed) else {
            return false;
        };
        let Ok(header_str) = header_val.to_str() else {
            return false;
        };
        header_str.split(';').any(|pair| {
            let pair = pair.trim();
            let Some((name, value)) = pair.split_once('=') else {
                return false;
            };
            let name = name.trim();
            name.contains("anubis") && !name.contains("verification") && !value.trim().is_empty()
        })
    }

    fn capture_host_cookies(&self, response: &reqwest::Response, request_url: &str) {
        let Ok(url) = Url::parse(request_url) else {
            return;
        };
        for header in response.headers().get_all(reqwest::header::SET_COOKIE) {
            let Ok(cookie) = header.to_str() else {
                continue;
            };
            self.cookie_jar.add_cookie_str(cookie, &url);
        }
    }

    /// Solves the Anubis challenge embedded in `body` for `full_url` and stores
    /// the clearance cookie. Serialized per host; short-circuits when a
    /// clearance cookie is already present.
    async fn solve_anubis_challenge(&self, body: &str, full_url: &str) -> Result<(), String> {
        if self.has_anubis_clearance(full_url) {
            return Ok(());
        }

        let host = Url::parse(full_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_default();

        let map = ANUBIS_INFLIGHT.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()));
        let host_lock = {
            let mut guard = map.lock().await;
            guard
                .entry(host.clone())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        let _guard = host_lock.lock().await;

        // Re-check under the lock: another task may have solved it meanwhile.
        if self.has_anubis_clearance(full_url) {
            return Ok(());
        }

        log::info!("Anubis challenge encountered for {}, solving...", full_url);
        let solution = AnubisSolver::extract_and_solve(&self.base_url, body, full_url).await?;
        self.pass_anubis(&solution, full_url).await
    }

    /// Sends the pass-challenge request. Redirects are NOT followed, because
    /// following the 302 would hide the Set-Cookie header. Only the
    /// verification cookie is echoed; any clearance Set-Cookie is captured and
    /// stored in the jar manually.
    async fn pass_anubis(&self, solution: &AnubisSolution, full_url: &str) -> Result<(), String> {
        let mut pass_req = self
            .pass_client
            .get(&solution.pass_url)
            .query(&[
                ("id", solution.challenge_id.as_str()),
                ("redir", &solution.redir_url),
                ("elapsedTime", &solution.elapsed_ms.to_string()),
            ])
            .header(REFERER, full_url);
        if !solution.hash.is_empty() {
            pass_req = pass_req.query(&[
                ("response", solution.hash.as_str()),
                ("nonce", &solution.nonce.to_string()),
            ]);
        }
        // Echo ONLY the bare verification cookie and no others; Domain/Path
        // attributes do not belong in the Cookie header value.
        if let Ok(url) = Url::parse(full_url) {
            if let Some(host) = url.host_str() {
                let _ = host;
                pass_req = pass_req.header(
                    reqwest::header::COOKIE,
                    format!(
                        "{}={}",
                        super::anubis::VERIFICATION_COOKIE,
                        solution.challenge_id
                    ),
                );
            }
        }

        let resp = pass_req
            .send()
            .await
            .map_err(|e| format!("Failed to send Anubis pass challenge: {}", e.without_url()))?;

        // Store any clearance cookies set on the (non-redirected) response.
        let cookie_url = Url::parse(&solution.pass_url)
            .or_else(|_| Url::parse(full_url))
            .unwrap_or_else(|_| Url::parse("https://localhost").expect("static url"));
        for header in resp.headers().get_all(reqwest::header::SET_COOKIE) {
            if let Ok(val) = header.to_str() {
                self.cookie_jar.add_cookie_str(val, &cookie_url);
            }
        }
        if !self.has_anubis_clearance(full_url) {
            return Err(format!(
                "Anubis clearance cookie not obtained after solving for {}",
                full_url
            ));
        }
        Ok(())
    }

    pub async fn get_html(&self, target_url: &str) -> Result<String, String> {
        let full_url = if target_url.starts_with("http://") || target_url.starts_with("https://") {
            let parsed = Url::parse(target_url)
                .map_err(|error| format!("Invalid provider URL {}: {}", target_url, error))?;
            format!(
                "{}{}{}",
                self.base_url,
                parsed.path(),
                parsed
                    .query()
                    .map(|query| format!("?{}", query))
                    .unwrap_or_default()
            )
        } else {
            format!("{}/{}", self.base_url, target_url.trim_start_matches('/'))
        };

        let mut last_err = String::new();
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }

            match self.execute_get(&full_url).await {
                Ok(html) => return Ok(html),
                Err(e) => {
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    async fn execute_get(&self, full_url: &str) -> Result<String, String> {
        let resp = self
            .client
            .get(full_url)
            .header(REFERER, &self.base_url)
            .send()
            .await
            .map_err(|e| format!("HTTP GET request failed: {}", e.without_url()))?;
        self.capture_host_cookies(&resp, full_url);
        let (status, content_type, html) = Self::read_response(resp).await?;

        if AnubisSolver::is_challenge(&html) {
            log::info!(
                "Anubis challenge encountered on GET {}, solving PoW...",
                full_url
            );
            self.solve_anubis_challenge(&html, full_url).await?;

            // Re-fetch the target URL now that the clearance cookie is stored
            let final_resp = self
                .client
                .get(full_url)
                .header(REFERER, &self.base_url)
                .send()
                .await
                .map_err(|e| format!("Post-Anubis GET request failed: {}", e.without_url()))?;
            self.capture_host_cookies(&final_resp, full_url);
            let (status, content_type, final_html) = Self::read_response(final_resp).await?;
            return Self::require_success("GET", status, &content_type, final_html);
        }

        Self::require_success("GET", status, &content_type, html)
    }

    pub async fn post_ajax(
        &self,
        endpoint: &str,
        form_data: &[(&str, &str)],
    ) -> Result<String, String> {
        let mut full_url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'))
        };
        let separator = if full_url.contains('?') { '&' } else { '?' };
        full_url.push(separator);
        full_url.push_str(&format!("t={}", chrono::Utc::now().timestamp_millis()));

        let mut last_err = String::new();
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }

            match self.execute_post(&full_url, form_data).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    async fn execute_post(
        &self,
        full_url: &str,
        form_data: &[(&str, &str)],
    ) -> Result<String, String> {
        let request = self
            .client
            .post(full_url)
            .header(REFERER, &self.base_url)
            .multipart(Self::multipart_form(form_data));
        let resp = request
            .send()
            .await
            .map_err(|e| format!("HTTP POST request failed: {}", e.without_url()))?;
        self.capture_host_cookies(&resp, full_url);
        let (status, content_type, body) = Self::read_response(resp).await?;

        if AnubisSolver::is_challenge(&body) {
            log::info!(
                "Anubis challenge encountered on POST {}, solving PoW...",
                full_url
            );
            self.solve_anubis_challenge(&body, full_url).await?;

            // Retry original post after passing PoW
            let retry_request = self
                .client
                .post(full_url)
                .header(REFERER, &self.base_url)
                .multipart(Self::multipart_form(form_data));
            let retry_resp = retry_request
                .send()
                .await
                .map_err(|e| format!("Post-Anubis POST request failed: {}", e.without_url()))?;
            self.capture_host_cookies(&retry_resp, full_url);
            let (status, content_type, body) = Self::read_response(retry_resp).await?;
            return Self::require_success("POST", status, &content_type, body);
        }

        Self::require_success("POST", status, &content_type, body)
    }

    async fn read_response(
        response: reqwest::Response,
    ) -> Result<(reqwest::StatusCode, String, String), String> {
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .chars()
            .take(80)
            .collect();
        let body = response
            .text()
            .await
            .map_err(|error| format!("Failed to read HTTP response: {}", error.without_url()))?;
        Ok((status, content_type, body))
    }

    fn require_success(
        operation: &str,
        status: reqwest::StatusCode,
        content_type: &str,
        body: String,
    ) -> Result<String, String> {
        if status.is_success() {
            return Ok(body);
        }
        let kind = if body.trim().is_empty() {
            "empty"
        } else if content_type.contains("json") || body.trim_start().starts_with(['{', '[']) {
            "JSON"
        } else if content_type.contains("html") || body.trim_start().starts_with('<') {
            "HTML"
        } else {
            "text"
        };
        Err(format!(
            "{operation} failed: HTTP {}; content-type {content_type}; {kind} body ({} bytes)",
            status.as_u16(),
            body.len()
        ))
    }

    fn multipart_form(form_data: &[(&str, &str)]) -> reqwest::multipart::Form {
        form_data
            .iter()
            .fold(reqwest::multipart::Form::new(), |form, (name, value)| {
                form.text((*name).to_string(), (*value).to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_provider_and_headers_match_the_site() {
        let session = RezkaSession::new();
        assert_eq!(session.base_url(), OFFICIAL_MIRROR);
        let headers = RezkaSession::default_headers();
        assert_eq!(headers["X-Hdrezka-Android-App"], "1");
        assert_eq!(headers["X-Hdrezka-Android-App-Version"], "2.2.1");
    }

    #[test]
    fn legacy_cookie_file_migrates_to_current_host() {
        let parsed = RezkaSession::parse_host_cookies(
            r#"{"cookies":{"dle_user_id":"42","PHPSESSID":"abc"}}"#,
            Some("hdrzk.org"),
        );
        assert_eq!(parsed.hosts["hdrzk.org"]["dle_user_id"], "42");
        assert_eq!(parsed.hosts["hdrzk.org"]["PHPSESSID"], "abc");
    }

    #[test]
    fn host_cookie_file_keeps_hosts_separate() {
        let parsed = RezkaSession::parse_host_cookies(
            r#"{"hosts":{"one.example":{"a":"1"},"two.example":{"b":"2"}}}"#,
            Some("ignored.example"),
        );
        assert_eq!(parsed.hosts["one.example"]["a"], "1");
        assert_eq!(parsed.hosts["two.example"]["b"], "2");
    }

    #[test]
    fn session_secret_preserves_cookie_metadata() {
        let url = Url::parse(OFFICIAL_MIRROR).unwrap();
        let jar = SharedCookieJar::default();
        jar.add_cookie_str(
            "dle_user_id=42; Domain=hdrzk.org; Path=/user; Expires=Wed, 21 Oct 2037 07:28:00 GMT; Secure; HttpOnly",
            &url,
        );

        let cookies = jar.snapshot();
        assert_eq!(cookies.len(), 1);
        assert!(cookies[0]
            .header
            .contains("Expires=Wed, 21 Oct 2037 07:28:00 GMT"));
        assert!(cookies[0].header.contains("Path=/user"));
        assert!(cookies[0].header.contains("Domain=hdrzk.org"));
        assert!(cookies[0].header.contains("Secure"));
        assert!(cookies[0].header.contains("HttpOnly"));

        let serialized = serde_json::to_vec(&SessionSecret {
            version: SESSION_VERSION,
            provider: OFFICIAL_MIRROR.to_string(),
            user_id: "42".to_string(),
            cookies,
        })
        .unwrap();
        let restored: SessionSecret = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(restored.cookies.len(), 1);
        assert!(restored.cookies[0].header.contains("HttpOnly"));
    }

    #[test]
    fn clearing_shared_jar_logs_out_every_clone() {
        let url = Url::parse(OFFICIAL_MIRROR).unwrap();
        let jar = Arc::new(SharedCookieJar::default());
        let clone = jar.clone();
        jar.add_cookie_str("dle_user_id=42; Domain=hdrzk.org; Path=/", &url);
        jar.add_cookie_str("dle_password=hash; Domain=hdrzk.org; Path=/", &url);
        assert!(clone.cookies(&url).is_some());

        jar.clear();

        assert!(jar.cookies(&url).is_none());
        assert!(clone.cookies(&url).is_none());
        assert!(jar.snapshot().is_empty());
    }

    #[test]
    fn clearing_session_invalidates_every_clone() {
        let session = RezkaSession::new();
        let clone = session.clone();
        let generation = session.auth_epoch();

        session.invalidate_auth();

        assert_eq!(session.auth_epoch(), generation + 1);
        assert_eq!(clone.auth_epoch(), generation + 1);
    }

    #[test]
    fn exported_session_restores_authenticated_cookies() {
        let original = RezkaSession::new_for_test("https://example.com");
        original.authenticate_for_test("42");
        let secret = original.export_session().unwrap();
        let restored = RezkaSession::new_for_test("https://example.com");

        assert_eq!(restored.import_session(&secret).unwrap(), "42");
        assert_eq!(restored.authenticated_user_id().as_deref(), Some("42"));
    }

    #[test]
    fn http_error_description_is_sanitized() {
        let error = RezkaSession::require_success(
            "POST",
            reqwest::StatusCode::UNAUTHORIZED,
            "application/json",
            r#"{"token":"do-not-leak"}"#.to_string(),
        )
        .unwrap_err();

        assert!(error.contains("HTTP 401"));
        assert!(error.contains("content-type application/json"));
        assert!(error.contains("JSON body"));
        assert!(!error.contains("do-not-leak"));
    }
}
