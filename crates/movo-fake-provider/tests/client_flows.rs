//! The core client driven end to end against the fake provider: real HTTP, real
//! page parsing, the provider's state changing behind it.

use movo_core::client::models::{parse_country_list, CatalogCategory, MediaType};
use movo_core::client::RezkaClient;
use movo_core::error::ErrorKind;
use movo_fake_provider::catalog::{ANNOUNCED_TITLE_ID, MARKUP_TITLE_ID, REFUSED_SERIES_ID};
use movo_fake_provider::state::{LOGIN, PASSWORD};
use movo_fake_provider::{FakeProvider, Fault, FaultKind};
use std::path::PathBuf;
use std::sync::Once;
use std::time::{Duration, Instant};

/// Keeps a test run off the developer's real settings and keyring: the storage
/// paths move under a scratch directory and the session bus points nowhere.
fn isolate_from_the_account() {
    static ISOLATED: Once = Once::new();
    ISOLATED.call_once(|| {
        let home = std::env::temp_dir().join(format!("movo-client-flows-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let path = |name: &str| -> PathBuf { home.join(name) };
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", path("config"));
        std::env::set_var("XDG_DATA_HOME", path("data"));
        std::env::set_var("XDG_CACHE_HOME", path("cache"));
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    });
}

fn start() -> (FakeProvider, RezkaClient) {
    isolate_from_the_account();
    let provider = FakeProvider::start().unwrap();
    let client = RezkaClient::against(&provider.base_url());
    (provider, client)
}

async fn signed_in() -> (FakeProvider, RezkaClient) {
    let (provider, client) = start();
    client.login(LOGIN, PASSWORD).await.unwrap();
    (provider, client)
}

fn page_of(provider: &FakeProvider, id: i64) -> String {
    let title = movo_fake_provider::catalog::title(id).unwrap();
    format!("{}{}", provider.base_url(), title.url_path())
}

#[tokio::test]
async fn home_fills_every_rail() {
    let (_provider, client) = start();

    let sections = client.home().await.unwrap();

    let ids: Vec<&str> = sections.iter().map(|section| section.id.as_str()).collect();
    assert_eq!(ids, ["hot", "new", "watching", "popular", "awaiting"]);
    assert_eq!(sections[0].items.len(), 12);
    assert_eq!(sections[1].items[0].id, 1);
    assert_eq!(sections[1].items[0].year, Some(1991));
    assert!(sections[1].items[0].rating.is_some());
}

#[tokio::test]
async fn one_failing_rail_leaves_the_others() {
    let (provider, client) = start();
    provider.inject(Fault::always("announce", FaultKind::Status(500)));

    let sections = client.home().await.unwrap();

    assert_eq!(sections.len(), 4);
    assert!(sections.iter().all(|section| section.id != "awaiting"));
}

#[tokio::test]
async fn a_home_with_no_rails_reports_the_failure() {
    let (provider, client) = start();
    provider.inject(Fault::always("", FaultKind::Status(503)));

    let error = client.home().await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::Provider);
    assert!(error.message.contains("503"), "{}", error.message);
}

#[tokio::test]
async fn a_failed_listing_keeps_its_kind() {
    let (provider, client) = start();
    provider.inject(Fault::always("films", FaultKind::Status(503)));

    let error = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Provider);
}

#[tokio::test]
async fn catalog_pages_differ_and_run_out() {
    let (_provider, client) = start();

    let first = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap();
    let second = client
        .fetch_catalog(CatalogCategory::Films, None, 2)
        .await
        .unwrap();
    let past_the_end = client
        .fetch_catalog(CatalogCategory::Films, None, 99)
        .await
        .unwrap();

    assert_eq!(first.len(), 36);
    assert_ne!(first[0].id, second[0].id);
    assert!(past_the_end.is_empty());
}

#[tokio::test]
async fn hidden_countries_leave_every_listing() {
    let (_provider, client) = start();
    client.set_hidden_countries(parse_country_list("США"));

    let films = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap();

    assert!(!films.is_empty());
    assert!(films
        .iter()
        .all(|item| !item.info.as_deref().unwrap_or_default().contains("США")));
}

#[tokio::test]
async fn search_finds_by_cyrillic_query() {
    let (provider, client) = start();

    let suggestions = client.search_suggestions("маяк").await.unwrap();
    let found = client.search_full("маяк", 1).await.unwrap();

    assert!(!suggestions.is_empty());
    assert!(found.iter().all(|item| item.title.contains("маяк")));
    let request = &provider.requests_to("search")[0];
    assert_eq!(request.form.get("q").map(String::as_str), Some("маяк"));
}

#[tokio::test]
async fn a_film_page_parses_in_full() {
    let (provider, client) = start();

    let details = client.fetch_details(&page_of(&provider, 4)).await.unwrap();

    assert_eq!(details.id, 4);
    assert_eq!(details.media_type, MediaType::Movie);
    assert_eq!(details.translators.len(), 3);
    assert!(details.translators[1].has_ads);
    assert!(details.translators[2].is_premium);
    assert_eq!(details.directors, ["Анна Режиссёр"]);
    assert_eq!(details.actors_details[0].role.as_deref(), Some("Актёр"));
    assert_eq!(details.franchises.len(), 2);
    assert_eq!(details.related.len(), 6);
    assert!(details.has_trailer);
    assert!(details.rating_imdb.is_some());
    assert!(details
        .poster_url
        .is_some_and(|url| url.starts_with(&provider.base_url())));
}

#[tokio::test]
async fn a_series_page_lists_its_episodes_and_schedule() {
    let (provider, client) = start();

    let details = client.fetch_details(&page_of(&provider, 1)).await.unwrap();

    assert_eq!(details.media_type, MediaType::TVSeries);
    assert_eq!(details.seasons.len(), 2);
    assert_eq!(details.seasons[1].episodes.len(), 6);
    assert_eq!(details.schedules.len(), 2);
    assert_eq!(
        details.seasons[0].episodes[2].watch_id.as_deref(),
        Some("1-1-3")
    );
}

#[tokio::test]
async fn markup_in_a_title_stays_text() {
    let (provider, client) = start();

    let details = client
        .fetch_details(&page_of(&provider, MARKUP_TITLE_ID))
        .await
        .unwrap();

    assert!(
        details.title.contains("<b>Жирный</b> & "),
        "{}",
        details.title
    );
    assert!(
        details.description.contains("<i>разметка"),
        "{}",
        details.description
    );
}

#[tokio::test]
async fn an_announced_title_offers_no_voice_over() {
    let (provider, client) = start();

    let details = client
        .fetch_details(&page_of(&provider, ANNOUNCED_TITLE_ID))
        .await
        .unwrap();

    assert!(details.translators.is_empty());
}

#[tokio::test]
async fn a_series_refused_under_its_only_voice_over_offers_none() {
    let (provider, client) = start();

    let details = client
        .fetch_details(&page_of(&provider, REFUSED_SERIES_ID))
        .await
        .unwrap();

    assert!(details.translators.is_empty());
    assert!(details.seasons.is_empty());
}

#[tokio::test]
async fn a_stream_decodes_through_the_noise() {
    let (provider, client) = start();
    let details = client.fetch_details(&page_of(&provider, 4)).await.unwrap();

    let bundle = client
        .fetch_movie_stream(4, &details.translators[0])
        .await
        .unwrap();

    let qualities: Vec<&str> = bundle
        .streams
        .iter()
        .map(|stream| stream.quality.as_str())
        .collect();
    assert_eq!(qualities, ["1080p Ultra", "720p", "360p"]);
    assert!(bundle.streams[0].is_premium);
    assert!(bundle.streams[1].urls[0].contains("/stream/4/238/movie/720.mp4"));
    assert_eq!(bundle.subtitles.len(), 2);
    assert_eq!(bundle.storyboard.len(), 2);
    assert_eq!(bundle.referer, provider.base_url());
}

#[tokio::test]
async fn a_hung_storyboard_does_not_hold_up_playback() {
    let (provider, client) = start();
    provider.inject(Fault::always(
        "/storyboard/",
        FaultKind::Delay(Duration::from_secs(20)),
    ));

    let started = Instant::now();
    let bundle = client.fetch_episode_stream(1, 238, 1, 1).await.unwrap();

    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(bundle.storyboard.is_empty());
    assert!(!bundle.streams.is_empty());
}

#[tokio::test]
async fn an_episode_stream_names_its_episode() {
    let (_provider, client) = start();

    let bundle = client.fetch_episode_stream(1, 238, 2, 3).await.unwrap();

    assert_eq!((bundle.season, bundle.episode), (Some(2), Some(3)));
    assert!(bundle.streams[0].urls[0].contains("/2x3/"));
}

#[tokio::test]
async fn comments_page_through() {
    let (_provider, client) = start();

    let page = client.fetch_comments(4, 2).await.unwrap();

    assert_eq!(page.items.len(), 5);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.items[2].username, "Зритель 3");
    assert!(page.items[2].has_spoiler);
    assert_eq!(page.items[1].indent, 1);
}

#[tokio::test]
async fn actor_and_collections_parse() {
    let (provider, client) = start();

    let actor = client
        .fetch_actor(&format!("{}/person/8-actor/", provider.base_url()))
        .await
        .unwrap();
    let collections = client.fetch_collections(1).await.unwrap();
    let filters = client.search_filters().await.unwrap();

    assert_eq!(actor.name, "Иван Актёр");
    assert_eq!(actor.roles[0].films.len(), 8);
    assert_eq!(collections.len(), 8);
    assert_eq!(collections[1].count, 10);
    assert_eq!(filters.len(), 2);
    assert_eq!(filters[0].years[0].url, "-1");
}

#[tokio::test]
async fn wrong_credentials_are_refused_with_plain_text() {
    let (_provider, client) = start();

    let error = client.login(LOGIN, "wrong").await.unwrap_err();

    assert_eq!(error.message, "Неверный логин или пароль");
    assert!(client.user().is_none());
}

#[tokio::test]
async fn login_verifies_the_profile() {
    let (_provider, client) = signed_in().await;

    let user = client.user().unwrap();

    assert_eq!(user.username, "Тестер");
    assert_eq!(user.user_id, "42");
    assert!(user.is_vip);
    assert_eq!(user.email.as_deref(), Some("tester@example.invalid"));
}

#[tokio::test]
async fn account_writes_need_a_sign_in() {
    let (provider, client) = start();

    assert!(client.fetch_history().await.is_err());
    assert!(client.post_rating(4, 8).await.is_err());
    assert!(client.fetch_favorites_categories().await.is_err());
    assert!(
        provider.requests().is_empty(),
        "nothing should reach the site"
    );
}

#[tokio::test]
async fn an_exported_session_comes_back_verified() {
    let (provider, client) = signed_in().await;
    let secret = client.session().export_session().unwrap();

    let restored = RezkaClient::against(&provider.base_url());
    let Ok(profile) = restored.import_session(&secret).await else {
        panic!("the exported session was not restored");
    };

    assert_eq!(profile.username, "Тестер");
    assert!(restored.fetch_history().await.is_ok());
}

#[tokio::test]
async fn favorites_converge_on_what_was_asked() {
    let (provider, client) = signed_in().await;
    let url = page_of(&provider, 8);

    client.set_favorite(&url, 8, 3, true).await.unwrap();
    client.set_favorite(&url, 8, 3, true).await.unwrap();

    assert_eq!(provider.requests_to("ajax/favorites").len(), 1);
    assert_eq!(provider.with_state(|state| state.categories_of(8)), vec![3]);

    client.set_favorite(&url, 8, 3, false).await.unwrap();
    assert!(provider
        .with_state(|state| state.categories_of(8))
        .is_empty());

    let categories = client.fetch_favorites_categories().await.unwrap();
    assert_eq!(categories[0].count, 3);
    let page = client.fetch_favorites_page(Some(1), 1).await.unwrap();
    assert_eq!(page.len(), 3);
}

#[tokio::test]
async fn saving_a_position_reads_it_back() {
    let (provider, client) = signed_in().await;

    client.save_watch(1, 238, Some(2), Some(4)).await.unwrap();

    let history = client.fetch_history().await.unwrap();
    assert_eq!(history[0].media_id(), Some(1));
    assert_eq!(history[0].labelled_position(), (Some(2), Some(4)));
    assert_eq!(
        provider.requests_to("send_save")[0]
            .form
            .get("current_time")
            .map(String::as_str),
        Some("1")
    );
}

#[tokio::test]
async fn a_history_row_can_be_ticked_and_removed() {
    let (provider, client) = signed_in().await;

    client.set_history_watched("h5", true).await.unwrap();
    client.set_history_watched("h5", true).await.unwrap();

    assert_eq!(provider.requests_to("cdn_saves_view").len(), 1);
    let row = client
        .fetch_history()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == "h5")
        .unwrap();
    assert!(row.is_watched);

    assert_eq!(client.remove_history_with_media("h5").await.unwrap(), 5);
    assert!(client
        .fetch_history()
        .await
        .unwrap()
        .iter()
        .all(|row| row.id != "h5"));
}

#[tokio::test]
async fn a_refused_toggle_is_reported_and_not_sent_again() {
    let (provider, client) = signed_in().await;
    provider.inject(Fault::times("cdn_saves_view", FaultKind::Status(502), 1));

    assert!(client.set_history_watched("h5", true).await.is_err());

    assert_eq!(provider.requests_to("cdn_saves_view").len(), 1);
}

#[tokio::test]
async fn a_toggle_whose_reply_was_lost_counts_once_read_back() {
    let (provider, client) = signed_in().await;
    provider.inject(Fault::times("cdn_saves_view", FaultKind::LoseReply, 1));

    client.set_history_watched("h5", true).await.unwrap();

    assert_eq!(provider.requests_to("cdn_saves_view").len(), 1);
    assert!(provider.with_state(|state| state.history[0].watched));
}

#[tokio::test]
async fn a_toggle_is_never_sent_twice_on_failure() {
    let (provider, client) = signed_in().await;
    provider.inject(Fault::always("rating.php", FaultKind::Drop));

    assert!(client.post_rating(4, 8).await.is_err());

    assert_eq!(provider.requests_to("rating.php").len(), 1);
}

#[tokio::test]
async fn reads_retry_past_a_passing_failure() {
    let (provider, client) = start();
    provider.inject(Fault::times("films", FaultKind::Status(502), 2));

    let items = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap();

    assert_eq!(items.len(), 36);
    assert_eq!(provider.requests_to("films").len(), 3);
}

#[tokio::test]
async fn the_request_limit_is_resent_past_without_spending_a_retry() {
    let (provider, client) = start();
    provider.inject(Fault::times("films", FaultKind::LimitRefusal, 1));

    let items = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap();

    assert_eq!(items.len(), 36);
}

#[tokio::test]
async fn garbage_in_a_stream_reply_is_an_error_not_a_panic() {
    let (provider, client) = start();
    provider.inject(Fault::always("get_cdn_series", FaultKind::Garbage));

    assert!(client.fetch_episode_stream(1, 238, 1, 1).await.is_err());
}

#[tokio::test]
async fn an_expired_session_is_noticed_and_cleared() {
    let (provider, client) = signed_in().await;
    provider.inject(Fault::times("continue", FaultKind::SessionExpired, 1));

    assert!(client.fetch_history().await.is_err());

    assert!(client.user().is_none(), "the session must be dropped");
}

#[tokio::test]
async fn the_proof_of_work_gate_opens_and_stays_open() {
    let (provider, client) = start();
    provider.require_anubis(2);

    let items = client
        .fetch_catalog(CatalogCategory::Films, None, 1)
        .await
        .unwrap();
    assert_eq!(items.len(), 36);

    let requests_after_solving = provider.requests_to("pass-challenge").len();
    client
        .fetch_catalog(CatalogCategory::Series, None, 1)
        .await
        .unwrap();
    assert_eq!(
        provider.requests_to("pass-challenge").len(),
        requests_after_solving
    );
}

#[tokio::test]
async fn a_slow_page_does_not_hold_the_others_up() {
    let (provider, client) = start();
    provider.inject(Fault::times(
        "films",
        FaultKind::Delay(Duration::from_millis(1500)),
        1,
    ));

    let slow_client = client.clone();
    let slow = tokio::spawn(async move {
        slow_client
            .fetch_catalog(CatalogCategory::Films, None, 1)
            .await
    });
    let started = Instant::now();
    client
        .fetch_catalog(CatalogCategory::Series, None, 1)
        .await
        .unwrap();

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(slow.await.unwrap().is_ok());
}
