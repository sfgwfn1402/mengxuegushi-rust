use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct UserStatsResponse {
    pub stars: u32,
    pub total_days: u32,
    pub streak: u32,
    pub learned_poem_count: u32,
    pub learned_idiom_count: u32,
    pub today_checked: bool,
    pub today_tasks_done: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckinResponse {
    pub today_checked: bool,
    pub total_days: u32,
    pub streak: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub stars: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompleteTaskResponse {
    pub task_id: String,
    pub stars_added: u32,
    pub total_stars: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateIdiomProgressRequest {
    pub idiom_id: u32,
    pub learned: Option<bool>,
    pub read_count_delta: Option<u32>,
    pub quiz_correct_delta: Option<u32>,
    pub quiz_wrong_delta: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdiomProgress {
    pub idiom_id: u32,
    pub learned: bool,
    pub read_count: u32,
    pub quiz_correct_count: u32,
    pub quiz_wrong_count: u32,
    pub last_learned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdiomProgressListResponse {
    pub total: usize,
    pub items: Vec<IdiomProgress>,
}
