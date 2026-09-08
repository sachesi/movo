//! Drives the real playback path end to end against a local stream.
//!
//! The provider needs an account, so this test stands in for it: a throwaway
//! HTTP server serves a short clip, mpv plays it through the same launch and
//! IPC code the app uses, and the progress it reports has to reach the watch
//! history.
//!
//! It needs mpv and a display, so it is ignored by default. The runtime
//! directory has to be short, because a unix socket path cannot exceed 108
//! bytes and the socket is created inside it:
//!
//! ```text
//! mpv 'av://lavfi:testsrc=size=160x120:rate=10:duration=6' --o=/tmp/sample.mp4
//! XDG_RUNTIME_DIR=/tmp/movo-rt MOVO_PLAYBACK_SAMPLE=/tmp/sample.mp4 xvfb-run -a \
//!     cargo test -p movo --test playback_test -- --ignored --nocapture
//! ```

use movo::playback::mpv::{self, HistorySeed, LaunchRequest, PlaybackEvent};
use movo::state::Account;
use movo_core::client::models::MediaDetails;
use movo_core::storage::history::WatchHistory;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// An account id no real profile can collide with.
const TEST_USER: &str = "movo-playback-selftest";

/// Serve one file over HTTP for as long as the test runs.
async fn serve(path: PathBuf) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let body = std::fs::read(&path).expect("read sample");

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let body = body.clone();
            tokio::spawn(async move {
                let mut request = [0_u8; 2048];
                let _ = stream.read(&mut request).await;
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nAccept-Ranges: none\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes()).await;
                let _ = stream.write_all(&body).await;
                let _ = stream.flush().await;
            });
        }
    });

    port
}

fn details() -> MediaDetails {
    serde_json::from_str(
        r#"{"id":424242,"title":"Playback self test","orig_title":null,"url":"https://example.test/x",
            "poster_url":null,"poster_hq_url":null,"description":"","year":null,"media_type":"Movie",
            "rating_rezka":null,"rating_imdb":null,"rating_kp":null,"genres":[],"countries":[],
            "directors":[],"actors":[],"duration":null,"translators":[],"seasons":[],"franchises":[],
            "genre_links":[],"country_links":[],"directors_details":[],"actors_details":[],
            "ratings":[],"voice_ratings":[],"schedules":[],"related":[],"included_in":[],
            "from_collections":[],"trailer_available":false,"rating_posted":false,
            "favorite_category_ids":[]}"#,
    )
    .expect("details fixture")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs mpv, a display, and MOVO_PLAYBACK_SAMPLE"]
async fn mpv_plays_a_stream_and_its_progress_reaches_the_history() {
    let sample = PathBuf::from(
        std::env::var("MOVO_PLAYBACK_SAMPLE").expect("MOVO_PLAYBACK_SAMPLE must name a media file"),
    );
    let port = serve(sample).await;

    let request = LaunchRequest {
        url: format!("http://127.0.0.1:{port}/sample.mp4"),
        subtitle: None,
        title: "Playback self test".to_string(),
        user_agent: "Movo/selftest".to_string(),
        referer: "https://example.test/".to_string(),
        // Resuming has to survive the round trip to mpv and back.
        start_secs: 2.0,
        duration_secs: 0.0,
        history: HistorySeed {
            details: details(),
            account: Account {
                generation: 0,
                user_id: TEST_USER.to_string(),
            },
            translator_id: 7,
            season: None,
            episode: None,
        },
    };

    let history_file = WatchHistory::file_path(TEST_USER);
    let _ = std::fs::remove_file(&history_file);

    let socket = mpv::socket_path().expect("runtime directory");
    mpv::spawn_mpv("mpv", &request, &socket).expect("mpv starts");

    let (events, incoming) = relm4::channel::<PlaybackEvent>();
    let monitor = tokio::spawn(mpv::monitor(
        socket.clone(),
        request.history.clone(),
        request.start_secs,
        request.duration_secs,
        events,
    ));

    let first = tokio::time::timeout(Duration::from_secs(60), incoming.recv())
        .await
        .expect("mpv finished within a minute");
    let (ended, failure) = match first {
        Some(PlaybackEvent::Ended(_)) => (true, None),
        Some(PlaybackEvent::Failed(_, message)) | Some(PlaybackEvent::TrackingLost(_, message)) => {
            (false, Some(message))
        }
        None => (
            false,
            Some("the monitor stopped without reporting".to_string()),
        ),
    };
    let _ = tokio::time::timeout(Duration::from_secs(10), monitor).await;

    assert!(failure.is_none(), "playback reported {failure:?}");
    assert!(ended, "mpv never reported the end of the clip");

    let entry = WatchHistory::load(TEST_USER)
        .expect("the played title history was readable")
        .get_entry(424242, None, None)
        .cloned()
        .expect("the played title reached the history");
    // Leave no account behind, not even an empty directory.
    let _ = std::fs::remove_file(&history_file);
    if let Some(account) = history_file.parent() {
        let _ = std::fs::remove_dir(account);
    }

    assert!(
        entry.duration_secs > 4.0,
        "duration was not observed: {entry:?}"
    );
    assert!(
        entry.position_secs >= entry.duration_secs - 0.5,
        "a finished title should be recorded at its end: {entry:?}"
    );
    assert!(!socket.exists(), "the IPC socket was left behind");
}
