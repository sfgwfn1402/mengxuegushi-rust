use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MomentItem {
    pub id: String,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub content: String,
    pub image_url: String,
    pub like_count: i32,
    pub liked_by_me: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MomentListResponse {
    pub items: Vec<MomentItem>,
}

#[derive(Debug, Serialize)]
pub struct DeleteMomentResponse {
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminMomentListResponse {
    pub total: i64,
    pub items: Vec<MomentItem>,
}
