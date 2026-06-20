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
        artwork::{ArtworkItem, ArtworkListResponse, DeleteArtworkResponse},
        recitation::LikeResponse,
    },
    routes::me::current_user,
    services::{artwork_store, minio_store, poem_store},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub mine: Option<bool>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<ArtworkListResponse>, AppError> {
    let current_user = current_user(&state, &headers).await.ok();
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;
    let items = if query.mine.unwrap_or(false) {
        let user = current_user
            .ok_or_else(|| AppError::Unauthorized("missing Bearer token".to_string()))?;
        artwork_store::list_mine(&state.db, &user.id, limit).await?
    } else {
        artwork_store::list_recent(
            &state.db,
            current_user.as_ref().map(|u| u.id.as_str()),
            limit,
            offset,
        )
        .await?
    };
    Ok(Json(ArtworkListResponse { items }))
}

pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Json<ArtworkItem>, AppError> {
    let user = current_user(&state, &headers).await?;
    ensure_poem_exists(&state, poem_id).await?;

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut ext = "jpg".to_string();
    let mut content_type = "image/jpeg".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid multipart: {err}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "title" => {
                let text = field
                    .text()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("invalid title: {err}")))?;
                let value = text.trim().to_string();
                if !value.is_empty() {
                    title = Some(value);
                }
            }
            "description" => {
                let text = field
                    .text()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("invalid description: {err}")))?;
                let value = text.trim().to_string();
                if !value.is_empty() {
                    description = Some(value);
                }
            }
            "file" => {
                if let Some(file_name) = field.file_name().map(|value| value.to_string()) {
                    if let Some(candidate) = file_name.rsplit('.').next() {
                        let clean = candidate.to_lowercase();
                        if ["jpg", "jpeg", "png", "webp"].contains(&clean.as_str()) {
                            ext = clean;
                        }
                    }
                }
                if let Some(field_content_type) =
                    field.content_type().map(|value| value.to_string())
                {
                    if ["image/jpeg", "image/png", "image/webp"]
                        .contains(&field_content_type.as_str())
                    {
                        content_type = field_content_type;
                    }
                }
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("invalid artwork file: {err}")))?;
                if bytes.is_empty() {
                    return Err(AppError::BadRequest("empty artwork file".to_string()));
                }
                if bytes.len() > 8 * 1024 * 1024 {
                    return Err(AppError::BadRequest("artwork file too large".to_string()));
                }
                image_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let title = title.ok_or_else(|| AppError::BadRequest("title is required".to_string()))?;
    let image_bytes =
        image_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;
    let artwork_id = Uuid::new_v4().to_string();
    let object_path = format!("artworks/poem-{poem_id}/{artwork_id}.{ext}");
    minio_store::put_object(&state.config, &object_path, image_bytes, &content_type).await?;

    let image_url = if let Some(base) = state.config.avatar_public_base_url.as_ref() {
        // Reuse the API media proxy base and switch /avatars to /artworks.
        let base = base.trim_end_matches('/').replace("/avatars", "/artworks");
        let relative = object_path.trim_start_matches("artworks/");
        format!("{base}/{relative}")
    } else {
        format!("/artworks/{}", object_path.trim_start_matches("artworks/"))
    };

    Ok(Json(
        artwork_store::create_artwork(
            &state.db,
            &user.id,
            poem_id,
            &title,
            description.as_deref(),
            &image_url,
            &object_path,
        )
        .await?,
    ))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
) -> Result<Json<ArtworkItem>, AppError> {
    let current_user_id = current_user(&state, &headers)
        .await
        .ok()
        .map(|user| user.id);
    Ok(Json(
        artwork_store::get_artwork(&state.db, &artwork_id, current_user_id.as_deref()).await?,
    ))
}

pub async fn image(
    State(state): State<AppState>,
    Path(artwork_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    let object_path = artwork_store::get_object_path(&state.db, &artwork_id).await?;
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

pub async fn media(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response<Body>, AppError> {
    if path.is_empty() || path.contains("..") || path.starts_with('/') || path.contains('\\') {
        return Err(AppError::BadRequest("invalid media path".to_string()));
    }
    let object_path = format!("artworks/{path}");
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
    Path(artwork_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        artwork_store::like_artwork(&state.db, &artwork_id, &user.id).await?,
    ))
}

pub async fn unlike(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        artwork_store::unlike_artwork(&state.db, &artwork_id, &user.id).await?,
    ))
}

pub async fn submit_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
) -> Result<Json<DeleteArtworkResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        artwork_store::set_submission_status(&state.db, &artwork_id, &user.id, "public").await?,
    ))
}

pub async fn withdraw_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
) -> Result<Json<DeleteArtworkResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        artwork_store::set_submission_status(&state.db, &artwork_id, &user.id, "active").await?,
    ))
}

pub async fn delete_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artwork_id): Path<String>,
) -> Result<Json<DeleteArtworkResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        artwork_store::soft_delete(&state.db, &artwork_id, &user.id).await?,
    ))
}

async fn ensure_poem_exists(state: &AppState, poem_id: i32) -> Result<(), AppError> {
    poem_store::find_poem(&state.db, poem_id as u32)
        .await?
        .map(|_| ())
        .ok_or_else(|| AppError::NotFound(format!("poem {poem_id}")))
}
