use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct AdminArtworkQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<String>,
}

impl AdminArtworkQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).clamp(1, 1000)
    }
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).clamp(1, 100)
    }
    pub fn status_filter(&self) -> Option<String> {
        self.status
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct AdminArtworkListResponse {
    pub total: i64,
    pub items: Vec<ArtworkItem>,
}
