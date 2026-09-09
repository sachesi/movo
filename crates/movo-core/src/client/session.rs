use super::anubis;
use super::anubis::AnubisSolution;
use crate::error::{ClientError, ErrorKind};
use reqwest::cookie::CookieStore;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use url::Url;

mod cookies;
mod credentials;

use cookies::SharedCookieJar;

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 15; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
pub const OFFICIAL_MIRROR: &str = "https://hdrzk.org";

/// First retry waits this long; each further attempt doubles it.
const RETRY_BASE_DELAY: Duration = Duration::from_millis(300);

/// Upper bound of the random-ish spread added to every retry delay.
const RETRY_SPREAD_MS: u64 = 100;

/// Ceiling on a single response body. The largest page the provider serves is
/// a detail page well under a megabyte; this only stops a broken or hostile
/// response from being read into memory without limit.
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

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
        Self::new_for_base(OFFICIAL_MIRROR, Duration::from_secs(25))
    }

    #[cfg(test)]
    pub(super) fn new_for_test(base_url: &str) -> Self {
        Self::new_for_base(base_url, Duration::from_secs(25))
    }

    /// A session whose request timeout is short enough to exercise in a test,
    /// against a server that never answers.
    #[cfg(test)]
    pub(super) fn new_for_test_with_timeout(base_url: &str, timeout: Duration) -> Self {
        Self::new_for_base(base_url, timeout)
    }

    #[cfg(test)]
    pub(super) fn authenticate_for_test(&self, user_id: &str) {
        let url = Url::parse(&self.base_url).unwrap();
        self.cookie_jar
            .add_cookie_str(&format!("dle_user_id={user_id}; Path=/"), &url);
        self.cookie_jar
            .add_cookie_str("dle_password=hash; Path=/", &url);
    }

    fn new_for_base(base_url: &str, timeout: Duration) -> Self {
        let cookie_jar = Arc::new(SharedCookieJar::default());
        let base = base_url.trim_end_matches('/').to_string();

        let headers = Self::default_headers();

        let client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .default_headers(headers.clone())
            .timeout(timeout)
            .gzip(true)
            .brotli(true)
            .build()
            .expect("Failed to build reqwest HTTP client");

        let pass_client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .default_headers(headers)
            .timeout(timeout)
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
        let solution = anubis::extract_and_solve(&self.base_url, body, full_url).await?;
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

    pub async fn get_html(&self, target_url: &str) -> Result<String, ClientError> {
        let full_url = if target_url.starts_with("http://") || target_url.starts_with("https://") {
            let parsed = Url::parse(target_url).map_err(|error| {
                ClientError::new(
                    ErrorKind::Other,
                    format!("Invalid provider URL {}: {}", target_url, error),
                )
            })?;
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

        let mut last_err = None;
        for attempt in 0..3 {
            Self::back_off(attempt).await;

            match self.execute_get(&full_url).await {
                Ok(html) => return Ok(html),
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ClientError::new(ErrorKind::Network, "GET request failed")))
    }

    /// Turns a transport-level failure into a [`ClientError`] with an explicit
    /// kind instead of leaving it to be read back out of the message later:
    /// a timed-out request is [`ErrorKind::Timeout`], anything else that never
    /// reached the provider or never got a reply back is [`ErrorKind::Network`].
    fn transport_error(context: &str, error: reqwest::Error) -> ClientError {
        let kind = if error.is_timeout() {
            ErrorKind::Timeout
        } else {
            ErrorKind::Network
        };
        ClientError::new(kind, format!("{context}: {}", error.without_url()))
    }

    async fn execute_get(&self, full_url: &str) -> Result<String, ClientError> {
        let resp = self
            .client
            .get(full_url)
            .header(REFERER, &self.base_url)
            .send()
            .await
            .map_err(|e| Self::transport_error("HTTP GET request failed", e))?;
        self.capture_host_cookies(&resp, full_url);
        let (status, content_type, html) = Self::read_response(resp).await?;
        log::debug!(
            "GET {full_url} -> {} {content_type} ({} bytes)",
            status.as_u16(),
            html.len()
        );

        if anubis::is_challenge(&html) {
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
                .map_err(|e| Self::transport_error("Post-Anubis GET request failed", e))?;
            self.capture_host_cookies(&final_resp, full_url);
            let (status, content_type, final_html) = Self::read_response(final_resp).await?;
            return Self::require_success("GET", status, &content_type, final_html);
        }

        Self::require_success("GET", status, &content_type, html)
    }

    fn ajax_url(&self, endpoint: &str) -> String {
        let mut full_url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'))
        };
        let separator = if full_url.contains('?') { '&' } else { '?' };
        full_url.push(separator);
        full_url.push_str(&format!("t={}", chrono::Utc::now().timestamp_millis()));
        full_url
    }

    /// Posts to an endpoint that can be repeated safely, retrying on failure.
    ///
    /// Only for requests whose effect does not depend on how many times the
    /// provider ran them: reads shaped as posts, and upserts. Anything that
    /// toggles or accumulates must use [`post_ajax_once`](Self::post_ajax_once).
    pub async fn post_ajax(
        &self,
        endpoint: &str,
        form_data: &[(&str, &str)],
    ) -> Result<String, ClientError> {
        let full_url = self.ajax_url(endpoint);

        let mut last_err = None;
        for attempt in 0..3 {
            Self::back_off(attempt).await;

            match self.execute_post(&full_url, form_data).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ClientError::new(ErrorKind::Network, "POST request failed")))
    }

    /// Posts once, with no retry.
    ///
    /// The provider's toggle endpoints flip a flag rather than set it, and its
    /// rating endpoint records a vote. A request that reached the server and
    /// then failed on the way back — a 5xx from a proxy, a dropped connection
    /// mid-body — has already been applied, so a second attempt would flip the
    /// flag back or vote twice. Reporting the failure is the lesser harm.
    pub async fn post_ajax_once(
        &self,
        endpoint: &str,
        form_data: &[(&str, &str)],
    ) -> Result<String, ClientError> {
        self.execute_post(&self.ajax_url(endpoint), form_data).await
    }

    fn post_request(
        &self,
        full_url: &str,
        form_data: &[(&str, &str)],
    ) -> Result<reqwest::Request, ClientError> {
        self.client
            .post(full_url)
            .multipart(Self::multipart_form(form_data))
            .header(REFERER, &self.base_url)
            .build()
            .map_err(|e| Self::transport_error("HTTP POST request failed", e))
    }

    async fn execute_post(
        &self,
        full_url: &str,
        form_data: &[(&str, &str)],
    ) -> Result<String, ClientError> {
        let request = self.post_request(full_url, form_data)?;
        let resp = self
            .client
            .execute(request)
            .await
            .map_err(|e| Self::transport_error("HTTP POST request failed", e))?;
        self.capture_host_cookies(&resp, full_url);
        let (status, content_type, body) = Self::read_response(resp).await?;
        log::debug!(
            "POST {full_url} -> {} {content_type} ({} bytes): {}",
            status.as_u16(),
            body.len(),
            body.chars()
                .take(300)
                .collect::<String>()
                .replace('\n', " ")
        );

        if anubis::is_challenge(&body) {
            log::info!(
                "Anubis challenge encountered on POST {}, solving PoW...",
                full_url
            );
            self.solve_anubis_challenge(&body, full_url).await?;

            // Retry original post after passing PoW
            let retry_request = self.post_request(full_url, form_data)?;
            let retry_resp = self
                .client
                .execute(retry_request)
                .await
                .map_err(|e| Self::transport_error("Post-Anubis POST request failed", e))?;
            self.capture_host_cookies(&retry_resp, full_url);
            let (status, content_type, body) = Self::read_response(retry_resp).await?;
            return Self::require_success("POST", status, &content_type, body);
        }

        Self::require_success("POST", status, &content_type, body)
    }

    /// Waits before retry `attempt`, doubling each time and adding a little
    /// spread so that requests failing together do not retry in lockstep.
    /// Returns immediately for the first attempt.
    async fn back_off(attempt: u32) {
        if attempt == 0 {
            return;
        }
        let base = RETRY_BASE_DELAY * 2u32.pow(attempt - 1);
        tokio::time::sleep(base + Duration::from_millis(Self::retry_spread_ms())).await;
    }

    /// A 0..RETRY_SPREAD_MS value taken from the clock. The spread only has to
    /// break ties between concurrent callers, so the wall clock is enough and
    /// saves pulling in a random number generator.
    fn retry_spread_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.subsec_nanos() as u64 % RETRY_SPREAD_MS)
            .unwrap_or(0)
    }

    async fn read_response(
        response: reqwest::Response,
    ) -> Result<(reqwest::StatusCode, String, String), ClientError> {
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .chars()
            .take(80)
            .collect();
        let mut body = Vec::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| Self::transport_error("Failed to read HTTP response", error))?
        {
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(ClientError::new(
                    ErrorKind::Network,
                    format!("HTTP response exceeded {} bytes", MAX_RESPONSE_BYTES),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        // Matches what `Response::text` does: the provider labels pages UTF-8
        // and a stray byte should not lose the whole page.
        let body = String::from_utf8_lossy(&body).into_owned();
        Ok((status, content_type, body))
    }

    fn require_success(
        operation: &str,
        status: reqwest::StatusCode,
        content_type: &str,
        body: String,
    ) -> Result<String, ClientError> {
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
        Err(ClientError::new(
            ErrorKind::Provider,
            format!(
                "{operation} failed: HTTP {}; content-type {content_type}; {kind} body ({} bytes)",
                status.as_u16(),
                body.len()
            ),
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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    /// Reads one request off `stream` and answers with `status`, so the caller
    /// sees a server that replied but did not succeed.
    fn refuse(stream: &mut TcpStream, status: &str) {
        let mut buffer = [0; 4096];
        let _ = stream.read(&mut buffer);
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        );
    }

    /// Accepts exactly `count` requests, refusing each, and returns how many
    /// actually arrived.
    fn refusing_server(listener: TcpListener, count: usize) -> std::thread::JoinHandle<usize> {
        std::thread::spawn(move || {
            let mut seen = 0;
            for _ in 0..count {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                seen += 1;
                refuse(&mut stream, "502 Bad Gateway");
            }
            seen
        })
    }

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

    /// A toggle the provider already applied must not be sent again just
    /// because the reply was lost; a second POST would flip it back.
    #[tokio::test]
    async fn a_failed_toggle_post_is_not_repeated() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = refusing_server(listener, 1);

        let session = RezkaSession::new_for_test(&base_url);
        let error = session
            .post_ajax_once("engine/ajax/cdn_saves_view.php", &[("id", "7")])
            .await
            .unwrap_err();

        assert!(error.message.contains("HTTP 502"), "{error}");
        assert_eq!(error.kind, crate::error::ErrorKind::Provider);
        assert_eq!(server.join().unwrap(), 1);
    }

    /// Repeatable posts keep their retries: the provider drops requests often
    /// enough that a single attempt would show avoidable failures.
    #[tokio::test]
    async fn a_repeatable_post_is_retried_three_times() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = refusing_server(listener, 3);

        let session = RezkaSession::new_for_test(&base_url);
        let error = session
            .post_ajax("ajax/get_cdn_series/", &[("id", "7")])
            .await
            .unwrap_err();

        assert!(error.message.contains("HTTP 502"), "{error}");
        assert_eq!(server.join().unwrap(), 3);
    }

    /// A request that outlives the client's timeout is reported with
    /// `ErrorKind::Timeout` directly, not by reading the message back through
    /// `classify`.
    #[tokio::test]
    async fn a_request_timeout_carries_the_timeout_kind() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        // Accept the connection and never answer it, so the client's own
        // timeout is what ends the request.
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(1500));
            drop(stream);
        });

        let session = RezkaSession::new_for_test_with_timeout(&base_url, Duration::from_millis(50));
        let error = session.get_html("/").await.unwrap_err();

        assert_eq!(error.kind, crate::error::ErrorKind::Timeout);
        server.join().unwrap();
    }

    #[test]
    fn retry_delays_grow_and_stay_within_their_spread() {
        assert!(RezkaSession::retry_spread_ms() < RETRY_SPREAD_MS);
        assert_eq!(RETRY_BASE_DELAY * 2u32.pow(1), Duration::from_millis(600));
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

        assert!(error.message.contains("HTTP 401"));
        assert!(error.message.contains("content-type application/json"));
        assert!(error.message.contains("JSON body"));
        assert!(!error.message.contains("do-not-leak"));
        assert_eq!(error.kind, crate::error::ErrorKind::Provider);
    }
}
