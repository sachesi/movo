use crate::i18n::{tr, trf};
use movo_core::client::models::{MediaDetails, MediaItem, MediaType, Season, Translator};
use relm4::adw;
use relm4::gtk::{self, prelude::WidgetExt};

pub(super) fn chip_box() -> adw::WrapBox {
    adw::WrapBox::builder()
        .child_spacing(4)
        .line_spacing(4)
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

fn media_type_label(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Movie => tr("Film"),
        MediaType::TVSeries => tr("TV Series"),
        MediaType::Cartoon => tr("Cartoon"),
        MediaType::Anime => tr("Anime"),
        MediaType::Other => tr("Video"),
    }
}

pub(super) fn metadata_line(details: &MediaDetails) -> String {
    let mut parts = vec![media_type_label(details.media_type).to_string()];
    if let Some(year) = details.year {
        parts.push(year.to_string());
    }
    if let Some(rating) = details.rating_rezka {
        parts.push(format!("Rezka {rating:.1}"));
    }
    // The itemised ratings name the same sources with their vote counts.
    if details.ratings.is_empty() {
        if let Some(rating) = details.rating_imdb {
            parts.push(format!("IMDb {rating:.1}"));
        }
        if let Some(rating) = details.rating_kp {
            parts.push(format!("KinoPoisk {rating:.1}"));
        }
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
