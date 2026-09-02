use super::models::{CatalogCategory, MediaItem};
use super::session::RezkaSession;
use scraper::{Html, Selector};

pub struct CatalogScraper;

impl CatalogScraper {
    pub async fn fetch_catalog(
        session: &RezkaSession,
        category: CatalogCategory,
        filter: Option<&str>, // "popular", "watching", "new", "last"
        page: usize,
    ) -> Result<Vec<MediaItem>, String> {
        let mut url_path = category.path().to_string();

        if page > 1 {
            url_path.push_str(&format!("page/{}/", page));
        }

        let mut query_params = Vec::new();
        if let Some(f) = filter {
            let f_clean = f.trim_matches('/');
            if !f_clean.is_empty() {
                query_params.push(format!("filter={}", f_clean));
            }
        }

        if !query_params.is_empty() {
            url_path.push('?');
            url_path.push_str(&query_params.join("&"));
        }

        let html_content = session.get_html(&url_path).await?;
        let items = Self::parse_catalog_html(&html_content);
        Ok(items)
    }

    pub fn parse_catalog_html(html: &str) -> Vec<MediaItem> {
        let document = Html::parse_document(html);
        let item_selector = Selector::parse(".b-content__inline_item").unwrap();
        let link_selector = Selector::parse(".b-content__inline_item-link a").unwrap();
        let cover_selector = Selector::parse(".b-content__inline_item-cover img").unwrap();
        let misc_selector = Selector::parse(".b-content__inline_item-link div").unwrap();
        let cat_selector = Selector::parse(".cat").unwrap();
        let rating_selector =
            Selector::parse(".rating, .b-content__inline_item-cover .rating").unwrap();

        let mut results = Vec::new();

        for element in document.select(&item_selector) {
            let (title, url, id) = if let Some(link) = element.select(&link_selector).next() {
                let href = link.value().attr("href").unwrap_or_default().to_string();
                let title = link.text().collect::<String>().trim().to_string();
                let id = Self::extract_id_from_url(&href);
                (title, href, id)
            } else {
                continue;
            };

            let poster_url = element
                .select(&cover_selector)
                .next()
                .and_then(|img| {
                    img.value()
                        .attr("data-src")
                        .or_else(|| img.value().attr("data-original"))
                        .or_else(|| img.value().attr("src"))
                })
                .map(Self::secure_poster_url);

            let info = element
                .select(&misc_selector)
                .next()
                .map(|div| div.text().collect::<String>().trim().to_string());

            let year = info
                .as_ref()
                .and_then(|info_text| Self::extract_year(info_text));

            let rating = element
                .select(&rating_selector)
                .next()
                .and_then(|r| r.text().collect::<String>().trim().parse::<f32>().ok());

            let category = element.select(&cat_selector).next().and_then(|cat| {
                cat.value()
                    .attr("class")
                    .map(|c| c.replace("cat", "").trim().to_string())
            });

            results.push(MediaItem {
                id,
                title,
                orig_title: None,
                url,
                poster_url,
                year,
                category,
                rating,
                info,
            });
        }

        results
    }

    fn extract_id_from_url(url: &str) -> i64 {
        let clean = url.trim_end_matches('/').trim_end_matches(".html");
        if let Some(pos) = clean.rfind('/') {
            let slug = &clean[pos + 1..];
            if let Some(dash) = slug.find('-') {
                if let Ok(id) = slug[..dash].parse::<i64>() {
                    return id;
                }
            }
        }
        0
    }

    fn secure_poster_url(url: &str) -> String {
        url.strip_prefix("http://")
            .map(|value| format!("https://{value}"))
            .or_else(|| {
                url.strip_prefix("//")
                    .map(|value| format!("https://{value}"))
            })
            .unwrap_or_else(|| url.to_string())
    }

    fn extract_year(info: &str) -> Option<i32> {
        let parts: Vec<&str> = info.split(',').collect();
        if let Some(first) = parts.first() {
            let year_str: String = first.chars().filter(|c| c.is_ascii_digit()).collect();
            if year_str.len() == 4 {
                return year_str.parse::<i32>().ok();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::CatalogScraper;

    #[test]
    fn catalog_uses_secure_lazy_poster_url() {
        let items = CatalogScraper::parse_catalog_html(
            r#"<div class="b-content__inline_item">
                <div class="b-content__inline_item-cover">
                    <img src="placeholder.gif" data-src="http://statichdrezka.ac/poster.jpg">
                </div>
                <div class="b-content__inline_item-link"><a href="/1-test.html">Test</a></div>
            </div>"#,
        );

        assert_eq!(
            items[0].poster_url.as_deref(),
            Some("https://statichdrezka.ac/poster.jpg")
        );
    }
}
