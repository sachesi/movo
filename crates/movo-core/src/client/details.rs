use super::catalog;
use super::models::{
    FranchisePart, LinkedItem, MediaDetails, MediaType, Person, Rating, ScheduleGroup, Translator,
    VoiceRating,
};
use super::session::RezkaSession;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

/// The numeric score in a rating line such as "IMDb: 7.8 (12 345)".
static RATING_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d+(?:[.,]\d+)?").expect("static regex"));
/// The vote count in parentheses in the same line.
static RATING_VOTES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(([\d\s]+)\)").expect("static regex"));

mod actor;
mod comments;
mod episodes;
mod schedule;

// The scrapers split out of this module stay part of its surface.
pub use actor::fetch_actor;
pub use comments::{fetch_comments, like_comment};
pub use episodes::fetch_episodes;
use episodes::{request_episodes, Episodes};
pub use schedule::{labelled_position, mark_scheduled, schedule_item_for};

pub async fn fetch_details(session: &RezkaSession, url: &str) -> Result<MediaDetails, String> {
    let html_content = session.get_html(url).await?;
    let mut details = parse_details_html(&html_content, url)?;
    details.poster_url = details.poster_url.map(|value| session.resolve_url(&value));
    details.poster_hq_url = details
        .poster_hq_url
        .map(|value| session.resolve_url(&value));
    for person in details
        .directors_details
        .iter_mut()
        .chain(details.actors_details.iter_mut())
    {
        person.photo_url = person
            .photo_url
            .take()
            .map(|value| session.resolve_url(&value));
    }

    // If it's a TV series, fetch seasons and episodes for the first translator
    if details.media_type == MediaType::TVSeries && !details.translators.is_empty() {
        let first_tr_id = details.translators[0].id;
        if let Ok(reply) = request_episodes(session, details.id, first_tr_id).await {
            take_first_episodes(&mut details, reply);
        }
    }
    mark_scheduled(&mut details.seasons, &details.schedules);

    Ok(details)
}

/// Takes the provider's answer for a series' first voice-over. When that is
/// the only voice-over and the provider has no episodes under it, the series
/// has nothing to play, so none is offered: the app would otherwise offer Play
/// and meet the same refusal.
fn take_first_episodes(details: &mut MediaDetails, reply: Episodes) {
    match reply {
        Episodes::Listed(seasons) => details.seasons = seasons,
        Episodes::Refused(_) if details.translators.len() == 1 => details.translators.clear(),
        Episodes::Refused(_) => {}
    }
}

/// The schedule table of a title's page, where the provider keeps the
/// watched flag of every episode it lists.
pub async fn fetch_schedules(
    session: &RezkaSession,
    url: &str,
) -> Result<Vec<ScheduleGroup>, String> {
    let html = session.get_html(url).await?;
    Ok(parse_details_html(&html, url)?.schedules)
}

pub fn parse_details_html(html: &str, url: &str) -> Result<MediaDetails, String> {
    let document = Html::parse_document(html);

    let id = extract_post_id(&document, url)
        .ok_or_else(|| "The provider page has no valid media ID".to_string())?;

    let title_selector = Selector::parse(".b-post__title h1, .b-post__title").unwrap();
    let orig_title_selector = Selector::parse(".b-post__origtitle").unwrap();
    let desc_selector = Selector::parse(".b-post__description_text").unwrap();
    let cover_img_selector = Selector::parse(".b-sidecover img").unwrap();
    let cover_link_selector = Selector::parse(".b-sidecover a").unwrap();
    let rating_num_selector = Selector::parse(".b-post__rating .num").unwrap();
    let meta_type_selector = Selector::parse("meta[property=\"og:type\"]").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_else(|| "Без названия".to_string());

    let orig_title = document
        .select(&orig_title_selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    let description = document
        .select(&desc_selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let poster_url = document
        .select(&cover_img_selector)
        .next()
        .and_then(|el| el.value().attr("src").map(|s| s.to_string()));

    let poster_hq_url = document
        .select(&cover_link_selector)
        .next()
        .and_then(|el| el.value().attr("href").map(|s| s.to_string()));

    let rating_rezka = document
        .select(&rating_num_selector)
        .next()
        .and_then(|el| el.text().collect::<String>().trim().parse::<f32>().ok());

    let media_type = if let Some(meta) = document.select(&meta_type_selector).next() {
        match meta.value().attr("content") {
            Some("video.tv_series") => MediaType::TVSeries,
            Some("video.movie") => MediaType::Movie,
            _ => {
                if url.contains("/series/") {
                    MediaType::TVSeries
                } else if url.contains("/cartoons/") {
                    MediaType::Cartoon
                } else if url.contains("/animation/") {
                    MediaType::Anime
                } else {
                    MediaType::Movie
                }
            }
        }
    } else if url.contains("/series/") {
        MediaType::TVSeries
    } else {
        MediaType::Movie
    };

    let mut year = None;
    let mut genres = Vec::new();
    let mut countries = Vec::new();
    let mut directors = Vec::new();
    let mut actors = Vec::new();
    let mut duration = None;
    let mut rating_imdb = None;
    let mut rating_kp = None;
    let mut genre_links = Vec::new();
    let mut country_links = Vec::new();
    let mut directors_details = Vec::new();
    let mut actors_details = Vec::new();
    let mut ratings = Vec::new();
    let mut included_in = Vec::new();
    let mut from_collections = Vec::new();

    let table_row_selector = Selector::parse(".b-post__info tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let a_selector = Selector::parse("a").unwrap();

    for row in document.select(&table_row_selector) {
        let cells: Vec<_> = row.select(&td_selector).collect();
        if cells.len() >= 2 {
            let key = cells[0].text().collect::<String>().trim().to_lowercase();
            let val_cell = &cells[1];

            if key.contains("год") || key.contains("дата выхода") {
                for link in val_cell.select(&a_selector) {
                    let text = link.text().collect::<String>();
                    if let Ok(y) = text.trim().parse::<i32>() {
                        year = Some(y);
                        break;
                    }
                }
            } else if key.contains("страна") {
                countries = val_cell
                    .select(&a_selector)
                    .map(|a| a.text().collect::<String>().trim().to_string())
                    .collect();
                country_links = linked_items(val_cell);
            } else if key.contains("жанр") {
                genres = val_cell
                    .select(&a_selector)
                    .map(|a| a.text().collect::<String>().trim().to_string())
                    .collect();
                genre_links = linked_items(val_cell);
            } else if key.contains("режиссер") {
                directors = val_cell
                    .select(&a_selector)
                    .map(|a| a.text().collect::<String>().trim().to_string())
                    .collect();
                directors_details = people(val_cell);
            } else if key.contains("в ролях") {
                actors = val_cell
                    .select(&a_selector)
                    .map(|a| a.text().collect::<String>().trim().to_string())
                    .collect();
                actors_details = people(val_cell);
            } else if key.contains("входит в списки") {
                included_in = linked_items(val_cell);
            } else if key.contains("из серии") {
                from_collections = linked_items(val_cell);
            } else if key.contains("время") {
                duration = Some(val_cell.text().collect::<String>().trim().to_string());
            } else if key.contains("рейтинг") {
                let text = val_cell.text().collect::<String>();
                if text.contains("IMDb") {
                    if let Some(pos) = text.find("IMDb:") {
                        let part = &text[pos + 5..];
                        let score_str: String = part
                            .chars()
                            .skip_while(|c| !c.is_ascii_digit())
                            .take_while(|c| c.is_ascii_digit() || *c == '.')
                            .collect();
                        rating_imdb = score_str.parse::<f32>().ok();
                    }
                }
                if text.contains("Кинопоиск") {
                    if let Some(pos) = text.find("Кинопоиск:") {
                        let part = &text[pos + 10..];
                        let score_str: String = part
                            .chars()
                            .skip_while(|c| !c.is_ascii_digit())
                            .take_while(|c| c.is_ascii_digit() || *c == '.')
                            .collect();
                        rating_kp = score_str.parse::<f32>().ok();
                    }
                }
                ratings = val_cell
                    .select(&Selector::parse("span").unwrap())
                    .filter_map(|node| parse_rating(node.text().collect::<String>()))
                    .collect();
            }
        }
    }

    // A title announced but not released yet has no player, so no voice-over:
    // the provider turns down every one the app asks it for.
    let pending_release = document
        .select(&Selector::parse(".b-post__go_status").unwrap())
        .next()
        .is_some();

    // Parse translators list
    let mut translators = Vec::new();
    let tr_selector = Selector::parse(".b-translator__item, #translators-list li").unwrap();
    for tr in document.select(&tr_selector) {
        if let Some(tr_id_str) = tr.value().attr("data-translator_id") {
            if let Ok(tr_id) = tr_id_str.parse::<i64>() {
                let name = tr.text().collect::<String>().trim().to_string();
                let is_premium = tr
                    .value()
                    .attr("class")
                    .map(|c| c.contains("b-prem_translator"))
                    .unwrap_or(false);
                translators.push(Translator {
                    id: tr_id,
                    name,
                    is_premium,
                    is_camrip: flag(tr.value().attr("data-camrip")),
                    has_ads: flag(tr.value().attr("data-ad")),
                    is_director_cut: flag(tr.value().attr("data-director")),
                });
            }
        }
    }

    // With no list, the title's one voice-over is the one its player starts
    // with. A page with no player has none to offer.
    if translators.is_empty() {
        if let Some(tr_id) = extract_script_translator_id(html) {
            translators.push(Translator {
                id: tr_id,
                name: "По умолчанию".to_string(),
                is_premium: false,
                is_camrip: false,
                has_ads: false,
                is_director_cut: false,
            });
        }
    }
    if pending_release {
        translators.clear();
    }

    // Parse franchises/parts
    let mut franchises = Vec::new();
    let part_selector = Selector::parse(".b-post__partcontent_item").unwrap();
    for part in document.select(&part_selector) {
        let is_current = part
            .value()
            .attr("class")
            .map(|c| c.contains("current"))
            .unwrap_or(false);
        let part_title = part
            .select(&Selector::parse(".title").unwrap())
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let part_url = if is_current {
            url.to_string()
        } else {
            part.value()
                .attr("data-url")
                .unwrap_or_default()
                .to_string()
        };

        if !part_title.is_empty() {
            franchises.push(FranchisePart {
                title: part_title,
                url: part_url,
                is_current,
            });
        }
    }

    let favorite_category_ids = document
        .select(&Selector::parse(".hd-label-row input[checked]").unwrap())
        .filter_map(|input| input.value().attr("value")?.parse().ok())
        .collect();

    Ok(MediaDetails {
        id,
        title,
        orig_title,
        url: url.to_string(),
        poster_url,
        poster_hq_url,
        description,
        year,
        media_type,
        rating_rezka,
        rating_imdb,
        rating_kp,
        genres,
        countries,
        directors,
        actors,
        duration,
        translators,
        seasons: Vec::new(),
        franchises,
        genre_links,
        country_links,
        directors_details,
        actors_details,
        ratings,
        voice_ratings: parse_voice_ratings(&document),
        schedules: schedule::parse_schedules(&document),
        related: catalog::parse_catalog_html(html),
        included_in,
        from_collections,
        has_trailer: document
            .select(&Selector::parse(".b-post__trailer_button, .b-post__trailer").unwrap())
            .next()
            .is_some(),
        has_posted_rating: document
            .select(&Selector::parse(".b-rating .current").unwrap())
            .next()
            .is_some(),
        favorite_category_ids,
    })
}

fn flag(value: Option<&str>) -> bool {
    matches!(value, Some(value) if value != "0" && !value.is_empty())
}

fn linked_items(cell: &scraper::ElementRef<'_>) -> Vec<LinkedItem> {
    cell.select(&Selector::parse("a[href]").unwrap())
        .filter_map(|link| {
            let name = link.text().collect::<String>().trim().to_string();
            (!name.is_empty()).then(|| LinkedItem {
                name,
                url: link.value().attr("href").unwrap_or_default().to_string(),
            })
        })
        .collect()
}

fn people(cell: &scraper::ElementRef<'_>) -> Vec<Person> {
    let people = cell
        .select(&Selector::parse(".person-name-item").unwrap())
        .collect::<Vec<_>>();
    let nodes = if people.is_empty() {
        cell.select(&Selector::parse("a[href]").unwrap()).collect()
    } else {
        people
    };
    nodes
        .into_iter()
        .filter_map(|node| {
            let link = node
                .select(&Selector::parse("a[href]").unwrap())
                .next()
                .unwrap_or(node);
            let name = node.text().collect::<String>().trim().to_string();
            (!name.is_empty()).then(|| Person {
                name,
                url: link.value().attr("href").unwrap_or_default().to_string(),
                photo_url: node.value().attr("data-photo").map(str::to_string),
                role: node.value().attr("data-job").map(str::to_string),
            })
        })
        .collect()
}

fn parse_rating(text: String) -> Option<Rating> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let value = RATING_VALUE
        .find(&text)?
        .as_str()
        .replace(',', ".")
        .parse()
        .ok()?;
    Some(Rating {
        name: text
            .split(':')
            .next()
            .unwrap_or("Rating")
            .trim()
            .to_string(),
        value,
        votes: RATING_VOTES
            .captures(&text)
            .and_then(|capture| capture.get(1))
            .map(|votes| votes.as_str().replace(' ', ""))
            .and_then(|votes| votes.parse().ok()),
        url: None,
    })
}

fn parse_voice_ratings(document: &Html) -> Vec<VoiceRating> {
    let Some(raw) = document
        .select(&Selector::parse(".b-rgstats__help[title]").unwrap())
        .next()
        .and_then(|node| node.value().attr("title"))
    else {
        return Vec::new();
    };
    let fragment = Html::parse_fragment(raw);
    let item_selector = Selector::parse(".b-rgstats__list_item").unwrap();
    let title_selector = Selector::parse(".title").unwrap();
    let count_selector = Selector::parse(".count").unwrap();
    fragment
        .select(&item_selector)
        .filter_map(|item| {
            let title = item
                .select(&title_selector)
                .next()?
                .text()
                .collect::<String>()
                .trim()
                .to_string();
            let rating = item
                .select(&count_selector)
                .next()?
                .text()
                .collect::<String>()
                .replace('%', "")
                .replace(',', ".")
                .trim()
                .parse()
                .ok()?;
            Some(VoiceRating { title, rating })
        })
        .collect()
}

pub async fn fetch_trailer(session: &RezkaSession, post_id: i64) -> Result<Option<String>, String> {
    let id = post_id.to_string();
    let response = session
        .post_ajax("engine/ajax/gettrailervideo.php", &[("id", &id)])
        .await?;
    let value: Value = serde_json::from_str(&response)
        .map_err(|_| "Trailer response was malformed".to_string())?;
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Ok(None);
    }
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(regex::Regex::new(r#"https://[^\"'\s<]+"#)
        .unwrap()
        .find(code)
        .map(|url| url.as_str().to_string()))
}

pub async fn post_rating(session: &RezkaSession, post_id: i64, rating: u8) -> Result<(), String> {
    if !(1..=10).contains(&rating) {
        return Err("Rating must be between 1 and 10".to_string());
    }
    let id = post_id.to_string();
    let rating = rating.to_string();
    require_success(
        &session
            .post_ajax_once(
                "engine/ajax/rating.php",
                &[("news_id", &id), ("go_rate", &rating), ("skin", "hdrezka")],
            )
            .await?,
        "Failed to update rating",
    )
}

fn require_success(response: &str, fallback: &str) -> Result<(), String> {
    let value: Value = serde_json::from_str(response)
        .map_err(|_| "Provider response was malformed".to_string())?;
    if value.get("success").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(fallback.to_string())
    }
}

fn extract_post_id(doc: &Html, url: &str) -> Option<i64> {
    let holder = Selector::parse("#user-favorites-holder[data-post_id]").unwrap();
    doc.select(&holder)
        .next()
        .and_then(|element| element.value().attr("data-post_id"))
        .and_then(|id| id.parse().ok())
        .filter(|id| *id > 0)
        .or_else(|| {
            url.trim_end_matches('/')
                .rsplit('/')
                .next()?
                .trim_end_matches(".html")
                .split('-')
                .next()?
                .parse()
                .ok()
                .filter(|id| *id > 0)
        })
}

fn extract_script_translator_id(html: &str) -> Option<i64> {
    let re = regex::Regex::new(r#"initCDN(?:Movies|Series)Events\s*\(\s*\d+\s*,\s*(\d+)"#).ok()?;
    let caps = re.captures(html)?;
    caps.get(1)?.as_str().parse::<i64>().ok()
}
#[cfg(test)]
mod tests {
    use super::episodes::{parse_episodes_html, Episodes};
    use super::{parse_details_html, take_first_episodes};
    use crate::client::models::Translator;

    const SERIES_PAGE: &str = r#"
        <h1 class="b-post__title">Upcoming</h1>
        <script>sof.tv.initCDNSeriesEvents(42, 56, 1, 1, false, 'hdrezka');</script>
    "#;

    fn voice(id: i64) -> Translator {
        Translator {
            id,
            name: format!("voice {id}"),
            is_premium: false,
            is_camrip: false,
            has_ads: false,
            is_director_cut: false,
        }
    }

    #[test]
    fn a_title_without_a_list_offers_the_voice_over_its_player_starts_with() {
        let details =
            parse_details_html(SERIES_PAGE, "https://example.test/series/42-upcoming.html")
                .unwrap();

        assert_eq!(
            details.translators.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![56]
        );
    }

    #[test]
    fn a_title_not_released_yet_offers_no_voice_over() {
        let page = format!(r#"{SERIES_PAGE}<div class="b-post__go_status">Ожидается</div>"#);

        let details =
            parse_details_html(&page, "https://example.test/series/42-upcoming.html").unwrap();

        assert!(details.translators.is_empty());
    }

    #[test]
    fn a_page_without_a_player_offers_no_voice_over() {
        let details = parse_details_html(
            r#"<h1 class="b-post__title">No player</h1>"#,
            "https://example.test/films/42-no-player.html",
        )
        .unwrap();

        assert!(details.translators.is_empty());
    }

    #[test]
    fn a_series_whose_only_voice_over_has_no_episodes_offers_none() {
        let mut details =
            parse_details_html(SERIES_PAGE, "https://example.test/series/42-upcoming.html")
                .unwrap();

        take_first_episodes(&mut details, Episodes::Refused("not found".to_string()));

        assert!(details.translators.is_empty());
    }

    #[test]
    fn a_refused_first_voice_over_leaves_the_others_to_choose_from() {
        let mut details =
            parse_details_html(SERIES_PAGE, "https://example.test/series/42-upcoming.html")
                .unwrap();
        details.translators = vec![voice(1), voice(2)];

        take_first_episodes(&mut details, Episodes::Refused("not found".to_string()));

        assert_eq!(details.translators.len(), 2);
    }

    #[test]
    fn parses_account_membership_and_watched_episode_state() {
        let details = parse_details_html(
            r#"
                <h1 class="b-post__title">Test</h1>
                <div class="hd-label-row"><input value="2" checked></div>
                <div class="hd-label-row"><input value="7"></div>
            "#,
            "https://example.test/films/42-test.html",
        )
        .unwrap();
        assert_eq!(details.favorite_category_ids, vec![2]);

        let seasons = parse_episodes_html(
            r#"<li class="b-simple_season__item" data-tab_id="1">Season 1</li>"#,
            r#"<li class="b-simple_episode__item watched" data-season_id="1" data-episode_id="3" data-id="watch-9">Episode 3</li>"#,
        );
        assert_eq!(seasons[0].episodes[0].watch_id.as_deref(), Some("watch-9"));
        assert!(seasons[0].episodes[0].is_watched);
    }

    #[test]
    fn rejects_missing_or_zero_media_id() {
        assert!(parse_details_html(
            r#"<h1 class="b-post__title">Unknown</h1>"#,
            "https://example.test/films/unknown.html",
        )
        .is_err());
        assert!(parse_details_html(
            r#"<div id="user-favorites-holder" data-post_id="0"></div>"#,
            "https://example.test/films/unknown.html",
        )
        .is_err());
    }

    #[test]
    fn parses_detail_page_surfaces() {
        let details = parse_details_html(r#"
            <div id="user-favorites-holder" data-post_id="42"></div>
            <div class="b-post__title"><h1>Show</h1></div>
            <table class="b-post__info">
              <tr><td>Страна:</td><td><a href="/country/us/">США</a></td></tr>
              <tr><td>Жанр:</td><td><a href="/genre/drama/">Драма</a></td></tr>
              <tr><td>Режиссер:</td><td><span class="person-name-item" data-photo="p.jpg" data-job="director"><a href="/person/1/"><span>Director</span></a></span></td></tr>
              <tr><td>В ролях актеры:</td><td><span class="person-name-item"><a href="/person/2/"><span>Actor</span></a></span></td></tr>
              <tr><td>Из серии:</td><td><a href="/collections/x/">Saga</a></td></tr>
            </table>
            <div class="b-post__schedule_block"><div class="b-post__schedule_block_title"><span class="title">Season 1</span></div><table><tr><td>1 серия</td><td><b>Pilot</b><span>Pilot original</span></td><td><i class="watched" data-id="w1"></i></td><td>1 Jan</td><td><span class="exists-episode">✓</span></td></tr></table></div>
            <div class="b-rgstats__help" title="&lt;div class='b-rgstats__list_item'&gt;&lt;span class='title'&gt;Voice&lt;/span&gt;&lt;span class='count'&gt;87,5%&lt;/span&gt;&lt;/div&gt;"></div>
            <button class="b-post__trailer_button"></button><div class="b-rating"><span class="current"></span></div>
        "#, "https://example.test/series/42-show.html").unwrap();
        assert_eq!(details.genre_links[0].url, "/genre/drama/");
        assert_eq!(details.directors_details[0].name, "Director");
        assert_eq!(details.actors_details[0].url, "/person/2/");
        assert_eq!(details.from_collections[0].name, "Saga");
        assert_eq!(details.schedules[0].items[0].id, "w1");
        assert!(details.schedules[0].items[0].is_watched);
        // The release marker reaches the model as text, never as markup.
        assert!(details.schedules[0].items[0].is_released);
        assert_eq!(
            details.schedules[0].items[0].release_status.as_deref(),
            Some("\u{2713}")
        );
        assert_eq!(details.voice_ratings[0].rating, 87.5);
        assert!(details.has_trailer && details.has_posted_rating);
    }
}
