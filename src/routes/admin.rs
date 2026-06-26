use axum::{extract::{Path, Query, State}, http::HeaderMap, Json};
use serde::Deserialize;

use crate::{
    error::AppError,
    models::{
        artwork::{AdminArtworkListResponse, DeleteArtworkResponse},
        feedback::{AdminFeedbackListResponse, AdminFeedbackQuery, UpdateFeedbackStatusRequest},
        recitation::{AdminRecitationListResponse, AdminRecitationQuery, DeleteRecitationResponse},
        user::User,
    },
    routes::me,
    services::{artwork_store, feedback_store, recitation_store, reminder},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub status: String,
}

fn ensure_admin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let expected = state
        .config
        .admin_token
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("admin token is not configured".to_string()))?;
    let actual = headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if actual != expected {
        return Err(AppError::Unauthorized("invalid admin token".to_string()));
    }
    Ok(())
}

async fn current_admin(state: &AppState, headers: &HeaderMap) -> Result<User, AppError> {
    if let Ok(user) = me::current_user(state, headers).await {
        if user.role == "admin" {
            return Ok(user);
        }
        return Err(AppError::Forbidden("admin required".to_string()));
    }

    ensure_admin(state, headers)?;
    Ok(User {
        id: "admin-token".to_string(),
        openid: "admin-token".to_string(),
        unionid: None,
        nickname: Some("Admin Token".to_string()),
        avatar_url: None,
        role: "admin".to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<i64>,
}

// 数据看板：核心事件计数、活跃用户、每日活跃、热门诗
pub async fn analytics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<crate::models::event::AnalyticsResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let days = query.days.unwrap_or(7);
    Ok(Json(
        crate::services::event_store::analytics(&state.db, days).await?,
    ))
}

// 手动触发学习提醒发送（管理员/测试用，定时任务也调同一逻辑）
pub async fn send_reminders(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    current_admin(&state, &headers).await?;
    let sent = reminder::send_daily_reminders(&state).await;
    Ok(Json(serde_json::json!({ "sent": sent })))
}

fn normalize_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim() {
        "public" => Ok("public"),
        "rejected" => Ok("rejected"),
        "active" | "private" => Ok("active"),
        "submitted" => Ok("submitted"),
        _ => Err(AppError::BadRequest("invalid review status".to_string())),
    }
}

pub async fn review_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
    Json(payload): Json<ReviewRequest>,
) -> Result<Json<DeleteArtworkResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let status = normalize_status(&payload.status)?;
    // Refuse to review private (active/draft) works — those are user drafts
    // and shouldn't appear in the admin review queue at all.
    let current = artwork_store::get_status(&state.db, &artwork_id).await?;
    if current.as_deref() == Some("active") {
        return Err(AppError::BadRequest(
            "private (active) works do not need admin review".to_string(),
        ));
    }
    Ok(Json(
        artwork_store::admin_set_status(&state.db, &artwork_id, status).await?,
    ))
}

pub async fn review_recitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
    Json(payload): Json<ReviewRequest>,
) -> Result<Json<DeleteRecitationResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let status = normalize_status(&payload.status)?;
    // Refuse to review private (active/draft) works — those are user drafts
    // and shouldn't appear in the admin review queue at all.
    let current = recitation_store::get_status(&state.db, &recitation_id).await?;
    if current.as_deref() == Some("active") {
        return Err(AppError::BadRequest(
            "private (active) works do not need admin review".to_string(),
        ));
    }
    let deleted = recitation_store::admin_set_status(&state.db, &recitation_id, status).await?;
    Ok(Json(DeleteRecitationResponse { deleted }))
}

pub async fn list_recitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminRecitationQuery>,
) -> Result<Json<AdminRecitationListResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let page = query.page();
    let page_size = query.page_size();
    let status_filter = query.status_filter();
    let (total, items) = recitation_store::list_admin_recitations(
        &state.db,
        page,
        page_size,
        status_filter.as_deref(),
    )
    .await?;
    Ok(Json(AdminRecitationListResponse { total, items }))
}

pub async fn list_artworks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<crate::models::artwork::AdminArtworkQuery>,
) -> Result<Json<AdminArtworkListResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let page = query.page();
    let page_size = query.page_size();
    let status_filter = query.status_filter();
    let (total, items) = artwork_store::list_admin_artworks(
        &state.db,
        page,
        page_size,
        status_filter.as_deref(),
    )
    .await?;
    Ok(Json(AdminArtworkListResponse { total, items }))
}

pub async fn list_feedback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminFeedbackQuery>,
) -> Result<Json<AdminFeedbackListResponse>, AppError> {
    current_admin(&state, &headers).await?;
    let (total, items) = feedback_store::list_admin_feedback(&state.db, &query).await?;
    Ok(Json(AdminFeedbackListResponse { total, items }))
}

pub async fn update_feedback_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(feedback_id): Path<String>,
    Json(payload): Json<UpdateFeedbackStatusRequest>,
) -> Result<Json<crate::models::feedback::AdminFeedbackItem>, AppError> {
    let admin = current_admin(&state, &headers).await?;
    let item = feedback_store::update_feedback_status(
        &state.db,
        &feedback_id,
        &payload.status,
        payload.admin_note.as_deref(),
        &admin.id,
    )
    .await?;
    Ok(Json(item))
}
