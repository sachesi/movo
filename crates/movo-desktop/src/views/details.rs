use crate::api::{guarded, Guarded};
use crate::i18n::{tr, trf};
use crate::state::AppState;
use crate::ui::content::{ContentStack, ContentState};
use crate::ui::image::{self, ImageToken};
use crate::ui::poster_grid::{poster_row, PosterItem, PosterRow};
use movo_core::client::models::{
    FavoritesCollection, MediaDetails, MediaItem, MediaType, Season, Translator,
};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};
use std::cell::Cell;
use std::rc::Rc;

const POSTER_WIDTH: i32 = 165;
const POSTER_HEIGHT: i32 = 248;

pub struct DetailsView {
    state: Rc<AppState>,
    url: String,
    details: Option<MediaDetails>,
    favorite_categories: Vec<FavoritesCollection>,
    translator: Option<Translator>,
    season: Option<i64>,
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

#[derive(Debug)]
pub enum DetailsCommand {
    Loaded(Box<Guarded<(MediaDetails, Vec<FavoritesCollection>)>>),
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
            .spacing(16)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&body)
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
                        Ok(details) => self.render(details, &sender),
                        Err(error) => self.content.set(ContentState::Error(&error)),
                    }
                    return;
                }

                let client = self.state.client.clone();
                let url = self.url.clone();
                let signed_in = self.state.user().is_some();
                sender.oneshot_command(async move {
                    DetailsCommand::Loaded(Box::new(
                        guarded(client, move |client| async move {
                            let details = client.fetch_details(&url).await?;
                            let categories = if signed_in {
                                client
                                    .fetch_favorites_categories()
                                    .await
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            };
                            Ok((details, categories))
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
                    Ok((details, categories)) => {
                        self.favorite_categories = categories;
                        self.render(details, &sender);
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
    fn render(&mut self, mut details: MediaDetails, sender: &relm4::ComponentSender<Self>) {
        if self.state.settings().sort_voices {
            sort_by_voice_rating(&mut details);
        }
        self.translator = details.translators.first().cloned();
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
            .append(&self.playback_group(&details, is_series, sender));
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

        if !details.seasons.is_empty() {
            let seasons = details.seasons.clone();
            self.season = seasons.first().map(|season| season.id);
            self.show_seasons(&seasons, sender);
        }
    }

    fn hero(&self, details: &MediaDetails, sender: &relm4::ComponentSender<Self>) -> gtk::Box {
        let hero = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
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
            .spacing(8)
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

        if details.trailer_available {
            let trailer = gtk::Button::with_label(tr("Trailer"));
            trailer.add_css_class("pill");
            let sender = sender.clone();
            trailer.connect_clicked(move |_| sender.input(DetailsMsg::ShowTrailer));
            row.append(&trailer);
        }

        let rate = gtk::Button::with_label(if details.rating_posted {
            tr("Rated")
        } else {
            tr("Rate")
        });
        rate.add_css_class("pill");
        rate.set_sensitive(!details.rating_posted);
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
        let group = adw::PreferencesGroup::builder()
            .title(tr("Episode Schedule"))
            .build();

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
                group.add(&row);
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

        for episode in &season.episodes {
            let row = adw::ActionRow::builder()
                .use_markup(false)
                .title(&episode.title)
                .activatable(true)
                .build();
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
}

/// A wrapping row of flat buttons. `homogeneous` is off and the children are
/// start-aligned so a single chip does not stretch across the page.
fn chip_box() -> gtk::FlowBox {
    gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(false)
        .min_children_per_line(1)
        .max_children_per_line(30)
        .row_spacing(4)
        .column_spacing(4)
        .halign(gtk::Align::Start)
        .build()
}

fn caption(text: &str) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .build();
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label
}

fn metadata_line(details: &MediaDetails) -> String {
    let mut parts = vec![details.media_type.label().to_string()];
    if let Some(year) = details.year {
        parts.push(year.to_string());
    }
    if let Some(rating) = details.rating_rezka {
        parts.push(format!("Rezka {rating:.1}"));
    }
    if let Some(rating) = details.rating_imdb {
        parts.push(format!("IMDb {rating:.1}"));
    }
    if let Some(rating) = details.rating_kp {
        parts.push(format!("KinoPoisk {rating:.1}"));
    }
    for rating in &details.ratings {
        let value = match rating.votes {
            Some(votes) => format!("{} {:.1} ({votes})", rating.name, rating.value),
            None => format!("{} {:.1}", rating.name, rating.value),
        };
        if !parts.contains(&value) {
            parts.push(value);
        }
    }
    parts.join(" · ")
}

fn translator_label(translator: &Translator, details: &MediaDetails) -> String {
    let mut label = if translator.is_premium {
        trf("Premium · {}", &[&translator.name])
    } else {
        translator.name.clone()
    };
    if let Some(rating) = details
        .voice_ratings
        .iter()
        .find(|voice| voice.title == translator.name)
    {
        label.push_str(&format!("  ·  {:.0}%", rating.rating));
    }
    label
}

fn season_title(season: &Season) -> String {
    if season.title.is_empty() {
        trf("Season {}", &[&season.id])
    } else {
        season.title.clone()
    }
}

/// Order voice-overs by the provider's rating, keeping unrated ones last.
fn sort_by_voice_rating(details: &mut MediaDetails) {
    let rating_of = |translator: &Translator| {
        details
            .voice_ratings
            .iter()
            .find(|voice| voice.title == translator.name)
            .map(|voice| voice.rating)
            .unwrap_or(f32::MIN)
    };
    let ratings = details
        .translators
        .iter()
        .map(|translator| (translator.clone(), rating_of(translator)))
        .collect::<Vec<_>>();
    let mut ordered = ratings;
    ordered.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    details.translators = ordered
        .into_iter()
        .map(|(translator, _)| translator)
        .collect();
}

fn link_media(title: &str, url: &str) -> MediaItem {
    MediaItem {
        id: 0,
        title: title.to_string(),
        orig_title: None,
        url: url.to_string(),
        poster_url: None,
        year: None,
        category: None,
        rating: None,
        info: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{season_title, sort_by_voice_rating};
    use movo_core::client::models::{MediaDetails, MediaType, Season, Translator, VoiceRating};

    fn translator(id: i64, name: &str) -> Translator {
        Translator {
            id,
            name: name.to_string(),
            is_premium: false,
            is_camrip: false,
            has_ads: false,
            is_director_cut: false,
        }
    }

    fn details(translators: Vec<Translator>, voice_ratings: Vec<VoiceRating>) -> MediaDetails {
        MediaDetails {
            id: 1,
            title: String::new(),
            orig_title: None,
            url: String::new(),
            poster_url: None,
            poster_hq_url: None,
            description: String::new(),
            year: None,
            media_type: MediaType::Movie,
            rating_rezka: None,
            rating_imdb: None,
            rating_kp: None,
            genres: Vec::new(),
            countries: Vec::new(),
            directors: Vec::new(),
            actors: Vec::new(),
            duration: None,
            translators,
            seasons: Vec::new(),
            franchises: Vec::new(),
            genre_links: Vec::new(),
            country_links: Vec::new(),
            directors_details: Vec::new(),
            actors_details: Vec::new(),
            ratings: Vec::new(),
            voice_ratings,
            schedules: Vec::new(),
            related: Vec::new(),
            included_in: Vec::new(),
            from_collections: Vec::new(),
            trailer_available: false,
            rating_posted: false,
            favorite_category_ids: Vec::new(),
        }
    }

    #[test]
    fn voice_ratings_sort_known_translators_first() {
        let mut media = details(
            vec![
                translator(1, "Low"),
                translator(2, "High"),
                translator(3, "Unrated"),
            ],
            vec![
                VoiceRating {
                    title: "Low".to_string(),
                    rating: 12.0,
                },
                VoiceRating {
                    title: "High".to_string(),
                    rating: 88.0,
                },
            ],
        );

        sort_by_voice_rating(&mut media);

        let order = media
            .translators
            .iter()
            .map(|translator| translator.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(order, ["High", "Low", "Unrated"]);
    }

    #[test]
    fn seasons_without_a_title_fall_back_to_their_number() {
        let season = Season {
            id: 3,
            title: String::new(),
            episodes: Vec::new(),
        };
        assert_eq!(season_title(&season), "Season 3");
    }
}
