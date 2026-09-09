use super::catalog;
use super::models::{Collection, HomeSection, LinkedItem, MediaItem, SearchFilter};
use super::session::RezkaSession;
use crate::error::{ClientError, ErrorKind};
use scraper::{Html, Selector};
use std::time::Duration;

/// How long any one home rail may take. The rails are fetched together and shown together, so
/// one page hanging to the session's full timeout and its retries held the four that had arrived.
const HOME_SECTION_TIMEOUT: Duration = Duration::from_secs(15);

async fn bounded(
    page: impl std::future::Future<Output = Result<String, ClientError>>,
) -> Result<String, ClientError> {
    tokio::time::timeout(HOME_SECTION_TIMEOUT, page)
        .await
        .unwrap_or_else(|_| Err(ClientError::new(ErrorKind::Timeout, "Timed out")))
}

pub async fn home(session: &RezkaSession) -> Result<Vec<HomeSection>, ClientError> {
    let hot =
        bounded(session.post_ajax("engine/ajax/get_newest_slider_content.php", &[("id", "0")]));
    let new = bounded(session.get_html("new"));
    let watching = bounded(session.get_html("new?filter=watching"));
    let popular = bounded(session.get_html("new?filter=popular"));
    let awaiting = bounded(session.get_html("announce"));
    let (hot, new, watching, popular, awaiting) =
        tokio::join!(hot, new, watching, popular, awaiting);
    // One slow or refused page must not take the whole home page down with it: the
    // sections that did arrive are shown, and only a home with nothing in it is an error.
    let mut failure = None;
    let sections: Vec<HomeSection> = [
        ("hot", hot),
        ("new", new),
        ("watching", watching),
        ("popular", popular),
        ("awaiting", awaiting),
    ]
    .into_iter()
    .filter_map(|(id, html)| match html {
        Ok(html) => Some(HomeSection {
            id: id.to_string(),
            items: catalog::parse_catalog_html(&html),
        }),
        Err(error) => {
            log::warn!("Home section {id} failed: {error}");
            failure.get_or_insert(error);
            None
        }
    })
    .filter(|section| !section.items.is_empty())
    .collect();
    if sections.is_empty() {
        return Err(failure.unwrap_or_else(|| {
            ClientError::new(ErrorKind::Other, "The home page came back empty")
        }));
    }
    Ok(sections)
}

pub async fn suggestions(session: &RezkaSession, query: &str) -> Result<Vec<String>, String> {
    let html = session
        .post_ajax("engine/ajax/search.php", &[("q", query)])
        .await?;
    Ok(parse_suggestions(&html))
}

fn parse_suggestions(html: &str) -> Vec<String> {
    let document = Html::parse_fragment(html);
    let mut suggestions = Vec::new();
    for node in document.select(&Selector::parse("li .enty").unwrap()) {
        let value = node.text().collect::<String>().trim().to_string();
        if !value.is_empty() && !suggestions.contains(&value) {
            suggestions.push(value);
        }
    }
    suggestions
}

pub async fn filters(session: &RezkaSession) -> Result<Vec<SearchFilter>, String> {
    Ok(parse_filters(&session.get_html("collections").await?))
}

fn parse_filters(html: &str) -> Vec<SearchFilter> {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse("li.b-topnav__item:not(.single)").unwrap())
        .filter_map(|category| {
            let link = category
                .select(&Selector::parse(".b-topnav__item-link").unwrap())
                .next()?;
            let name = link.text().collect::<String>().trim().to_string();
            (!name.is_empty()).then(|| SearchFilter {
                name,
                path: link.value().attr("href").unwrap_or_default().to_string(),
                genres: options(&category, "select.select-category option"),
                years: std::iter::once(LinkedItem {
                    name: "Recent".to_string(),
                    url: "-1".to_string(),
                })
                .chain(options(&category, "select.select-year option"))
                .collect(),
            })
        })
        .collect()
}

fn options(category: &scraper::ElementRef<'_>, selector: &str) -> Vec<LinkedItem> {
    let mut values = Vec::new();
    for option in category.select(&Selector::parse(selector).unwrap()) {
        let value = LinkedItem {
            name: option.text().collect::<String>().trim().to_string(),
            url: option.value().attr("value").unwrap_or_default().to_string(),
        };
        if !value.name.is_empty() && !values.iter().any(|item: &LinkedItem| item.url == value.url) {
            values.push(value);
        }
    }
    values
}

pub async fn collections(session: &RezkaSession, page: usize) -> Result<Vec<Collection>, String> {
    let path = if page > 1 {
        format!("collections/page/{page}/")
    } else {
        "collections/".to_string()
    };
    let mut collections = parse_collections(&session.get_html(&path).await?);
    for collection in &mut collections {
        collection.image_url = collection
            .image_url
            .take()
            .map(|value| session.resolve_url(&value));
    }
    Ok(collections)
}

fn parse_collections(html: &str) -> Vec<Collection> {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse(".b-content__collections_item").unwrap())
        .map(|item| Collection {
            title: item
                .select(&Selector::parse(".title").unwrap())
                .next()
                .map(|node| node.text().collect::<String>().trim().to_string())
                .unwrap_or_default(),
            url: item
                .value()
                .attr("data-url")
                .unwrap_or_default()
                .to_string(),
            image_url: item
                .select(&Selector::parse(".cover").unwrap())
                .next()
                .and_then(|node| node.value().attr("src"))
                .map(str::to_string),
            count: item
                .select(&Selector::parse(".num").unwrap())
                .next()
                .map(|node| {
                    node.text()
                        .collect::<String>()
                        .chars()
                        .filter(|ch| ch.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0)
                })
                .unwrap_or(0),
        })
        .filter(|item| !item.title.is_empty() && !item.url.is_empty())
        .collect()
}

pub async fn path(
    session: &RezkaSession,
    path: &str,
    page: usize,
) -> Result<Vec<MediaItem>, String> {
    Ok(catalog::parse_catalog_html(
        &session.get_html(&paged_path(path, page)).await?,
    ))
}

/// The page segment goes before any query string, the way the site's own
/// listing links are built: `new?filter=watching` pages as
/// `new/page/2/?filter=watching`.
fn paged_path(path: &str, page: usize) -> String {
    if page <= 1 {
        return path.to_string();
    }
    let (base, query) = match path.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (path, None),
    };
    let mut paged = format!("{}/page/{page}/", base.trim_matches('/'));
    if let Some(query) = query {
        paged.push('?');
        paged.push_str(query);
    }
    paged
}

pub async fn search_full(
    session: &RezkaSession,
    query: &str,
    page: usize,
) -> Result<Vec<MediaItem>, String> {
    let search_path = if page > 1 {
        format!(
            "search/?do=search&subaction=search&q={}&page={}",
            urlencoding(query),
            page
        )
    } else {
        format!(
            "search/?do=search&subaction=search&q={}",
            urlencoding(query)
        )
    };

    let html_content = session.get_html(&search_path).await?;
    let items = catalog::parse_catalog_html(&html_content);
    Ok(items)
}
fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::{paged_path, parse_collections, parse_suggestions};

    #[test]
    fn paging_keeps_the_query_string_after_the_page_segment() {
        assert_eq!(paged_path("new", 1), "new");
        assert_eq!(paged_path("/announce/", 3), "announce/page/3/");
        assert_eq!(
            paged_path("new?filter=watching", 2),
            "new/page/2/?filter=watching"
        );
    }

    #[test]
    fn parses_unique_suggestions_and_collections() {
        assert_eq!(
            parse_suggestions(
                "<li><span class='enty'>One</span></li><li><span class='enty'>One</span></li>"
            ),
            vec!["One"]
        );
        let collections = parse_collections("<div class='b-content__collections_item' data-url='/collections/a/'><img class='cover' src='a.jpg'><span class='title'>A</span><span class='num'>12</span></div>");
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].count, 12);
    }
}
