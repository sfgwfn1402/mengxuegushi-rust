use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArtworkItem {
    pub id: String,
    pub poem_id: i32,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub poem_title: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub image_url: String,
    pub like_count: i32,
    pub liked_by_me: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ArtworkListResponse {
    pub items: Vec<ArtworkItem>,
}

#[derive(Debug, Serialize)]
pub struct DeleteArtworkResponse {
    pub deleted: bool,
}
