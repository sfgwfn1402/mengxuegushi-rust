use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WechatLoginRequest {
    pub code: String,
    pub invite_from: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DevLoginRequest {
    pub openid: Option<String>,
    pub unionid: Option<String>,
}

/// 手机号 / 邮箱 + 密码 注册或登录。
/// `kind` 取 "phone" 或 "email"；`account` 为手机号或邮箱。
#[derive(Debug, Deserialize)]
pub struct AccountRegisterRequest {
    pub kind: String,
    pub account: String,
    pub password: String,
    pub nickname: Option<String>,
    pub invite_from: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountLoginRequest {
    pub account: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AccountLoginResponse {
    pub token: String,
    pub user_id: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: String,
    pub openid: String,
    pub unionid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WechatLoginResponse {
    pub token: String,
    pub user_id: String,
    pub openid: String,
    pub session_key: Option<String>,
    pub unionid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WechatCode2SessionResponse {
    pub openid: Option<String>,
    pub session_key: Option<String>,
    pub unionid: Option<String>,
    pub errcode: Option<i64>,
    pub errmsg: Option<String>,
}
