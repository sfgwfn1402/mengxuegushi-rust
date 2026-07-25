use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

use crate::{
    error::AppError,
    models::classroom::{
        ClassInfo, ClassTreeResponse, CreateClassRequest, JoinClassRequest,
    },
    routes::me::current_user,
    services::classroom_store,
    AppState,
};

// POST /api/classes —— 家长建群
pub async fn create_class(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateClassRequest>,
) -> Result<Json<ClassInfo>, AppError> {
    let user = current_user(&state, &headers).await?;
    let info = classroom_store::create_class(&state.db, &user.id, &body.name).await?;
    Ok(Json(info))
}

// POST /api/classes/join —— 凭邀请码加群
pub async fn join_class(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<JoinClassRequest>,
) -> Result<Json<ClassInfo>, AppError> {
    let user = current_user(&state, &headers).await?;
    let info = classroom_store::join_class(&state.db, &user.id, &body.invite_code).await?;
    Ok(Json(info))
}

// GET /api/classes/mine —— 我加入的班级
pub async fn my_classes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ClassInfo>>, AppError> {
    let user = current_user(&state, &headers).await?;
    let list = classroom_store::my_classes(&state.db, &user.id).await?;
    Ok(Json(list))
}

// GET /api/classes/{id}/tree —— 班级诗词树聚合状态
pub async fn class_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(class_id): Path<String>,
) -> Result<Json<ClassTreeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let tree = classroom_store::class_tree(&state.db, &class_id, &user.id).await?;
    Ok(Json(tree))
}

// POST /api/classes/{id}/leave —— 退群(owner 退群=解散)
pub async fn leave_class(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(class_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    classroom_store::leave_class(&state.db, &user.id, &class_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
