use super::models::{StoryboardCue, StreamBundle, StreamEntry, SubtitleTrack, Translator};
use super::session::RezkaSession;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use reqwest::header::{RANGE, REFERER};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Duration;

/// Marks the start of a noise run inside an encoded stream list.
const NOISE_MARKER: &str = "//_//";

/// Every noise run is two or three characters long, and base64 spells both
/// lengths in exactly four characters, so one fixed window matches them all.
const NOISE_WIDTH: usize = 4;

/// The base64 spellings of every noise run the site emits.
static NOISE: LazyLock<HashSet<[u8; NOISE_WIDTH]>> = LazyLock::new(|| {
    const RUN_CHARS: [u8; 5] = [b'@', b'#', b'!', b'^', b'$'];
    let mut spellings = HashSet::with_capacity(RUN_CHARS.len().pow(2) + RUN_CHARS.len().pow(3));
    for &first in &RUN_CHARS {
        for &second in &RUN_CHARS {
            spellings.extend(noise_spelling(&[first, second]));
            for &third in &RUN_CHARS {
                spellings.extend(noise_spelling(&[first, second, third]));
            }
        }
    }
    spellings
});

fn noise_spelling(run: &[u8]) -> Option<[u8; NOISE_WIDTH]> {
    BASE64_STANDARD.encode(run).into_bytes().try_into().ok()
}

/// Hosts that serve the same files under the same paths. The provider names
/// one of them for every stream and offers no alternative, and when that one
/// stops answering the files are still there on the others.
const CDN_HOSTS: [&str; 11] = [
    "prx-cogent.ukrtelcdn.net",
    "prx2-cogent.ukrtelcdn.net",
    "prx3-cogent.ukrtelcdn.net",
    "prx4-cogent.ukrtelcdn.net",
    "prx5-cogent.ukrtelcdn.net",
    "prx6-cogent.ukrtelcdn.net",
    "prx-ams.ukrtelcdn.net",
    "stream.voidboost.cc",
    "stream.voidboost.top",
    "stream.voidboost.link",
    "stream.voidboost.club",
];

/// How long a CDN host gets to start answering before it counts as down.
const CDN_PROBE_TIMEOUT: Duration = Duration::from_secs(4);

/// Hosts that did not answer the last time they were the provider's choice.
/// They are still tried, but playback no longer waits for them to time out.
static UNREACHABLE_CDNS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);

pub async fn fetch_movie_stream(
    session: &RezkaSession,
    post_id: i64,
    translator: &Translator,
) -> Result<StreamBundle, String> {
    let tr_id = if translator.id <= 0 {
        238
    } else {
        translator.id
    };
    let post_id_str = post_id.to_string();
    let tr_id_str = tr_id.to_string();
    let is_camrip = if translator.is_camrip { "1" } else { "0" };
    let is_ads = if translator.has_ads { "1" } else { "0" };
    let is_director = if translator.is_director_cut { "1" } else { "0" };
    let form_data = [
        ("id", post_id_str.as_str()),
        ("translator_id", tr_id_str.as_str()),
        ("is_camrip", is_camrip),
        ("is_ads", is_ads),
        ("is_director", is_director),
        ("action", "get_movie"),
    ];

    let json_resp = session
        .post_ajax("ajax/get_cdn_series/", &form_data)
        .await?;
    let mut bundle = parse_stream_json(&json_resp, post_id, tr_id, None, None)?;
    prefer_reachable_cdn(session, &mut bundle).await;
    absolutize_storyboard(session, &mut bundle);
    load_storyboard(session, &mut bundle).await;
    Ok(bundle)
}

pub async fn fetch_episode_stream(
    session: &RezkaSession,
    post_id: i64,
    translator_id: i64,
    season: i64,
    episode: i64,
) -> Result<StreamBundle, String> {
    let tr_id = if translator_id <= 0 {
        238
    } else {
        translator_id
    };
    let post_id_str = post_id.to_string();
    let tr_id_str = tr_id.to_string();
    let season_str = season.to_string();
    let episode_str = episode.to_string();

    let form_data = [
        ("id", post_id_str.as_str()),
        ("translator_id", tr_id_str.as_str()),
        ("season", season_str.as_str()),
        ("episode", episode_str.as_str()),
        ("action", "get_stream"),
    ];

    let json_resp = session
        .post_ajax("ajax/get_cdn_series/", &form_data)
        .await?;
    let mut bundle = parse_stream_json(&json_resp, post_id, tr_id, Some(season), Some(episode))?;
    prefer_reachable_cdn(session, &mut bundle).await;
    absolutize_storyboard(session, &mut bundle);
    load_storyboard(session, &mut bundle).await;
    Ok(bundle)
}

pub fn parse_stream_json(
    json_str: &str,
    post_id: i64,
    translator_id: i64,
    season: Option<i64>,
    episode: Option<i64>,
) -> Result<StreamBundle, String> {
    let parsed: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse stream JSON: {}", e))?;

    if !parsed
        .get("success")
        .and_then(|s| s.as_bool())
        .unwrap_or(false)
    {
        let msg = parsed
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("Stream request failed: {}", msg));
    }

    // If the response contains a `streams` object (some mirrors return the
    // already-split variant), prefer it over the encoded `url` field.
    let decoded_str = match parsed.get("streams") {
        Some(Value::Object(streams_map)) => {
            // Rebuild the "url" format: [quality]url,...
            let mut parts = Vec::new();
            for (q, url_v) in streams_map {
                if let Some(url) = url_v.as_str() {
                    parts.push(format!("[{}]{}", q, url));
                }
            }
            if parts.is_empty() {
                decoded_url(&parsed)?
            } else {
                parts.join(",")
            }
        }
        _ => decoded_url(&parsed)?,
    };
    let streams = parse_stream_entries(&decoded_str);

    let mut subtitles = Vec::new();
    if let Some(Value::String(sub_str)) = parsed.get("subtitle") {
        // Subtitles arrive as three fields: subtitle, subtitle_def and
        // subtitle_lns.
        let default_code = parsed
            .get("subtitle_def")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let lns = parsed
            .get("subtitle_lns")
            .and_then(|v| v.as_object())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|lang| (k.clone(), lang.to_string())))
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_default();
        subtitles = parse_subtitles_with_meta(sub_str, &default_code, &lns);
    }

    Ok(StreamBundle {
        id: post_id,
        translator_id,
        season,
        episode,
        streams,
        subtitles,
        storyboard_url: parsed
            .get("thumbnails")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        storyboard: Vec::new(),
        user_agent: String::new(),
        referer: String::new(),
    })
}

fn absolutize_storyboard(session: &RezkaSession, bundle: &mut StreamBundle) {
    let Some(path) = bundle.storyboard_url.as_ref() else {
        return;
    };
    if !path.starts_with("http://") && !path.starts_with("https://") {
        bundle.storyboard_url = Some(format!(
            "{}/{}",
            session.base_url().trim_end_matches('/'),
            path.trim_start_matches('/')
        ));
    }
}

async fn load_storyboard(session: &RezkaSession, bundle: &mut StreamBundle) {
    let Some(url) = bundle.storyboard_url.as_deref() else {
        return;
    };
    if let Ok(vtt) = session.get_html(url).await {
        bundle.storyboard = parse_storyboard(&vtt, url);
    }
}

fn parse_storyboard(vtt: &str, source_url: &str) -> Vec<StoryboardCue> {
    let source = url::Url::parse(source_url).ok();
    let mut cues = Vec::new();
    let lines = vtt.lines().map(str::trim).collect::<Vec<_>>();
    for window in lines.windows(2) {
        let Some((start, end)) = window[0].split_once(" --> ") else {
            continue;
        };
        let Some(start_ms) = parse_timestamp(start) else {
            continue;
        };
        let Some(end_ms) = parse_timestamp(end) else {
            continue;
        };
        let (image, fragment) = window[1].split_once('#').unwrap_or((window[1], ""));
        let image_url = source
            .as_ref()
            .and_then(|base| base.join(image).ok())
            .map(|url| url.to_string())
            .unwrap_or_else(|| image.to_string());
        let dimensions = fragment
            .strip_prefix("xywh=")
            .and_then(|value| {
                let values = value
                    .split(',')
                    .filter_map(|part| part.parse().ok())
                    .collect::<Vec<u32>>();
                (values.len() == 4).then(|| (values[0], values[1], values[2], values[3]))
            })
            .unwrap_or((0, 0, 150, 75));
        cues.push(StoryboardCue {
            start_ms,
            end_ms,
            image_url,
            x: dimensions.0,
            y: dimensions.1,
            width: dimensions.2,
            height: dimensions.3,
        });
    }
    cues
}

fn parse_timestamp(value: &str) -> Option<u64> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (
            0,
            minutes.parse::<u64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        [hours, minutes, seconds] => (
            hours.parse().ok()?,
            minutes.parse().ok()?,
            seconds.parse().ok()?,
        ),
        _ => return None,
    };
    Some(hours * 3_600_000 + minutes * 60_000 + (seconds * 1_000.0) as u64)
}

fn decoded_url(parsed: &Value) -> Result<String, String> {
    parsed
        .get("url")
        .and_then(Value::as_str)
        .map(decrypt_streams)
        .ok_or_else(|| "No stream URL found (may be restricted or require login)".to_string())
}

/// Decodes the obfuscated stream list the player endpoint returns.
///
/// The payload is base64 with noise stitched into it: a `//_//` marker
/// followed by the base64 spelling of a short run of `@#!^$`. Dropping the
/// markers and the noise leaves the original base64, which may have lost
/// its padding on the way.
fn decrypt_streams(encrypted: &str) -> String {
    let Some(payload) = encrypted.strip_prefix('#') else {
        return encrypted.to_string();
    };
    // One marker character follows the `#` and is not part of the payload.
    let mut payload = payload.chars();
    payload.next();

    let mut cleaned = Vec::with_capacity(encrypted.len());
    for segment in payload.as_str().split(NOISE_MARKER) {
        let bytes = segment.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            if let Some(window) = bytes.get(at..at + NOISE_WIDTH) {
                if NOISE.contains(window) {
                    at += NOISE_WIDTH;
                    continue;
                }
            }
            if !bytes[at].is_ascii_whitespace() {
                cleaned.push(bytes[at]);
            }
            at += 1;
        }
    }

    while cleaned.len() % 4 != 0 {
        cleaned.push(b'=');
    }
    match BASE64_STANDARD.decode(&cleaned) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            log::warn!("Base64 decode failed for stream: {}", error);
            String::from_utf8_lossy(&cleaned).into_owned()
        }
    }
}

pub fn parse_stream_entries(decoded: &str) -> Vec<StreamEntry> {
    // Preserve server order: the first URL is primary, the rest are fallbacks.
    let mut entries = Vec::new();
    let mut primary_urls = std::collections::HashSet::new();
    let chunks: Vec<&str> = decoded.split(',').collect();

    for chunk in chunks {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        // The closing bracket has to follow the opening one; a chunk that
        // spells them the other way round would slice a reversed range.
        let raw_q = match (chunk.find('['), chunk.find(']')) {
            (Some(start), Some(end)) if start < end => &chunk[start + 1..end],
            _ => "Unknown",
        };
        let quality = clean_quality_label(raw_q);

        let url_part = if let Some(end) = chunk.find(']') {
            &chunk[end + 1..]
        } else {
            chunk
        };

        let is_premium = chunk.contains("pjs-prem-quality")
            || raw_q.contains("Ultra")
            || raw_q.contains("4K")
            || raw_q.contains("2160")
            || raw_q.contains("1440")
            || raw_q.contains("2K")
            || quality.contains("Ultra")
            || quality.contains("4K");

        let urls: Vec<String> = url_part
            .split(" or ")
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let urls = with_cdn_mirrors(urls);
        let Some(primary) = urls.first() else {
            continue;
        };
        if !primary_urls.insert(primary.clone()) {
            continue;
        }

        entries.push(StreamEntry {
            quality,
            is_premium,
            urls,
        });
    }

    // Sort descending by quality priority
    entries.sort_by_key(|entry| std::cmp::Reverse(quality_rank(&entry.quality)));

    entries
}

/// Follows `urls` with the same files on every other known CDN host, so a
/// player that cannot reach the provider's choice has somewhere to go next.
fn with_cdn_mirrors(urls: Vec<String>) -> Vec<String> {
    let mut all = urls.clone();
    for url in &urls {
        let Ok(parsed) = url::Url::parse(url) else {
            continue;
        };
        let Some(own_host) = parsed.host_str().filter(|host| CDN_HOSTS.contains(host)) else {
            continue;
        };
        for host in CDN_HOSTS.into_iter().filter(|host| *host != own_host) {
            let mut mirror = parsed.clone();
            if mirror.set_host(Some(host)).is_err() {
                continue;
            }
            let mirror = String::from(mirror);
            if !all.contains(&mirror) {
                all.push(mirror);
            }
        }
    }
    all
}

/// Puts a CDN host that answers at the front of every stream's URLs.
///
/// Players other than the Android one only ever see the first URL, so one on
/// a host that is down would fail however many alternatives follow it.
async fn prefer_reachable_cdn(session: &RezkaSession, bundle: &mut StreamBundle) {
    let client = match reqwest::Client::builder()
        .user_agent(session.user_agent())
        .connect_timeout(CDN_PROBE_TIMEOUT)
        .timeout(CDN_PROBE_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            log::warn!("Could not build the CDN probe client: {}", error);
            return;
        }
    };
    promote_reachable_url(
        &client,
        session.referer(),
        &UNREACHABLE_CDNS,
        &mut bundle.streams,
    )
    .await;
}

async fn promote_reachable_url(
    client: &reqwest::Client,
    referer: &str,
    unreachable: &Mutex<HashSet<String>>,
    streams: &mut [StreamEntry],
) {
    // Every quality lives on the same hosts, so one of them speaks for all;
    // premium ones may refuse an account without a subscription.
    let Some(entry) = streams
        .iter()
        .filter(|entry| entry.urls.len() > 1)
        .min_by_key(|entry| entry.is_premium)
    else {
        return;
    };
    // One request per host is enough to tell whether it is up.
    let mut origins = HashSet::new();
    let candidates: Vec<String> = entry
        .urls
        .iter()
        .filter(|url| origins.insert(url_origin(url)))
        .cloned()
        .collect();
    if candidates.len() < 2 {
        return;
    }
    match reachable_candidate(client, referer, unreachable, &candidates).await {
        Some(0) => {}
        Some(index) => {
            let origin = url_origin(&candidates[index]);
            log::info!(
                "Stream host {} did not answer, playing from {}",
                url_origin(&candidates[0]),
                origin
            );
            for entry in streams.iter_mut() {
                if let Some(at) = entry.urls.iter().position(|url| url_origin(url) == origin) {
                    let url = entry.urls.remove(at);
                    entry.urls.insert(0, url);
                }
            }
        }
        None => log::warn!("No stream host answered; keeping the provider's order"),
    }
}

/// Picks the candidate playback should start from: the first one whenever
/// it answers, since that is the provider's choice, and otherwise whichever
/// of the rest answered first. A first candidate on a host in `unreachable`
/// only wins by answering before all the others.
async fn reachable_candidate(
    client: &reqwest::Client,
    referer: &str,
    unreachable: &Mutex<HashSet<String>>,
    candidates: &[String],
) -> Option<usize> {
    let first_origin = url_origin(&candidates[0]);
    let mut first_failed = unreachable
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&first_origin);

    let mut probes = tokio::task::JoinSet::new();
    for (index, url) in candidates.iter().enumerate() {
        let request = client
            .get(url)
            .header(REFERER, referer)
            .header(RANGE, "bytes=0-0");
        probes.spawn(async move {
            let answered = request
                .send()
                .await
                .is_ok_and(|response| response.status().is_success());
            (index, answered)
        });
    }

    let mut first_answer = None;
    while let Some(result) = probes.join_next().await {
        let Ok((index, answered)) = result else {
            continue;
        };
        if index == 0 {
            let mut unreachable = unreachable.lock().unwrap_or_else(PoisonError::into_inner);
            if answered {
                unreachable.remove(&first_origin);
                return Some(0);
            }
            unreachable.insert(first_origin.clone());
            first_failed = true;
        } else if answered && first_answer.is_none() {
            first_answer = Some(index);
        }
        if first_failed && first_answer.is_some() {
            break;
        }
    }
    first_answer
}

fn url_origin(url: &str) -> String {
    url::Url::parse(url)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

/// Strips any markup the provider leaves in a quality label.
static TAG_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<[^>]*>").expect("static regex"));

fn clean_quality_label(raw: &str) -> String {
    let cleaned = TAG_REGEX.replace_all(raw, "").trim().to_string();
    if cleaned.is_empty() {
        raw.to_string()
    } else if cleaned.contains("2160") || cleaned.contains("4K") {
        "4K (2160p)".to_string()
    } else if cleaned.contains("1440") || cleaned.contains("2K") {
        "2K (1440p)".to_string()
    } else if cleaned.contains("1080p Ultra")
        || (cleaned.contains("1080") && cleaned.contains("Ultra"))
    {
        "1080p Ultra".to_string()
    } else if cleaned.contains("1080") {
        "1080p".to_string()
    } else if cleaned.contains("720") {
        "720p".to_string()
    } else if cleaned.contains("480") {
        "480p".to_string()
    } else if cleaned.contains("360") {
        "360p".to_string()
    } else {
        cleaned
    }
}

pub fn parse_subtitles(raw_sub: &str) -> Vec<SubtitleTrack> {
    parse_subtitles_with_meta(raw_sub, "", &HashMap::new())
}

/// Parses `subtitle` string plus the `subtitle_def` default language code and
/// `subtitle_lns` name→language-code map.
pub fn parse_subtitles_with_meta(
    raw_sub: &str,
    default_code: &str,
    lns: &HashMap<String, String>,
) -> Vec<SubtitleTrack> {
    let mut raw_tracks: Vec<(String, String)> = Vec::new();
    let chunks: Vec<&str> = raw_sub.split(',').collect();

    for chunk in chunks {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        if let (Some(start), Some(end)) = (chunk.find('['), chunk.find(']')) {
            let code = chunk[start + 1..end].trim().to_string();
            let url = chunk[end + 1..].trim().to_string();
            if !code.is_empty() && !url.is_empty() {
                raw_tracks.push((code, url));
            }
        }
    }

    // subtitle_lns carries proper language codes, so it wins over the raw
    // names; the subtitle_def code marks the default track.
    let mut tracks: Vec<SubtitleTrack> = Vec::new();
    for (name, url) in raw_tracks {
        let language_code = lns.get(&name).cloned();
        let is_default =
            !default_code.is_empty() && (language_code.as_deref() == Some(default_code));
        tracks.push(SubtitleTrack {
            code: name.clone(),
            title: map_subtitle_code_to_label(&name),
            url,
            language_code,
            is_default,
        });
    }
    tracks
}

fn map_subtitle_code_to_label(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "rus" | "ru" => "Русские".to_string(),
        "eng" | "en" => "English".to_string(),
        "ukr" | "ua" => "Українські".to_string(),
        "ger" | "de" => "Deutsch".to_string(),
        "fre" | "fr" => "Français".to_string(),
        "spa" | "es" => "Español".to_string(),
        _ => code.to_uppercase(),
    }
}
fn quality_rank(quality: &str) -> u32 {
    if quality.contains("2160") || quality.contains("4K") {
        60
    } else if quality.contains("1440") || quality.contains("2K") {
        50
    } else if quality.contains("1080p Ultra") || quality.contains("Ultra") {
        45
    } else if quality.contains("1080") {
        40
    } else if quality.contains("720") {
        30
    } else if quality.contains("480") {
        20
    } else if quality.contains("360") {
        10
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::super::models::StreamEntry;
    use super::super::test_support::{read_request, respond};
    use super::{decrypt_streams, parse_storyboard, parse_stream_entries, promote_reachable_url};
    use super::{BASE64_STANDARD, CDN_HOSTS, NOISE_MARKER};
    use base64::Engine;
    use std::collections::HashSet;
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Rebuilds a payload the way the site does: base64, cut into pieces, with
    /// a marker and a run of `@#!^$` wedged into every seam.
    fn obfuscate(plain: &str, pieces: usize) -> String {
        let encoded = BASE64_STANDARD.encode(plain).replace('=', "");
        let step = encoded.len().div_ceil(pieces).max(1);
        let runs = ["@@", "#!^", "$$", "^@!", "!!"];

        let mut payload = String::from("#h");
        for (index, piece) in encoded.as_bytes().chunks(step).enumerate() {
            if index > 0 {
                payload.push_str(NOISE_MARKER);
                payload.push_str(&BASE64_STANDARD.encode(runs[index % runs.len()]));
            }
            payload.push_str(std::str::from_utf8(piece).expect("base64 is ASCII"));
        }
        payload
    }

    #[test]
    fn decodes_a_payload_split_by_noise_runs() {
        let plain = "[1080p]https://cdn.example.org/video/master.m3u8 or https://cdn.example.org/video/1080.mp4";
        for pieces in 1..=8 {
            assert_eq!(
                decrypt_streams(&obfuscate(plain, pieces)),
                plain,
                "failed with {pieces} pieces"
            );
        }
    }

    #[test]
    fn decodes_a_payload_whose_padding_was_dropped() {
        // "abcd" encodes to "YWJjZA==": both pad characters are stripped above.
        assert_eq!(decrypt_streams("#hYWJjZA"), "abcd");
    }

    #[test]
    fn returns_a_plain_url_untouched() {
        let url = "https://cdn.example.org/video/1080.mp4";
        assert_eq!(decrypt_streams(url), url);
    }

    #[test]
    fn parses_storyboard_cues_and_resolves_relative_images() {
        let cues = parse_storyboard(
            "WEBVTT\n\n00:00:01.500 --> 00:00:03.000\nsprites/0.jpg#xywh=150,75,150,75\n",
            "https://hdrzk.org/storyboards/video/index.vtt",
        );

        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_ms, 1_500);
        assert_eq!(cues[0].end_ms, 3_000);
        assert_eq!(
            cues[0].image_url,
            "https://hdrzk.org/storyboards/video/sprites/0.jpg"
        );
        assert_eq!(
            (cues[0].x, cues[0].y, cues[0].width, cues[0].height),
            (150, 75, 150, 75)
        );
    }

    #[test]
    fn offers_every_stream_on_the_other_cdn_hosts() {
        let entries = parse_stream_entries(
            "[720p]https://prx-ams.ukrtelcdn.net/s/a/720.mp4:hls:manifest.m3u8 or https://prx-ams.ukrtelcdn.net/s/a/720.mp4",
        );

        let urls = &entries[0].urls;
        assert_eq!(
            &urls[..2],
            [
                "https://prx-ams.ukrtelcdn.net/s/a/720.mp4:hls:manifest.m3u8",
                "https://prx-ams.ukrtelcdn.net/s/a/720.mp4",
            ]
        );
        assert_eq!(urls.len(), 2 * CDN_HOSTS.len());
        assert!(
            urls.contains(&"https://stream.voidboost.cc/s/a/720.mp4:hls:manifest.m3u8".to_string())
        );
        assert!(urls.contains(&"https://prx-cogent.ukrtelcdn.net/s/a/720.mp4".to_string()));
    }

    #[test]
    fn leaves_urls_on_other_hosts_alone() {
        let entries = parse_stream_entries("[720p]https://cdn.example.com/720.mp4");
        assert_eq!(entries[0].urls, ["https://cdn.example.com/720.mp4"]);
    }

    /// A host that answers every request with a success.
    fn live_host() -> String {
        slow_host(Duration::ZERO)
    }

    /// A host that answers every request with a success after `delay`.
    fn slow_host(delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                read_request(&mut stream);
                std::thread::sleep(delay);
                respond(&mut stream, "video/mp4", "x");
            }
        });
        origin
    }

    /// A host nothing is listening on.
    fn dead_host() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    }

    fn entry(quality: &str, is_premium: bool, hosts: &[&str]) -> StreamEntry {
        StreamEntry {
            quality: quality.to_string(),
            is_premium,
            urls: hosts
                .iter()
                .map(|host| format!("{host}/{quality}.mp4"))
                .collect(),
        }
    }

    #[tokio::test]
    async fn plays_from_a_host_that_answers_when_the_first_does_not() {
        let (dead, live) = (dead_host(), live_host());
        let unreachable = Mutex::default();
        let mut streams = vec![
            entry("1080p", false, &[&dead, &live]),
            entry("720p", false, &[&dead, &live]),
        ];

        promote(&unreachable, &mut streams).await;

        for stream in &streams {
            assert_eq!(stream.urls[0], format!("{live}/{}.mp4", stream.quality));
            assert_eq!(stream.urls[1], format!("{dead}/{}.mp4", stream.quality));
        }
        assert!(unreachable.lock().unwrap().contains(&dead));
    }

    #[tokio::test]
    async fn stops_waiting_for_a_host_that_did_not_answer_before() {
        let (slow, live) = (slow_host(Duration::from_secs(1)), live_host());
        let unreachable = Mutex::new(HashSet::from([slow.clone()]));
        let mut streams = vec![entry("720p", false, &[&slow, &live])];

        promote(&unreachable, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{live}/720p.mp4"));
    }

    #[tokio::test]
    async fn trusts_a_host_again_once_it_answers() {
        let (first, second) = (live_host(), dead_host());
        let unreachable = Mutex::new(HashSet::from([first.clone()]));
        let mut streams = vec![entry("720p", false, &[&first, &second])];

        promote(&unreachable, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
        assert!(unreachable.lock().unwrap().is_empty());
    }

    async fn promote(unreachable: &Mutex<HashSet<String>>, streams: &mut [StreamEntry]) {
        promote_reachable_url(
            &reqwest::Client::new(),
            "https://hdrzk.org",
            unreachable,
            streams,
        )
        .await;
    }

    #[tokio::test]
    async fn keeps_the_providers_host_while_it_answers() {
        let (first, second) = (live_host(), live_host());
        let unreachable = Mutex::default();
        let mut streams = vec![entry("720p", false, &[&first, &second])];

        promote(&unreachable, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
    }

    #[tokio::test]
    async fn keeps_the_order_when_no_host_answers() {
        let (first, second) = (dead_host(), dead_host());
        let unreachable = Mutex::default();
        let mut streams = vec![entry("720p", false, &[&first, &second])];

        promote(&unreachable, &mut streams).await;

        assert_eq!(streams[0].urls[0], format!("{first}/720p.mp4"));
    }
}
