use crate::ui::image::{self, ImageToken};
use movo_core::client::models::MediaItem;
use relm4::gtk::{self, pango, prelude::*};
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::typed_view::list::{RelmListItem, TypedListView};
use std::cell::Cell;
use std::rc::Rc;

const POSTER_WIDTH: i32 = 170;
const POSTER_HEIGHT: i32 = 255;

pub type PosterGrid = TypedGridView<PosterItem, gtk::SingleSelection>;

pub struct PosterItem {
    pub media: MediaItem,
}

impl PosterItem {
    pub fn new(media: MediaItem) -> Self {
        Self { media }
    }
}

pub struct PosterWidgets {
    picture: gtk::Picture,
    title: gtk::Label,
    subtitle: gtk::Label,
    rating: gtk::Label,
    year: gtk::Label,
    token: ImageToken,
}

impl RelmGridItem for PosterItem {
    type Root = gtk::Box;
    type Widgets = PosterWidgets;

    fn setup(_grid_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        build_card()
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        bind_card(&self.media, widgets, root);
    }
}

impl RelmListItem for PosterItem {
    type Root = gtk::Box;
    type Widgets = PosterWidgets;

    fn setup(_list_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        build_card()
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        bind_card(&self.media, widgets, root);
    }
}

fn build_card() -> (gtk::Box, PosterWidgets) {
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Cover)
        .can_shrink(true)
        .width_request(POSTER_WIDTH)
        .height_request(POSTER_HEIGHT)
        .build();
    picture.add_css_class("card");

    let rating = gtk::Label::builder()
        .halign(gtk::Align::End)
        .valign(gtk::Align::Start)
        .margin_top(6)
        .margin_end(6)
        .build();
    rating.add_css_class("osd");

    let year = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .valign(gtk::Align::End)
        .margin_bottom(6)
        .margin_start(6)
        .build();
    year.add_css_class("osd");

    let overlay = gtk::Overlay::builder()
        .width_request(POSTER_WIDTH)
        .height_request(POSTER_HEIGHT)
        .child(&picture)
        .build();
    overlay.add_overlay(&rating);
    overlay.add_overlay(&year);

    // A wrapping label measures at its full text width, which stretches the
    // card it sits in; a fixed character width keeps every card the same size.
    let title = gtk::Label::builder()
        .ellipsize(pango::EllipsizeMode::End)
        .max_width_chars(16)
        .width_chars(16)
        .xalign(0.0)
        .halign(gtk::Align::Start)
        .build();
    title.add_css_class("heading");
    title.set_size_request(POSTER_WIDTH, -1);

    let subtitle = gtk::Label::builder()
        .ellipsize(pango::EllipsizeMode::End)
        .lines(1)
        .max_width_chars(18)
        .width_chars(18)
        .xalign(0.0)
        .halign(gtk::Align::Start)
        .build();
    subtitle.add_css_class("dim-label");
    subtitle.add_css_class("caption");
    subtitle.set_size_request(POSTER_WIDTH, -1);

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .width_request(POSTER_WIDTH)
        .hexpand(false)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Start)
        .margin_bottom(6)
        .build();
    root.append(&overlay);
    root.append(&title);
    root.append(&subtitle);

    let widgets = PosterWidgets {
        picture,
        title,
        subtitle,
        rating,
        year,
        token: Rc::new(Cell::new(0)),
    };
    (root, widgets)
}

fn bind_card(media: &MediaItem, widgets: &mut PosterWidgets, root: &mut gtk::Box) {
    root.set_tooltip_text(Some(&media.title));
    widgets.title.set_label(&media.title);

    widgets.subtitle.set_label(&subtitle_text(media));
    widgets
        .subtitle
        .set_visible(!subtitle_text(media).is_empty());

    match media.rating {
        Some(rating) => {
            widgets.rating.set_label(&format!("★ {rating:.1}"));
            widgets.rating.set_visible(true);
        }
        None => widgets.rating.set_visible(false),
    }

    match media.year {
        Some(year) => {
            widgets.year.set_label(&year.to_string());
            widgets.year.set_visible(true);
        }
        None => widgets.year.set_visible(false),
    }

    image::load(
        &widgets.picture,
        media.poster_url.as_deref(),
        POSTER_WIDTH,
        POSTER_HEIGHT,
        &widgets.token,
    );
}

fn subtitle_text(media: &MediaItem) -> String {
    let mut parts = Vec::new();
    if let Some(category) = &media.category {
        parts.push(category.as_str());
    }
    if let Some(info) = media.info.as_deref() {
        if !info.is_empty() && info != media.title {
            parts.push(info);
        }
    }
    parts.join(" • ")
}

/// Extract the media item a `GridView`/`ListView` activation landed on.
fn activated_media(
    model: &impl IsA<relm4::gtk::gio::ListModel>,
    position: u32,
) -> Option<MediaItem> {
    let object = model.item(position)?;
    let boxed = object.downcast::<relm4::gtk::glib::BoxedAnyObject>().ok()?;
    let item: std::cell::Ref<'_, PosterItem> = boxed.borrow();
    Some(item.media.clone())
}

/// A recycling poster grid that reports the item a click landed on.
pub fn poster_grid<F: Fn(MediaItem) + 'static>(on_activate: F) -> PosterGrid {
    let grid: PosterGrid = TypedGridView::new();
    grid.view.set_single_click_activate(true);
    grid.view.set_max_columns(12);
    grid.view.set_min_columns(1);
    grid.view.set_margin_start(8);
    grid.view.set_margin_end(8);
    grid.view.set_margin_top(8);
    grid.view.set_margin_bottom(8);

    let model = grid.selection_model.clone();
    grid.view.connect_activate(move |_, position| {
        if let Some(media) = activated_media(&model, position) {
            on_activate(media);
        }
    });
    grid
}

pub type PosterRow = TypedListView<PosterItem, gtk::SingleSelection>;

/// A horizontal, recycling poster rail for the home sections.
pub fn poster_row<F: Fn(MediaItem) + 'static>(on_activate: F) -> PosterRow {
    let row: PosterRow = TypedListView::new();
    row.view.set_orientation(gtk::Orientation::Horizontal);
    row.view.set_single_click_activate(true);

    let model = row.selection_model.clone();
    row.view.connect_activate(move |_, position| {
        if let Some(media) = activated_media(&model, position) {
            on_activate(media);
        }
    });
    row
}

#[cfg(test)]
mod tests {
    use super::subtitle_text;
    use movo_core::client::models::MediaItem;

    fn media(title: &str, category: Option<&str>, info: Option<&str>) -> MediaItem {
        MediaItem {
            id: 1,
            title: title.to_string(),
            orig_title: None,
            url: String::new(),
            poster_url: None,
            year: None,
            category: category.map(str::to_string),
            rating: None,
            info: info.map(str::to_string),
        }
    }

    #[test]
    fn subtitle_omits_info_repeating_the_title() {
        assert_eq!(
            subtitle_text(&media("Test", Some("Movie"), Some("Test"))),
            "Movie"
        );
        assert_eq!(
            subtitle_text(&media("Test", Some("Movie"), Some("2021, Drama"))),
            "Movie • 2021, Drama"
        );
        assert_eq!(subtitle_text(&media("Test", None, None)), "");
    }
}
