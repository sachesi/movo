use super::models::ServerHistoryEntry;
use super::{auth, blocking, details, RezkaClient, SyncedHistory};
use crate::error::ClientError;
use crate::storage::history::WatchHistory;
use std::collections::HashSet;
use std::time::Duration;

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

impl RezkaClient {
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
}

#[cfg(test)]
mod tests {
    use super::history_entry_matches;
    use crate::client::auth;
    use crate::client::test_support::{authenticated_test_client, read_request, respond};
    use std::net::TcpListener;

    fn history_row(watched: bool) -> String {
        format!(
            r#"<div class="b-videosaves__list_item"></div><div class="b-videosaves__list_item{}"><button class="delete" data-id="saved"></button><div class="title"><a href="/films/7-test.html">Test</a></div></div>"#,
            if watched { " watched-row" } else { "" }
        )
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
}
