use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ParentFeedbackRequest {
    pub age: Option<String>,
    #[serde(rename = "type")]
    pub feedback_type: String,
    pub pain_point: Option<String>,
    pub suggestion: Option<String>,
    pub contact: Option<String>,
    pub client_info: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ParentFeedbackResponse {
    pub id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AdminFeedbackQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub feedback_type: Option<String>,
}

impl AdminFeedbackQuery {
    pub fn page(&self) -> u32 { self.page.unwrap_or(1).clamp(1, 1000) }
    pub fn page_size(&self) -> u32 { self.page_size.unwrap_or(50).clamp(1, 100) }
}

#[derive(Debug, Serialize)]
pub struct AdminFeedbackItem {
    pub id: String,
    pub user_id: Option<String>,
    pub age: Option<String>,
    pub feedback_type: String,
    pub pain_point: Option<String>,
    pub suggestion: Option<String>,
    pub contact: Option<String>,
    pub status: String,
    pub admin_note: Option<String>,
    pub handled_at: Option<DateTime<Utc>>,
    pub handled_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminFeedbackListResponse {
    pub total: i64,
    pub items: Vec<AdminFeedbackItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFeedbackStatusRequest {
    pub status: String,
    pub admin_note: Option<String>,
}
