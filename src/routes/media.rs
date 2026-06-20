use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, Response},
};

use crate::{error::AppError, services::minio_store, AppState};

pub async fn audio(
    State(state): State<AppState>,
    Path(file_name): Path<String>,
) -> Result<Response<Body>, AppError> {
    let file_name = sanitize_file_name(&file_name, &["mp3", "m4a", "aac", "wav"])?;
    let object_key = format!("audios-id/{file_name}");
    proxy_minio_object(&state, &object_key, Some(content_type_for(&file_name))).await
}

pub async fn image(
    State(state): State<AppState>,
    Path(file_name): Path<String>,
) -> Result<Response<Body>, AppError> {
    let file_name = sanitize_file_name(&file_name, &["jpg", "jpeg", "png", "webp"])?;
    let object_key = format!("images-id/{file_name}");
    proxy_minio_object(&state, &object_key, Some(content_type_for(&file_name))).await
}

pub async fn recitation(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response<Body>, AppError> {
    let path = sanitize_relative_path(&path, &["mp3", "m4a", "aac", "wav"])?;
    let object_key = format!("recitations/{path}");
    proxy_minio_object(&state, &object_key, None).await
}

pub async fn avatar(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response<Body>, AppError> {
    let path = sanitize_relative_path(&path, &["jpg", "jpeg", "png", "webp"])?;
    let object_key = format!("avatars/{path}");
    proxy_minio_object(&state, &object_key, Some(content_type_for(&path))).await
}

async fn proxy_minio_object(
    state: &AppState,
    object_key: &str,
    fallback_content_type: Option<&'static str>,
) -> Result<Response<Body>, AppError> {
    let (bytes, content_type) = minio_store::get_object(&state.config, object_key).await?;
    let content_type = if content_type == "application/octet-stream" {
        fallback_content_type.unwrap_or("application/octet-stream")
    } else {
        content_type.as_str()
    };

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    );
    Ok(response)
}

fn sanitize_file_name(file_name: &str, allowed_exts: &[&str]) -> Result<String, AppError> {
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return Err(AppError::BadRequest("invalid media path".to_string()));
    }

    let ext = extension(file_name)
        .ok_or_else(|| AppError::BadRequest("missing extension".to_string()))?;
    if !allowed_exts.contains(&ext.as_str()) {
        return Err(AppError::BadRequest("unsupported media type".to_string()));
    }

    Ok(file_name.to_string())
}

fn sanitize_relative_path(path: &str, allowed_exts: &[&str]) -> Result<String, AppError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(AppError::BadRequest("invalid media path".to_string()));
    }

    let ext =
        extension(path).ok_or_else(|| AppError::BadRequest("missing extension".to_string()))?;
    if !allowed_exts.contains(&ext.as_str()) {
        return Err(AppError::BadRequest("unsupported media type".to_string()));
    }

    Ok(path.to_string())
}

fn extension(path: &str) -> Option<String> {
    path.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
}

fn content_type_for(file_name: &str) -> &'static str {
    match extension(file_name).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("mp3") => "audio/mpeg",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}
