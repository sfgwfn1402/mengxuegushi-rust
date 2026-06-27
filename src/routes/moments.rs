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
        moment::{DeleteMomentResponse, MomentItem, MomentListResponse},
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

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<MomentItem>, AppError> {
    let user = current_user(&state, &headers).await?;
    if !minio_store::enabled(&state.config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }

    let mut content = String::new();
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut ext = "jpg".to_string();
    let mut content_type = "image/jpeg".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid multipart: {err}")))?
    {
        match field.name().unwrap_or_default().to_string().as_str() {
            "content" => {
                content = field
                    .text()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("invalid content: {err}")))?
                    .trim()
                    .chars()
                    .take(300)
                    .collect();
            }
            "file" => {
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
            _ => {}
        }
    }

    let image_bytes = image_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;
    let mid = Uuid::new_v4().to_string();
    let object_path = format!("moments/{mid}.{ext}");
    minio_store::put_object(&state.config, &object_path, image_bytes, &content_type).await?;

    // 走已被 Nginx 代理的 /api 路径取图，避免新增 /moments-media 未配置导致 404
    let image_url = format!("/api/moments/{mid}/image");

    Ok(Json(
        moment_store::create_moment(&state.db, &mid, &user.id, &content, &image_url, &object_path)
            .await?,
    ))
}

// 公开取图（img src 无法带鉴权头）
pub async fn image(
    State(state): State<AppState>,
    Path(moment_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    let object_path = moment_store::get_object_path(&state.db, &moment_id).await?;
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
    Ok(Json(moment_store::soft_delete(&state.db, &moment_id, &user.id).await?))
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
