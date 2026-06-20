use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::feedback::{AdminFeedbackItem, AdminFeedbackQuery, ParentFeedbackRequest},
};

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

pub async fn list_admin_feedback(
    db: &PgPool,
    query: &AdminFeedbackQuery,
) -> Result<(i64, Vec<AdminFeedbackItem>), AppError> {
    let mut count_builder = QueryBuilder::new("SELECT COUNT(*) FROM parent_feedback");
    push_admin_filters(&mut count_builder, query);
    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut builder = QueryBuilder::new(
        r#"
        SELECT id::text, user_id, age, feedback_type, pain_point, suggestion, contact,
               COALESCE(status, 'pending') AS status, admin_note, handled_at, handled_by, created_at
        FROM parent_feedback
        "#,
    );
    push_admin_filters(&mut builder, query);
    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(query.page_size() as i64);
    builder.push(" OFFSET ");
    builder.push_bind(((query.page() - 1) * query.page_size()) as i64);

    let rows = builder
        .build()
        .fetch_all(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let items = rows.into_iter().map(row_to_admin_feedback).collect();
    Ok((total, items))
}

pub async fn update_feedback_status(
    db: &PgPool,
    id: &str,
    status: &str,
    admin_note: Option<&str>,
    handled_by: &str,
) -> Result<AdminFeedbackItem, AppError> {
    let normalized_status = normalize_status(status)?;
    let row = sqlx::query(
        r#"
        UPDATE parent_feedback
        SET status = $1,
            admin_note = $2,
            handled_by = $3,
            handled_at = CURRENT_TIMESTAMP
        WHERE id = $4
        RETURNING id::text, user_id, age, feedback_type, pain_point, suggestion, contact,
                  COALESCE(status, 'pending') AS status, admin_note, handled_at, handled_by, created_at
        "#,
    )
    .bind(normalized_status)
    .bind(trim_opt(admin_note, 1000))
    .bind(handled_by)
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("feedback {id}")))?;

    Ok(row_to_admin_feedback(row))
}

fn push_admin_filters(builder: &mut QueryBuilder<'_, sqlx::Postgres>, query: &AdminFeedbackQuery) {
    let status = trim_opt(query.status.as_deref(), 40);
    let feedback_type = trim_opt(query.feedback_type.as_deref(), 40);
    if status.is_none() && feedback_type.is_none() {
        return;
    }
    builder.push(" WHERE ");
    let mut separated = builder.separated(" AND ");
    if let Some(status) = status {
        separated.push("status = ").push_bind_unseparated(status);
    }
    if let Some(feedback_type) = feedback_type {
        separated
            .push("feedback_type = ")
            .push_bind_unseparated(feedback_type);
    }
}

fn normalize_status(status: &str) -> Result<&'static str, AppError> {
    match status.trim() {
        "pending" => Ok("pending"),
        "reviewed" => Ok("reviewed"),
        "resolved" => Ok("resolved"),
        "ignored" => Ok("ignored"),
        other => Err(AppError::BadRequest(format!("invalid feedback status: {other}"))),
    }
}

fn row_to_admin_feedback(row: sqlx::postgres::PgRow) -> AdminFeedbackItem {
    AdminFeedbackItem {
        id: row.get("id"),
        user_id: row.get("user_id"),
        age: row.get("age"),
        feedback_type: row.get("feedback_type"),
        pain_point: row.get("pain_point"),
        suggestion: row.get("suggestion"),
        contact: row.get("contact"),
        status: row.get("status"),
        admin_note: row.get("admin_note"),
        handled_at: row.get("handled_at"),
        handled_by: row.get("handled_by"),
        created_at: row.get("created_at"),
    }
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
