use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct MomentItem {
    pub id: String,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub content: String,
    pub image_url: String,
    pub images: Vec<String>,
    pub object_paths: Vec<String>,
    pub like_count: i32,
    pub liked_by_me: bool,
    pub comment_count: i32,
    pub followed_by_me: bool,
    pub location: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMomentRequest {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub object_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MomentComment {
    pub id: String,
    pub moment_id: String,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub parent_id: Option<String>,
    pub reply_to_nickname: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub reply_to_nickname: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentListResponse {
    pub items: Vec<MomentComment>,
    pub comment_count: i32,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub following_count: i64,
    pub follower_count: i64,
    pub moment_count: i64,
    pub followed_by_me: bool,
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
