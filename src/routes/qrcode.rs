use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderValue, Response},
};
use serde::Deserialize;

use crate::{
    error::AppError,
    services::{artwork_store, minio_store, recitation_store, wechat},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct WorkQrcodeQuery {
    #[serde(rename = "type")]
    pub work_type: String,
    pub id: String,
}

pub async fn work_qrcode(
    State(state): State<AppState>,
    Query(query): Query<WorkQrcodeQuery>,
) -> Result<Response<Body>, AppError> {
    let work_type = query.work_type.trim();
    let id = query.id.trim();
    if id.is_empty() || !matches!(work_type, "recitation" | "artwork") {
        return Err(AppError::BadRequest(
            "invalid work qrcode query".to_string(),
        ));
    }

    // Ensure the shared work exists and is active before generating a code.
    match work_type {
        "recitation" => {
            recitation_store::get_recitation(&state.db, id, None).await?;
        }
        "artwork" => {
            artwork_store::get_artwork(&state.db, id, None).await?;
        }
        _ => unreachable!(),
    }

    let object_path = format!("qrcodes/works/{work_type}-{id}.png");
    if minio_store::enabled(&state.config) {
        if let Ok((bytes, _)) = minio_store::get_object(&state.config, &object_path).await {
            return png_response(bytes);
        }
    }

    let page = format!("pages/work-detail/work-detail?type={work_type}&id={id}");
    let bytes = wechat::get_wxacode(&state, &page).await?;

    if minio_store::enabled(&state.config) {
        let _ =
            minio_store::put_object(&state.config, &object_path, bytes.clone(), "image/png").await;
    }

    png_response(bytes)
}

fn png_response(bytes: Vec<u8>) -> Result<Response<Body>, AppError> {
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    );
    Ok(response)
}
