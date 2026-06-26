use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct InviteInfoResponse {
    pub invite_code: String,
    pub invite_count: i32,
    pub badge: Option<&'static str>,
    pub badge_label: Option<&'static str>,
    pub next_badge_at: Option<i32>,
}

pub fn invite_badge(count: i32) -> (Option<&'static str>, Option<&'static str>, Option<i32>) {
    if count >= 10 {
        (Some("👑"), Some("诗坛领袖"), None)
    } else if count >= 5 {
        (Some("🌟"), Some("诗坛大使"), Some(10))
    } else if count >= 3 {
        (Some("📜"), Some("诗词传播者"), Some(5))
    } else if count >= 1 {
        (Some("🎖"), Some("小推官"), Some(3))
    } else {
        (None, None, Some(1))
    }
}
