use super::catalog::CatalogScraper;
use super::models::{
    FavoritesCollection, MediaItem, NotificationGroup, NotificationItem, ServerHistoryEntry,
    UserProfile,
};
use super::session::RezkaSession;
use scraper::{Html, Selector};
use serde_json::Value;

pub struct AuthManager;

impl AuthManager {
    pub async fn login(
        session: &RezkaSession,
        email_or_login: &str,
        password: &str,
    ) -> Result<UserProfile, String> {
        let form_data = [
            ("login_name", email_or_login),
            ("login_password", password),
            ("login_not_save", "0"),
        ];

        let json_resp = session.post_ajax("ajax/login/", &form_data).await?;
        let parsed: Value = serde_json::from_str(&json_resp)
            .map_err(|e| format!("Failed to parse login response JSON: {}", e))?;

        let success = parsed
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);
        if !success {
            let msg = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Неверный логин или пароль");
            return Err(Self::sanitize_message(msg));
        }

        let user_id = session.authenticated_user_id().ok_or_else(|| {
            let _ = session.clear_session(None);
            "Login response did not establish the required account cookies".to_string()
        })?;
        let mut profile = match Self::fetch_verified_profile(session, &user_id).await {
            Ok(profile) => profile,
            Err(error) => {
                let _ = session.clear_session(None);
                return Err(error);
            }
        };
        profile.is_session_persistent = session.persist_session(&user_id).is_ok();
        Ok(profile)
    }

    pub fn extract_logged_in_username(html: &str) -> Option<String> {
        let doc = Html::parse_document(html);
        let user_sel =
            Selector::parse(".b-users__panel .user_name, .b-topbar__profile .name, .user-name")
                .unwrap();
        doc.select(&user_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    }

    pub fn extract_vip_status(html: &str) -> bool {
        let doc = Html::parse_document(html);
        let vip_sel = Selector::parse(
            ".b-users__panel .vip, .vip-badge, .b-topbar__profile .vip, .b-users__panel_vip",
        )
        .unwrap();
        if doc.select(&vip_sel).next().is_some() {
            return true;
        }
        html.contains("pjs-prem-quality") || html.contains("user-vip") || html.contains("Премиум")
    }

    pub async fn check_profile(session: &RezkaSession) -> Result<Option<UserProfile>, String> {
        let Some(user_id) = session.authenticated_user_id() else {
            return Ok(None);
        };
        Self::fetch_verified_profile(session, &user_id)
            .await
            .map(Some)
    }

    async fn fetch_verified_profile(
        session: &RezkaSession,
        user_id: &str,
    ) -> Result<UserProfile, String> {
        let profile_html = session.get_html(&format!("user/{user_id}")).await?;
        let mut profile = Self::parse_verified_profile(&profile_html, user_id)?;
        profile.avatar_url = profile.avatar_url.map(|value| session.resolve_url(&value));
        Ok(profile)
    }

    fn parse_verified_profile(html: &str, user_id: &str) -> Result<UserProfile, String> {
        let doc = Html::parse_document(html);
        let profile_marker = Selector::parse("#email, .b-userprofile__avatar_holder").unwrap();
        if doc.select(&profile_marker).next().is_none() {
            return Err("Authenticated profile verification failed".to_string());
        }
        let title = Selector::parse("head title").unwrap();
        let username = doc
            .select(&title)
            .next()
            .map(|node| node.text().collect::<String>().trim().to_string())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| "Authenticated profile has no account name".to_string())?;
        let email = doc
            .select(&Selector::parse("#email").unwrap())
            .next()
            .and_then(|node| node.value().attr("value"))
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        let avatar_url = doc
            .select(&Selector::parse(".b-userprofile__avatar_holder img").unwrap())
            .next()
            .and_then(|node| node.value().attr("src"))
            .map(str::to_string);
        Ok(UserProfile {
            user_id: user_id.to_string(),
            username,
            is_logged_in: true,
            is_vip: Self::extract_vip_status(html),
            email,
            avatar_url,
            premium_days: None,
            is_session_persistent: true,
        })
    }

    pub async fn fetch_notifications(
        session: &RezkaSession,
    ) -> Result<(Vec<NotificationGroup>, Option<u32>), String> {
        let html = session.get_html("").await?;
        Self::require_authenticated_html(session, &html)?;
        Ok(Self::parse_notifications(&html))
    }

    fn parse_notifications(html: &str) -> (Vec<NotificationGroup>, Option<u32>) {
        let document = Html::parse_document(html);
        let groups = document
            .select(&Selector::parse(".b-seriesupdate__block").unwrap())
            .map(|group| NotificationGroup {
                date: group
                    .select(&Selector::parse(".b-seriesupdate__block_date").unwrap())
                    .next()
                    .map(|node| {
                        node.text()
                            .collect::<String>()
                            .replace("развернуть", "")
                            .trim()
                            .to_string()
                    })
                    .unwrap_or_default(),
                items: group
                    .select(&Selector::parse(".tracked").unwrap())
                    .filter_map(|item| {
                        let link = item
                            .select(&Selector::parse(".b-seriesupdate__block_list_link").unwrap())
                            .next()?;
                        Some(NotificationItem {
                            title: link.text().collect::<String>().trim().to_string(),
                            url: link.value().attr("href")?.to_string(),
                            info: [".season", ".cell-2"]
                                .iter()
                                .filter_map(|selector| {
                                    item.select(&Selector::parse(selector).unwrap()).next()
                                })
                                .map(|node| node.text().collect::<String>().trim().to_string())
                                .filter(|value| !value.is_empty())
                                .collect::<Vec<_>>()
                                .join(" - "),
                        })
                    })
                    .collect(),
            })
            .filter(|group| !group.items.is_empty())
            .collect();
        let premium_days = document
            .select(&Selector::parse(".b-tophead-premuser").unwrap())
            .next()
            .map(|node| node.text().collect::<String>())
            .map(|value| {
                value
                    .chars()
                    .filter(|ch| ch.is_ascii_digit())
                    .collect::<String>()
            })
            .and_then(|value| value.parse().ok());
        (groups, premium_days)
    }

    pub fn logout(session: &RezkaSession, user_id: Option<&str>) -> Result<(), String> {
        session.clear_session(user_id)
    }

    pub async fn fetch_favorites_categories(
        session: &RezkaSession,
    ) -> Result<Vec<FavoritesCollection>, String> {
        let html_content = session.get_html("favorites/").await?;
        Self::require_authenticated_html(session, &html_content)?;
        Ok(Self::parse_favorites_categories(&html_content))
    }

    pub fn parse_favorites_categories(html_content: &str) -> Vec<FavoritesCollection> {
        let doc = Html::parse_document(html_content);
        let item_sel =
            Selector::parse(".b-favorites_content__cats_list_item[data-cat_id]").unwrap();
        let name_sel = Selector::parse(".name").unwrap();
        let count_sel = Selector::parse(".num-holder, .num").unwrap();

        let mut cats = Vec::new();

        for item in doc.select(&item_sel) {
            let Some(id) = item
                .value()
                .attr("data-cat_id")
                .and_then(|id| id.parse::<i64>().ok())
            else {
                continue;
            };
            let name = item
                .select(&name_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "Коллекция".to_string());
            let count_str = item
                .select(&count_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let count = count_str
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<usize>()
                .unwrap_or(0);

            cats.push(FavoritesCollection {
                id: Some(id),
                name,
                url: format!("favorites/{id}/"),
                count,
            });
        }

        cats
    }

    pub async fn fetch_favorites_page(
        session: &RezkaSession,
        cat_id: Option<i64>,
        page: usize,
    ) -> Result<Vec<MediaItem>, String> {
        let url_path = match cat_id {
            Some(id) if page > 1 => format!("favorites/{}/page/{}/", id, page),
            Some(id) => format!("favorites/{}/", id),
            None if page > 1 => format!("favorites/page/{}/", page),
            None => "favorites/".to_string(),
        };

        let html_content = session.get_html(&url_path).await?;
        Self::require_authenticated_html(session, &html_content)?;
        let items = CatalogScraper::parse_catalog_html(&html_content);
        Ok(items)
    }

    pub async fn add_to_favorites(
        session: &RezkaSession,
        post_id: i64,
        cat_id: i64,
    ) -> Result<(), String> {
        let post_id_str = post_id.to_string();
        let cat_id_str = cat_id.to_string();
        let form_data = [
            ("post_id", post_id_str.as_str()),
            ("cat_id", cat_id_str.as_str()),
            ("action", "add_post"),
        ];

        let json_resp = session.post_ajax("ajax/favorites/", &form_data).await?;
        Self::require_json_success(&json_resp, "Failed to update favorites")
    }

    pub async fn fetch_history(session: &RezkaSession) -> Result<Vec<ServerHistoryEntry>, String> {
        let html = session.get_html("continue/").await?;
        Self::require_authenticated_html(session, &html)?;
        let doc = Html::parse_document(&html);
        let history_page =
            Selector::parse(".b-videosaves, .b-videosaves__list, .b-videosaves__list_item")
                .unwrap();
        if doc.select(&history_page).next().is_none() {
            return Err("The provider returned an invalid history page".to_string());
        }
        Ok(Self::parse_history(&html))
    }

    pub fn parse_history(html: &str) -> Vec<ServerHistoryEntry> {
        let doc = Html::parse_document(html);
        let item_sel = Selector::parse(".b-videosaves__list_item").unwrap();
        let link_sel = Selector::parse(".title a").unwrap();
        let delete_sel = Selector::parse(".delete[data-id]").unwrap();
        let date_sel = Selector::parse(".date").unwrap();
        let info_sel = Selector::parse(".info").unwrap();
        let additional_sel = Selector::parse(".new-episode").unwrap();
        doc.select(&item_sel)
            .skip(1)
            .filter_map(|item| {
                let link = item.select(&link_sel).next()?;
                let id = item
                    .select(&delete_sel)
                    .next()?
                    .value()
                    .attr("data-id")?
                    .to_string();
                Some(ServerHistoryEntry {
                    id,
                    title: link.text().collect::<String>().trim().to_string(),
                    url: link.value().attr("href")?.to_string(),
                    poster_url: link.value().attr("data-cover_url").map(str::to_string),
                    info: item
                        .select(&info_sel)
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .filter(|text| !text.is_empty()),
                    additional_info: item
                        .select(&additional_sel)
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .filter(|text| !text.is_empty()),
                    date: item
                        .select(&date_sel)
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .filter(|text| !text.is_empty()),
                    is_watched: item.value().attr("class").is_some_and(|classes| {
                        classes.split_whitespace().any(|c| c == "watched-row")
                    }),
                })
            })
            .collect()
    }

    pub async fn save_watch(
        session: &RezkaSession,
        post_id: i64,
        translator_id: i64,
        season: Option<i64>,
        episode: Option<i64>,
    ) -> Result<(), String> {
        let post_id = post_id.to_string();
        let translator_id = translator_id.to_string();
        let season = season.unwrap_or(0).to_string();
        let episode = episode.unwrap_or(0).to_string();
        let response = session
            .post_ajax(
                "ajax/send_save/",
                &[
                    ("post_id", &post_id),
                    ("translator_id", &translator_id),
                    ("season", &season),
                    ("episode", &episode),
                    ("current_time", "1"),
                ],
            )
            .await?;
        serde_json::from_str::<Value>(&response)
            .map(|_| ())
            .map_err(|_| "Account operation returned malformed JSON".to_string())
    }

    pub async fn remove_history(session: &RezkaSession, id: &str) -> Result<(), String> {
        let response = session
            .post_ajax("engine/ajax/cdn_saves_remove.php", &[("id", id)])
            .await?;
        Self::require_json_success(&response, "Failed to remove history item")
    }

    pub async fn toggle_history_watched(session: &RezkaSession, id: &str) -> Result<(), String> {
        let response = session
            .post_ajax("engine/ajax/cdn_saves_view.php", &[("id", id)])
            .await?;
        Self::require_json_success(&response, "Failed to update watched state")
    }

    pub async fn toggle_schedule_watched(session: &RezkaSession, id: &str) -> Result<(), String> {
        let response = session
            .post_ajax("engine/ajax/schedule_watched.php", &[("id", id)])
            .await?;
        Self::require_json_success(&response, "Failed to update episode watched state")
    }

    fn require_json_success(response: &str, fallback: &str) -> Result<(), String> {
        let parsed: Value = serde_json::from_str(response)
            .map_err(|_| "Account operation returned malformed JSON".to_string())?;
        if parsed.get("success").and_then(Value::as_bool) == Some(true) {
            return Ok(());
        }
        let message = parsed
            .get("message")
            .and_then(Value::as_str)
            .map(Self::sanitize_message)
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| fallback.to_string());
        Err(message)
    }

    fn sanitize_message(message: &str) -> String {
        Html::parse_fragment(message)
            .root_element()
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(160)
            .collect()
    }

    fn require_authenticated_html(session: &RezkaSession, html: &str) -> Result<(), String> {
        if html.contains("login_password") || html.contains("login_name") {
            let _ = session.clear_session(None);
            Err("Authenticated session was rejected by the official provider".to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn parses_notifications_and_premium_days() {
        let (groups, days) = AuthManager::parse_notifications(
            r#"
            <div class="b-tophead-premuser">Осталось 12 дней Продлить</div>
            <div class="b-seriesupdate__block"><div class="b-seriesupdate__block_date">Today развернуть</div><div class="tracked"><span class="season">Season 1</span><span class="cell-2">Episode 2</span><a class="b-seriesupdate__block_list_link" href="/series/42-show.html">Show</a></div></div>
        "#,
        );
        assert_eq!(days, Some(12));
        assert_eq!(groups[0].items[0].info, "Season 1 - Episode 2");
    }

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

    fn respond(stream: &mut TcpStream, headers: &str, body: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    }

    #[tokio::test]
    async fn login_requires_cookies_and_verified_profile() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut login, _) = listener.accept().unwrap();
            let login_request = read_request(&mut login);
            assert!(login_request.starts_with("POST /ajax/login/"));
            assert!(login_request.contains("name=\"login_name\""));
            assert!(login_request.contains("tester"));
            assert!(login_request.contains("name=\"login_password\""));
            respond(
                &mut login,
                "Content-Type: application/json\r\nSet-Cookie: dle_user_id=42; Path=/; HttpOnly\r\nSet-Cookie: dle_password=hash; Path=/; HttpOnly\r\n",
                r#"{"success":true}"#,
            );

            let (mut profile, _) = listener.accept().unwrap();
            let profile_request = read_request(&mut profile);
            assert!(profile_request.starts_with("GET /user/42"));
            assert!(profile_request.contains("dle_user_id=42"));
            assert!(profile_request.contains("dle_password=hash"));
            respond(
                &mut profile,
                "Content-Type: text/html\r\n",
                "<html><head><title>Tester</title></head><body><input id=\"email\"></body></html>",
            );
        });

        let session = RezkaSession::new_for_test(&base_url);
        let profile = AuthManager::login(&session, "tester", "password")
            .await
            .unwrap();
        assert_eq!(profile.user_id, "42");
        assert_eq!(profile.username, "Tester");
        assert!(!profile.is_session_persistent);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn save_watch_accepts_provider_false_response_for_later_verification() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut request, _) = listener.accept().unwrap();
            let request_text = read_request(&mut request);
            assert!(request_text.starts_with("POST /ajax/send_save/"));
            assert!(request_text.contains("name=\"post_id\""));
            respond(
                &mut request,
                "Content-Type: application/json\r\n",
                r#"{"success":false}"#,
            );
        });

        let session = RezkaSession::new_for_test(&base_url);
        AuthManager::save_watch(&session, 7, 8, Some(1), Some(2))
            .await
            .unwrap();
        server.join().unwrap();
    }

    #[test]
    fn account_parsers_reject_landing_page_and_parse_server_state() {
        assert!(AuthManager::parse_verified_profile(
            "<html><head><title>HDRezka</title></head></html>",
            "42"
        )
        .is_err());

        let categories = AuthManager::parse_favorites_categories(
            r#"<li class="b-favorites_content__cats_list_item" data-cat_id="3">
                <span class="name">Later</span><span class="num">2</span>
            </li>"#,
        );
        assert_eq!(categories[0].id, Some(3));
        assert_eq!(categories[0].count, 2);
        let history = AuthManager::parse_history(
            r#"<div class="b-videosaves__list_item"></div>
            <div class="b-videosaves__list_item watched-row">
                <a class="delete" data-id="save-7"></a>
                <div class="title"><a href="/films/7-test.html" data-cover_url="cover.jpg">Test</a></div>
                <div class="date">today</div>
            </div>"#,
        );
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "save-7");
        assert!(history[0].is_watched);
    }
}
