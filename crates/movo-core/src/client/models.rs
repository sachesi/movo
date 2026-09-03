use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MediaType {
    #[default]
    Movie,
    TVSeries,
    Cartoon,
    Anime,
    Other,
}

impl MediaType {
    pub fn label(&self) -> &'static str {
        match self {
            MediaType::Movie => "Фильм",
            MediaType::TVSeries => "Сериал",
            MediaType::Cartoon => "Мультфильм",
            MediaType::Anime => "Аниме",
            MediaType::Other => "Видео",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogCategory {
    All,
    Films,
    Series,
    Cartoons,
    Animation,
}

impl CatalogCategory {
    pub fn path(&self) -> &'static str {
        match self {
            CatalogCategory::All => "",
            CatalogCategory::Films => "films/",
            CatalogCategory::Series => "series/",
            CatalogCategory::Cartoons => "cartoons/",
            CatalogCategory::Animation => "animation/",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CatalogCategory::All => "Все",
            CatalogCategory::Films => "Фильмы",
            CatalogCategory::Series => "Сериалы",
            CatalogCategory::Cartoons => "Мультфильмы",
            CatalogCategory::Animation => "Аниме",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: i64,
    pub title: String,
    pub orig_title: Option<String>,
    pub url: String,
    pub poster_url: Option<String>,
    pub year: Option<i32>,
    pub category: Option<String>,
    pub rating: Option<f32>,
    pub info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translator {
    pub id: i64,
    pub name: String,
    pub is_premium: bool,
    #[serde(default)]
    pub is_camrip: bool,
    #[serde(default)]
    pub has_ads: bool,
    #[serde(default)]
    pub is_director_cut: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: i64,
    pub season_id: i64,
    pub title: String,
    #[serde(default)]
    pub watch_id: Option<String>,
    #[serde(default)]
    pub is_watched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub id: i64,
    pub title: String,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FranchisePart {
    pub title: String,
    pub url: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedItem {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub url: String,
    pub photo_url: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub name: String,
    pub value: f32,
    pub votes: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceRating {
    pub title: String,
    pub rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleItem {
    pub id: String,
    pub episode: String,
    pub title: String,
    pub original_title: Option<String>,
    pub date: Option<String>,
    pub release_status: Option<String>,
    pub is_released: bool,
    pub is_watched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleGroup {
    pub name: String,
    pub items: Vec<ScheduleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub title: String,
    pub url: String,
    pub image_url: Option<String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeSection {
    pub id: String,
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub date: String,
    pub text: String,
    pub has_spoiler: bool,
    pub indent: usize,
    pub likes: i64,
    #[serde(rename = "liked")]
    pub is_liked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentsPage {
    pub items: Vec<Comment>,
    pub page: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorDetails {
    pub name: String,
    pub original_name: Option<String>,
    pub photo_url: Option<String>,
    pub careers: Vec<String>,
    pub birth_date: Option<String>,
    pub birth_place: Option<String>,
    pub height: Option<String>,
    pub films: Vec<MediaItem>,
    pub roles: Vec<ActorRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorRole {
    pub name: String,
    pub info: String,
    pub films: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub name: String,
    pub path: String,
    pub genres: Vec<LinkedItem>,
    pub years: Vec<LinkedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationGroup {
    pub date: String,
    pub items: Vec<NotificationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationItem {
    pub title: String,
    pub url: String,
    pub info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub notifications: Vec<NotificationGroup>,
    pub premium_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDetails {
    pub id: i64,
    pub title: String,
    pub orig_title: Option<String>,
    pub url: String,
    pub poster_url: Option<String>,
    pub poster_hq_url: Option<String>,
    pub description: String,
    pub year: Option<i32>,
    pub media_type: MediaType,
    pub rating_rezka: Option<f32>,
    pub rating_imdb: Option<f32>,
    pub rating_kp: Option<f32>,
    pub genres: Vec<String>,
    pub countries: Vec<String>,
    pub directors: Vec<String>,
    pub actors: Vec<String>,
    pub duration: Option<String>,
    pub translators: Vec<Translator>,
    pub seasons: Vec<Season>,
    pub franchises: Vec<FranchisePart>,
    #[serde(default)]
    pub genre_links: Vec<LinkedItem>,
    #[serde(default)]
    pub country_links: Vec<LinkedItem>,
    #[serde(default)]
    pub directors_details: Vec<Person>,
    #[serde(default)]
    pub actors_details: Vec<Person>,
    #[serde(default)]
    pub ratings: Vec<Rating>,
    #[serde(default)]
    pub voice_ratings: Vec<VoiceRating>,
    #[serde(default)]
    pub schedules: Vec<ScheduleGroup>,
    #[serde(default)]
    pub related: Vec<MediaItem>,
    #[serde(default)]
    pub included_in: Vec<LinkedItem>,
    #[serde(default)]
    pub from_collections: Vec<LinkedItem>,
    #[serde(default, rename = "trailer_available")]
    pub has_trailer: bool,
    #[serde(default, rename = "rating_posted")]
    pub has_posted_rating: bool,
    #[serde(default)]
    pub favorite_category_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEntry {
    pub quality: String,
    pub is_premium: bool,
    pub urls: Vec<String>,
}

impl StreamEntry {
    pub fn best_url(&self) -> Option<&str> {
        self.urls.first().map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub code: String,
    pub title: String,
    pub url: String,
    pub language_code: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamBundle {
    pub id: i64,
    pub translator_id: i64,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub streams: Vec<StreamEntry>,
    pub subtitles: Vec<SubtitleTrack>,
    #[serde(default)]
    pub storyboard_url: Option<String>,
    #[serde(default)]
    pub storyboard: Vec<StoryboardCue>,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub referer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryboardCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub image_url: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritesCollection {
    pub id: Option<i64>,
    pub name: String,
    pub url: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHistoryEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub poster_url: Option<String>,
    pub info: Option<String>,
    pub additional_info: Option<String>,
    pub date: Option<String>,
    pub is_watched: bool,
}

impl ServerHistoryEntry {
    pub fn media_id(&self) -> Option<i64> {
        self.url
            .trim_end_matches('/')
            .rsplit('/')
            .next()?
            .split('-')
            .next()?
            .parse()
            .ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub is_logged_in: bool,
    pub is_vip: bool,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub premium_days: Option<u32>,
    #[serde(default, rename = "session_persistent")]
    pub is_session_persistent: bool,
}

#[cfg(test)]
mod tests {
    use super::{Comment, UserProfile};

    /// The Android client reads these keys, so the wire names have to survive a
    /// rename of the Rust fields behind them.
    #[test]
    fn boolean_fields_keep_the_key_the_client_reads() {
        let comment = Comment {
            id: "c1".to_string(),
            username: "User".to_string(),
            avatar_url: None,
            date: "Today".to_string(),
            text: "Hello".to_string(),
            has_spoiler: false,
            indent: 0,
            likes: 7,
            is_liked: true,
        };
        let comment = serde_json::to_value(&comment).unwrap();
        assert_eq!(comment["liked"], true);
        assert!(comment.get("is_liked").is_none());

        let profile = UserProfile {
            user_id: "42".to_string(),
            username: "User".to_string(),
            is_logged_in: true,
            is_vip: false,
            email: None,
            avatar_url: None,
            premium_days: None,
            is_session_persistent: true,
        };
        let profile = serde_json::to_value(&profile).unwrap();
        assert_eq!(profile["session_persistent"], true);
        assert!(profile.get("is_session_persistent").is_none());
    }
}
