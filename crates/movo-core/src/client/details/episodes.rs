use super::super::models::{Episode, Season};
use super::super::session::RezkaSession;
use scraper::{Html, Selector};
use serde_json::Value;

/// What the provider answered for one voice-over's episodes.
pub(super) enum Episodes {
    Listed(Vec<Season>),
    /// It has none under this voice-over, for the reason it gives: one the
    /// title does not have, or a title not released yet.
    Refused(String),
}

pub async fn fetch_episodes(
    session: &RezkaSession,
    post_id: i64,
    translator_id: i64,
) -> Result<Vec<Season>, String> {
    match request_episodes(session, post_id, translator_id).await? {
        Episodes::Listed(seasons) => Ok(seasons),
        Episodes::Refused(reason) => Err(format!("No episodes for this voice-over: {reason}")),
    }
}

pub(super) async fn request_episodes(
    session: &RezkaSession,
    post_id: i64,
    translator_id: i64,
) -> Result<Episodes, String> {
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
    read_episodes(&json_resp)
}

fn read_episodes(json_resp: &str) -> Result<Episodes, String> {
    let parsed: Value = serde_json::from_str(json_resp)
        .map_err(|e| format!("Failed to parse get_episodes JSON: {}", e))?;

    if !parsed
        .get("success")
        .and_then(|s| s.as_bool())
        .unwrap_or(false)
    {
        let reason = parsed
            .get("message")
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
            .unwrap_or("the provider gave no reason");
        return Ok(Episodes::Refused(reason.to_string()));
    }

    let seasons_html = parsed.get("seasons").and_then(|s| s.as_str()).unwrap_or("");
    let episodes_html = parsed
        .get("episodes")
        .and_then(|e| e.as_str())
        .unwrap_or("");

    Ok(Episodes::Listed(parse_episodes_html(
        seasons_html,
        episodes_html,
    )))
}

pub(super) fn parse_episodes_html(seasons_html: &str, episodes_html: &str) -> Vec<Season> {
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
                episodes.push(parse_episode(ep_el, season_id));
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
            episodes.push(parse_episode(ep_el, 1));
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

#[cfg(test)]
mod tests {
    use super::{read_episodes, Episodes};

    #[test]
    fn a_refusal_keeps_the_providers_reason() {
        let reply =
            read_episodes(r#"{"success":false,"message":"Не удалось найти выбранную озвучку"}"#);

        assert!(
            matches!(reply, Ok(Episodes::Refused(reason)) if reason == "Не удалось найти выбранную озвучку")
        );
    }

    #[test]
    fn a_listing_reads_its_seasons() {
        let reply = read_episodes(
            r#"{"success":true,"message":"","seasons":"<li class=\"b-simple_season__item\" data-tab_id=\"1\">Сезон 1</li>","episodes":"<li class=\"b-simple_episode__item\" data-season_id=\"1\" data-episode_id=\"1\">Серия 1</li>"}"#,
        );

        assert!(
            matches!(reply, Ok(Episodes::Listed(seasons)) if seasons.len() == 1 && seasons[0].episodes.len() == 1)
        );
    }
}
