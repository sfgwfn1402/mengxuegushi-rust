use std::path::PathBuf;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, Response},
    Json,
};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::recitation::{
        DeleteRecitationResponse, FeaturedRecitationResponse, LikeResponse, RecitationListResponse,
    },
    routes::me::current_user,
    services::{minio_store, poem_store, recitation_store},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct TopQuery {
    pub limit: Option<i64>,
}

pub async fn featured(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
) -> Result<Json<FeaturedRecitationResponse>, AppError> {
    ensure_poem_exists(&state.db, poem_id).await?;
    let current_user_id = current_user(&state, &headers)
        .await
        .ok()
        .map(|user| user.id);
    let item = recitation_store::get_featured_by_poem(
        &state.db,
        poem_id,
        current_user_id.as_deref(),
        state.config.featured_recitation_min_likes,
    )
    .await?;
    Ok(Json(FeaturedRecitationResponse { item }))
}

pub async fn list_top(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
    Query(query): Query<TopQuery>,
) -> Result<Json<RecitationListResponse>, AppError> {
    ensure_poem_exists(&state.db, poem_id).await?;
    let current_user_id = current_user(&state, &headers)
        .await
        .ok()
        .map(|user| user.id);
    let limit = query.limit.unwrap_or(5).clamp(1, 20);
    let items =
        recitation_store::list_top_by_poem(&state.db, poem_id, current_user_id.as_deref(), limit)
            .await?;
    Ok(Json(RecitationListResponse { items }))
}

pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Json<crate::models::recitation::RecitationItem>, AppError> {
    let user = current_user(&state, &headers).await?;
    ensure_poem_exists(&state.db, poem_id).await?;

    let mut duration_seconds: Option<i32> = None;
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut ext = "m4a".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid multipart: {err}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "duration_seconds" {
            let text = field
                .text()
                .await
                .map_err(|err| AppError::BadRequest(format!("invalid duration: {err}")))?;
            duration_seconds = text.trim().parse::<i32>().ok();
            continue;
        }

        if name == "file" {
            if let Some(file_name) = field.file_name().map(|value| value.to_string()) {
                if let Some(candidate) = file_name.rsplit('.').next() {
                    let clean = candidate.to_lowercase();
                    if ["m4a", "mp3", "aac", "wav"].contains(&clean.as_str()) {
                        ext = clean;
                    }
                }
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|err| AppError::BadRequest(format!("invalid audio file: {err}")))?;
            if bytes.is_empty() {
                return Err(AppError::BadRequest("empty audio file".to_string()));
            }
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(AppError::BadRequest("audio file too large".to_string()));
            }
            audio_bytes = Some(bytes.to_vec());
        }
    }

    let audio_bytes =
        audio_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;
    if let Some(duration) = duration_seconds {
        if duration <= 0 || duration > 90 {
            return Err(AppError::BadRequest(
                "duration_seconds must be 1..90".to_string(),
            ));
        }
    }

    let recitation_id = Uuid::new_v4().to_string();
    let relative_path = format!("poem-{poem_id}/{recitation_id}.{ext}");
    let object_path = format!("recitations/{relative_path}");
    let content_type = match ext.as_str() {
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    };

    if minio_store::enabled(&state.config) {
        minio_store::put_object(&state.config, &object_path, audio_bytes, content_type).await?;
    } else {
        let mut file_path = PathBuf::from(&state.config.recitation_dir);
        file_path.push(&relative_path);

        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        file.write_all(&audio_bytes)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        file.flush()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let audio_url = format!(
        "{}/{}",
        state
            .config
            .recitation_public_base_url
            .trim_end_matches('/'),
        relative_path
    );

    let item = recitation_store::create_recitation(
        &state.db,
        &user.id,
        poem_id,
        &audio_url,
        &object_path,
        duration_seconds,
    )
    .await?;

    Ok(Json(item))
}

/// AI 朗诵评分：接收录音 + poem_id，转调本机 FunASR 评分服务，返回字准确率结果。
/// 不持久化录音，纯评分；前端拿到分数即时展示。
pub async fn score(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let _user = current_user(&state, &headers).await?;

    let poem = poem_store::find_poem(&state.db, poem_id as u32)
        .await?
        .ok_or_else(|| AppError::NotFound("poem not found".to_string()))?;

    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut ext = "mp3".to_string();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid multipart: {err}")))?
    {
        if field.name().unwrap_or_default() == "file" {
            if let Some(file_name) = field.file_name().map(|v| v.to_string()) {
                if let Some(candidate) = file_name.rsplit('.').next() {
                    let clean = candidate.to_lowercase();
                    if ["m4a", "mp3", "aac", "wav"].contains(&clean.as_str()) {
                        ext = clean;
                    }
                }
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|err| AppError::BadRequest(format!("invalid audio file: {err}")))?;
            if bytes.is_empty() {
                return Err(AppError::BadRequest("empty audio file".to_string()));
            }
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(AppError::BadRequest("audio file too large".to_string()));
            }
            audio_bytes = Some(bytes.to_vec());
        }
    }
    let audio_bytes = audio_bytes.ok_or_else(|| AppError::BadRequest("missing file".to_string()))?;

    let score_url = state
        .config
        .funasr_score_url
        .clone()
        .ok_or_else(|| AppError::Internal("scoring service not configured".to_string()))?;

    use base64::Engine as _;
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
    let payload = serde_json::json!({
        "expected": poem.content,
        "audio_base64": audio_base64,
        "ext": ext,
    });

    let resp = state
        .http_client
        .post(format!("{}/score", score_url.trim_end_matches('/')))
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("scoring service unreachable: {err}")))?;

    if !resp.status().is_success() {
        let code = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "scoring service error {code}: {body}"
        )));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("invalid scoring response: {err}")))?;

    Ok(Json(result))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
) -> Result<Json<crate::models::recitation::RecitationItem>, AppError> {
    let current_user_id = current_user(&state, &headers)
        .await
        .ok()
        .map(|user| user.id);
    Ok(Json(
        recitation_store::get_recitation(&state.db, &recitation_id, current_user_id.as_deref())
            .await?,
    ))
}

pub async fn audio(
    State(state): State<AppState>,
    Path(recitation_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    let object_path = recitation_store::get_object_path(&state.db, &recitation_id).await?;
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
    Path(recitation_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        recitation_store::like_recitation(&state.db, &recitation_id, &user.id).await?,
    ))
}

pub async fn unlike(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
) -> Result<Json<LikeResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(
        recitation_store::unlike_recitation(&state.db, &recitation_id, &user.id).await?,
    ))
}

pub async fn submit_recitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
) -> Result<Json<DeleteRecitationResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    // 进入待审核队列；管理员审核通过后才会变更为 public，进入发现页。
    let deleted =
        recitation_store::set_submission_status(&state.db, &recitation_id, &user.id, "submitted")
            .await?;
    Ok(Json(DeleteRecitationResponse { deleted }))
}

pub async fn withdraw_recitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
) -> Result<Json<DeleteRecitationResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let deleted =
        recitation_store::set_submission_status(&state.db, &recitation_id, &user.id, "active")
            .await?;
    Ok(Json(DeleteRecitationResponse { deleted }))
}

pub async fn delete_recitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recitation_id): Path<String>,
) -> Result<Json<DeleteRecitationResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let deleted =
        recitation_store::soft_delete_recitation(&state.db, &recitation_id, &user.id).await?;
    Ok(Json(DeleteRecitationResponse { deleted }))
}

async fn ensure_poem_exists(db: &sqlx::PgPool, poem_id: i32) -> Result<(), AppError> {
    if poem_id <= 0 {
        return Err(AppError::BadRequest("invalid poem_id".to_string()));
    }
    poem_store::find_poem(db, poem_id as u32)
        .await?
        .ok_or_else(|| AppError::NotFound("poem not found".to_string()))?;
    Ok(())
}
