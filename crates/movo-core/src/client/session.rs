use reqwest::cookie::CookieStore;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use url::Url;

use super::anubis::{AnubisSolution, AnubisSolver};

mod cookies;
mod credentials;

use cookies::SharedCookieJar;

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 15; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
pub const OFFICIAL_MIRROR: &str = "https://hdrzk.org";

/// Per-host in-flight Anubis solves: one solve at a time per host so
/// concurrent challenges can't clobber each other's verification cookie in
/// the shared jar.
static ANUBIS_INFLIGHT: OnceLock<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();

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
    fn clearing_session_invalidates_every_clone() {
        let session = RezkaSession::new();
        let clone = session.clone();
        let generation = session.auth_epoch();

        session.invalidate_auth();

        assert_eq!(session.auth_epoch(), generation + 1);
        assert_eq!(clone.auth_epoch(), generation + 1);
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
