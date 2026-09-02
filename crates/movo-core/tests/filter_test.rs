use movo_core::client::catalog::CatalogScraper;
use movo_core::client::session::RezkaSession;

#[tokio::test]
#[ignore = "hits the live provider"]
async fn test_query_param_filters() {
    let session = RezkaSession::new();

    let endpoints = [
        ("Films Last", "films/?filter=last"),
        ("Films Popular", "films/?filter=popular"),
        ("Films Watching", "films/?filter=watching"),
        ("Films New", "films/?filter=new"),
        ("Series Last", "series/?filter=last"),
        ("Series Popular", "series/?filter=popular"),
        ("Series Watching", "series/?filter=watching"),
        ("Cartoons Last", "cartoons/?filter=last"),
        ("Cartoons Popular", "cartoons/?filter=popular"),
        ("Animation Last", "animation/?filter=last"),
        ("Animation Popular", "animation/?filter=popular"),
    ];

    for (label, path) in endpoints {
        let html = session.get_html(path).await.expect("get_html failed");
        let items = CatalogScraper::parse_catalog_html(&html);
        println!(
            "{}: Got {} items. First: '{}' (ID: {})",
            label,
            items.len(),
            items.first().map(|i| i.title.as_str()).unwrap_or(""),
            items.first().map(|i| i.id).unwrap_or(0)
        );
    }
}

#[tokio::test]
#[ignore = "hits the live provider"]
async fn test_filtered_pagination() {
    let session = RezkaSession::new();

    let p1 = session.get_html("series/?filter=watching").await.unwrap();
    let p2 = session
        .get_html("series/page/2/?filter=watching")
        .await
        .unwrap();

    let items1 = CatalogScraper::parse_catalog_html(&p1);
    let items2 = CatalogScraper::parse_catalog_html(&p2);

    println!(
        "Series Watching P1 first: '{}' (ID: {})",
        items1[0].title, items1[0].id
    );
    println!(
        "Series Watching P2 first: '{}' (ID: {})",
        items2[0].title, items2[0].id
    );

    assert_ne!(items1[0].id, items2[0].id);
}
