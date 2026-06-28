use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        moment::{
            CommentListResponse, CreateCommentRequest, DeleteMomentResponse, MomentComment,
            MomentItem, MomentListResponse,
        },
        recitation::LikeResponse,
    },
    routes::me::current_user,
    services::{minio_store, moment_store},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<MomentListResponse>, AppError> {
    let uid = current_user(&state, &headers).await.ok().map(|u| u.id);
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 50);
    let items = moment_store::list_public(
        &state.db,
        uid.as_deref(),
        page_size,
        (page - 1) * page_size,
    )
    .await?;
    Ok(Json(MomentListResponse { items }))
}

// 单图上传 → 返回 object_path，前端攒齐后再 create
pub async fn upload_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    current_user(&state, &headers).await?;
    if !minio_store::enabled(&state.config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }
    let mut image_bytes: Option<Vec<u8>> = None;
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
        if let Some(file_name) = field.file_name().map(|v| v.to_string()) {
            if let Some(c) = file_name.rsplit('.').next() {
                let clean = c.to_lowercase();
                if ["jpg", "jpeg", "png", "webp"].contains(&clean.as_str()) {
                    ext = clean;
                }
            }
        }
        if let Some(ct) = field.content_type().map(|v| v.to_string()) {
            if ["image/jpeg", "image/png", "image/webp"].contains(&ct.as_str()) {
                content_type = ct;
            }
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|err| AppError::BadRequest(format!("invalid file: {err}")))?;
        if bytes.is_empty() {
            return Err(AppError::BadRequest("empty file".to_string()));
        }
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(AppError::BadRequest("file too large".to_string()));
        }
        image_bytes = Some(bytes.to_vec());
    }
    let image_bytes = image_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;
    let object_path = format!("moments/{}.{ext}", Uuid::new_v4());
    minio_store::put_object(&state.config, &object_path, image_bytes, &content_type).await?;
    Ok(Json(serde_json::json!({ "object_path": object_path })))
}

pub async fn list_mine(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MomentListResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let items = moment_store::list_mine(&state.db, &user.id, 100).await?;
    Ok(Json(MomentListResponse { items }))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::moment::CreateMomentRequest>,
) -> Result<Json<MomentItem>, AppError> {
    let user = current_user(&state, &headers).await?;
    let content: String = payload.content.trim().chars().take(300).collect();
    let mut paths: Vec<String> = payload
        .object_paths
        .into_iter()
        .filter(|p| p.starts_with("moments/") && !p.contains("..") && !p.contains('\\'))
        .take(6)
        .collect();
    if paths.is_empty() {
        return Err(AppError::BadRequest("at least one image required".to_string()));
    }
    paths.dedup();
    let mid = Uuid::new_v4().to_string();
    Ok(Json(
        moment_store::create_moment(&state.db, &mid, &user.id, &content, &paths).await?,
    ))
}

// 公开取图（img src 无法带鉴权头），支持 /image 和 /image/{idx}
pub async fn image(
    State(state): State<AppState>,
    Path(moment_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    serve_image(&state, &moment_id, 0).await
}

pub async fn image_idx(
    State(state): State<AppState>,
    Path((moment_id, idx)): Path<(String, usize)>,
) -> Result<Response<Body>, AppError> {
    serve_image(&state, &moment_id, idx).await
}

async fn serve_image(
    state: &AppState,
    moment_id: &str,
    idx: usize,
) -> Result<Response<Body>, AppError> {
    let object_path = moment_store::get_image_object_path(&state.db, moment_id, idx).await?;
    let (bytes, content_type) = minio_store::get_object(&state.config, &object_path).await?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    );
    Ok(response)
}

// 编辑动态(驳回/审核中/已公开均可)，更新内容+图片，重新进入审核
pub async fn edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(moment_id): Path<String>,
    Json(payload): Json<crate::models::moment::CreateMomentRequest>,
) -> Result<Json<MomentItem>, AppError> {
    let user = current_user(&state, &headers).await?;
    let content: String = payload.content.trim().chars().take(300).collect();
    let mut paths: Vec<String> = payload
        .object_paths
        .into_iter()
        .filter(|p| p.starts_with("moments/") && !p.contains("..") && !p.contains('\\'))
        .take(6)
        .collect();
    if paths.is_empty() {
        return Err(AppError::BadRequest("at least one image required".to_string()));
    }
    paths.dedup();
    let (item, removed) =
        moment_store::update_moment(&state.db, &moment_id, &user.id, &content, &paths).await?;
    // 删掉被移除的旧图(best-effort)
    for p in removed {
        if let Err(err) = minio_store::delete_object(&state.config, &p).await {
            tracing::warn!(object = %p, error = %err, "moment edit minio delete failed");
        }
    }
    Ok(Json(item))
}

pub async fn like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(moment_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(moment_store::like_moment(&state.db, &moment_id, &user.id).await?))
}

pub async fn unlike(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(moment_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(moment_store::unlike_moment(&state.db, &moment_id, &user.id).await?))
}

pub async fn delete_moment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(moment_id): Path<String>,
) -> Result<Json<DeleteMomentResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    // 先取出图片对象键，软删后顺带把 MinIO 文件删掉（best-effort，失败不影响删除结果）
    let paths = moment_store::owned_object_paths(&state.db, &moment_id, &user.id)
        .await
        .unwrap_or_default();
    let resp = moment_store::soft_delete(&state.db, &moment_id, &user.id).await?;
    if resp.deleted {
        for p in paths {
            if let Err(err) = minio_store::delete_object(&state.config, &p).await {
                tracing::warn!(object = %p, error = %err, "moment minio delete failed");
            }
        }
    }
    Ok(Json(resp))
}

// 列出某条动态的所有评论（含楼中楼），公开可读
pub async fn list_comments(
    State(state): State<AppState>,
    Path(moment_id): Path<String>,
) -> Result<Json<CommentListResponse>, AppError> {
    let items = moment_store::list_comments(&state.db, &moment_id).await?;
    let comment_count = items.len() as i32;
    Ok(Json(CommentListResponse { items, comment_count }))
}

// 发表评论 / 回复（parent_id 为顶层评论 id 时是楼中楼）
pub async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(moment_id): Path<String>,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<Json<MomentComment>, AppError> {
    let user = current_user(&state, &headers).await?;
    let content: String = payload.content.trim().chars().take(300).collect();
    if content.is_empty() {
        return Err(AppError::BadRequest("评论内容不能为空".to_string()));
    }
    let reply_to: Option<String> = payload
        .reply_to_nickname
        .map(|s| s.trim().chars().take(40).collect::<String>())
        .filter(|s| !s.is_empty());
    let id = Uuid::new_v4().to_string();
    Ok(Json(
        moment_store::create_comment(
            &state.db,
            &id,
            &moment_id,
            &user.id,
            payload.parent_id.as_deref(),
            reply_to.as_deref(),
            &content,
        )
        .await?,
    ))
}

// 删除自己的评论（顶层评论连同回复一起删）
pub async fn delete_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((moment_id, comment_id)): Path<(String, String)>,
) -> Result<Json<DeleteMomentResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        moment_store::delete_comment(&state.db, &moment_id, &comment_id, &user.id).await?,
    ))
}

pub async fn media(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response<Body>, AppError> {
    if path.is_empty() || path.contains("..") || path.starts_with('/') || path.contains('\\') {
        return Err(AppError::BadRequest("invalid media path".to_string()));
    }
    let object_path = format!("moments/{path}");
    let (bytes, content_type) = minio_store::get_object(&state.config, &object_path).await?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    );
    Ok(response)
}
