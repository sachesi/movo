use movo_core::client::details::DetailsScraper;
use movo_core::client::stream::StreamExtractor;
use std::collections::HashMap;

#[test]
fn test_parse_cleartext_streams() {
    let raw = "[360p]https://cdn.example.com/360.mp4:hls:manifest.m3u8 or https://cdn.example.com/360.mp4,[720p]https://cdn.example.com/720.mp4:hls:manifest.m3u8,[<span class=\"pjs-prem-quality\">1080p Ultra</span>]https://cdn.example.com/1080u.mp4,[<i class=\"pjs-prem-quality\">4K (2160p)</i>]https://cdn.example.com/4k.mp4:hls:manifest.m3u8";
    let entries = StreamExtractor::parse_stream_entries(raw);

    assert_eq!(entries.len(), 4);
    // Highest quality first: 4K -> 1080p Ultra -> 720p -> 360p
    assert_eq!(entries[0].quality, "4K (2160p)");
    assert!(entries[0].is_premium);
    assert_eq!(entries[0].urls.len(), 1);

    assert_eq!(entries[1].quality, "1080p Ultra");
    assert!(entries[1].is_premium);

    assert_eq!(entries[2].quality, "720p");
    assert!(!entries[2].is_premium);

    assert_eq!(entries[3].quality, "360p");
    assert!(!entries[3].is_premium);
    assert_eq!(entries[3].urls.len(), 2);
    assert_eq!(
        entries[3].best_url(),
        Some("https://cdn.example.com/360.mp4:hls:manifest.m3u8")
    );
}

#[test]
fn test_translator_stream_flags() {
    let html = r#"
        <meta property="og:type" content="video.movie">
        <h1 class="b-post__title">Test</h1>
        <ul id="translators-list">
          <li data-translator_id="77" data-camrip="1" data-ad="0" data-director="1">Voice</li>
        </ul>
    "#;
    let details =
        DetailsScraper::parse_details_html(html, "https://example.test/films/123-test.html")
            .unwrap();
    let translator = &details.translators[0];
    assert!(translator.is_camrip);
    assert!(!translator.has_ads);
    assert!(translator.is_director_cut);
}

#[test]
fn test_parse_subtitles() {
    let raw_sub = "[rus]https://cdn.example.com/sub_ru.vtt,[eng]https://cdn.example.com/sub_en.vtt";
    let tracks = StreamExtractor::parse_subtitles(raw_sub);

    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].code, "rus");
    assert_eq!(tracks[0].url, "https://cdn.example.com/sub_ru.vtt");
    assert_eq!(tracks[1].code, "eng");
    assert_eq!(tracks[1].url, "https://cdn.example.com/sub_en.vtt");
}

#[test]
fn test_parse_subtitles_with_meta_default_and_lns() {
    let raw_sub = "[sub_rus]https://cdn.example.com/ru.vtt,[sub_eng]https://cdn.example.com/en.vtt";
    let mut lns = HashMap::new();
    lns.insert("sub_rus".to_string(), "rus".to_string());
    lns.insert("sub_eng".to_string(), "eng".to_string());

    let tracks = StreamExtractor::parse_subtitles_with_meta(raw_sub, "rus", &lns);

    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].code, "sub_rus");
    assert_eq!(tracks[0].language_code.as_deref(), Some("rus"));
    assert!(tracks[0].is_default);

    assert_eq!(tracks[1].code, "sub_eng");
    assert_eq!(tracks[1].language_code.as_deref(), Some("eng"));
    assert!(!tracks[1].is_default);
}

#[test]
fn test_dedupe_by_url_first_wins() {
    let raw = "[720p]https://cdn.example.com/720.mp4,[1080p]https://cdn.example.com/720.mp4";
    let entries = StreamExtractor::parse_stream_entries(raw);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].quality, "720p");
}

#[test]
fn test_hls_best_url_selection() {
    let entry_both = movo_core::client::models::StreamEntry {
        quality: "1080p".to_string(),
        is_premium: false,
        urls: vec![
            "https://cdn.example.com/stream.m3u8".to_string(),
            "https://cdn.example.com/stream.mp4".to_string(),
        ],
    };
    assert_eq!(
        entry_both.best_url(),
        Some("https://cdn.example.com/stream.m3u8"),
        "best_url must prefer HLS when available"
    );

    let entry_mp4_only = movo_core::client::models::StreamEntry {
        quality: "720p".to_string(),
        is_premium: false,
        urls: vec!["https://cdn.example.com/720.mp4".to_string()],
    };
    assert_eq!(
        entry_mp4_only.best_url(),
        Some("https://cdn.example.com/720.mp4"),
        "best_url should fall back to MP4 when HLS is not available"
    );
}

#[test]
fn test_streams_object_does_not_require_legacy_url() {
    let bundle = StreamExtractor::parse_stream_json(
        r#"{"success":true,"streams":{"720p":"https://cdn.example.com/video.m3u8"}}"#,
        1,
        2,
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        bundle.streams[0].best_url(),
        Some("https://cdn.example.com/video.m3u8")
    );
}
