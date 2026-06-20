use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::auth::{DevLoginRequest, LoginResponse, WechatLoginRequest, WechatLoginResponse},
    services::{user_store, wechat},
    AppState,
};

pub async fn wechat_login(
    State(state): State<AppState>,
    Json(payload): Json<WechatLoginRequest>,
) -> Result<Json<WechatLoginResponse>, AppError> {
    if payload.code.trim().is_empty() {
        return Err(AppError::BadRequest("code is required".to_string()));
    }

    let session = wechat::code2session(&state, payload.code.trim()).await?;
    let openid = session
        .openid
        .clone()
        .ok_or_else(|| AppError::Upstream("wechat response missing openid".to_string()))?;

    let user =
        user_store::upsert_wechat_user(&state.db, &openid, session.unionid.as_deref()).await?;

    // TODO: 后续换成 JWT。开发期 token 直接携带 user_id，方便小程序先联调。
    let token = format!("dev-token-{}", user.id);

    Ok(Json(WechatLoginResponse {
        token,
        user_id: user.id,
        openid,
        session_key: session.session_key,
        unionid: session.unionid,
    }))
}

pub async fn dev_login(
    State(state): State<AppState>,
    Json(payload): Json<DevLoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    if !state.config.enable_dev_login {
        return Err(AppError::NotFound("dev login is disabled".to_string()));
    }

    let openid = payload
        .openid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("dev-openid-{}", Uuid::new_v4()));

    let unionid = payload
        .unionid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let user = user_store::upsert_wechat_user(&state.db, &openid, unionid).await?;
    let token = format!("dev-token-{}", user.id);

    Ok(Json(LoginResponse {
        token,
        user_id: user.id,
        openid,
        unionid: user.unionid,
    }))
}
