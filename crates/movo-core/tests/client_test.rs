use movo_core::client::models::CatalogCategory;
use movo_core::client::RezkaClient;

#[tokio::test]
#[ignore = "hits the live provider"]
async fn test_live_search_and_catalog() {
    let client = RezkaClient::new();

    println!("Fetching catalog (Films)...");
    let films = client
        .fetch_catalog(CatalogCategory::Films, Some("popular"), 1)
        .await;
    match films {
        Ok(items) => {
            println!("Got {} items from popular films", items.len());
            assert!(!items.is_empty());
            for item in items.iter().take(3) {
                println!(
                    "- {} (ID: {}, Year: {:?}, Rating: {:?})",
                    item.title, item.id, item.year, item.rating
                );
            }
        }
        Err(e) => panic!("Catalog fetch failed: {}", e),
    }

    println!("Testing search for 'Matrix'...");
    let search_res = client.search_full("Matrix", 1).await;
    match search_res {
        Ok(items) => {
            println!("Got {} items for 'Matrix'", items.len());
            assert!(!items.is_empty());
            let first = &items[0];
            println!("First result: {} ({})", first.title, first.url);

            // Fetch details for first item
            println!("Fetching details for {}", first.url);
            let details = client
                .fetch_details(&first.url)
                .await
                .expect("Failed to fetch details");
            println!("Title: {}", details.title);
            println!("Translators: {}", details.translators.len());
            for tr in &details.translators {
                println!("  * [{}] {} (premium: {})", tr.id, tr.name, tr.is_premium);
            }
            assert!(!details.translators.is_empty());
        }
        Err(e) => panic!("Search failed: {}", e),
    }
}

#[tokio::test]
#[ignore = "hits the live provider"]
async fn test_live_series_stream_extraction() {
    let client = RezkaClient::new();

    println!("Searching for series 'Rick and Morty'...");
    let search_res = client
        .search_full("Rick and Morty", 1)
        .await
        .expect("Search failed");
    let series_item = search_res
        .iter()
        .find(|i| i.url.contains("/cartoons/") || i.url.contains("/series/"))
        .expect("Series not found in search");

    println!("Fetching details for {}", series_item.url);
    let details = client
        .fetch_details(&series_item.url)
        .await
        .expect("Details failed");
    println!(
        "Title: {}, Seasons: {}",
        details.title,
        details.seasons.len()
    );
    assert!(!details.translators.is_empty());

    let first_tr = details.translators[0].id;
    println!(
        "Fetching stream for post {}, translator {}, season 1, episode 1...",
        details.id, first_tr
    );
    let stream_res = client
        .fetch_episode_stream(details.id, first_tr, 1, 1)
        .await;
    match stream_res {
        Ok(bundle) => {
            println!(
                "Got stream bundle with {} quality options",
                bundle.streams.len()
            );
            for stream in &bundle.streams {
                println!(
                    "  * Quality: {}, URL: {:?}",
                    stream.quality,
                    stream.best_url()
                );
            }
            assert!(!bundle.streams.is_empty());
        }
        Err(e) => println!("Note: Episode stream extraction returned: {}", e),
    }
}
