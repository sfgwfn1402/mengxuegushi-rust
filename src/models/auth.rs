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
