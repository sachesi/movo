use crate::api::{guarded_as, Guarded};
use crate::i18n::{tr, trf};
use crate::state::{account_of, Account, AppState};
use crate::ui::content::{ContentStack, ContentState};
use crate::ui::image::ImageToken;
use crate::ui::poster_grid::{poster_row, PosterItem, PosterRow};
use movo_core::client::models::{
    FavoritesCollection, MediaDetails, MediaItem, MediaType, Season, Translator,
};
use movo_core::storage::history::{WatchHistory, WatchHistoryEntry};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{BoxExt, WidgetExt},
};
use std::cell::Cell;
use std::rc::Rc;

mod episodes;
mod format;
mod sections;
mod sort;

use format::link_media;
use sort::sort_by_voice_rating;

const POSTER_WIDTH: i32 = 165;
const POSTER_HEIGHT: i32 = 248;

pub struct DetailsView {
    state: Rc<AppState>,
    url: String,
    account: Option<Account>,
    details: Option<MediaDetails>,
    favorite_categories: Vec<FavoritesCollection>,
    translator: Option<Translator>,
    season: Option<i64>,
    /// What this account played last of this title, if anything.
    resume: Option<WatchHistoryEntry>,
    /// Where the account's history says this series stopped, for a player
    /// that reports no progress of its own.
    account_point: Option<(i64, i64)>,
    /// The saved season has not been applied yet: the voice-over it was
    /// watched in may still be loading its episodes.
    resume_pending: bool,
    content: ContentStack,
    body: gtk::Box,
    /// Rails keep their models alive for as long as they are shown.
    rails: Vec<PosterRow>,
    poster_token: ImageToken,
    episodes: gtk::ListBox,
    seasons: adw::ComboRow,
}

#[derive(Debug)]
pub enum DetailsMsg {
    Reload,
    SelectTranslator(usize),
    SelectSeason(usize),
    Play(Option<i64>, Option<i64>),
    ToggleFavorite(i64),
    ToggleScheduleWatched(String),
    ShowTrailer,
    ShowRating,
    ShowComments,
    ShowActor(String),
    Open(MediaItem),
    OpenPath(String, String),
    Notify(String),
}

#[derive(Debug)]
pub enum DetailsOutput {
    /// Start playback: details, voice-over, season and episode.
    Play(Box<MediaDetails>, i64, Option<i64>, Option<i64>),
    Open(MediaItem),
    OpenPath(String, String),
    AccountInvalidated,
    Notify(String),
}

/// A title with the account's favorites groups, its last local playback, and
/// failing that the season and episode the account's history stopped at.
type LoadedDetails = (
    MediaDetails,
    Vec<FavoritesCollection>,
    Option<WatchHistoryEntry>,
    Option<(i64, i64)>,
);

#[derive(Debug)]
pub enum DetailsCommand {
    Loaded(Box<Guarded<LoadedDetails>>),
    Episodes {
        translator_id: i64,
        loaded: Guarded<Vec<Season>>,
    },
    Trailer(Guarded<Option<String>>),
    /// A mutation that only needs a success or failure report.
    Changed(Guarded<()>, &'static str),
}

#[relm4::component(pub)]
impl relm4::Component for DetailsView {
    type Init = (Rc<AppState>, String);
    type Input = DetailsMsg;
    type Output = DetailsOutput;
    type CommandOutput = DetailsCommand;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            #[local_ref]
            content -> gtk::Stack {},
        }
    }

    fn init(
        (state, url): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let body = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .margin_start(18)
            .margin_end(18)
            .margin_top(18)
            .margin_bottom(18)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(500)
            .child(&body)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&clamp)
            .build();

        let content = ContentStack::new(&scrolled, tr("This Title Did Not Load"));

        let episodes = crate::ui::activatable_list();

        let model = DetailsView {
            state,
            url,
            account: None,
            details: None,
            favorite_categories: Vec::new(),
            translator: None,
            season: None,
            resume: None,
            account_point: None,
            resume_pending: false,
            content,
            body,
            rails: Vec::new(),
            poster_token: Rc::new(Cell::new(0)),
            episodes,
            seasons: adw::ComboRow::new(),
        };

        let content = model.content.widget.clone();
        let widgets = view_output!();
        sender.input(DetailsMsg::Reload);
        relm4::ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: relm4::ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DetailsMsg::Reload => {
                self.content.set(ContentState::Loading(tr("Loading")));
                self.account = account_of(&self.state.client);

                // A saved page lets the layout be checked without provider
                // access, which some networks refuse.
                if let Some(path) = std::env::var_os("MOVO_DETAILS_FIXTURE") {
                    match std::fs::read_to_string(&path)
                        .map_err(|error| error.to_string())
                        .and_then(|json| {
                            serde_json::from_str::<MediaDetails>(&json)
                                .map_err(|error| error.to_string())
                        }) {
                        Ok(details) => self.render(details, None, None, &sender),
                        Err(error) => self.content.set(ContentState::Error(&error)),
                    }
                    return;
                }

                let client = self.state.client.clone();
                let url = self.url.clone();
                let user_id = self.state.user().map(|user| user.user_id);
                let account = self.account.clone();
                sender.oneshot_command(async move {
                    DetailsCommand::Loaded(Box::new(
                        guarded_as(client, account, move |client| async move {
                            let details = client.fetch_details(&url).await?;
                            let Some(user_id) = user_id else {
                                return Ok((details, Vec::new(), None, None));
                            };
                            let categories = client
                                .fetch_favorites_categories()
                                .await
                                .unwrap_or_default();
                            let media_id = details.id;
                            let resume = relm4::spawn_blocking(move || {
                                WatchHistory::load(&user_id)
                                    .map(|history| history.latest_for(media_id).cloned())
                            })
                            .await
                            .map_err(|_| {
                                tr("Loading local history stopped unexpectedly").to_string()
                            })??;
                            // Without a local position, the account's history
                            // still names the episode a series stopped at.
                            let is_series = details.media_type == MediaType::TVSeries
                                || !details.seasons.is_empty();
                            let account_point = if resume.is_none() && is_series {
                                client
                                    .fetch_history()
                                    .await
                                    .unwrap_or_default()
                                    .iter()
                                    .find(|row| row.media_id() == Some(media_id))
                                    .and_then(|row| row.position())
                            } else {
                                None
                            };
                            Ok((details, categories, resume, account_point))
                        })
                        .await,
                    ))
                });
            }
            DetailsMsg::SelectTranslator(index) => {
                let Some(details) = self.details.as_ref() else {
                    return;
                };
                let Some(translator) = details.translators.get(index).cloned() else {
                    return;
                };
                self.translator = Some(translator.clone());
                if !details.seasons.is_empty() || details.media_type == MediaType::TVSeries {
                    let client = self.state.client.clone();
                    let post_id = details.id;
                    let schedules = details.schedules.clone();
                    let account = self.account.clone();
                    sender.oneshot_command(async move {
                        DetailsCommand::Episodes {
                            translator_id: translator.id,
                            loaded: guarded_as(client, account, move |client| async move {
                                client
                                    .fetch_episodes(post_id, translator.id, &schedules)
                                    .await
                            })
                            .await,
                        }
                    });
                }
            }
            DetailsMsg::SelectSeason(index) => {
                let seasons = self
                    .details
                    .as_ref()
                    .map(|details| details.seasons.clone())
                    .unwrap_or_default();
                self.season = seasons.get(index).map(|season| season.id);
                self.show_episodes(&seasons, &sender);
            }
            DetailsMsg::Play(season, episode) => {
                let (Some(details), Some(translator)) =
                    (self.details.clone(), self.translator.clone())
                else {
                    return;
                };
                let _ = sender.output(DetailsOutput::Play(
                    Box::new(details),
                    translator.id,
                    season,
                    episode,
                ));
            }
            DetailsMsg::ToggleFavorite(category_id) => {
                let Some(details) = self.details.as_ref() else {
                    return;
                };
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                if self.state.user().is_none() {
                    let _ = sender.output(DetailsOutput::Notify(
                        tr("Sign in to manage favorites").to_string(),
                    ));
                    return;
                }
                let favorite = !details.favorite_category_ids.contains(&category_id);
                let client = self.state.client.clone();
                let url = details.url.clone();
                let post_id = details.id;
                let account = self.account.clone();
                sender.oneshot_command(async move {
                    DetailsCommand::Changed(
                        guarded_as(client, account, move |client| async move {
                            client
                                .set_favorite(&url, post_id, category_id, favorite)
                                .await
                        })
                        .await,
                        "favorite",
                    )
                });
            }
            DetailsMsg::ToggleScheduleWatched(id) => {
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                let client = self.state.client.clone();
                let account = self.account.clone();
                sender.oneshot_command(async move {
                    DetailsCommand::Changed(
                        guarded_as(client, account, move |client| async move {
                            client.toggle_schedule_watched(&id).await
                        })
                        .await,
                        "schedule",
                    )
                });
            }
            DetailsMsg::ShowTrailer => {
                let Some(details) = self.details.as_ref() else {
                    return;
                };
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                let client = self.state.client.clone();
                let post_id = details.id;
                let account = self.account.clone();
                sender.oneshot_command(async move {
                    DetailsCommand::Trailer(
                        guarded_as(client, account, move |client| async move {
                            client.fetch_trailer(post_id).await
                        })
                        .await,
                    )
                });
            }
            DetailsMsg::ShowRating => {
                let Some(details) = self.details.as_ref() else {
                    return;
                };
                if self.state.user().is_none() {
                    let _ = sender.output(DetailsOutput::Notify(
                        tr("Sign in to rate this title").to_string(),
                    ));
                    return;
                }
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                let notify = sender.clone();
                crate::dialogs::rating::present(
                    root,
                    self.state.clone(),
                    details.id,
                    self.account.clone(),
                    move |result| {
                        let message = match result {
                            Ok(rating) => trf("Rated {} out of 10", &[&rating]),
                            Err(error) => error,
                        };
                        notify.input(DetailsMsg::Notify(message));
                    },
                );
            }
            DetailsMsg::ShowComments => {
                let Some(details) = self.details.as_ref() else {
                    return;
                };
                if !self.state.accepts(&self.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                let notify = sender.clone();
                crate::dialogs::comments::present(
                    root,
                    self.state.clone(),
                    details.id,
                    self.account.clone(),
                    move |error| notify.input(DetailsMsg::Notify(error)),
                );
            }
            DetailsMsg::ShowActor(url) => {
                let open = sender.clone();
                let notify = sender.clone();
                crate::dialogs::actor::present(
                    root,
                    self.state.clone(),
                    url,
                    move |item| open.input(DetailsMsg::Open(item)),
                    move |error| notify.input(DetailsMsg::Notify(error)),
                );
            }
            DetailsMsg::Open(item) => {
                let _ = sender.output(DetailsOutput::Open(item));
            }
            DetailsMsg::OpenPath(title, path) => {
                let _ = sender.output(DetailsOutput::OpenPath(title, path));
            }
            DetailsMsg::Notify(message) => {
                let _ = sender.output(DetailsOutput::Notify(message));
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DetailsCommand::Loaded(loaded) => {
                // Narrower than `accepts`, which lets any signed-out result
                // through as public: the page reloads under the account as soon
                // as one appears, and the anonymous reply landing after that one
                // would put the thinner, favorite-less version back on screen.
                if loaded.account.is_none() && self.state.account().is_some() {
                    return;
                }
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                self.account = loaded.account.clone();
                match loaded.result {
                    Ok((details, categories, resume, account_point)) => {
                        self.favorite_categories = categories;
                        self.render(details, resume, account_point, &sender);
                    }
                    Err(error) => self.content.set(ContentState::Error(&error.to_string())),
                }
            }
            DetailsCommand::Episodes {
                translator_id,
                loaded,
            } => {
                // Narrower than `accepts`, which lets any signed-out result
                // through as public: the page reloads under the account as soon
                // as one appears, and the anonymous reply landing after that one
                // would put the thinner, favorite-less version back on screen.
                if loaded.account.is_none() && self.state.account().is_some() {
                    return;
                }
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                if self.translator.as_ref().map(|translator| translator.id) != Some(translator_id) {
                    return;
                }
                match loaded.result {
                    Ok(seasons) => {
                        if let Some(details) = self.details.as_mut() {
                            details.seasons = seasons.clone();
                        }
                        self.show_seasons(&seasons, &sender);
                    }
                    Err(error) => {
                        // The page's own episode list is better than none.
                        if let Some(seasons) = self.details.as_ref().map(|d| d.seasons.clone()) {
                            if self.episodes.first_child().is_none() {
                                self.show_seasons(&seasons, &sender);
                            }
                        }
                        let _ = sender.output(DetailsOutput::Notify(error.to_string()));
                    }
                }
            }
            DetailsCommand::Trailer(loaded) => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                match loaded.result {
                    Ok(Some(url)) => {
                        if let Err(error) = gtk::gio::AppInfo::launch_default_for_uri(
                            &url,
                            None::<&gtk::gio::AppLaunchContext>,
                        ) {
                            let _ = sender.output(DetailsOutput::Notify(error.to_string()));
                        }
                    }
                    Ok(None) => {
                        let _ = sender.output(DetailsOutput::Notify(
                            tr("No trailer is available for this title").to_string(),
                        ));
                    }
                    Err(error) => {
                        let _ = sender.output(DetailsOutput::Notify(error.to_string()));
                    }
                }
            }
            DetailsCommand::Changed(changed, kind) => {
                if !self.state.accepts(&changed.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                match changed.result {
                    Ok(()) => {
                        if matches!(kind, "favorite" | "schedule") {
                            sender.input(DetailsMsg::Reload);
                        }
                    }
                    Err(error) => {
                        let _ = sender.output(DetailsOutput::Notify(error.to_string()));
                        sender.input(DetailsMsg::Reload);
                    }
                }
            }
        }
    }
}

impl DetailsView {
    fn render(
        &mut self,
        mut details: MediaDetails,
        resume: Option<WatchHistoryEntry>,
        account_point: Option<(i64, i64)>,
        sender: &relm4::ComponentSender<Self>,
    ) {
        if self.state.settings().sort_voices {
            sort_by_voice_rating(&mut details);
        }
        // Come back to the voice-over this title was last played in.
        let translator = resume
            .as_ref()
            .and_then(|entry| entry.translator_id)
            .and_then(|id| details.translators.iter().position(|t| t.id == id))
            .unwrap_or(0);
        self.translator = details.translators.get(translator).cloned();
        self.season = None;
        self.resume = resume;
        self.account_point = account_point;
        self.resume_pending = self.resume.is_some() || self.account_point.is_some();
        self.details = Some(details.clone());
        self.rails.clear();

        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }

        self.body.append(&self.hero(&details, sender));

        if !details.description.is_empty() {
            let description = gtk::Label::builder()
                .label(&details.description)
                .wrap(true)
                .xalign(0.0)
                .selectable(true)
                .build();
            self.body.append(&description);
        }

        let is_series = details.media_type == MediaType::TVSeries || !details.seasons.is_empty();
        self.body.append(&self.actions(&details, is_series, sender));
        self.body
            .append(&self.playback_group(&details, translator, is_series, sender));
        if is_series {
            let title = gtk::Label::builder()
                .label(tr("Episodes"))
                .xalign(0.0)
                .build();
            title.add_css_class("title-4");
            self.body.append(&title);
            self.body.append(&self.episodes);
        }

        if !details.schedules.is_empty() {
            self.body.append(&self.schedule(&details, sender));
        }

        for (title, links) in [
            (tr("Genres"), &details.genre_links),
            (tr("Countries"), &details.country_links),
            (tr("Collections"), &details.from_collections),
            (tr("Included In"), &details.included_in),
        ] {
            if let Some(group) = self.links(title, links, sender) {
                self.body.append(&group);
            }
        }

        if !details.franchises.is_empty() {
            let group = adw::PreferencesGroup::builder()
                .title(tr("Franchise"))
                .build();
            for part in &details.franchises {
                let row = adw::ActionRow::builder()
                    .use_markup(false)
                    .title(&part.title)
                    .activatable(!part.is_current)
                    .build();
                if part.is_current {
                    row.add_suffix(&gtk::Image::from_icon_name("object-select-symbolic"));
                } else {
                    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                    let sender = sender.clone();
                    let item = link_media(&part.title, &part.url);
                    row.connect_activated(move |_| sender.input(DetailsMsg::Open(item.clone())));
                }
                group.add(&row);
            }
            self.body.append(&group);
        }

        if !details.related.is_empty() {
            let title = gtk::Label::builder()
                .label(tr("Related"))
                .xalign(0.0)
                .build();
            title.add_css_class("title-4");

            let open = sender.clone();
            let mut rail = poster_row(move |item| open.input(DetailsMsg::Open(item)));
            rail.extend_from_iter(details.related.iter().cloned().map(PosterItem::new));

            let scrolled = gtk::ScrolledWindow::builder()
                .vscrollbar_policy(gtk::PolicyType::Never)
                .child(&rail.view)
                .build();

            self.body.append(&title);
            self.body.append(&scrolled);
            self.rails.push(rail);
        }

        self.content.set(ContentState::Content);

        // The page lists the episodes of its first voice-over; selecting any
        // other one above fetches its own list, which replaces these.
        if !details.seasons.is_empty() && translator == 0 {
            let seasons = details.seasons.clone();
            self.show_seasons(&seasons, sender);
        }
    }
}
