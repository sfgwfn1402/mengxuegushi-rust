use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub openid: String,
    pub unionid: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPoemProgress {
    pub poem_id: u32,
    pub learned: bool,
    pub read_count: u32,
    pub quiz_correct_count: u32,
    pub quiz_wrong_count: u32,
    pub last_learned_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProgressRequest {
    pub learned: Option<bool>,
    pub read_count_delta: Option<u32>,
    pub quiz_correct_delta: Option<u32>,
    pub quiz_wrong_delta: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressListResponse {
    pub total: usize,
    pub items: Vec<UserPoemProgress>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FavoriteListResponse {
    pub total: usize,
    pub items: Vec<crate::models::poem::Poem>,
}
