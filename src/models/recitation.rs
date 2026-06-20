use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecitationItem {
    pub id: String,
    pub poem_id: i32,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub audio_url: String,
    pub duration_seconds: Option<i32>,
    pub like_count: i32,
    pub liked_by_me: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RecitationListResponse {
    pub items: Vec<RecitationItem>,
}

#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub liked: bool,
    pub like_count: i32,
}

#[derive(Debug, Serialize)]
pub struct DeleteRecitationResponse {
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct FeaturedRecitationResponse {
    pub item: Option<RecitationItem>,
}
