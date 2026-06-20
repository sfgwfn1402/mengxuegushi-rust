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
