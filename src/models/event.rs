use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TrackEventInput {
    pub event: String,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub props: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct TrackEventsRequest {
    pub events: Vec<TrackEventInput>,
}

#[derive(Debug, Serialize)]
pub struct EventCount {
    pub event_name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyActive {
    pub day: String,
    pub users: i64,
    pub events: i64,
}

#[derive(Debug, Serialize)]
pub struct TopPoem {
    pub poem_id: String,
    pub title: Option<String>,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub range_days: i64,
    pub total_events: i64,
    pub active_users: i64,
    pub event_counts: Vec<EventCount>,
    pub daily_active: Vec<DailyActive>,
    pub top_poems: Vec<TopPoem>,
}
