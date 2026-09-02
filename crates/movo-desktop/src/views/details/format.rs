use crate::i18n::trf;
use movo_core::client::models::{MediaDetails, MediaItem, Season, Translator};
use relm4::gtk::{self, prelude::WidgetExt};

/// A wrapping row of flat buttons. `homogeneous` is off and the children are
/// start-aligned so a single chip does not stretch across the page.
pub(super) fn chip_box() -> gtk::FlowBox {
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

pub(super) fn caption(text: &str) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .build();
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label
}

pub(super) fn metadata_line(details: &MediaDetails) -> String {
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

pub(super) fn translator_label(translator: &Translator, details: &MediaDetails) -> String {
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

pub(super) fn season_title(season: &Season) -> String {
    if season.title.is_empty() {
        trf("Season {}", &[&season.id])
    } else {
        season.title.clone()
    }
}

pub(super) fn link_media(title: &str, url: &str) -> MediaItem {
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
    use super::season_title;
    use movo_core::client::models::Season;

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
