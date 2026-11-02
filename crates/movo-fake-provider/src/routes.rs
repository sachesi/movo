//! What each path answers.

use crate::catalog::{self, Kind, Title, ANNOUNCED_TITLE_ID, REFUSED_SERIES_ID};
use crate::http::Response;
use crate::pages::{self, DetailsState};
use crate::state::{Account, State};
use crate::{lock, Fault, FaultKind, Request, Shared};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;

const ANUBIS_COOKIE: &str = "techaro.lol-anubis-auth";
const ANUBIS_ID: &str = "fake-challenge";

/// The harness's own endpoints, for a provider running as a separate process.
pub(crate) fn control(request: &Request, shared: &Shared) -> Option<Response> {
    let action = request.path.strip_prefix("/__fake/")?;
    let reply = match action {
        "faults" => {
            let Some(fault) = fault_from_query(request) else {
                return Some(Response::new(400, "text/plain", "unknown fault"));
            };
            lock(&shared.faults).push(fault);
            Response::new(200, "text/plain", "ok")
        }
        "clear" => {
            lock(&shared.faults).clear();
            *lock(&shared.anubis_difficulty) = None;
            Response::new(200, "text/plain", "ok")
        }
        "requests" => {
            let lines: Vec<String> = lock(&shared.log)
                .iter()
                .map(|logged| format!("{} {}", logged.method, logged.path))
                .collect();
            Response::new(200, "text/plain", lines.join("\n"))
        }
        "reset-log" => {
            lock(&shared.log).clear();
            Response::new(200, "text/plain", "ok")
        }
        _ => Response::not_found(),
    };
    Some(reply)
}

fn fault_from_query(request: &Request) -> Option<Fault> {
    let path = request.query.get("path").cloned().unwrap_or_default();
    let number = |name: &str| {
        request
            .query
            .get(name)
            .and_then(|value| value.parse::<u64>().ok())
    };
    let kind = match request.query.get("kind")?.as_str() {
        "status" => FaultKind::Status(u16::try_from(number("value")?).ok()?),
        "delay" => FaultKind::Delay(Duration::from_millis(number("value")?)),
        "garbage" => FaultKind::Garbage,
        "empty" => FaultKind::Empty,
        "drop" => FaultKind::Drop,
        "lose-reply" => FaultKind::LoseReply,
        "limit" => FaultKind::LimitRefusal,
        "expired" => FaultKind::SessionExpired,
        _ => return None,
    };
    let remaining = number("times").and_then(|times| u32::try_from(times).ok());
    Some(Fault {
        path_contains: path,
        kind,
        remaining,
    })
}

pub(crate) fn respond(request: &Request, shared: &Shared, base: &str) -> Response {
    if request.path == "/.within.website/x/cmd/anubis/api/pass-challenge" {
        return pass_anubis(request, shared);
    }
    if let Some(difficulty) = *lock(&shared.anubis_difficulty) {
        if !request.cookies.contains_key(ANUBIS_COOKIE) {
            return Response::html(pages::anubis_challenge(difficulty, ANUBIS_ID));
        }
    }
    let mut state = lock(&shared.state);
    if request.method == "POST" {
        post(request, &mut state, base)
    } else {
        get(request, &state, base)
    }
}

fn pass_anubis(request: &Request, shared: &Shared) -> Response {
    let difficulty = lock(&shared.anubis_difficulty).unwrap_or(0);
    let nonce = request.query.get("nonce").cloned().unwrap_or_default();
    let answer = request.query.get("response").cloned().unwrap_or_default();
    let expected = hex::encode(Sha256::digest(format!("seed-{ANUBIS_ID}{nonce}")));
    let echoed = request
        .cookies
        .contains_key("techaro.lol-anubis-cookie-verification");
    let solved = answer == expected
        && expected.bytes().take(difficulty).all(|byte| byte == b'0')
        && request.query.get("id").map(String::as_str) == Some(ANUBIS_ID);
    if !solved || !echoed {
        return Response::new(403, "text/html", "wrong answer");
    }
    Response::new(302, "text/html", "")
        .with_header("Location", "/")
        .with_header("Set-Cookie", format!("{ANUBIS_COOKIE}=passed; Path=/"))
}

fn page_number(request: &Request, path: &str) -> usize {
    request
        .query
        .get("page")
        .and_then(|page| page.parse().ok())
        .or_else(|| {
            path.split("/page/")
                .nth(1)?
                .trim_end_matches('/')
                .parse()
                .ok()
        })
        .unwrap_or(1)
}

fn get(request: &Request, state: &State, base: &str) -> Response {
    let path = request.path.as_str();
    let account = state.account_for(&request.cookies);
    let page = page_number(request, path);
    let trimmed = path.trim_matches('/');
    let section = trimmed.split("/page/").next().unwrap_or_default();

    if let Some(id) = catalog::id_from_path(path) {
        return details_page(id, state, account, base);
    }
    if path.starts_with("/poster/") || path.starts_with("/avatar/") {
        return poster(path);
    }
    if path.starts_with("/stream/") {
        return stream_clip(request.range.as_deref());
    }
    if path.starts_with("/subtitle/") {
        return Response::new(200, "text/vtt", pages::SUBTITLE_VTT);
    }
    if path.starts_with("/storyboard/") {
        return Response::new(200, "text/vtt", pages::storyboard_vtt(base));
    }
    if path.starts_with("/person/") {
        let films: Vec<Title> = catalog::all().take(8).collect();
        return Response::html(pages::actor(base, "Иван Актёр", &films));
    }
    if path == "/ajax/get_comments" {
        let post_id = request
            .query
            .get("news_id")
            .and_then(|id| id.parse().ok())
            .unwrap_or(0);
        let wanted = request
            .query
            .get("cstart")
            .and_then(|page| page.parse().ok())
            .unwrap_or(1);
        let (comments, navigation) = pages::comments_page(post_id, wanted);
        return Response::json(
            &json!({ "success": true, "comments": comments, "navigation": navigation }),
        );
    }
    if let Some(account) = account {
        match section {
            "continue" => return Response::html(pages::history(base, account, &state.history)),
            "favorites" => return favorites_page(state, account, None, page, base),
            _ => {}
        }
        if let Some(category) = section
            .strip_prefix("favorites/")
            .and_then(|rest| rest.parse().ok())
        {
            return favorites_page(state, account, Some(category), page, base);
        }
        if section.starts_with("user/") {
            return Response::html(pages::profile(account));
        }
    } else if section == "continue"
        || section.starts_with("favorites")
        || section.starts_with("user/")
    {
        return Response::html(pages::document(None, "Вход", ""));
    }
    if section == "collections" {
        return Response::html(pages::collections(base));
    }
    if section == "search" {
        let query = request
            .query
            .get("q")
            .map(|query| query.to_lowercase())
            .unwrap_or_default();
        let found: Vec<Title> = catalog::all()
            .filter(|title| title.name.to_lowercase().contains(&query))
            .collect();
        return listing_response(&found, page, account, base);
    }
    if section == "announce" {
        let announced: Vec<Title> = catalog::title(ANNOUNCED_TITLE_ID).into_iter().collect();
        return listing_response(&announced, page, account, base);
    }
    if section == "new"
        || section.starts_with("collections/")
        || section.starts_with("country/")
        || section.starts_with("year/")
    {
        let all: Vec<Title> = catalog::all().collect();
        return listing_response(&all, page, account, base);
    }
    if section.is_empty() {
        let tracked: Vec<Title> = state
            .tracked_series
            .iter()
            .filter_map(|id| catalog::title(*id))
            .collect();
        let tracked = if account.is_some() {
            tracked
        } else {
            Vec::new()
        };
        let all: Vec<Title> = catalog::all().collect();
        return Response::html(pages::home_with_notifications(
            base,
            account,
            &catalog::page_of(&all, page),
            &tracked,
        ));
    }
    let kind = section.split('/').next().and_then(Kind::from_path);
    if let Some(kind) = kind {
        return listing_response(&catalog::of_kind(kind), page, account, base);
    }
    Response::not_found()
}

fn listing_response(
    titles: &[Title],
    page: usize,
    account: Option<&Account>,
    base: &str,
) -> Response {
    Response::html(pages::listing_page(
        base,
        account,
        &catalog::page_of(titles, page),
    ))
}

fn favorites_page(
    state: &State,
    account: &Account,
    category: Option<i64>,
    page: usize,
    base: &str,
) -> Response {
    let counts: Vec<usize> = crate::state::FAVORITE_CATEGORIES
        .iter()
        .map(|(id, _)| state.favorites.get(id).map_or(0, Vec::len))
        .collect();
    let ids = state
        .favorites
        .get(&category.unwrap_or(1))
        .cloned()
        .unwrap_or_default();
    let titles: Vec<Title> = ids.into_iter().filter_map(catalog::title).collect();
    Response::html(pages::favorites(
        base,
        account,
        &counts,
        &catalog::page_of(&titles, page),
    ))
}

fn details_page(id: i64, state: &State, account: Option<&Account>, base: &str) -> Response {
    let Some(title) = catalog::title(id) else {
        return Response::not_found();
    };
    let watched: Vec<String> = state.watched_schedule.iter().cloned().collect();
    let categories = if account.is_some() {
        state.categories_of(id)
    } else {
        Vec::new()
    };
    Response::html(pages::details(
        base,
        &title,
        &DetailsState {
            account,
            favorite_categories: &categories,
            posted_rating: state.rated.contains_key(&id),
            watched_rows: &watched,
        },
    ))
}

/// Eight seconds of real video, so a player has something it can decode, served
/// in the pieces a player asks for when it seeks.
fn stream_clip(range: Option<&str>) -> Response {
    let clip: &[u8] = include_bytes!("../assets/clip.mp4");
    let wanted = range
        .and_then(|range| range.strip_prefix("bytes="))
        .and_then(|range| range.split_once('-'))
        .and_then(|(start, end)| {
            let start: usize = start.parse().ok()?;
            let end = end.parse().unwrap_or(clip.len() - 1).min(clip.len() - 1);
            (start <= end).then_some((start, end))
        });
    let Some((start, end)) = wanted else {
        return Response::new(200, "video/mp4", clip).with_header("Accept-Ranges", "bytes");
    };
    Response::new(206, "video/mp4", &clip[start..=end])
        .with_header("Accept-Ranges", "bytes")
        .with_header(
            "Content-Range",
            format!("bytes {start}-{end}/{}", clip.len()),
        )
}

fn poster(path: &str) -> Response {
    let id: u32 = path
        .rsplit('/')
        .next()
        .and_then(|name| name.split('.').next())
        .and_then(|id| id.parse().ok())
        .unwrap_or(0);
    Response::new(200, "image/png", solid_png(id))
}

/// A one-colour image whose colour depends on `seed`, so neighbouring posters
/// can be told apart in a screenshot.
fn solid_png(seed: u32) -> Vec<u8> {
    const WIDTH: u32 = 90;
    const HEIGHT: u32 = 135;
    let hue = seed.wrapping_mul(47) % 256;
    let colour = [hue as u8, (255 - hue) as u8, (hue / 2 + 64) as u8];
    let mut row = vec![0];
    row.extend(colour.repeat(WIDTH as usize));
    let raw = row.repeat(HEIGHT as usize);

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&WIDTH.to_be_bytes());
    header.extend_from_slice(&HEIGHT.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &header);
    png_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    png_chunk(&mut png, b"IEND", &[]);
    png
}

fn png_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Wraps `data` in a zlib stream of uncompressed blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut chunks = data.chunks(0xFFFF).peekable();
    while let Some(chunk) = chunks.next() {
        out.push(u8::from(chunks.peek().is_none()));
        let length = chunk.len() as u16;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    let (mut low, mut high) = (1u32, 0u32);
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    out.extend_from_slice(&((high << 16) | low).to_be_bytes());
    out
}

fn post(request: &Request, state: &mut State, base: &str) -> Response {
    let path = request.path.trim_matches('/');
    let form = &request.form;
    let field = |name: &str| form.get(name).map(String::as_str).unwrap_or_default();
    let number = |name: &str| field(name).parse::<i64>().unwrap_or(0);
    let signed_in = state.account_for(&request.cookies).is_some();
    let refused =
        || Response::json(&json!({ "success": false, "message": "Требуется авторизация" }));

    match path {
        "ajax/login" => {
            if !State::accepts_credentials(field("login_name"), field("login_password")) {
                return Response::json(&json!({
                    "success": false,
                    "message": "<b>Неверный</b> логин или пароль",
                }));
            }
            state.sessions_valid = true;
            let [user, password] = state.session_cookies();
            Response::json(&json!({ "success": true }))
                .with_header("Set-Cookie", user)
                .with_header("Set-Cookie", password)
        }
        "ajax/get_cdn_series" => cdn_series(request, base),
        "engine/ajax/get_newest_slider_content.php" => {
            let hot: Vec<Title> = catalog::all().take(12).collect();
            Response::html(pages::listing(base, &hot))
        }
        "engine/ajax/search.php" => {
            let query = field("q").to_lowercase();
            let rows: String = catalog::all()
                .filter(|title| title.name.to_lowercase().contains(&query))
                .take(5)
                .map(|title| {
                    format!(
                        r#"<li><a href="{base}{path}"><span class="enty">{name}</span></a></li>"#,
                        path = title.url_path(),
                        name = pages::escape(&title.name),
                    )
                })
                .collect();
            Response::html(format!("<ul>{rows}</ul>"))
        }
        "engine/ajax/gettrailervideo.php" => Response::json(&json!({
            "success": true,
            "code": format!(
                r#"<iframe src="{}/trailer/{}"></iframe>"#,
                base.replace("http://", "https://"),
                number("id")
            ),
        })),
        "engine/ajax/rating.php" if signed_in => {
            state
                .rated
                .insert(number("news_id"), number("go_rate") as u8);
            Response::json(&json!({ "success": true }))
        }
        "engine/ajax/comments_like.php" if signed_in => {
            state.liked_comments.insert(field("id").to_string());
            Response::json(&json!({ "success": true }))
        }
        "ajax/favorites" if signed_in => {
            let accepted = state.toggle_favorite(number("cat_id"), number("post_id"));
            Response::json(&json!({ "success": accepted }))
        }
        "ajax/send_save" if signed_in => {
            let position = match (number("season"), number("episode")) {
                (0, _) | (_, 0) => None,
                position => Some(position),
            };
            state.save_watch(number("post_id"), position);
            Response::json(&json!({ "success": true }))
        }
        "engine/ajax/cdn_saves_remove.php" if signed_in => {
            let id = field("id");
            let before = state.history.len();
            state.history.retain(|row| row.id != id);
            Response::json(&json!({ "success": state.history.len() < before }))
        }
        "engine/ajax/cdn_saves_view.php" if signed_in => {
            let id = field("id");
            let row = state.history.iter_mut().find(|row| row.id == id);
            let found = row.is_some();
            if let Some(row) = row {
                row.watched = !row.watched;
            }
            Response::json(&json!({ "success": found }))
        }
        "engine/ajax/schedule_watched.php" if signed_in => {
            let id = field("id").to_string();
            if !state.watched_schedule.remove(&id) {
                state.watched_schedule.insert(id);
            }
            Response::json(&json!({ "success": true }))
        }
        "ajax/favorites"
        | "ajax/send_save"
        | "engine/ajax/rating.php"
        | "engine/ajax/comments_like.php"
        | "engine/ajax/cdn_saves_remove.php"
        | "engine/ajax/cdn_saves_view.php"
        | "engine/ajax/schedule_watched.php" => refused(),
        _ => Response::not_found(),
    }
}

fn cdn_series(request: &Request, base: &str) -> Response {
    let form = &request.form;
    let number = |name: &str| {
        form.get(name)
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0)
    };
    let post_id = number("id");
    let translator = number("translator_id");
    let Some(title) = catalog::title(post_id) else {
        return Response::json(&json!({ "success": false, "message": "Не найдено" }));
    };
    let action = form.get("action").map(String::as_str).unwrap_or_default();
    if post_id == REFUSED_SERIES_ID {
        return Response::json(&json!({
            "success": false,
            "message": "Не удалось найти выбранную озвучку",
        }));
    }
    match action {
        "get_episodes" => {
            let (seasons, episodes) = pages::episodes(&title);
            Response::json(&json!({ "success": true, "seasons": seasons, "episodes": episodes }))
        }
        "get_movie" | "get_stream" => {
            let part = if action == "get_movie" {
                "movie".to_string()
            } else {
                format!("{}x{}", number("season"), number("episode"))
            };
            Response::json(&json!({
                "success": true,
                "url": pages::encoded_streams(base, post_id, translator, &part),
                "subtitle": pages::subtitles(base, post_id),
                "subtitle_def": "ru",
                "subtitle_lns": { "Русский": "ru", "English": "en" },
                "thumbnails": format!("/storyboard/{post_id}.vtt"),
            }))
        }
        other => Response::json(
            &json!({ "success": false, "message": format!("unknown action {other}") }),
        ),
    }
}
