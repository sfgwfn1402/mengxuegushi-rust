use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, models::feedback::ParentFeedbackRequest};

pub async fn create_parent_feedback(
    db: &PgPool,
    user_id: Option<String>,
    payload: &ParentFeedbackRequest,
) -> Result<(Uuid, DateTime<Utc>), AppError> {
    let id = Uuid::new_v4();
    let client_info = payload.client_info.clone().unwrap_or(Value::Null);
    let row: (DateTime<Utc>,) = sqlx::query_as(
        r#"
        INSERT INTO parent_feedback (
            id, user_id, age, feedback_type, pain_point, suggestion, contact, client_info
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING created_at
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(trim_opt(payload.age.as_deref(), 50))
    .bind(trim_required(&payload.feedback_type, 40))
    .bind(trim_opt(payload.pain_point.as_deref(), 1000))
    .bind(trim_opt(payload.suggestion.as_deref(), 1500))
    .bind(trim_opt(payload.contact.as_deref(), 200))
    .bind(client_info)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok((id, row.0))
}

fn trim_required(value: &str, max_chars: usize) -> String {
    trim_to(value, max_chars).unwrap_or_else(|| "feedback".to_string())
}

fn trim_opt(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.and_then(|item| trim_to(item, max_chars))
}

fn trim_to(value: &str, max_chars: usize) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(max_chars).collect())
}
