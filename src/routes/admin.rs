use axum::{extract::{Path, Query, State}, http::HeaderMap, Json};
use serde::Deserialize;

use crate::{
    error::AppError,
    models::{
        artwork::DeleteArtworkResponse,
        feedback::{AdminFeedbackListResponse, AdminFeedbackQuery, UpdateFeedbackStatusRequest},
        recitation::DeleteRecitationResponse,
        user::User,
    },
    routes::me,
    services::{artwork_store, feedback_store, recitation_store},
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
    ensure_admin(&state, &headers)?;
    let status = normalize_status(&payload.status)?;
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
    ensure_admin(&state, &headers)?;
    let status = normalize_status(&payload.status)?;
    let deleted = recitation_store::admin_set_status(&state.db, &recitation_id, status).await?;
    Ok(Json(DeleteRecitationResponse { deleted }))
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
