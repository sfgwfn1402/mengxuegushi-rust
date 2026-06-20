use axum::{extract::Path, extract::State, http::HeaderMap, Json};
use serde::Deserialize;

use crate::{
    error::AppError,
    models::{artwork::DeleteArtworkResponse, recitation::DeleteRecitationResponse},
    services::{artwork_store, recitation_store},
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
