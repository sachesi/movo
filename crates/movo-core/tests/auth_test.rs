use movo_core::client::auth::AuthManager;
use movo_core::client::models::UserProfile;

#[test]
fn test_official_account_fixtures() {
    let categories = AuthManager::parse_favorites_categories(
        r#"<li class="b-favorites_content__cats_list_item" data-cat_id="4">
            <span class="name">Избранное</span><span class="num">12</span>
        </li>"#,
    );
    assert_eq!(categories[0].id, Some(4));
    assert_eq!(categories[0].count, 12);

    let history = AuthManager::parse_history(
        r#"<div class="b-videosaves__list_item"></div>
        <div class="b-videosaves__list_item watched-row">
            <button class="delete" data-id="77"></button>
            <div class="title"><a href="/films/77-test.html">Test</a></div>
        </div>"#,
    );
    assert_eq!(history[0].id, "77");
    assert_eq!(history[0].media_id(), Some(77));
    assert!(history[0].is_watched);
}

#[test]
fn test_extract_user_info_and_vip() {
    let html_vip = r#"
        <div class="b-users__panel">
            <span class="user_name">RezkaFan</span>
            <span class="vip badge">VIP</span>
        </div>
    "#;

    let username = AuthManager::extract_logged_in_username(html_vip);
    assert_eq!(username, Some("RezkaFan".to_string()));

    let is_vip = AuthManager::extract_vip_status(html_vip);
    assert!(is_vip);

    let html_normal = r#"
        <div class="b-users__panel">
            <span class="user_name">RegularUser</span>
        </div>
    "#;

    let username_normal = AuthManager::extract_logged_in_username(html_normal);
    assert_eq!(username_normal, Some("RegularUser".to_string()));

    let is_vip_normal = AuthManager::extract_vip_status(html_normal);
    assert!(!is_vip_normal);
}

#[test]
fn test_user_profile_fields() {
    let profile = UserProfile {
        user_id: "4242".to_string(),
        username: "Alex".to_string(),
        is_logged_in: true,
        is_vip: true,
        email: None,
        avatar_url: None,
        premium_days: None,
        is_session_persistent: true,
    };

    let json = serde_json::to_string(&profile).unwrap();
    let parsed: UserProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.user_id, "4242");
    assert_eq!(parsed.username, "Alex");
    assert!(parsed.is_vip);
    assert!(parsed.is_session_persistent);
}
