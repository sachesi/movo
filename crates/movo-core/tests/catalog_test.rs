use movo_core::client::models::CatalogCategory;
use movo_core::client::RezkaClient;

#[tokio::test]
#[ignore = "hits the live provider"]
async fn test_category_pages() {
    let client = RezkaClient::new();

    let categories = [
        (CatalogCategory::All, "All"),
        (CatalogCategory::Films, "Films"),
        (CatalogCategory::Series, "Series"),
        (CatalogCategory::Cartoons, "Cartoons"),
        (CatalogCategory::Animation, "Animation"),
    ];

    for (cat, name) in categories {
        println!("Testing category: {}", name);
        let p1 = client.fetch_catalog(cat, None, 1).await.expect("p1 failed");
        let p2 = client.fetch_catalog(cat, None, 2).await.expect("p2 failed");

        println!(
            "  Page 1 ({} items): first is '{}'",
            p1.len(),
            p1.first().map(|i| i.title.as_str()).unwrap_or("")
        );
        println!(
            "  Page 2 ({} items): first is '{}'",
            p2.len(),
            p2.first().map(|i| i.title.as_str()).unwrap_or("")
        );

        assert!(!p1.is_empty(), "Page 1 of {} is empty", name);
        assert!(!p2.is_empty(), "Page 2 of {} is empty", name);
        if let (Some(f1), Some(f2)) = (p1.first(), p2.first()) {
            assert_ne!(
                f1.id, f2.id,
                "Page 1 and Page 2 should have different items for {}",
                name
            );
        }
    }
}
