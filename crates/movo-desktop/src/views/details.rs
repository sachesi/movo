use crate::api::{guarded, Guarded};
use crate::i18n::{tr, trf};
use crate::state::AppState;
use crate::ui::content::{ContentStack, ContentState};
use crate::ui::image::{self, ImageToken};
use crate::ui::poster_grid::{poster_row, PosterItem, PosterRow};
use movo_core::client::models::{
    FavoritesCollection, MediaDetails, MediaItem, MediaType, Season, Translator,
};
use movo_core::storage::history::{WatchHistory, WatchHistoryEntry};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};
use std::cell::Cell;
use std::rc::Rc;

mod format;
mod sort;

use format::{caption, chip_box, link_media, metadata_line, season_title, translator_label};
use sort::sort_by_voice_rating;

const POSTER_WIDTH: i32 = 165;
const POSTER_HEIGHT: i32 = 248;

pub struct DetailsView {
    state: Rc<AppState>,
    url: String,
    details: Option<MediaDetails>,
    favorite_categories: Vec<FavoritesCollection>,
    translator: Option<Translator>,
    season: Option<i64>,
    /// What this account played last of this title, if anything.
    resume: Option<WatchHistoryEntry>,
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

/// A title with the account's favorites groups and its last local playback.
type LoadedDetails = (
    MediaDetails,
    Vec<FavoritesCollection>,
    Option<WatchHistoryEntry>,
);

#[derive(Debug)]
pub enum DetailsCommand {
    Loaded(Box<Guarded<LoadedDetails>>),
    Episodes(Guarded<Vec<Season>>),
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
            details: None,
            favorite_categories: Vec::new(),
            translator: None,
            season: None,
            resume: None,
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

                // A saved page lets the layout be checked without provider
                // access, which some networks refuse.
                if let Some(path) = std::env::var_os("MOVO_DETAILS_FIXTURE") {
                    match std::fs::read_to_string(&path)
                        .map_err(|error| error.to_string())
                        .and_then(|json| {
                            serde_json::from_str::<MediaDetails>(&json)
                                .map_err(|error| error.to_string())
                        }) {
                        Ok(details) => self.render(details, None, &sender),
                        Err(error) => self.content.set(ContentState::Error(&error)),
                    }
                    return;
                }

                let client = self.state.client.clone();
                let url = self.url.clone();
                let user_id = self.state.user().map(|user| user.user_id);
                sender.oneshot_command(async move {
                    DetailsCommand::Loaded(Box::new(
                        guarded(client, move |client| async move {
                            let details = client.fetch_details(&url).await?;
                            let Some(user_id) = user_id else {
                                return Ok((details, Vec::new(), None));
                            };
                            let categories = client
                                .fetch_favorites_categories()
                                .await
                                .unwrap_or_default();
                            let media_id = details.id;
                            let resume = relm4::spawn_blocking(move || {
                                WatchHistory::load(&user_id).latest_for(media_id).cloned()
                            })
                            .await
                            .ok()
                            .flatten();
                            Ok((details, categories, resume))
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
                    sender.oneshot_command(async move {
                        DetailsCommand::Episodes(
                            guarded(client, move |client| async move {
                                client.fetch_episodes(post_id, translator.id).await
                            })
                            .await,
                        )
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
                sender.oneshot_command(async move {
                    DetailsCommand::Changed(
                        guarded(client, move |client| async move {
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
                let client = self.state.client.clone();
                sender.oneshot_command(async move {
                    DetailsCommand::Changed(
                        guarded(client, move |client| async move {
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
                let client = self.state.client.clone();
                let post_id = details.id;
                sender.oneshot_command(async move {
                    DetailsCommand::Trailer(
                        guarded(client, move |client| async move {
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
                let notify = sender.clone();
                crate::dialogs::rating::present(
                    root,
                    self.state.clone(),
                    details.id,
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
                let notify = sender.clone();
                crate::dialogs::comments::present(
                    root,
                    self.state.clone(),
                    details.id,
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
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                match loaded.result {
                    Ok((details, categories, resume)) => {
                        self.favorite_categories = categories;
                        self.render(details, resume, &sender);
                    }
                    Err(error) => self.content.set(ContentState::Error(&error)),
                }
            }
            DetailsCommand::Episodes(loaded) => {
                if !self.state.accepts(&loaded.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
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
                        let _ = sender.output(DetailsOutput::Notify(error));
                    }
                }
            }
            DetailsCommand::Trailer(loaded) => match loaded.result {
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
                    let _ = sender.output(DetailsOutput::Notify(error));
                }
            },
            DetailsCommand::Changed(changed, kind) => {
                if !self.state.accepts(&changed.account) {
                    let _ = sender.output(DetailsOutput::AccountInvalidated);
                    return;
                }
                match changed.result {
                    Ok(()) => {
                        if kind == "favorite" {
                            sender.input(DetailsMsg::Reload);
                        }
                    }
                    Err(error) => {
                        let _ = sender.output(DetailsOutput::Notify(error));
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
        self.resume_pending = self.resume.is_some();
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

    fn hero(&self, details: &MediaDetails, sender: &relm4::ComponentSender<Self>) -> gtk::Box {
        let hero = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(18)
            .valign(gtk::Align::Start)
            .build();

        let poster = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Cover)
            .width_request(POSTER_WIDTH)
            .height_request(POSTER_HEIGHT)
            .valign(gtk::Align::Start)
            .build();
        poster.add_css_class("card");
        image::load(
            &poster,
            details
                .poster_hq_url
                .as_deref()
                .or(details.poster_url.as_deref()),
            POSTER_WIDTH,
            POSTER_HEIGHT,
            &self.poster_token,
        );
        hero.append(&poster);

        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();

        let title_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        let title = gtk::Label::builder()
            .label(&details.title)
            .wrap(true)
            .xalign(0.0)
            .hexpand(true)
            .build();
        title.add_css_class("title-2");
        title_row.append(&title);
        title_row.append(&self.favorite_controls(details, sender));
        column.append(&title_row);

        if let Some(original) = &details.orig_title {
            let label = gtk::Label::builder()
                .label(original)
                .wrap(true)
                .xalign(0.0)
                .build();
            label.add_css_class("dim-label");
            column.append(&label);
        }

        let metadata = gtk::Label::builder()
            .label(metadata_line(details))
            .wrap(true)
            .xalign(0.0)
            .build();
        metadata.add_css_class("dim-label");
        column.append(&metadata);

        // The linked lists further down say the same thing, so the plain text
        // is only a fallback for a page that has no links.
        if details.genre_links.is_empty() && !details.genres.is_empty() {
            column.append(&caption(&trf("Genres: {}", &[&details.genres.join(", ")])));
        }
        if details.country_links.is_empty() && !details.countries.is_empty() {
            column.append(&caption(&trf(
                "Country: {}",
                &[&details.countries.join(", ")],
            )));
        }
        if let Some(duration) = &details.duration {
            column.append(&caption(duration));
        }

        for (title, people) in [
            (tr("Director"), &details.directors_details),
            (tr("Cast"), &details.actors_details),
        ] {
            if people.is_empty() {
                continue;
            }
            let row = chip_box();
            for person in people {
                let button = gtk::Button::with_label(&person.name);
                button.add_css_class("flat");
                button.set_halign(gtk::Align::Start);
                let sender = sender.clone();
                let url = person.url.clone();
                button.connect_clicked(move |_| {
                    sender.input(DetailsMsg::ShowActor(url.clone()));
                });
                row.append(&button);
            }
            let group = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            group.append(&caption(title));
            group.append(&row);
            column.append(&group);
        }

        hero.append(&column);
        hero
    }

    fn favorite_controls(
        &self,
        details: &MediaDetails,
        sender: &relm4::ComponentSender<Self>,
    ) -> gtk::Box {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .valign(gtk::Align::Center)
            .build();

        let ids = self
            .favorite_categories
            .iter()
            .filter_map(|category| category.id)
            .collect::<Vec<_>>();
        let names = self
            .favorite_categories
            .iter()
            .filter(|category| category.id.is_some())
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>();

        let categories = gtk::DropDown::from_strings(&names);
        categories.set_tooltip_text(Some(tr("Favorites Group")));
        categories.set_visible(ids.len() > 1);
        categories.set_selected(
            ids.iter()
                .position(|id| details.favorite_category_ids.contains(id))
                .unwrap_or(0) as u32,
        );

        let is_favorite = !details.favorite_category_ids.is_empty();
        let button = gtk::Button::builder()
            .icon_name(if is_favorite {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            })
            .tooltip_text(tr("Favorite"))
            .valign(gtk::Align::Center)
            .build();
        button.set_sensitive(!ids.is_empty());

        let sender = sender.clone();
        let categories_for_click = categories.clone();
        button.connect_clicked(move |_| {
            let Some(id) = ids.get(categories_for_click.selected() as usize).copied() else {
                return;
            };
            sender.input(DetailsMsg::ToggleFavorite(id));
        });

        container.append(&categories);
        container.append(&button);
        container
    }

    /// The voice-over and season pickers.
    ///
    /// Both are `AdwComboRow`s: a plain `GtkDropDown` in a row suffix does not
    /// ellipsize, so a long voice-over name squeezes the row title out of view.
    fn playback_group(
        &self,
        details: &MediaDetails,
        translator: usize,
        is_series: bool,
        sender: &relm4::ComponentSender<Self>,
    ) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::builder()
            .title(tr("Playback"))
            .build();

        if !details.translators.is_empty() {
            let names = details
                .translators
                .iter()
                .map(|translator| translator_label(translator, details))
                .collect::<Vec<_>>();
            let row = adw::ComboRow::builder()
                .title(tr("Voice-over"))
                .model(&gtk::StringList::new(
                    &names.iter().map(String::as_str).collect::<Vec<_>>(),
                ))
                .build();
            let sender = sender.clone();
            row.connect_selected_notify(move |row| {
                sender.input(DetailsMsg::SelectTranslator(row.selected() as usize));
            });
            // Fires the handler above for anything but the first entry, which
            // is what loads that voice-over's episodes.
            row.set_selected(translator as u32);
            group.add(&row);
        }

        if is_series {
            self.seasons.set_title(tr("Season"));
            group.add(&self.seasons);
        }

        group
    }

    fn actions(
        &self,
        details: &MediaDetails,
        is_series: bool,
        sender: &relm4::ComponentSender<Self>,
    ) -> gtk::Box {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk::Align::Start)
            .build();

        if !is_series {
            let watch = gtk::Button::with_label(tr("Watch"));
            watch.add_css_class("suggested-action");
            watch.add_css_class("pill");
            let sender = sender.clone();
            watch.connect_clicked(move |_| sender.input(DetailsMsg::Play(None, None)));
            row.append(&watch);
        }

        if details.has_trailer {
            let trailer = gtk::Button::with_label(tr("Trailer"));
            trailer.add_css_class("pill");
            let sender = sender.clone();
            trailer.connect_clicked(move |_| sender.input(DetailsMsg::ShowTrailer));
            row.append(&trailer);
        }

        let rate = gtk::Button::with_label(if details.has_posted_rating {
            tr("Rated")
        } else {
            tr("Rate")
        });
        rate.add_css_class("pill");
        rate.set_sensitive(!details.has_posted_rating);
        let rate_sender = sender.clone();
        rate.connect_clicked(move |_| rate_sender.input(DetailsMsg::ShowRating));
        row.append(&rate);

        let comments = gtk::Button::with_label(tr("Comments"));
        comments.add_css_class("pill");
        let comments_sender = sender.clone();
        comments.connect_clicked(move |_| comments_sender.input(DetailsMsg::ShowComments));
        row.append(&comments);

        row
    }

    fn schedule(
        &self,
        details: &MediaDetails,
        sender: &relm4::ComponentSender<Self>,
    ) -> adw::PreferencesGroup {
        // Folded by default: the list runs long, and the play controls sit
        // above it.
        let expander = adw::ExpanderRow::builder()
            .title(tr("Episode Schedule"))
            .expanded(false)
            .build();
        let group = adw::PreferencesGroup::new();
        group.add(&expander);

        for schedule in &details.schedules {
            for item in &schedule.items {
                let subtitle = [
                    Some(schedule.name.as_str()),
                    item.date.as_deref(),
                    item.release_status.as_deref(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" • ");

                let row = adw::ActionRow::builder()
                    .use_markup(false)
                    .title(format!("{} · {}", item.episode, item.title))
                    .subtitle(subtitle)
                    .build();

                if item.is_released {
                    let watched = gtk::Button::builder()
                        .icon_name(if item.is_watched {
                            "object-select-symbolic"
                        } else {
                            "checkbox-symbolic"
                        })
                        .tooltip_text(tr("Toggle Watched Status"))
                        .has_frame(false)
                        .valign(gtk::Align::Center)
                        .build();
                    let sender = sender.clone();
                    let id = item.id.clone();
                    watched.connect_clicked(move |button| {
                        button.set_sensitive(false);
                        sender.input(DetailsMsg::ToggleScheduleWatched(id.clone()));
                    });
                    row.add_suffix(&watched);
                }
                expander.add_row(&row);
            }
        }
        group
    }

    fn links(
        &self,
        title: &str,
        links: &[movo_core::client::models::LinkedItem],
        sender: &relm4::ComponentSender<Self>,
    ) -> Option<gtk::Box> {
        if links.is_empty() {
            return None;
        }
        let row = chip_box();
        for link in links {
            let button = gtk::Button::with_label(&link.name);
            button.add_css_class("flat");
            button.set_halign(gtk::Align::Start);
            let sender = sender.clone();
            let name = link.name.clone();
            let path = link.url.clone();
            button.connect_clicked(move |_| {
                sender.input(DetailsMsg::OpenPath(name.clone(), path.clone()));
            });
            row.append(&button);
        }

        let group = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        group.append(&caption(title));
        group.append(&row);
        Some(group)
    }

    fn show_seasons(&mut self, seasons: &[Season], sender: &relm4::ComponentSender<Self>) {
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

    fn show_episodes(&mut self, seasons: &[Season], sender: &relm4::ComponentSender<Self>) {
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
    /// when that was watched to the end.
    fn continue_point(&self, seasons: &[Season]) -> Option<(i64, i64)> {
        continue_point(self.resume.as_ref()?, seasons)
    }
}

fn continue_point(entry: &WatchHistoryEntry, seasons: &[Season]) -> Option<(i64, i64)> {
    let (season, episode) = (entry.season?, entry.episode?);
    let episodes = seasons
        .iter()
        .flat_map(|s| s.episodes.iter().map(move |e| (s.id, e.id)))
        .collect::<Vec<_>>();
    let index = episodes.iter().position(|&at| at == (season, episode))?;
    let offset = usize::from(entry.is_finished() && index + 1 < episodes.len());
    Some(episodes[index + offset])
}

#[cfg(test)]
mod tests {
    use super::continue_point;
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
}
