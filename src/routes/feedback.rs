use axum::{extract::State, http::HeaderMap, Json};

use crate::{
    error::AppError,
    models::feedback::{ParentFeedbackRequest, ParentFeedbackResponse},
    routes::me,
    services::feedback_store,
    AppState,
};

pub async fn submit_parent_feedback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ParentFeedbackRequest>,
) -> Result<Json<ParentFeedbackResponse>, AppError> {
    let pain_point = payload.pain_point.as_deref().unwrap_or("").trim();
    let suggestion = payload.suggestion.as_deref().unwrap_or("").trim();
    if pain_point.is_empty() && suggestion.is_empty() {
        return Err(AppError::BadRequest(
            "pain_point or suggestion is required".to_string(),
        ));
    }

    // 如果有登录态则关联用户；没有或失效也允许匿名提交，避免家长反馈丢失。
    let user_id = me::current_user(&state, &headers)
        .await
        .ok()
        .and_then(|user| uuid::Uuid::parse_str(&user.id).ok());
    let (id, created_at) = feedback_store::create_parent_feedback(&state.db, user_id, &payload).await?;

    Ok(Json(ParentFeedbackResponse {
        id: id.to_string(),
        created_at,
    }))
}
