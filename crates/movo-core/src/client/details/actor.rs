use super::DetailsScraper;
use crate::client::catalog::CatalogScraper;
use crate::client::models::{ActorDetails, ActorRole};
use crate::client::session::RezkaSession;
use scraper::{Html, Selector};

impl DetailsScraper {
    pub async fn fetch_actor(session: &RezkaSession, url: &str) -> Result<ActorDetails, String> {
        let html = session.get_html(url).await?;
        let mut actor = Self::parse_actor_html(&html);
        actor.photo_url = actor.photo_url.map(|value| session.resolve_url(&value));
        Ok(actor)
    }

    fn parse_actor_html(html: &str) -> ActorDetails {
        let document = Html::parse_document(html);
        let text = |selector: &str| {
            document
                .select(&Selector::parse(selector).unwrap())
                .next()
                .map(|node| node.text().collect::<String>().trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let mut actor = ActorDetails {
            name: text(".b-post__title .t1").unwrap_or_default(),
            original_name: text(".b-post__title .t2"),
            photo_url: document
                .select(&Selector::parse(".b-sidecover img").unwrap())
                .next()
                .and_then(|node| node.value().attr("src"))
                .map(str::to_string),
            careers: Vec::new(),
            birth_date: None,
            birth_place: None,
            height: None,
            films: CatalogScraper::parse_catalog_html(html),
            roles: document
                .select(&Selector::parse(".b-person__career").unwrap())
                .map(|role| ActorRole {
                    name: role
                        .select(&Selector::parse("h2").unwrap())
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    info: role
                        .select(&Selector::parse(".b-person__career_stats").unwrap())
                        .next()
                        .map(|node| node.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    films: CatalogScraper::parse_catalog_html(&role.html()),
                })
                .collect(),
        };
        let cells = Selector::parse(".b-post__info tr").unwrap();
        let td = Selector::parse("td").unwrap();
        for row in document.select(&cells) {
            let parts = row.select(&td).collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            let key = parts[0].text().collect::<String>().to_lowercase();
            let value = parts[1].text().collect::<String>().trim().to_string();
            if key.contains("дата рождения") {
                actor.birth_date = Some(value);
            } else if key.contains("место рождения") {
                actor.birth_place = Some(value);
            } else if key.contains("рост") {
                actor.height = Some(value);
            } else if key.contains("карьера") {
                actor.careers = parts[1]
                    .select(&Selector::parse("a").unwrap())
                    .map(|node| node.text().collect::<String>().trim().to_string())
                    .collect();
            }
        }
        actor
    }
}

#[cfg(test)]
mod tests {
    use super::DetailsScraper;

    #[test]
    fn parses_actor_pages() {
        let actor = DetailsScraper::parse_actor_html(
            r#"<div class="b-post__title"><span class="t1">Name</span><span class="t2">Original</span></div><div class="b-sidecover"><img src="photo.jpg"></div><table class="b-post__info"><tr><td>Дата рождения:</td><td>1 Jan</td></tr><tr><td>Карьера:</td><td><a>Actor</a></td></tr></table>"#,
        );
        assert_eq!(actor.name, "Name");
        assert_eq!(actor.careers, vec!["Actor"]);
    }
}
