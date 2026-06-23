use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct AdminRecitationQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<String>,
}

impl AdminRecitationQuery {
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
pub struct AdminRecitationListResponse {
    pub total: i64,
    pub items: Vec<RecitationItem>,
}
