use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::session::OFFICIAL_MIRROR;

pub const DEFAULT_CDNS: &[&str] = &[
    "https://prx-cogent.ukrtelcdn.net",
    "https://prx2-cogent.ukrtelcdn.net",
    "https://prx3-cogent.ukrtelcdn.net",
    "https://prx4-cogent.ukrtelcdn.net",
    "https://prx5-cogent.ukrtelcdn.net",
    "https://prx6-cogent.ukrtelcdn.net",
    "https://prx-ams.ukrtelcdn.net",
    "https://stream.voidboost.cc",
    "https://stream.voidboost.top",
    "https://stream.voidboost.link",
    "https://stream.voidboost.club",
];

#[derive(Clone)]
pub struct CdnManager {
    active_cdn: Arc<RwLock<String>>,
    auto_select: Arc<RwLock<bool>>,
    http_client: Client,
}

impl Default for CdnManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CdnManager {
    fn benchmark_reachable(status: reqwest::StatusCode) -> bool {
        status.is_success() || status.is_redirection() || matches!(status.as_u16(), 401 | 403 | 404)
    }

    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        Self {
            active_cdn: Arc::new(RwLock::new(String::new())),
            auto_select: Arc::new(RwLock::new(true)),
            http_client: client,
        }
    }

    pub fn available_cdns() -> Vec<&'static str> {
        DEFAULT_CDNS.to_vec()
    }

    pub async fn get_active_cdn(&self) -> String {
        self.active_cdn.read().await.clone()
    }

    pub async fn set_active_cdn(&self, cdn: &str) {
        let trimmed = cdn.trim().trim_end_matches('/');
        if trimmed.is_empty() || trimmed == "auto" {
            *self.active_cdn.write().await = String::new();
            *self.auto_select.write().await = true;
        } else {
            *self.active_cdn.write().await = trimmed.to_string();
            *self.auto_select.write().await = false;
        }
    }

    pub async fn is_auto_select(&self) -> bool {
        *self.auto_select.read().await
    }

    pub async fn set_auto_select(&self, auto: bool) {
        *self.auto_select.write().await = auto;
        if auto {
            *self.active_cdn.write().await = String::new();
        }
    }

    /// Replaces the scheme and host of a given video URL with the active CDN
    /// when auto_select is disabled.
    pub async fn modify_url(&self, original_url: &str) -> String {
        if *self.auto_select.read().await {
            return original_url.to_string();
        }

        let active = self.get_active_cdn().await;
        if active.is_empty() || active == "auto" {
            return original_url.to_string();
        }

        if let Ok(mut parsed) = url::Url::parse(original_url) {
            if let Ok(cdn_url) = url::Url::parse(&active) {
                let _ = parsed.set_scheme(cdn_url.scheme());
                let _ = parsed.set_host(cdn_url.host_str());
                let _ = parsed.set_port(cdn_url.port());
                let candidate = parsed.to_string();
                let valid = self
                    .http_client
                    .get(&candidate)
                    .header(reqwest::header::RANGE, "bytes=0-0")
                    .send()
                    .await
                    .map(|response| {
                        response.status().is_success()
                            || response.status() == reqwest::StatusCode::PARTIAL_CONTENT
                    })
                    .unwrap_or(false);
                if valid {
                    return candidate;
                }
            }
        }
        original_url.to_string()
    }

    /// Returns the primary URL followed by alternative candidate URLs swapping the CDN host
    pub fn fallback_urls_for(original_url: &str) -> Vec<String> {
        let Ok(parsed) = url::Url::parse(original_url) else {
            return vec![original_url.to_string()];
        };

        let orig_host = parsed.host_str().unwrap_or("");
        let mut urls = vec![original_url.to_string()];

        for &cdn in DEFAULT_CDNS {
            if let Ok(cdn_u) = url::Url::parse(cdn) {
                if let Some(cdn_host) = cdn_u.host_str() {
                    if cdn_host != orig_host {
                        let mut fallback = parsed.clone();
                        let _ = fallback.set_scheme(cdn_u.scheme());
                        let _ = fallback.set_host(Some(cdn_host));
                        let _ = fallback.set_port(cdn_u.port());
                        urls.push(fallback.to_string());
                    }
                }
            }
        }

        urls
    }

    /// Tests all candidate CDNs concurrently and returns their latencies in ms
    pub async fn benchmark_all(&self) -> Vec<(String, Option<u64>)> {
        let mut tasks = Vec::new();
        for &cdn in DEFAULT_CDNS {
            let client = self.http_client.clone();
            tasks.push(tokio::spawn(async move {
                let start = Instant::now();
                let res = client
                    .get(cdn)
                    .header(
                        "User-Agent",
                        "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
                    )
                    .header("Referer", OFFICIAL_MIRROR)
                    .header("Range", "bytes=0-0")
                    .send()
                    .await;

                match res {
                    Ok(resp) => {
                        let st = resp.status();
                        if Self::benchmark_reachable(st) {
                            let ms = start.elapsed().as_millis() as u64;
                            (cdn.to_string(), Some(ms))
                        } else {
                            (cdn.to_string(), None)
                        }
                    }
                    _ => (cdn.to_string(), None),
                }
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(res) = task.await {
                results.push(res);
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_challenge_still_proves_cdn_reachability() {
        assert!(CdnManager::benchmark_reachable(
            reqwest::StatusCode::UNAUTHORIZED
        ));
        assert!(!CdnManager::benchmark_reachable(
            reqwest::StatusCode::BAD_GATEWAY
        ));
    }
}
