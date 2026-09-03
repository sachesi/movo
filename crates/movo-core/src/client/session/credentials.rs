use super::cookies::{HostCookies, PersistedCookies, StoredCookie};
use super::{RezkaSession, OFFICIAL_MIRROR};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use url::Url;

#[cfg(not(target_os = "android"))]
const KEYRING_SERVICE: &str = "org.gnome.Movo";
const SESSION_VERSION: u8 = 1;

#[derive(Serialize, Deserialize)]
pub(crate) struct SessionSecret {
    version: u8,
    provider: String,
    user_id: String,
    cookies: Vec<StoredCookie>,
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
}

#[cfg(test)]
mod tests {
    use super::super::cookies::SharedCookieJar;
    use super::*;

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
    fn exported_session_restores_authenticated_cookies() {
        let original = RezkaSession::new_for_test("https://example.com");
        original.authenticate_for_test("42");
        let secret = original.export_session().unwrap();
        let restored = RezkaSession::new_for_test("https://example.com");

        assert_eq!(restored.import_session(&secret).unwrap(), "42");
        assert_eq!(restored.authenticated_user_id().as_deref(), Some("42"));
    }
}
