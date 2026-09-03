use movo_core::client::models::{MediaDetails, Translator};

/// Order voice-overs by the provider's rating, keeping unrated ones last.
pub(super) fn sort_by_voice_rating(details: &mut MediaDetails) {
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

#[cfg(test)]
mod tests {
    use super::sort_by_voice_rating;
    use movo_core::client::models::{MediaDetails, MediaType, Translator, VoiceRating};

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
            has_trailer: false,
            has_posted_rating: false,
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
}
