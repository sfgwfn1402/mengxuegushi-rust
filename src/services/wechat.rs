use serde::Deserialize;
use serde_json::json;

use crate::{error::AppError, models::auth::WechatCode2SessionResponse, AppState};

const CODE2SESSION_URL: &str = "https://api.weixin.qq.com/sns/jscode2session";
const ACCESS_TOKEN_URL: &str = "https://api.weixin.qq.com/cgi-bin/token";
const GET_WXACODE_URL: &str = "https://api.weixin.qq.com/wxa/getwxacode";

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

pub async fn code2session(
    state: &AppState,
    code: &str,
) -> Result<WechatCode2SessionResponse, AppError> {
    let app_id = state
        .config
        .wechat_app_id
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("WECHAT_APP_ID is not configured".to_string()))?;
    let app_secret =
        state.config.wechat_app_secret.as_deref().ok_or_else(|| {
            AppError::BadRequest("WECHAT_APP_SECRET is not configured".to_string())
        })?;

    let response = state
        .http_client
        .get(CODE2SESSION_URL)
        .query(&[
            ("appid", app_id),
            ("secret", app_secret),
            ("js_code", code),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?;

    let body = response
        .json::<WechatCode2SessionResponse>()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?;

    if let Some(errcode) = body.errcode {
        if errcode != 0 {
            let errmsg = body.errmsg.clone().unwrap_or_default();
            // 40029/40163 等通常是小程序端传来的 code 无效/已使用/过期，属于客户端登录态问题，
            // 不应映射成 502，否则前端会误判为网关或 Nginx 故障并触发无意义重试。
            if matches!(errcode, 40029 | 40163 | 40013) {
                return Err(AppError::BadRequest(format!(
                    "wechat code2session failed: {errcode} {errmsg}"
                )));
            }
            return Err(AppError::Upstream(format!(
                "wechat code2session failed: {errcode} {errmsg}"
            )));
        }
    }

    Ok(body)
}

pub async fn access_token(state: &AppState) -> Result<String, AppError> {
    let app_id = state
        .config
        .wechat_app_id
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("WECHAT_APP_ID is not configured".to_string()))?;
    let app_secret =
        state.config.wechat_app_secret.as_deref().ok_or_else(|| {
            AppError::BadRequest("WECHAT_APP_SECRET is not configured".to_string())
        })?;

    let body = state
        .http_client
        .get(ACCESS_TOKEN_URL)
        .query(&[
            ("grant_type", "client_credential"),
            ("appid", app_id),
            ("secret", app_secret),
        ])
        .send()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?
        .json::<AccessTokenResponse>()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?;

    if let Some(token) = body.access_token {
        return Ok(token);
    }

    Err(AppError::Upstream(format!(
        "wechat access_token failed: {} {}",
        body.errcode.unwrap_or_default(),
        body.errmsg.unwrap_or_default()
    )))
}

pub async fn get_wxacode(state: &AppState, path: &str) -> Result<Vec<u8>, AppError> {
    let token = access_token(state).await?;
    let response = state
        .http_client
        .post(GET_WXACODE_URL)
        .query(&[("access_token", token.as_str())])
        .json(&json!({
            "path": path,
            "width": 280,
            "auto_color": false,
            "line_color": { "r": 255, "g": 107, "b": 74 },
            "is_hyaline": false
        }))
        .send()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|err| AppError::Upstream(err.to_string()))?;

    if bytes.starts_with(b"{") {
        let text = String::from_utf8_lossy(&bytes).to_string();
        return Err(AppError::Upstream(format!("wechat wxacode failed: {text}")));
    }

    Ok(bytes.to_vec())
}
