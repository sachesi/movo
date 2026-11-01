//! Renders pages in the markup the provider's site uses, as far as the client
//! reads it.

use crate::catalog::{Title, ANNOUNCED_TITLE_ID, MARKUP_TITLE_ID, REFUSED_SERIES_ID};
use crate::state::{Account, HistoryRow, EPISODES_PER_SEASON, FAVORITE_CATEGORIES, SEASONS};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn listing_item(base: &str, title: &Title) -> String {
    format!(
        r#"<div class="b-content__inline_item" data-id="{id}">
  <div class="b-content__inline_item-cover">
    <a href="{base}{path}"><img src="{base}/poster/{id}.png"><span class="cat {kind}"></span></a>
    <span class="rating">{rating:.1}</span>
  </div>
  <div class="b-content__inline_item-link"><a href="{base}{path}">{name}</a><div>{info}</div></div>
</div>"#,
        id = title.id,
        path = title.url_path(),
        kind = title.kind.path(),
        rating = title.rating,
        name = escape(&title.name),
        info = escape(&title.info()),
    )
}

pub fn listing(base: &str, titles: &[Title]) -> String {
    titles
        .iter()
        .map(|title| listing_item(base, title))
        .collect()
}

fn account_head(account: Option<&Account>) -> String {
    match account {
        Some(account) if account.premium_days > 0 => format!(
            r#"<div class="b-tophead-premuser">Осталось {} дней</div>"#,
            account.premium_days
        ),
        Some(_) => String::new(),
        None => {
            r#"<form id="login-form"><input name="login_name"><input name="login_password"></form>"#
                .to_string()
        }
    }
}

pub fn document(account: Option<&Account>, title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title}</title></head><body>{head}{body}</body></html>"#,
        title = escape(title),
        head = account_head(account),
    )
}

pub fn listing_page(base: &str, account: Option<&Account>, titles: &[Title]) -> String {
    document(account, "Movo", &listing(base, titles))
}

pub fn profile(account: &Account) -> String {
    format!(
        r#"<!doctype html><html><head><title>{name}</title></head><body>
<div class="b-userprofile__avatar_holder"><img src="/avatar/{id}.png"></div>
<input id="email" value="{email}">{vip}
</body></html>"#,
        name = escape(&account.username),
        id = account.user_id,
        email = escape(&account.email),
        vip = if account.premium_days > 0 {
            r#"<span class="user-vip">Премиум</span>"#
        } else {
            ""
        },
    )
}

pub fn home_with_notifications(
    base: &str,
    account: Option<&Account>,
    titles: &[Title],
    tracked: &[Title],
) -> String {
    let rows: String = tracked
        .iter()
        .enumerate()
        .map(|(index, title)| {
            format!(
                r#"<li class="tracked"><a class="b-seriesupdate__block_list_link" href="{base}{path}">{name}</a><span class="season">1 сезон</span><span class="cell-2">{episode} серия</span></li>"#,
                path = title.url_path(),
                name = escape(&title.name),
                episode = index + 1,
            )
        })
        .collect();
    let block = if rows.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="b-seriesupdate__block"><div class="b-seriesupdate__block_date">Сегодня развернуть</div><ul>{rows}</ul></div>"#
        )
    };
    document(
        account,
        "Movo",
        &format!("{block}{}", listing(base, titles)),
    )
}

pub fn favorites(base: &str, account: &Account, counts: &[usize], titles: &[Title]) -> String {
    let categories: String = FAVORITE_CATEGORIES
        .iter()
        .zip(counts)
        .map(|((id, name), count)| {
            format!(
                r#"<div class="b-favorites_content__cats_list_item" data-cat_id="{id}"><span class="name">{name}</span><span class="num-holder">{count}</span></div>"#
            )
        })
        .collect();
    document(
        Some(account),
        "Закладки",
        &format!("{categories}{}", listing(base, titles)),
    )
}

pub fn history(base: &str, account: &Account, rows: &[HistoryRow]) -> String {
    let rows: String = rows
        .iter()
        .filter_map(|row| {
            let title = crate::catalog::title(row.post_id)?;
            let position = match row.position {
                Some((season, episode)) => format!("{season} сезон {episode} серия"),
                None => String::new(),
            };
            Some(format!(
                r#"<div class="b-videosaves__list_item{watched}">
  <div class="title"><a href="{base}{path}" data-cover_url="{base}/poster/{id}.png">{name}</a></div>
  <div class="info">{position}</div>
  <div class="date">{date}</div>
  <i class="delete" data-id="{row_id}"></i>
</div>"#,
                watched = if row.watched { " watched-row" } else { "" },
                path = title.url_path(),
                id = title.id,
                name = escape(&title.name),
                date = row.date,
                row_id = row.id,
            ))
        })
        .collect();
    document(
        Some(account),
        "Продолжить",
        &format!(
            r#"<div class="b-videosaves"><div class="b-videosaves__list"><div class="b-videosaves__list_item">Заголовок таблицы</div>{rows}</div></div>"#
        ),
    )
}

pub struct DetailsState<'a> {
    pub account: Option<&'a Account>,
    pub favorite_categories: &'a [i64],
    pub posted_rating: bool,
    pub watched_rows: &'a [String],
}

fn schedule(title: &Title, watched_rows: &[String]) -> String {
    (1..=SEASONS)
        .map(|season| {
            let rows: String = (1..=EPISODES_PER_SEASON)
                .map(|episode| {
                    let id = format!("{}-{season}-{episode}", title.id);
                    let class = if watched_rows.contains(&id) {
                        "watch-episode-action watched"
                    } else {
                        "watch-episode-action"
                    };
                    format!(
                        r#"<tr><td>{season} сезон {episode} серия</td><td><b>Эпизод {episode}</b><span>Episode {episode}</span></td><td><i class="{class}" data-id="{id}"></i></td><td>{day} января 2024</td><td><i class="check"></i></td></tr>"#,
                        day = episode + 1,
                    )
                })
                .collect();
            format!(
                r#"<div class="b-post__schedule_block"><div class="b-post__schedule_block_title"><div class="title">{season} сезон</div></div><table>{rows}</table></div>"#
            )
        })
        .collect()
}

pub fn details(base: &str, title: &Title, state: &DetailsState<'_>) -> String {
    let translators = match title.id {
        ANNOUNCED_TITLE_ID => String::new(),
        REFUSED_SERIES_ID => {
            r#"<ul id="translators-list"><li class="b-translator__item" data-translator_id="56">Отозванная озвучка</li></ul>"#.to_string()
        }
        _ => {
            r#"<ul id="translators-list">
<li class="b-translator__item active" data-translator_id="238" data-camrip="0" data-ad="0" data-director="0">Оригинал</li>
<li class="b-translator__item" data-translator_id="56" data-camrip="0" data-ad="1" data-director="0">Дубляж</li>
<li class="b-translator__item b-prem_translator" data-translator_id="110" data-camrip="0" data-ad="0" data-director="1">Премиум</li>
</ul>"#.to_string()
        }
    };
    let favorites: String = FAVORITE_CATEGORIES
        .iter()
        .map(|(id, name)| {
            let checked = if state.favorite_categories.contains(id) {
                " checked"
            } else {
                ""
            };
            format!(r#"<label class="hd-label-row"><input type="checkbox" value="{id}"{checked}>{name}</label>"#)
        })
        .collect();
    let related: Vec<Title> = (1..=6)
        .filter_map(|step| crate::catalog::title((title.id + step - 1) % 240 + 1))
        .collect();
    let pending = if title.id == ANNOUNCED_TITLE_ID {
        r#"<div class="b-post__go_status">Ожидается</div>"#
    } else {
        ""
    };
    let series_schedule = if title.kind.is_episodic() && title.id != REFUSED_SERIES_ID {
        schedule(title, state.watched_rows)
    } else {
        String::new()
    };
    let current_rating = if state.posted_rating {
        r#"<div class="b-rating"><span class="current"></span></div>"#
    } else {
        ""
    };
    let og_type = if title.kind.is_episodic() {
        "video.tv_series"
    } else {
        "video.movie"
    };
    let duration = if title.kind.is_episodic() {
        "45 мин."
    } else {
        "120 мин."
    };
    let markup_note = if title.id == MARKUP_TITLE_ID {
        " &lt;i&gt;разметка в описании&lt;/i&gt; &amp; амперсанд"
    } else {
        ""
    };
    let body = format!(
        r#"<meta property="og:type" content="{og_type}">
<div id="user-favorites-holder" data-post_id="{id}"></div>
<div class="b-post__title"><h1>{name}</h1></div>
<div class="b-post__origtitle">Original {id}</div>
<div class="b-sidecover"><a href="/poster/{id}.png"><img src="/poster/{id}.png"></a></div>
<div class="b-post__rating"><span class="num">{rating:.1}</span></div>
<div class="b-post__description_text">Описание тайтла номер {id}. Сюжет изобретён для тестов.{markup_note}</div>
<table class="b-post__info">
<tr><td>Рейтинги:</td><td><span>IMDb: {rating:.1} (1 234)</span> <span>Кинопоиск: {rating:.1} (5 678)</span></td></tr>
<tr><td>Дата выхода:</td><td><a href="{base}/year/{year}/">{year}</a></td></tr>
<tr><td>Страна:</td><td><a href="{base}/country/{country}/">{country}</a></td></tr>
<tr><td>Жанр:</td><td><a href="{base}/{kind}/drama/">{genre}</a></td></tr>
<tr><td>Режиссер:</td><td><span class="person-name-item" data-photo="/avatar/7.png" data-job="Режиссер"><a href="{base}/person/7-director/">Анна Режиссёр</a></span></td></tr>
<tr><td>В ролях:</td><td><span class="person-name-item" data-photo="/avatar/8.png" data-job="Актёр"><a href="{base}/person/8-actor/">Иван Актёр</a></span></td></tr>
<tr><td>Время:</td><td>{duration}</td></tr>
<tr><td>Входит в списки:</td><td><a href="{base}/collections/1-best/">Лучшее</a></td></tr>
</table>
{pending}{translators}{series_schedule}{current_rating}
<div class="b-post__trailer_button"></div>
<div class="b-post__partcontent_item current"><div class="title">{name}</div></div>
<div class="b-post__partcontent_item" data-url="{base}/films/drama/4-title-1994.html"><div class="title">Другая часть</div></div>
{favorites}
{related}"#,
        id = title.id,
        name = escape(&title.name),
        rating = title.rating,
        year = title.year,
        country = title.country,
        kind = title.kind.path(),
        genre = title.genre,
        related = listing(base, &related),
    );
    document(state.account, &title.name, &body)
}

pub fn episodes(title: &Title) -> (String, String) {
    let seasons = (1..=SEASONS)
        .map(|season| {
            format!(
                r#"<li class="b-simple_season__item" data-tab_id="{season}">Сезон {season}</li>"#
            )
        })
        .collect();
    let episodes = (1..=SEASONS)
        .flat_map(|season| {
            (1..=EPISODES_PER_SEASON).map(move |episode| {
                format!(
                    r#"<li class="b-simple_episode__item" data-id="{id}" data-season_id="{season}" data-episode_id="{episode}">Серия {episode}</li>"#,
                    id = title.id,
                )
            })
        })
        .collect();
    (seasons, episodes)
}

/// Spells a stream list the way the player endpoint does: base64 with noise
/// runs stitched in behind `//_//` markers.
pub fn encoded_streams(base: &str, post_id: i64, translator_id: i64, part: &str) -> String {
    let stream =
        |quality: &str| format!("{base}/stream/{post_id}/{translator_id}/{part}/{quality}.mp4");
    let plain = format!(
        "[360p]{0}:hls:manifest.m3u8 or {0},[720p]{1}:hls:manifest.m3u8 or {1},[<span class=\"pjs-prem-quality\">1080p Ultra</span>]{2}",
        stream("360"),
        stream("720"),
        stream("1080u"),
    );
    let encoded = BASE64.encode(plain.as_bytes());
    let noise = BASE64.encode("@#!");
    let (head, tail) = encoded.split_at(encoded.len() / 2);
    format!("#h{head}//_//{noise}{tail}")
}

pub fn subtitles(base: &str, post_id: i64) -> String {
    format!("[Русский]{base}/subtitle/{post_id}/ru.vtt,[English]{base}/subtitle/{post_id}/en.vtt")
}

pub const SUBTITLE_VTT: &str = "WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nСубтитр тестовой записи\n";

pub fn storyboard_vtt(base: &str) -> String {
    format!(
        "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n{base}/poster/1.png#xywh=0,0,150,75\n\n00:00:10.000 --> 00:00:20.000\n{base}/poster/1.png#xywh=150,0,150,75\n"
    )
}

pub fn comments_page(post_id: i64, page: usize) -> (String, String) {
    let items: String = (1..=5)
        .map(|index| {
            let id = format!("{post_id}{page}{index}");
            let spoiler = if index == 3 {
                r#"<div class="text_spoiler">спойлер</div>"#
            } else {
                ""
            };
            format!(
                r#"<li class="comments-tree-item" data-id="{id}" data-indent="{indent}">
<div class="ava"><img src="/avatar/{index}.png"></div>
<span class="name">Зритель {index}</span><span class="date">оставлен 1 января 2024</span>
<div class="text"><div>Комментарий {index} на странице {page}{spoiler}</div></div>
<span class="b-comment__like_it" data-likes_num="{likes}"></span>
</li>"#,
                indent = usize::from(index % 2 == 0),
                likes = index * 3,
            )
        })
        .collect();
    let navigation = r#"<div class="b-navigation"><a>1</a><a>2</a><a>3</a></div>"#.to_string();
    (items, navigation)
}

pub fn actor(base: &str, name: &str, films: &[Title]) -> String {
    let body = format!(
        r#"<div class="b-post__title"><span class="t1">{name}</span><span class="t2">Original {name}</span></div>
<div class="b-sidecover"><img src="/avatar/8.png"></div>
<table class="b-post__info">
<tr><td>Дата рождения:</td><td>1 января 1970</td></tr>
<tr><td>Место рождения:</td><td>Город</td></tr>
<tr><td>Рост:</td><td>1.80 м</td></tr>
<tr><td>Карьера:</td><td><a>Актёр</a></td></tr>
</table>
<div class="b-person__career"><h2>Актёр</h2><div class="b-person__career_stats">{count} фильмов</div>{films}</div>"#,
        name = escape(name),
        count = films.len(),
        films = listing(base, films),
    );
    document(None, name, &body)
}

pub fn collections(base: &str) -> String {
    let items: String = (1..=8)
        .map(|id| {
            format!(
                r#"<div class="b-content__collections_item" data-url="{base}/collections/{id}-list/"><img class="cover" src="{base}/poster/{id}.png"><a class="title">Подборка {id}</a><span class="num">{count}</span></div>"#,
                count = id * 5,
            )
        })
        .collect();
    let nav = r#"<ul>
<li class="b-topnav__item"><a class="b-topnav__item-link" href="/films/">Фильмы</a>
<select class="select-category"><option value="drama">Драмы</option><option value="comedy">Комедии</option></select>
<select class="select-year"><option value="2020">2020</option><option value="2021">2021</option></select></li>
<li class="b-topnav__item"><a class="b-topnav__item-link" href="/series/">Сериалы</a>
<select class="select-category"><option value="drama">Драмы</option></select>
<select class="select-year"><option value="2020">2020</option></select></li>
<li class="b-topnav__item single"><a class="b-topnav__item-link" href="/new/">Новинки</a></li>
</ul>"#;
    document(None, "Подборки", &format!("{nav}{items}"))
}

pub fn anubis_challenge(difficulty: usize, id: &str) -> String {
    format!(
        r#"<!doctype html><html><head><title>Checking</title>
<script id="anubis_base_prefix" type="application/json">""</script>
<script id="anubis_challenge" type="application/json">{{"rules":{{"algorithm":"fast","difficulty":{difficulty}}},"challenge":{{"id":"{id}","randomData":"seed-{id}"}}}}</script>
</head><body>checking</body></html>"#
    )
}
