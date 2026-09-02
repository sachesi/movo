use crate::i18n::{tr, trf};
use movo_core::client::models::{MediaDetails, SubtitleTrack};
use movo_core::storage::history::{WatchHistory, WatchHistoryEntry};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

static LAUNCH_ID: AtomicU64 = AtomicU64::new(0);

/// How often progress is written while a title is playing.
const SAVE_INTERVAL: Duration = Duration::from_secs(5);

/// How long to wait for mpv to create its IPC socket.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Everything the launcher needs to start a player and record progress.
#[derive(Clone, Debug)]
pub struct LaunchRequest {
    pub url: String,
    pub subtitle: Option<SubtitleTrack>,
    pub title: String,
    pub user_agent: String,
    pub referer: String,
    pub start_secs: f64,
    pub duration_secs: f64,
    pub history: HistorySeed,
}

/// The history row a playback session writes to.
#[derive(Clone, Debug)]
pub struct HistorySeed {
    pub details: MediaDetails,
    pub user_id: String,
    pub translator_id: i64,
    pub season: Option<i64>,
    pub episode: Option<i64>,
}

#[derive(Debug)]
pub enum PlaybackEvent {
    /// Playback reached the end of the title.
    Ended,
    /// The player is running but progress is no longer being recorded.
    TrackingLost(String),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Default,
    /// Show the desktop's application chooser for every stream.
    Ask,
    Mpv,
    Vlc,
    Other,
}

pub fn player_kind(command: &str) -> PlayerKind {
    let command = command.trim();
    if command.is_empty() || matches!(command, "default" | "xdg-open" | "gio") {
        return PlayerKind::Default;
    }
    if command == "ask" {
        return PlayerKind::Ask;
    }
    match Path::new(command)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("")
    {
        "mpv" => PlayerKind::Mpv,
        "vlc" => PlayerKind::Vlc,
        _ => PlayerKind::Other,
    }
}

pub fn valid_media_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
        && !value.contains(['\r', '\n'])
}

pub fn spawn_mpv(command: &str, request: &LaunchRequest, socket: &Path) -> std::io::Result<()> {
    spawn_reaped(mpv_command(command, request, Some(socket)))
}

pub fn spawn_mpv_untracked(command: &str, request: &LaunchRequest) -> std::io::Result<()> {
    spawn_reaped(mpv_command(command, request, None))
}

/// Start a player and wait for it elsewhere, so an exited player does not stay
/// behind as a zombie for as long as the app runs.
pub fn spawn_reaped(mut process: std::process::Command) -> std::io::Result<()> {
    let mut child = process.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn spawn_vlc(command: &str, request: &LaunchRequest) -> std::io::Result<()> {
    let mut process = std::process::Command::new(command);
    process
        .arg(format!("--meta-title={}", request.title))
        .arg(format!("--http-user-agent={}", request.user_agent))
        .arg(format!("--http-referrer={}", request.referer));
    if request.start_secs > 0.0 {
        process.arg(format!("--start-time={:.3}", request.start_secs));
    }
    if let Some(subtitle) = &request.subtitle {
        if valid_media_url(&subtitle.url) {
            process.arg(format!("--sub-file={}", subtitle.url));
        }
    }
    process.arg(&request.url);
    spawn_reaped(process)
}

/// Build the mpv invocation. Every option has to precede the `--` separator,
/// or mpv reads it as a second file to play.
fn mpv_command(
    command: &str,
    request: &LaunchRequest,
    socket: Option<&Path>,
) -> std::process::Command {
    let mut process = std::process::Command::new(command);
    process
        .arg(format!("--title={}", request.title))
        .arg(format!("--user-agent={}", request.user_agent))
        .arg(format!("--referrer={}", request.referer));
    if let Some(socket) = socket {
        process.arg(format!("--input-ipc-server={}", socket.display()));
    }
    if request.start_secs > 0.0 {
        process.arg(format!("--start={:.3}", request.start_secs));
    }
    if let Some(subtitle) = &request.subtitle {
        if valid_media_url(&subtitle.url) {
            process.arg(format!("--sub-file={}", subtitle.url));
        }
    }
    process.arg("--").arg(&request.url);
    process
}

pub fn socket_path() -> Option<PathBuf> {
    let root = directories::ProjectDirs::from("org", "gnome", "Movo")?
        .runtime_dir()?
        .join("mpv");
    std::fs::create_dir_all(&root).ok()?;
    Some(root.join(format!(
        "{}-{}.sock",
        std::process::id(),
        LAUNCH_ID.fetch_add(1, Ordering::Relaxed) + 1
    )))
}

/// Follow one mpv session over its IPC socket, recording progress as it plays.
pub async fn monitor(
    socket: PathBuf,
    history: HistorySeed,
    mut position: f64,
    mut duration: f64,
    events: relm4::Sender<PlaybackEvent>,
) {
    let stream = {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        loop {
            match UnixStream::connect(&socket).await {
                Ok(stream) => break Some(stream),
                Err(_) if Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => break None,
            }
        }
    };
    let Some(stream) = stream else {
        events.emit(PlaybackEvent::TrackingLost(
            tr("MPV opened, but the history connection was not established.").to_string(),
        ));
        let _ = std::fs::remove_file(socket);
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let commands = concat!(
        "{\"command\":[\"observe_property\",1,\"playback-time\"]}\n",
        "{\"command\":[\"observe_property\",2,\"duration\"]}\n"
    );
    if writer.write_all(commands.as_bytes()).await.is_err() {
        events.emit(PlaybackEvent::TrackingLost(
            tr("MPV opened, but progress tracking did not start.").to_string(),
        ));
        let _ = std::fs::remove_file(socket);
        return;
    }

    let mut lines = BufReader::new(reader).lines();
    let mut last_saved = Instant::now();
    let mut reported = false;
    while let Ok(Some(line)) = lines.next_line().await {
        match parse_event(&line) {
            Some(MpvEvent::Position(value)) => position = value.max(0.0),
            Some(MpvEvent::Duration(value)) => duration = value.max(0.0),
            Some(MpvEvent::Ended) => {
                if duration > 0.0 {
                    position = duration;
                }
                match persist(&history, position, duration) {
                    Ok(()) => events.emit(PlaybackEvent::Ended),
                    Err(error) => events.emit(PlaybackEvent::TrackingLost(error)),
                }
                reported = true;
                break;
            }
            Some(MpvEvent::Failed(error)) => {
                let event = persist(&history, position, duration)
                    .map_or_else(PlaybackEvent::TrackingLost, |()| {
                        PlaybackEvent::Failed(trf("MPV could not play the stream: {}", &[&error]))
                    });
                events.emit(event);
                reported = true;
                break;
            }
            None => {}
        }
        if last_saved.elapsed() >= SAVE_INTERVAL {
            if let Err(error) = persist(&history, position, duration) {
                events.emit(PlaybackEvent::TrackingLost(error));
                reported = true;
                break;
            }
            last_saved = Instant::now();
        }
    }
    if !reported {
        // mpv exited without an end-file event: the window was closed.
        if let Err(error) = persist(&history, position, duration) {
            events.emit(PlaybackEvent::TrackingLost(error));
        }
    }
    let _ = std::fs::remove_file(socket);
}

#[derive(Debug, PartialEq)]
enum MpvEvent {
    Position(f64),
    Duration(f64),
    Ended,
    Failed(String),
}

fn parse_event(line: &str) -> Option<MpvEvent> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    match value.get("event").and_then(|event| event.as_str())? {
        "property-change" => {
            let data = value.get("data")?.as_f64()?;
            match value.get("name").and_then(|name| name.as_str())? {
                "playback-time" => Some(MpvEvent::Position(data)),
                "duration" => Some(MpvEvent::Duration(data)),
                _ => None,
            }
        }
        "end-file" => match value.get("reason").and_then(|reason| reason.as_str()) {
            Some("eof") => Some(MpvEvent::Ended),
            Some("error") => Some(MpvEvent::Failed(
                value
                    .get("file_error")
                    .and_then(|error| error.as_str())
                    .unwrap_or(tr("unknown error"))
                    .to_string(),
            )),
            _ => None,
        },
        _ => None,
    }
}

fn persist(history: &HistorySeed, position: f64, duration: f64) -> Result<(), String> {
    WatchHistory::update_entry_for(
        &history.user_id,
        WatchHistoryEntry {
            media_id: history.details.id,
            title: history.details.title.clone(),
            orig_title: history.details.orig_title.clone(),
            url: history.details.url.clone(),
            poster_url: history.details.poster_url.clone(),
            media_type: history.details.media_type,
            season: history.season,
            episode: history.episode,
            episode_title: None,
            translator_id: Some(history.translator_id),
            translator_name: None,
            position_secs: position.max(0.0),
            duration_secs: duration.max(0.0),
            updated_at: chrono::Utc::now(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{mpv_command, LaunchRequest};

    fn request() -> LaunchRequest {
        LaunchRequest {
            url: "https://cdn.example/stream.mp4".to_string(),
            subtitle: None,
            title: "Title".to_string(),
            user_agent: "Agent".to_string(),
            referer: "https://hdrzk.org/".to_string(),
            start_secs: 61.5,
            duration_secs: 0.0,
            history: super::HistorySeed {
                details: serde_json::from_str(FIXTURE).expect("fixture"),
                user_id: String::new(),
                translator_id: 1,
                season: None,
                episode: None,
            },
        }
    }

    const FIXTURE: &str = r#"{"id":1,"title":"","orig_title":null,"url":"","poster_url":null,
        "poster_hq_url":null,"description":"","year":null,"media_type":"Movie","rating_rezka":null,
        "rating_imdb":null,"rating_kp":null,"genres":[],"countries":[],"directors":[],"actors":[],
        "duration":null,"translators":[],"seasons":[],"franchises":[],"genre_links":[],
        "country_links":[],"directors_details":[],"actors_details":[],"ratings":[],
        "voice_ratings":[],"schedules":[],"related":[],"included_in":[],"from_collections":[],
        "trailer_available":false,"rating_posted":false,"favorite_category_ids":[]}"#;

    #[test]
    fn mpv_receives_the_headers_the_provider_requires() {
        let socket = std::path::PathBuf::from("/run/movo.sock");
        let command = mpv_command("mpv", &request(), Some(&socket));
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.contains(&"--user-agent=Agent".to_string()));
        assert!(args.contains(&"--referrer=https://hdrzk.org/".to_string()));
        assert!(args.contains(&"--start=61.500".to_string()));
        // Every option, the IPC socket included, has to land before the `--`.
        let separator = args.iter().position(|arg| arg == "--").expect("separator");
        let socket_arg = args
            .iter()
            .position(|arg| arg == "--input-ipc-server=/run/movo.sock")
            .expect("the socket is passed to mpv");
        assert!(socket_arg < separator);
        // The URL comes last, after `--`, so it is never read as an option.
        assert_eq!(
            args.iter().rev().take(2).collect::<Vec<_>>(),
            vec!["https://cdn.example/stream.mp4", "--"]
        );
    }

    use super::{parse_event, player_kind, valid_media_url, MpvEvent, PlayerKind};

    #[test]
    fn configured_player_detection_is_exact() {
        assert_eq!(player_kind("default"), PlayerKind::Default);
        assert_eq!(player_kind("xdg-open"), PlayerKind::Default);
        assert_eq!(player_kind("/usr/bin/mpv"), PlayerKind::Mpv);
        assert_eq!(player_kind("vlc"), PlayerKind::Vlc);
        assert_eq!(player_kind("my-mpv-wrapper"), PlayerKind::Other);
        assert_eq!(player_kind("ask"), PlayerKind::Ask);
    }

    #[test]
    fn parses_mpv_progress_and_failures() {
        assert_eq!(
            parse_event(r#"{"event":"property-change","id":1,"name":"playback-time","data":42.5}"#),
            Some(MpvEvent::Position(42.5))
        );
        assert_eq!(
            parse_event(r#"{"event":"property-change","id":2,"name":"duration","data":120.0}"#),
            Some(MpvEvent::Duration(120.0))
        );
        assert_eq!(
            parse_event(r#"{"event":"end-file","reason":"eof"}"#),
            Some(MpvEvent::Ended)
        );
        assert_eq!(
            parse_event(r#"{"event":"end-file","reason":"error","file_error":"loading failed"}"#),
            Some(MpvEvent::Failed("loading failed".to_string()))
        );
    }

    #[test]
    fn only_http_urls_are_passed_to_a_player() {
        assert!(valid_media_url("https://cdn.example/stream.m3u8"));
        assert!(!valid_media_url("file:///etc/passwd"));
        assert!(!valid_media_url("https://cdn.example/a\nb"));
    }
}
