//! App Store Server Notifications V2 回调。
//! Apple 服务器直接调用，无用户登录态——信任完全来自 JWS 验签（apple_iap）。
//! 在 App Store Connect → App 信息 → App Store 服务器通知 中配置：
//!   生产环境 URL: https://www.duwei.cloud/api/iap/notifications
//!   沙盒环境 URL: 同上（用 environment 字段区分）

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotificationRequest {
    pub signed_payload: String,
}

/// 接收 Apple 服务端通知（退款/撤销/续期/过期等），验签后合并进订阅记录。
/// 验签不过返回 400（Apple 会重试）；处理成功返回 200 + 通知类型。
pub async fn apple_notifications(
    State(state): State<AppState>,
    Json(payload): Json<AppleNotificationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let jws = payload.signed_payload.trim();
    if jws.is_empty() {
        return Err(AppError::BadRequest("signedPayload is required".to_string()));
    }
    let notification_type =
        crate::services::subscription_store::handle_apple_notification(&state.db, jws).await?;
    Ok(Json(serde_json::json!({ "ok": true, "type": notification_type })))
}
