use super::catalog::CatalogScraper;
use super::models::{
    ActorDetails, ActorRole, Comment, CommentsPage, Episode, FranchisePart, LinkedItem,
    MediaDetails, MediaType, Person, Rating, ScheduleGroup, ScheduleItem, Season, Translator,
    VoiceRating,
};
use super::session::RezkaSession;
use scraper::{Html, Selector};
use serde_json::Value;

pub struct DetailsScraper;

impl DetailsScraper {
    pub async fn fetch_details(session: &RezkaSession, url: &str) -> Result<MediaDetails, String> {
        let html_content = session.get_html(url).await?;
        let mut details = Self::parse_details_html(&html_content, url)?;
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
            if let Ok(seasons) = Self::fetch_episodes(session, details.id, first_tr_id).await {
                details.seasons = seasons;
            }
        }

        Ok(details)
    }

    pub fn parse_details_html(html: &str, url: &str) -> Result<MediaDetails, String> {
        let document = Html::parse_document(html);

        let id = Self::extract_post_id(&document, url)
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
                    country_links = Self::linked_items(val_cell);
                } else if key.contains("жанр") {
                    genres = val_cell
                        .select(&a_selector)
                        .map(|a| a.text().collect::<String>().trim().to_string())
                        .collect();
                    genre_links = Self::linked_items(val_cell);
                } else if key.contains("режиссер") {
                    directors = val_cell
                        .select(&a_selector)
                        .map(|a| a.text().collect::<String>().trim().to_string())
                        .collect();
                    directors_details = Self::people(val_cell);
                } else if key.contains("в ролях") {
                    actors = val_cell
                        .select(&a_selector)
                        .map(|a| a.text().collect::<String>().trim().to_string())
                        .collect();
                    actors_details = Self::people(val_cell);
                } else if key.contains("входит в списки") {
                    included_in = Self::linked_items(val_cell);
                } else if key.contains("из серии") {
                    from_collections = Self::linked_items(val_cell);
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
                        .filter_map(|node| Self::parse_rating(node.text().collect::<String>()))
                        .collect();
                }
            }
        }

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
                        is_camrip: Self::flag(tr.value().attr("data-camrip")),
                        has_ads: Self::flag(tr.value().attr("data-ad")),
                        is_director_cut: Self::flag(tr.value().attr("data-director")),
                    });
                }
            }
        }

        // If no translators in list, try to find default from scripts
        if translators.is_empty() {
            let tr_id = Self::extract_script_translator_id(html).unwrap_or(238);
            translators.push(Translator {
                id: tr_id,
                name: "По умолчанию".to_string(),
                is_premium: false,
                is_camrip: false,
                has_ads: false,
                is_director_cut: false,
            });
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
            voice_ratings: Self::parse_voice_ratings(&document),
            schedules: Self::parse_schedules(&document),
            related: CatalogScraper::parse_catalog_html(html),
            included_in,
            from_collections,
            trailer_available: document
                .select(&Selector::parse(".b-post__trailer_button, .b-post__trailer").unwrap())
                .next()
                .is_some(),
            rating_posted: document
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
        let value = regex::Regex::new(r"\d+(?:[.,]\d+)?")
            .ok()?
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
            votes: regex::Regex::new(r"\(([\d\s]+)\)")
                .ok()?
                .captures(&text)
                .and_then(|capture| capture.get(1))
                .map(|votes| votes.as_str().replace(' ', ""))
                .and_then(|votes| votes.parse().ok()),
            url: None,
        })
    }

    fn parse_schedules(document: &Html) -> Vec<ScheduleGroup> {
        let block = Selector::parse(".b-post__schedule_block").unwrap();
        let title = Selector::parse(".b-post__schedule_block_title .title").unwrap();
        let row = Selector::parse("tr").unwrap();
        let cell = Selector::parse("td").unwrap();
        let marker = Selector::parse("i[data-id]").unwrap();
        document
            .select(&block)
            .map(|table| ScheduleGroup {
                name: table
                    .select(&title)
                    .next()
                    .map(|node| node.text().collect::<String>().trim().to_string())
                    .unwrap_or_default(),
                items: table
                    .select(&row)
                    .filter_map(|row| {
                        let cells = row.select(&cell).collect::<Vec<_>>();
                        (cells.len() > 2).then(|| {
                            // The provider marks a released episode with a
                            // check mark inside a span; only its text belongs
                            // in the model, never the markup.
                            let status_markup = cells.get(4).map(|node| node.inner_html());
                            let status = cells
                                .get(4)
                                .map(|node| node.text().collect::<String>().trim().to_string())
                                .filter(|text| !text.is_empty());
                            let watch = cells.get(2).and_then(|node| node.select(&marker).next());
                            ScheduleItem {
                                id: watch
                                    .and_then(|node| node.value().attr("data-id"))
                                    .unwrap_or_default()
                                    .to_string(),
                                episode: cells[0].text().collect::<String>().trim().to_string(),
                                title: cells[1]
                                    .select(&Selector::parse("b").unwrap())
                                    .next()
                                    .map(|node| node.text().collect::<String>().trim().to_string())
                                    .unwrap_or_else(|| {
                                        cells[1].text().collect::<String>().trim().to_string()
                                    }),
                                original_title: cells[1]
                                    .select(&Selector::parse("span").unwrap())
                                    .next()
                                    .map(|node| node.text().collect::<String>().trim().to_string()),
                                date: cells
                                    .get(3)
                                    .map(|node| node.text().collect::<String>().trim().to_string()),
                                release_status: status.clone(),
                                is_released: status_markup.is_some_and(|markup| {
                                    markup.contains("check") || markup.contains("exists-episode")
                                }),
                                is_watched: watch.is_some_and(|node| {
                                    node.value()
                                        .attr("class")
                                        .is_some_and(|value| value.contains("watched"))
                                }),
                            }
                        })
                    })
                    .collect(),
            })
            .collect()
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
        fragment
            .select(&Selector::parse(".b-rgstats__list_item").unwrap())
            .filter_map(|item| {
                let title = item
                    .select(&Selector::parse(".title").unwrap())
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();
                let rating = item
                    .select(&Selector::parse(".count").unwrap())
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

    pub async fn fetch_actor(session: &RezkaSession, url: &str) -> Result<ActorDetails, String> {
        let html = session.get_html(url).await?;
        let mut actor = Self::parse_actor_html(&html);
        actor.photo_url = actor.photo_url.map(|value| session.resolve_url(&value));
        Ok(actor)
    }

    fn parse_actor_html(html: &str) -> ActorDetails {
        let document = Html::parse_document(html);
        let text = |selector: &str| {
            document
                .select(&Selector::parse(selector).unwrap())
                .next()
                .map(|node| node.text().collect::<String>().trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let mut actor = ActorDetails {
            name: text(".b-post__title .t1").unwrap_or_default(),
            original_name: text(".b-post__title .t2"),
            photo_url: document
                .select(&Selector::parse(".b-sidecover img").unwrap())
                .next()
                .and_then(|node| node.value().attr("src"))
                .map(str::to_string),
            careers: Vec::new(),
            birth_date: None,
            birth_place: None,
            height: None,
            films: CatalogScraper::parse_catalog_html(html),
            roles: document
                .select(&Selector::parse(".b-person__career").unwrap())
                .map(|role| ActorRole {
                    name: role
                        .select(&Selector::parse("h2").unwrap())
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    info: role
                        .select(&Selector::parse(".b-person__career_stats").unwrap())
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    films: CatalogScraper::parse_catalog_html(&role.html()),
                })
                .collect(),
        };
        let cells = Selector::parse(".b-post__info tr").unwrap();
        let td = Selector::parse("td").unwrap();
        for row in document.select(&cells) {
            let parts = row.select(&td).collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            let key = parts[0].text().collect::<String>().to_lowercase();
            let value = parts[1].text().collect::<String>().trim().to_string();
            if key.contains("дата рождения") {
                actor.birth_date = Some(value);
            } else if key.contains("место рождения") {
                actor.birth_place = Some(value);
            } else if key.contains("рост") {
                actor.height = Some(value);
            } else if key.contains("карьера") {
                actor.careers = parts[1]
                    .select(&Selector::parse("a").unwrap())
                    .map(|node| node.text().collect::<String>().trim().to_string())
                    .collect();
            }
        }
        actor
    }

    pub async fn fetch_comments(
        session: &RezkaSession,
        post_id: i64,
        page: usize,
    ) -> Result<CommentsPage, String> {
        let response = session.get_html(&format!("ajax/get_comments?news_id={post_id}&cstart={page}&type=0&comment_id=0&skin=hdrezka")).await?;
        let value: Value = serde_json::from_str(&response)
            .map_err(|_| "Comments response was malformed".to_string())?;
        let mut page = Self::parse_comments(
            value
                .get("comments")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value
                .get("navigation")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            page,
        );
        for comment in &mut page.items {
            comment.avatar_url = comment
                .avatar_url
                .take()
                .map(|value| session.resolve_url(&value));
        }
        Ok(page)
    }

    fn parse_comments(html: &str, navigation: &str, page: usize) -> CommentsPage {
        let document = Html::parse_fragment(html);
        let items = document
            .select(&Selector::parse(".comments-tree-item").unwrap())
            .map(|node| {
                let find = |selector: &str| node.select(&Selector::parse(selector).unwrap()).next();
                Comment {
                    id: node.value().attr("data-id").unwrap_or_default().to_string(),
                    username: find(".name")
                        .map(|item| item.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    avatar_url: find(".ava img")
                        .and_then(|item| item.value().attr("src"))
                        .map(str::to_string),
                    date: find(".date")
                        .map(|item| {
                            item.text()
                                .collect::<String>()
                                .replace("оставлен ", "")
                                .trim()
                                .to_string()
                        })
                        .unwrap_or_default(),
                    text: find(".text div")
                        .map(|item| {
                            item.text()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default(),
                    has_spoiler: find(".text_spoiler").is_some(),
                    indent: node
                        .value()
                        .attr("data-indent")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                    likes: find(".b-comment__like_it")
                        .and_then(|item| item.value().attr("data-likes_num"))
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                    liked: find(".show-likes-comment").is_some_and(|item| {
                        item.value()
                            .attr("class")
                            .unwrap_or_default()
                            .contains("disabled")
                    }),
                }
            })
            .collect();
        let nav = Html::parse_fragment(navigation);
        let total_pages = nav
            .select(&Selector::parse(".b-navigation a").unwrap())
            .filter_map(|node| node.text().collect::<String>().trim().parse().ok())
            .max()
            .unwrap_or(1);
        CommentsPage {
            items,
            page,
            total_pages,
        }
    }

    pub async fn fetch_trailer(
        session: &RezkaSession,
        post_id: i64,
    ) -> Result<Option<String>, String> {
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

    pub async fn post_rating(
        session: &RezkaSession,
        post_id: i64,
        rating: u8,
    ) -> Result<(), String> {
        if !(1..=10).contains(&rating) {
            return Err("Rating must be between 1 and 10".to_string());
        }
        let id = post_id.to_string();
        let rating = rating.to_string();
        Self::require_success(
            &session
                .post_ajax(
                    "engine/ajax/rating.php",
                    &[("news_id", &id), ("go_rate", &rating), ("skin", "hdrezka")],
                )
                .await?,
            "Failed to update rating",
        )
    }

    pub async fn like_comment(session: &RezkaSession, id: &str) -> Result<(), String> {
        Self::require_success(
            &session
                .post_ajax("engine/ajax/comments_like.php", &[("id", id)])
                .await?,
            "Failed to like comment",
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

    pub async fn fetch_episodes(
        session: &RezkaSession,
        post_id: i64,
        translator_id: i64,
    ) -> Result<Vec<Season>, String> {
        let post_id_str = post_id.to_string();
        let tr_id_str = translator_id.to_string();
        let form_data = [
            ("id", post_id_str.as_str()),
            ("translator_id", tr_id_str.as_str()),
            ("action", "get_episodes"),
        ];

        let json_resp = session
            .post_ajax("ajax/get_cdn_series/", &form_data)
            .await?;
        let parsed: Value = serde_json::from_str(&json_resp)
            .map_err(|e| format!("Failed to parse get_episodes JSON: {}", e))?;

        if !parsed
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
        {
            return Err("get_episodes returned success: false".to_string());
        }

        let seasons_html = parsed.get("seasons").and_then(|s| s.as_str()).unwrap_or("");
        let episodes_html = parsed
            .get("episodes")
            .and_then(|e| e.as_str())
            .unwrap_or("");

        Ok(Self::parse_episodes_html(seasons_html, episodes_html))
    }

    fn parse_episodes_html(seasons_html: &str, episodes_html: &str) -> Vec<Season> {
        let seasons_doc = Html::parse_fragment(seasons_html);
        let episodes_doc = Html::parse_fragment(episodes_html);

        let season_item_sel = Selector::parse(".b-simple_season__item").unwrap();
        let episode_item_sel = Selector::parse(".b-simple_episode__item").unwrap();

        let mut seasons = Vec::new();

        for s_el in seasons_doc.select(&season_item_sel) {
            let season_id = s_el
                .value()
                .attr("data-tab_id")
                .and_then(|id| id.parse::<i64>().ok())
                .unwrap_or(1);
            let season_title = s_el.text().collect::<String>().trim().to_string();

            let mut episodes = Vec::new();
            for ep_el in episodes_doc.select(&episode_item_sel) {
                let ep_season_id = ep_el
                    .value()
                    .attr("data-season_id")
                    .and_then(|id| id.parse::<i64>().ok())
                    .unwrap_or(1);

                if ep_season_id == season_id {
                    episodes.push(Self::parse_episode(ep_el, season_id));
                }
            }

            seasons.push(Season {
                id: season_id,
                title: season_title,
                episodes,
            });
        }

        // If no separate season tabs, maybe single season
        if seasons.is_empty() {
            let mut episodes = Vec::new();
            for ep_el in episodes_doc.select(&episode_item_sel) {
                episodes.push(Self::parse_episode(ep_el, 1));
            }
            if !episodes.is_empty() {
                seasons.push(Season {
                    id: 1,
                    title: "Сезон 1".to_string(),
                    episodes,
                });
            }
        }

        seasons
    }

    fn parse_episode(ep: scraper::ElementRef<'_>, season_id: i64) -> Episode {
        Episode {
            id: ep
                .value()
                .attr("data-episode_id")
                .and_then(|id| id.parse().ok())
                .unwrap_or(1),
            season_id,
            title: ep.text().collect::<String>().trim().to_string(),
            watch_id: ep.value().attr("data-id").map(str::to_string),
            is_watched: ep
                .value()
                .attr("class")
                .is_some_and(|class| class.split_whitespace().any(|c| c == "watched")),
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
        let re =
            regex::Regex::new(r#"initCDN(?:Movies|Series)Events\s*\(\s*\d+\s*,\s*(\d+)"#).ok()?;
        let caps = re.captures(html)?;
        caps.get(1)?.as_str().parse::<i64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::DetailsScraper;

    #[test]
    fn parses_account_membership_and_watched_episode_state() {
        let details = DetailsScraper::parse_details_html(
            r#"
                <h1 class="b-post__title">Test</h1>
                <div class="hd-label-row"><input value="2" checked></div>
                <div class="hd-label-row"><input value="7"></div>
            "#,
            "https://example.test/films/42-test.html",
        )
        .unwrap();
        assert_eq!(details.favorite_category_ids, vec![2]);

        let seasons = DetailsScraper::parse_episodes_html(
            r#"<li class="b-simple_season__item" data-tab_id="1">Season 1</li>"#,
            r#"<li class="b-simple_episode__item watched" data-season_id="1" data-episode_id="3" data-id="watch-9">Episode 3</li>"#,
        );
        assert_eq!(seasons[0].episodes[0].watch_id.as_deref(), Some("watch-9"));
        assert!(seasons[0].episodes[0].is_watched);
    }

    #[test]
    fn rejects_missing_or_zero_media_id() {
        assert!(DetailsScraper::parse_details_html(
            r#"<h1 class="b-post__title">Unknown</h1>"#,
            "https://example.test/films/unknown.html",
        )
        .is_err());
        assert!(DetailsScraper::parse_details_html(
            r#"<div id="user-favorites-holder" data-post_id="0"></div>"#,
            "https://example.test/films/unknown.html",
        )
        .is_err());
    }

    #[test]
    fn parses_detail_page_surfaces() {
        let details = DetailsScraper::parse_details_html(r#"
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
        assert!(details.trailer_available && details.rating_posted);
    }

    #[test]
    fn parses_comments_and_actor_pages() {
        let comments = DetailsScraper::parse_comments(
            r#"<div class="comments-tree-item" data-id="c1" data-indent="2"><span class="name">User</span><span class="date">оставлен Today</span><div class="text"><div>Hello <b>world</b></div></div><span class="b-comment__like_it" data-likes_num="7"></span></div>"#,
            r#"<div class="b-navigation"><a>1</a><a>3</a><a>Next</a></div>"#,
            1,
        );
        assert_eq!(comments.items[0].text, "Hello world");
        assert_eq!(comments.total_pages, 3);

        let actor = DetailsScraper::parse_actor_html(
            r#"<div class="b-post__title"><span class="t1">Name</span><span class="t2">Original</span></div><div class="b-sidecover"><img src="photo.jpg"></div><table class="b-post__info"><tr><td>Дата рождения:</td><td>1 Jan</td></tr><tr><td>Карьера:</td><td><a>Actor</a></td></tr></table>"#,
        );
        assert_eq!(actor.name, "Name");
        assert_eq!(actor.careers, vec!["Actor"]);
    }
}
