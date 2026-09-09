use super::format::season_title;
use super::{DetailsMsg, DetailsView};
use crate::i18n::tr;
use movo_core::client::models::Season;
use movo_core::storage::history::WatchHistoryEntry;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{self, prelude::WidgetExt};

impl DetailsView {
    pub(super) fn show_seasons(
        &mut self,
        seasons: &[Season],
        sender: &relm4::ComponentSender<Self>,
    ) {
        let titles = seasons.iter().map(season_title).collect::<Vec<_>>();
        if self.resume_pending {
            self.resume_pending = false;
            self.season = self.continue_point(seasons).map(|(season, _)| season);
        }
        let selected = seasons
            .iter()
            .position(|season| Some(season.id) == self.season)
            .unwrap_or(0);
        self.season = seasons.get(selected).map(|season| season.id);

        self.seasons.set_model(Some(&gtk::StringList::new(
            &titles.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        self.seasons.set_selected(selected as u32);

        // Reconnecting on every reload would stack handlers on a long-lived
        // widget, so the season handler is connected once.
        if unsafe { self.seasons.data::<bool>("movo-connected") }.is_none() {
            let sender = sender.clone();
            self.seasons.connect_selected_notify(move |dropdown| {
                sender.input(DetailsMsg::SelectSeason(dropdown.selected() as usize));
            });
            unsafe { self.seasons.set_data("movo-connected", true) };
        }

        self.show_episodes(seasons, sender);
    }

    pub(super) fn show_episodes(
        &mut self,
        seasons: &[Season],
        sender: &relm4::ComponentSender<Self>,
    ) {
        while let Some(child) = self.episodes.first_child() {
            self.episodes.remove(&child);
        }

        let Some(season) = seasons
            .iter()
            .find(|season| Some(season.id) == self.season)
            .or_else(|| seasons.first())
        else {
            return;
        };

        let continue_at = self.continue_point(seasons);
        for episode in &season.episodes {
            let row = adw::ActionRow::builder()
                .use_markup(false)
                .title(&episode.title)
                .activatable(true)
                .build();
            if continue_at == Some((season.id, episode.id)) {
                let label = gtk::Label::new(Some(tr("Continue")));
                label.add_css_class("accent");
                label.add_css_class("caption");
                row.add_suffix(&label);
            }
            if episode.is_watched {
                row.add_css_class("dim-label");
                row.add_suffix(&gtk::Image::from_icon_name("object-select-symbolic"));
            }
            row.add_suffix(&gtk::Image::from_icon_name("media-playback-start-symbolic"));

            let sender = sender.clone();
            let season_id = season.id;
            let episode_id = episode.id;
            row.connect_activated(move |_| {
                sender.input(DetailsMsg::Play(Some(season_id), Some(episode_id)));
            });
            self.episodes.append(&row);
        }
    }

    /// The episode to offer next: the one last played, or the one after it
    /// when that was watched to the end. The local position knows the end
    /// was reached; the account's history row only does when the episode is
    /// marked watched.
    fn continue_point(&self, seasons: &[Season]) -> Option<(i64, i64)> {
        if let Some(entry) = self.resume.as_ref() {
            return continue_point(entry, seasons);
        }
        let at = self.account_point?;
        let watched = seasons
            .iter()
            .find(|season| season.id == at.0)?
            .episodes
            .iter()
            .find(|episode| episode.id == at.1)?
            .is_watched;
        continue_from(at, watched, seasons)
    }
}

fn continue_point(entry: &WatchHistoryEntry, seasons: &[Season]) -> Option<(i64, i64)> {
    continue_from(
        (entry.season?, entry.episode?),
        entry.is_finished(),
        seasons,
    )
}

fn continue_from(at: (i64, i64), finished: bool, seasons: &[Season]) -> Option<(i64, i64)> {
    let episodes = seasons
        .iter()
        .flat_map(|s| s.episodes.iter().map(move |e| (s.id, e.id)))
        .collect::<Vec<_>>();
    let index = episodes.iter().position(|&candidate| candidate == at)?;
    let offset = usize::from(finished && index + 1 < episodes.len());
    Some(episodes[index + offset])
}

#[cfg(test)]
mod tests {
    use super::{continue_from, continue_point};
    use movo_core::client::models::{Episode, MediaType, Season};
    use movo_core::storage::history::WatchHistoryEntry;

    fn seasons() -> Vec<Season> {
        (1..=2)
            .map(|season| Season {
                id: season,
                title: season.to_string(),
                episodes: (1..=2)
                    .map(|id| Episode {
                        id,
                        season_id: season,
                        title: id.to_string(),
                        watch_id: None,
                        is_watched: false,
                    })
                    .collect(),
            })
            .collect()
    }

    fn entry(season: i64, episode: i64, position_secs: f64) -> WatchHistoryEntry {
        WatchHistoryEntry {
            media_id: 1,
            title: String::new(),
            orig_title: None,
            url: String::new(),
            poster_url: None,
            media_type: MediaType::TVSeries,
            season: Some(season),
            episode: Some(episode),
            episode_title: None,
            translator_id: None,
            translator_name: None,
            position_secs,
            duration_secs: 100.0,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn a_finished_episode_offers_the_next_one_across_seasons() {
        assert_eq!(continue_point(&entry(1, 1, 40.0), &seasons()), Some((1, 1)));
        assert_eq!(
            continue_point(&entry(1, 2, 100.0), &seasons()),
            Some((2, 1))
        );
        assert_eq!(
            continue_point(&entry(2, 2, 100.0), &seasons()),
            Some((2, 2))
        );
        assert_eq!(continue_point(&entry(3, 1, 0.0), &seasons()), None);
    }

    #[test]
    fn account_history_continues_at_the_row_or_after_a_watched_one() {
        assert_eq!(continue_from((1, 1), false, &seasons()), Some((1, 1)));
        assert_eq!(continue_from((1, 2), true, &seasons()), Some((2, 1)));
        assert_eq!(continue_from((2, 2), true, &seasons()), Some((2, 2)));
        assert_eq!(continue_from((3, 1), false, &seasons()), None);
    }
}
