use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct PersistedCookies {
    pub cookies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct HostCookies {
    pub(super) hosts: HashMap<String, HashMap<String, String>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct StoredCookie {
    pub(super) url: String,
    pub(super) header: String,
}

#[derive(Default)]
pub(super) struct SharedCookieJar {
    state: RwLock<CookieState>,
}

#[derive(Default)]
struct CookieState {
    jar: Arc<Jar>,
    stored: Vec<StoredCookie>,
}

impl SharedCookieJar {
    pub(super) fn add_cookie_str(&self, header: &str, url: &Url) {
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

    pub(super) fn replace(&self, cookies: &[StoredCookie]) {
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

    pub(super) fn clear(&self) {
        self.replace(&[]);
    }

    pub(super) fn snapshot(&self) -> Vec<StoredCookie> {
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

#[cfg(test)]
mod tests {
    use super::super::OFFICIAL_MIRROR;
    use super::*;

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
}
