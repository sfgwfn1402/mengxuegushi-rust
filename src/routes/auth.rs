use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::auth::{
        AccountLoginRequest, AccountLoginResponse, AccountRegisterRequest, DevLoginRequest,
        LoginResponse, WechatLoginRequest, WechatLoginResponse,
    },
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

    let (user, is_new) =
        user_store::upsert_wechat_user(&state.db, &openid, session.unionid.as_deref()).await?;

    if is_new {
        if let Some(inviter_id) = payload.invite_from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let _ = user_store::record_invite(&state.db, &user.id, inviter_id).await;
        }
    }

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

/// 手机号 / 邮箱 + 密码 注册。
pub async fn account_register(
    State(state): State<AppState>,
    Json(payload): Json<AccountRegisterRequest>,
) -> Result<Json<AccountLoginResponse>, AppError> {
    let kind = payload.kind.trim();
    let account = normalize_account(kind, &payload.account)?;
    if payload.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少 6 位".to_string()));
    }

    let hash = hash_password(&payload.password)?;
    let nickname = payload
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let user = user_store::create_account_user(&state.db, kind, &account, &hash, nickname).await?;

    if let Some(inviter) = payload
        .invite_from
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let _ = user_store::record_invite(&state.db, &user.id, inviter).await;
    }

    Ok(Json(AccountLoginResponse {
        token: format!("dev-token-{}", user.id),
        user_id: user.id,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
    }))
}

/// 手机号 / 邮箱 + 密码 登录。
pub async fn account_login(
    State(state): State<AppState>,
    Json(payload): Json<AccountLoginRequest>,
) -> Result<Json<AccountLoginResponse>, AppError> {
    let raw = payload.account.trim();
    let kind = if raw.contains('@') { "email" } else { "phone" };
    let account = normalize_account(kind, raw)?;

    let (user, hash) = user_store::find_account_user(&state.db, kind, &account)
        .await?
        .ok_or_else(|| AppError::Unauthorized("账号或密码不正确".to_string()))?;

    if !verify_password(&payload.password, &hash) {
        return Err(AppError::Unauthorized("账号或密码不正确".to_string()));
    }

    Ok(Json(AccountLoginResponse {
        token: format!("dev-token-{}", user.id),
        user_id: user.id,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
    }))
}

fn normalize_account(kind: &str, account: &str) -> Result<String, AppError> {
    let a = account.trim();
    match kind {
        "phone" => {
            let ok = a.len() == 11 && a.starts_with('1') && a.chars().all(|c| c.is_ascii_digit());
            if !ok {
                return Err(AppError::BadRequest("手机号格式不正确".to_string()));
            }
            Ok(a.to_string())
        }
        "email" => {
            let a = a.to_lowercase();
            if !a.contains('@') || !a.contains('.') || a.len() < 5 {
                return Err(AppError::BadRequest("邮箱格式不正确".to_string()));
            }
            Ok(a)
        }
        _ => Err(AppError::BadRequest("kind must be phone or email".to_string())),
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|err| AppError::Internal(err.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::PasswordHash;
    use argon2::{Argon2, PasswordVerifier};

    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
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

    let (user, _) = user_store::upsert_wechat_user(&state.db, &openid, unionid).await?;
    let token = format!("dev-token-{}", user.id);

    Ok(Json(LoginResponse {
        token,
        user_id: user.id,
        openid,
        unionid: user.unionid,
    }))
}
