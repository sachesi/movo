use crate::i18n::tr;
use crate::state::AppState;
use crate::ui::image::{self, ImageToken};
use movo_core::client::models::{ActorDetails, MediaItem};
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk::{self, prelude::WidgetExt};
use std::cell::Cell;
use std::rc::Rc;

/// Show a person's profile, roles and filmography.
///
/// Selecting a title from the filmography closes the dialog and reports it
/// through `on_open`.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    state: Rc<AppState>,
    url: String,
    on_open: impl Fn(MediaItem) + 'static,
    on_error: impl Fn(String) + 'static,
) {
    let dialog = adw::Dialog::builder()
        .title(tr("Profile"))
        .content_width(560)
        .content_height(640)
        .build();

    let spinner = adw::StatusPage::builder().vexpand(true).build();
    spinner.set_paintable(Some(&adw::SpinnerPaintable::new(Some(&spinner))));

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&spinner));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));

    let client = state.client.clone();
    relm4::spawn_local(async move {
        let loaded = relm4::spawn(async move { client.fetch_actor(&url).await })
            .await
            .unwrap_or_else(|_| Err(tr("Loading the profile stopped unexpectedly").to_string()));

        match loaded {
            Ok(actor) => toolbar.set_content(Some(&profile(actor, &dialog, on_open))),
            Err(error) => {
                dialog.close();
                on_error(error);
            }
        }
    });
}

fn profile(
    actor: ActorDetails,
    dialog: &adw::Dialog,
    on_open: impl Fn(MediaItem) + 'static,
) -> gtk::Widget {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .build();

    let photo = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Cover)
        .width_request(120)
        .height_request(160)
        .build();
    photo.add_css_class("card");
    let token: ImageToken = Rc::new(Cell::new(0));
    image::load(&photo, actor.photo_url.as_deref(), 120, 160, &token);
    // The token lives as long as the photo it guards.
    unsafe { photo.set_data("movo-image-token", token) };
    header.append(&photo);

    let facts = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();
    let name = gtk::Label::builder()
        .label(&actor.name)
        .xalign(0.0)
        .wrap(true)
        .build();
    name.add_css_class("title-2");
    facts.append(&name);

    for line in [
        actor.original_name.clone(),
        (!actor.careers.is_empty()).then(|| actor.careers.join(", ")),
        actor.birth_date.clone(),
        actor.birth_place.clone(),
        actor.height.clone(),
    ]
    .into_iter()
    .flatten()
    {
        let label = gtk::Label::builder()
            .label(&line)
            .xalign(0.0)
            .wrap(true)
            .build();
        label.add_css_class("dim-label");
        facts.append(&label);
    }
    header.append(&facts);
    content.append(&header);

    if !actor.roles.is_empty() {
        let roles = adw::PreferencesGroup::builder().title(tr("Roles")).build();
        for role in &actor.roles {
            roles.add(
                &adw::ActionRow::builder()
                    .use_markup(false)
                    .title(&role.name)
                    .subtitle(&role.info)
                    .build(),
            );
        }
        content.append(&roles);
    }

    if !actor.films.is_empty() {
        let films = adw::PreferencesGroup::builder()
            .title(tr("Filmography"))
            .build();
        let on_open = Rc::new(on_open);
        for film in &actor.films {
            let row = adw::ActionRow::builder()
                .use_markup(false)
                .title(&film.title)
                .subtitle(film.info.as_deref().unwrap_or_default())
                .activatable(true)
                .build();
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            let on_open = on_open.clone();
            let film = film.clone();
            let dialog = dialog.clone();
            row.connect_activated(move |_| {
                on_open(film.clone());
                dialog.close();
            });
            films.add(&row);
        }
        content.append(&films);
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&content)
        .build()
        .upcast()
}
