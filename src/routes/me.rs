use axum::{
    extract::Multipart, extract::Path, extract::Query, extract::State, http::HeaderMap, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        activity::{CompleteTaskRequest, IdiomProgressListResponse, UpdateIdiomProgressRequest},
        invite::{invite_badge, InviteInfoResponse},
        profile::UpdateProfileRequest,
        recitation::RecitationListResponse,
        user::{FavoriteListResponse, ProgressListResponse, UpdateProgressRequest, User},
    },
    services::{activity_store, minio_store, poem_store, recitation_store, user_store},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct FavoriteStatusResponse {
    pub poem_id: u32,
    pub favorite: bool,
}

#[derive(Debug, Serialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MyRecitationsQuery {
    pub limit: Option<i64>,
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<User>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(user))
}

pub async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<User>, AppError> {
    let user = current_user(&state, &headers).await?;
    let nickname = payload
        .nickname
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let avatar_url = payload
        .avatar_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(Json(
        user_store::update_profile(&state.db, &user.id, nickname, avatar_url).await?,
    ))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<AvatarUploadResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    if !minio_store::enabled(&state.config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }

    let mut avatar_bytes: Option<Vec<u8>> = None;
    let mut ext = "jpg".to_string();
    let mut content_type = "image/jpeg".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid multipart: {err}")))?
    {
        if field.name().unwrap_or_default() != "file" {
            continue;
        }

        if let Some(file_name) = field.file_name().map(|value| value.to_string()) {
            if let Some(candidate) = file_name.rsplit('.').next() {
                let clean = candidate.to_lowercase();
                if ["jpg", "jpeg", "png", "webp"].contains(&clean.as_str()) {
                    ext = clean;
                }
            }
        }

        if let Some(field_content_type) = field.content_type().map(|value| value.to_string()) {
            if ["image/jpeg", "image/png", "image/webp"].contains(&field_content_type.as_str()) {
                content_type = field_content_type;
            }
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|err| AppError::BadRequest(format!("invalid avatar file: {err}")))?;
        if bytes.is_empty() {
            return Err(AppError::BadRequest("empty avatar file".to_string()));
        }
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(AppError::BadRequest("avatar file too large".to_string()));
        }
        avatar_bytes = Some(bytes.to_vec());
    }

    let avatar_bytes =
        avatar_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;
    let object_path = format!("avatars/{}/{}.{}", user.id, Uuid::new_v4(), ext);
    minio_store::put_object(&state.config, &object_path, avatar_bytes, &content_type).await?;

    let avatar_relative_path = object_path.trim_start_matches("avatars/");
    let avatar_url = if let Some(base) = state.config.avatar_public_base_url.as_ref() {
        format!("{}/{}", base.trim_end_matches('/'), avatar_relative_path)
    } else if let Some(base) = state.config.public_base_url.as_ref() {
        format!(
            "{}/avatars/{}",
            base.trim_end_matches('/'),
            avatar_relative_path
        )
    } else {
        format!("/avatars/{avatar_relative_path}")
    };

    user_store::update_profile(&state.db, &user.id, None, Some(avatar_url.clone())).await?;
    Ok(Json(AvatarUploadResponse { avatar_url }))
}

pub async fn stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::models::activity::UserStatsResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(activity_store::get_stats(&state.db, &user.id).await?))
}

pub async fn checkin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::models::activity::CheckinResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(activity_store::checkin(&state.db, &user.id).await?))
}

// 事件埋点上报：可选登录态，登录则记 user_id，否则记 null。永不因鉴权失败而拒绝。
pub async fn track_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::event::TrackEventsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if payload.events.is_empty() {
        return Ok(Json(serde_json::json!({ "inserted": 0 })));
    }
    let user = current_user(&state, &headers).await.ok();
    let user_id = user.as_ref().map(|u| u.id.as_str());
    let inserted =
        crate::services::event_store::insert_events(&state.db, user_id, &payload.events).await?;
    Ok(Json(serde_json::json!({ "inserted": inserted })))
}

// 公开接口：被邀请者落地时按邀请码展示邀请人昵称（仅返回昵称，无敏感信息）
pub async fn inviter_name(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let nickname = user_store::find_user_by_id(&state.db, code.trim())
        .await?
        .and_then(|u| u.nickname)
        .filter(|n| !n.trim().is_empty());
    Ok(Json(serde_json::json!({ "nickname": nickname })))
}

pub async fn invite_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<InviteInfoResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let count = user_store::get_invite_count(&state.db, &user.id).await?;
    let (badge, badge_label, next_badge_at) = invite_badge(count);
    Ok(Json(InviteInfoResponse {
        invite_code: user.id,
        invite_count: count,
        badge,
        badge_label,
        next_badge_at,
    }))
}

// 用户授权一次学习提醒订阅 → 额度 +1
pub async fn subscribe_reminder(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    let credits = user_store::add_reminder_credit(&state.db, &user.id).await?;
    Ok(Json(serde_json::json!({ "credits": credits })))
}

pub async fn complete_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<Json<crate::models::activity::CompleteTaskResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let task_id = payload.task_id.trim();
    if task_id.is_empty() {
        return Err(AppError::BadRequest("task_id is required".to_string()));
    }
    Ok(Json(
        activity_store::complete_task(&state.db, &user.id, task_id, payload.stars.unwrap_or(0))
            .await?,
    ))
}

pub async fn clear_data(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::models::activity::UserStatsResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    activity_store::clear_user_data(&state.db, &user.id).await?;
    Ok(Json(activity_store::get_stats(&state.db, &user.id).await?))
}

pub async fn list_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProgressListResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let items = user_store::list_progress(&state.db, &user.id).await?;
    Ok(Json(ProgressListResponse {
        total: items.len(),
        items,
    }))
}

pub async fn update_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<u32>,
    Json(payload): Json<UpdateProgressRequest>,
) -> Result<Json<crate::models::user::UserPoemProgress>, AppError> {
    let user = current_user(&state, &headers).await?;
    ensure_poem_exists(&state, poem_id).await?;
    let progress = user_store::update_progress(&state.db, &user.id, poem_id, payload).await?;
    Ok(Json(progress))
}

pub async fn list_idiom_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<IdiomProgressListResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let items = activity_store::list_idiom_progress(&state.db, &user.id).await?;
    Ok(Json(IdiomProgressListResponse {
        total: items.len(),
        items,
    }))
}

pub async fn update_idiom_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateIdiomProgressRequest>,
) -> Result<Json<crate::models::activity::IdiomProgress>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        activity_store::update_idiom_progress(&state.db, &user.id, payload).await?,
    ))
}

pub async fn list_recitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MyRecitationsQuery>,
) -> Result<Json<RecitationListResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let items = recitation_store::list_active_by_user(&state.db, &user.id, limit).await?;
    Ok(Json(RecitationListResponse { items }))
}

pub async fn list_favorites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<FavoriteListResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let poem_ids = user_store::list_favorite_poem_ids(&state.db, &user.id).await?;
    let mut items = Vec::with_capacity(poem_ids.len());

    for poem_id in poem_ids {
        if let Some(poem) = poem_store::find_poem(&state.db, poem_id).await? {
            items.push(poem);
        }
    }

    Ok(Json(FavoriteListResponse {
        total: items.len(),
        items,
    }))
}

pub async fn add_favorite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<u32>,
) -> Result<Json<FavoriteStatusResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    ensure_poem_exists(&state, poem_id).await?;
    user_store::set_favorite(&state.db, &user.id, poem_id, true).await?;
    Ok(Json(FavoriteStatusResponse {
        poem_id,
        favorite: true,
    }))
}

pub async fn remove_favorite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<u32>,
) -> Result<Json<FavoriteStatusResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    ensure_poem_exists(&state, poem_id).await?;
    user_store::set_favorite(&state.db, &user.id, poem_id, false).await?;
    Ok(Json(FavoriteStatusResponse {
        poem_id,
        favorite: false,
    }))
}

pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<User, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Bearer token".to_string()))?;

    let user_id = token
        .strip_prefix("dev-token-")
        .ok_or_else(|| AppError::Unauthorized("invalid token".to_string()))?;

    user_store::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user not found".to_string()))
}

async fn ensure_poem_exists(state: &AppState, poem_id: u32) -> Result<(), AppError> {
    poem_store::find_poem(&state.db, poem_id)
        .await?
        .map(|_| ())
        .ok_or_else(|| AppError::NotFound(format!("poem {poem_id}")))
}
