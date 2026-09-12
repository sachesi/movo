use super::format::{caption, chip_box, metadata_line, translator_label};
use super::{DetailsMsg, DetailsView, POSTER_HEIGHT, POSTER_WIDTH};
use crate::i18n::{tr, trf};
use crate::ui::image;
use movo_core::client::models::MediaDetails;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};

impl DetailsView {
    pub(super) fn hero(
        &self,
        details: &MediaDetails,
        sender: &relm4::ComponentSender<Self>,
    ) -> gtk::Box {
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
        button.update_property(&[gtk::accessible::Property::Label(tr("Favorite"))]);

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
    pub(super) fn playback_group(
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

    pub(super) fn actions(
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
            // A title not released yet has no voice-over to play in.
            watch.set_sensitive(!details.translators.is_empty());
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

    pub(super) fn schedule(
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
                    watched.update_property(&[gtk::accessible::Property::Label(tr(
                        "Toggle Watched Status",
                    ))]);
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

    pub(super) fn links(
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
}
