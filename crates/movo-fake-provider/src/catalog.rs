//! The invented library the provider serves. Everything is derived from a
//! title's id, so two runs show the same thing and tests can name their
//! expectations.

pub const TITLES_PER_PAGE: usize = 36;

/// A title whose name is markup, to catch a view that renders text as markup.
pub const MARKUP_TITLE_ID: i64 = 66;
/// A title announced but not released, which has no player.
pub const ANNOUNCED_TITLE_ID: i64 = 901;
/// A series whose one voice-over the provider refuses episodes for.
pub const REFUSED_SERIES_ID: i64 = 902;

const ADJECTIVES: [&str; 12] = [
    "Северный",
    "Тихий",
    "Последний",
    "Красный",
    "Забытый",
    "Ночной",
    "Дальний",
    "Белый",
    "Старый",
    "Золотой",
    "Зимний",
    "Морской",
];
const NOUNS: [&str; 10] = [
    "маяк",
    "поезд",
    "город",
    "берег",
    "сад",
    "дом",
    "ветер",
    "остров",
    "мост",
    "день",
];
const COUNTRIES: [&str; 6] = [
    "США",
    "Россия",
    "Франция",
    "Великобритания",
    "Украина",
    "Япония",
];
const GENRES: [&str; 5] = ["Драмы", "Комедии", "Боевики", "Триллеры", "Фантастика"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Film,
    Series,
    Cartoon,
    Anime,
}

impl Kind {
    pub fn path(self) -> &'static str {
        match self {
            Kind::Film => "films",
            Kind::Series => "series",
            Kind::Cartoon => "cartoons",
            Kind::Anime => "animation",
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        match path {
            "films" => Some(Kind::Film),
            "series" => Some(Kind::Series),
            "cartoons" => Some(Kind::Cartoon),
            "animation" => Some(Kind::Anime),
            _ => None,
        }
    }

    pub fn is_episodic(self) -> bool {
        matches!(self, Kind::Series | Kind::Anime)
    }
}

#[derive(Debug, Clone)]
pub struct Title {
    pub id: i64,
    pub name: String,
    pub kind: Kind,
    pub year: i32,
    pub country: &'static str,
    pub genre: &'static str,
    pub rating: f32,
}

impl Title {
    pub fn url_path(&self) -> String {
        format!(
            "/{}/{}/{}-title-{}.html",
            self.kind.path(),
            self.genre_slug(),
            self.id,
            self.year
        )
    }

    fn genre_slug(&self) -> &'static str {
        match self.genre {
            "Драмы" => "drama",
            "Комедии" => "comedy",
            "Боевики" => "action",
            "Триллеры" => "thriller",
            _ => "fiction",
        }
    }

    pub fn info(&self) -> String {
        format!("{}, {}, {}", self.year, self.country, self.genre)
    }
}

/// The title with this id, for ids 1..=240 and the special ones above.
pub fn title(id: i64) -> Option<Title> {
    match id {
        1..=240 | ANNOUNCED_TITLE_ID | REFUSED_SERIES_ID => Some(build(id)),
        _ => None,
    }
}

fn build(id: i64) -> Title {
    let index = id as usize;
    let kind = match id {
        ANNOUNCED_TITLE_ID => Kind::Film,
        REFUSED_SERIES_ID => Kind::Series,
        _ => match index % 4 {
            0 => Kind::Film,
            1 => Kind::Series,
            2 => Kind::Cartoon,
            _ => Kind::Anime,
        },
    };
    let name = if id == MARKUP_TITLE_ID {
        "<b>Жирный</b> & \"кавычки\" <span foreground='red'>разметка</span>".to_string()
    } else {
        format!(
            "{} {} {id}",
            ADJECTIVES[index % ADJECTIVES.len()],
            NOUNS[index % NOUNS.len()]
        )
    };
    Title {
        id,
        name,
        kind,
        year: 1990 + (id % 35) as i32,
        country: COUNTRIES[index % COUNTRIES.len()],
        genre: GENRES[index % GENRES.len()],
        rating: 5.0 + (id % 50) as f32 / 10.0,
    }
}

pub fn all() -> impl Iterator<Item = Title> {
    (1..=240).map(build)
}

pub fn of_kind(kind: Kind) -> Vec<Title> {
    all().filter(|title| title.kind == kind).collect()
}

/// Reads the title id out of a page path such as `/films/drama/12-title-2002.html`.
pub fn id_from_path(path: &str) -> Option<i64> {
    let slug = path.trim_end_matches('/').rsplit('/').next()?;
    if !slug.ends_with(".html") {
        return None;
    }
    slug.split('-').next()?.parse().ok()
}

pub fn page_of<T: Clone>(items: &[T], page: usize) -> Vec<T> {
    items
        .chunks(TITLES_PER_PAGE)
        .nth(page.saturating_sub(1))
        .map(<[T]>::to_vec)
        .unwrap_or_default()
}
