use super::super::models::{Comment, CommentsPage};
use super::super::session::RezkaSession;
use super::DetailsScraper;
use scraper::{Html, Selector};
use serde_json::Value;

impl DetailsScraper {
    pub async fn fetch_comments(
        session: &RezkaSession,
        post_id: i64,
        page: usize,
    ) -> Result<CommentsPage, String> {
        let response = session.get_html(&format!("ajax/get_comments?news_id={post_id}&cstart={page}&type=0&comment_id=0&skin=hdrezka")).await?;
        let value: Value = serde_json::from_str(&response)
            .map_err(|_| "Comments response was malformed".to_string())?;
        let mut page = Self::parse_comments(
            value
                .get("comments")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value
                .get("navigation")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            page,
        );
        for comment in &mut page.items {
            comment.avatar_url = comment
                .avatar_url
                .take()
                .map(|value| session.resolve_url(&value));
        }
        Ok(page)
    }

    fn parse_comments(html: &str, navigation: &str, page: usize) -> CommentsPage {
        let document = Html::parse_fragment(html);
        let items = document
            .select(&Selector::parse(".comments-tree-item").unwrap())
            .map(|node| {
                let find = |selector: &str| node.select(&Selector::parse(selector).unwrap()).next();
                Comment {
                    id: node.value().attr("data-id").unwrap_or_default().to_string(),
                    username: find(".name")
                        .map(|item| item.text().collect::<String>().trim().to_string())
                        .unwrap_or_default(),
                    avatar_url: find(".ava img")
                        .and_then(|item| item.value().attr("src"))
                        .map(str::to_string),
                    date: find(".date")
                        .map(|item| {
                            item.text()
                                .collect::<String>()
                                .replace("оставлен ", "")
                                .trim()
                                .to_string()
                        })
                        .unwrap_or_default(),
                    text: find(".text div")
                        .map(|item| {
                            item.text()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default(),
                    has_spoiler: find(".text_spoiler").is_some(),
                    indent: node
                        .value()
                        .attr("data-indent")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                    likes: find(".b-comment__like_it")
                        .and_then(|item| item.value().attr("data-likes_num"))
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                    liked: find(".show-likes-comment").is_some_and(|item| {
                        item.value()
                            .attr("class")
                            .unwrap_or_default()
                            .contains("disabled")
                    }),
                }
            })
            .collect();
        let nav = Html::parse_fragment(navigation);
        let total_pages = nav
            .select(&Selector::parse(".b-navigation a").unwrap())
            .filter_map(|node| node.text().collect::<String>().trim().parse().ok())
            .max()
            .unwrap_or(1);
        CommentsPage {
            items,
            page,
            total_pages,
        }
    }

    pub async fn like_comment(session: &RezkaSession, id: &str) -> Result<(), String> {
        Self::require_success(
            &session
                .post_ajax("engine/ajax/comments_like.php", &[("id", id)])
                .await?,
            "Failed to like comment",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::DetailsScraper;

    #[test]
    fn parses_comment_pages() {
        let comments = DetailsScraper::parse_comments(
            r#"<div class="comments-tree-item" data-id="c1" data-indent="2"><span class="name">User</span><span class="date">оставлен Today</span><div class="text"><div>Hello <b>world</b></div></div><span class="b-comment__like_it" data-likes_num="7"></span></div>"#,
            r#"<div class="b-navigation"><a>1</a><a>3</a><a>Next</a></div>"#,
            1,
        );
        assert_eq!(comments.items[0].text, "Hello world");
        assert_eq!(comments.total_pages, 3);
    }
}
