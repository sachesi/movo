use super::super::models::{ScheduleGroup, ScheduleItem, Season};
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Words the provider puts next to a season or an episode number, in the
/// languages it serves.
const SEASON_LABELS: [&str; 2] = ["season", "сезон"];
const EPISODE_LABELS: [&str; 5] = ["episode", "серия", "серія", "эпизод", "епізод"];

/// The season and episode numbers a provider label names, as in
/// "1 сезон 3 серия" or "Season 1 Episode 3"; each is absent when the label
/// does not state it.
pub fn labelled_position(text: &str) -> (Option<i64>, Option<i64>) {
    let lower = text.to_lowercase();
    let tokens = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut season = None;
    let mut episode = None;
    let mut taken = vec![false; tokens.len()];
    for (index, token) in tokens.iter().enumerate() {
        let slot = if SEASON_LABELS.contains(token) {
            &mut season
        } else if EPISODE_LABELS.contains(token) {
            &mut episode
        } else {
            continue;
        };
        if slot.is_some() {
            continue;
        }
        // The number precedes its label in Russian and follows it in English;
        // a number already claimed by the other label is never reused.
        let before = index
            .checked_sub(1)
            .filter(|&at| !taken[at])
            .and_then(|at| tokens[at].parse::<i64>().ok().map(|number| (at, number)));
        let after = tokens
            .get(index + 1)
            .and_then(|next| next.parse::<i64>().ok())
            .map(|number| (index + 1, number));
        if let Some((at, number)) = before.or(after) {
            taken[at] = true;
            *slot = Some(number);
        }
    }
    (season, episode)
}

/// Where a schedule row sits: the row names both numbers, or only the episode
/// under a block titled with the season.
fn schedule_position(group: &str, label: &str) -> Option<(i64, i64)> {
    let (season, episode) = labelled_position(label);
    let season = season.or_else(|| labelled_position(group).0);
    Some((season?, episode?))
}

/// The schedule row of an episode, if the provider lists it.
pub fn schedule_item_for(
    schedules: &[ScheduleGroup],
    season: i64,
    episode: i64,
) -> Option<&ScheduleItem> {
    schedules
        .iter()
        .flat_map(|group| group.items.iter().map(move |item| (group, item)))
        .find(|(group, item)| {
            schedule_position(&group.name, &item.episode) == Some((season, episode))
        })
        .map(|(_, item)| item)
}

/// Carries the schedule's watched state onto the episodes it lists. The
/// episode list itself never says which episodes were watched; the provider
/// keeps that on the schedule rows of the title's page.
pub fn mark_scheduled(seasons: &mut [Season], schedules: &[ScheduleGroup]) {
    let rows = schedules
        .iter()
        .flat_map(|group| {
            group.items.iter().filter_map(move |item| {
                schedule_position(&group.name, &item.episode).map(|at| (at, item))
            })
        })
        .collect::<HashMap<_, _>>();
    if rows.is_empty() {
        return;
    }
    for season in seasons.iter_mut() {
        for episode in season.episodes.iter_mut() {
            if let Some(item) = rows.get(&(season.id, episode.id)) {
                if !item.id.is_empty() {
                    episode.watch_id = Some(item.id.clone());
                }
                episode.is_watched = item.is_watched;
            }
        }
    }
}

pub(super) fn parse_schedules(document: &Html) -> Vec<ScheduleGroup> {
    let block = Selector::parse(".b-post__schedule_block").unwrap();
    let title = Selector::parse(".b-post__schedule_block_title .title").unwrap();
    let row = Selector::parse("tr").unwrap();
    let cell = Selector::parse("td").unwrap();
    let marker = Selector::parse("i[data-id]").unwrap();
    let title_bold = Selector::parse("b").unwrap();
    let title_span = Selector::parse("span").unwrap();
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
                                .select(&title_bold)
                                .next()
                                .map(|node| node.text().collect::<String>().trim().to_string())
                                .unwrap_or_else(|| {
                                    cells[1].text().collect::<String>().trim().to_string()
                                }),
                            original_title: cells[1]
                                .select(&title_span)
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

#[cfg(test)]
mod tests {
    use super::{labelled_position, mark_scheduled};
    use crate::client::details::episodes::parse_episodes_html;
    use crate::client::models::{ScheduleGroup, ScheduleItem};

    #[test]
    fn labels_are_read_in_either_word_order() {
        assert_eq!(labelled_position("1 сезон 3 серия"), (Some(1), Some(3)));
        assert_eq!(labelled_position("Season 1 Episode 3"), (Some(1), Some(3)));
        assert_eq!(labelled_position("Сезон 2, епізод 12"), (Some(2), Some(12)));
        assert_eq!(labelled_position("3 серия"), (None, Some(3)));
        assert_eq!(
            labelled_position("1 сезон 2 серия (Кириллица) смотреть ещё 14 серий в 1 сезоне"),
            (Some(1), Some(2))
        );
        assert_eq!(labelled_position("Украинский дубляж"), (None, None));
    }

    #[test]
    fn schedule_rows_mark_the_episodes_they_name() {
        let row = |episode: &str, id: &str, watched: bool| ScheduleItem {
            id: id.to_string(),
            episode: episode.to_string(),
            title: String::new(),
            original_title: None,
            date: None,
            release_status: None,
            is_released: true,
            is_watched: watched,
        };
        let schedules = vec![
            ScheduleGroup {
                name: "2 сезон".to_string(),
                items: vec![row("1 серия", "w21", true)],
            },
            ScheduleGroup {
                name: String::new(),
                items: vec![
                    row("1 сезон 2 серия", "w12", true),
                    row("1 сезон 1 серия", "", false),
                ],
            },
        ];
        let mut seasons = parse_episodes_html(
            r#"<li class="b-simple_season__item" data-tab_id="1">1</li><li class="b-simple_season__item" data-tab_id="2">2</li>"#,
            r#"<li class="b-simple_episode__item" data-season_id="1" data-episode_id="1">1</li>
               <li class="b-simple_episode__item" data-season_id="1" data-episode_id="2">2</li>
               <li class="b-simple_episode__item" data-season_id="1" data-episode_id="3">3</li>
               <li class="b-simple_episode__item" data-season_id="2" data-episode_id="1">1</li>"#,
        );
        mark_scheduled(&mut seasons, &schedules);
        let first = &seasons[0].episodes;
        assert!(!first[0].is_watched && first[0].watch_id.is_none());
        assert!(first[1].is_watched && first[1].watch_id.as_deref() == Some("w12"));
        assert!(!first[2].is_watched && first[2].watch_id.is_none());
        assert!(seasons[1].episodes[0].is_watched);
        assert_eq!(seasons[1].episodes[0].watch_id.as_deref(), Some("w21"));
    }
}
