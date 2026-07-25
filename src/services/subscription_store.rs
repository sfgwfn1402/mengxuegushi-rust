//! 会员订阅记账：验签后的交易落库 + 会员资格查询。
//! 同一 original_transaction_id 重复上报（续期/恢复/多端）→ 覆盖更新到最新状态。

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use super::apple_iap::{self, VerifiedTransaction};

#[derive(Debug, Serialize)]
pub struct EntitlementResponse {
    pub is_premium: bool,
    pub product_id: Option<String>,
    pub expires_at: Option<String>,
    pub environment: Option<String>,
}

/// 客户端上报签名交易：验签 → upsert → 返回最新会员资格。
/// 验签不过返回 400，不落库。
pub async fn report_subscription(
    db: &PgPool,
    user_id: &str,
    signed_transaction: &str,
) -> Result<EntitlementResponse, AppError> {
    let tx = apple_iap::verify_signed_transaction(signed_transaction)
        .map_err(AppError::BadRequest)?;
    upsert_transaction(db, user_id, &tx, signed_transaction).await?;
    entitlement(db, user_id).await
}

async fn upsert_transaction(
    db: &PgPool,
    user_id: &str,
    tx: &VerifiedTransaction,
    jws: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_subscriptions
            (id, user_id, original_transaction_id, transaction_id, product_id, bundle_id,
             purchase_at, expires_at, revoked_at, environment, signed_transaction)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (original_transaction_id) DO UPDATE SET
            user_id = EXCLUDED.user_id,
            transaction_id = EXCLUDED.transaction_id,
            product_id = EXCLUDED.product_id,
            purchase_at = EXCLUDED.purchase_at,
            expires_at = EXCLUDED.expires_at,
            revoked_at = EXCLUDED.revoked_at,
            environment = EXCLUDED.environment,
            signed_transaction = EXCLUDED.signed_transaction,
            updated_at = NOW()
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(&tx.original_transaction_id)
    .bind(&tx.transaction_id)
    .bind(&tx.product_id)
    .bind(&tx.bundle_id)
    .bind(tx.purchase_at)
    .bind(tx.expires_at)
    .bind(tx.revoked_at)
    .bind(&tx.environment)
    .bind(jws)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

/// 当前会员资格：有未撤销且未过期的订阅记录即会员。
/// 每月/每年订阅必有 expires_at；NULL（理论上的买断）按永久会员处理。
pub async fn entitlement(db: &PgPool, user_id: &str) -> Result<EntitlementResponse, AppError> {
    let row = sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>, String)>(
        r#"
        SELECT product_id, expires_at, environment
        FROM user_subscriptions
        WHERE user_id = $1
          AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        ORDER BY expires_at DESC NULLS FIRST
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(match row {
        Some((product_id, expires_at, environment)) => EntitlementResponse {
            is_premium: true,
            product_id: Some(product_id),
            expires_at: expires_at.map(|t| t.to_rfc3339()),
            environment: Some(environment),
        },
        None => EntitlementResponse {
            is_premium: false,
            product_id: None,
            expires_at: None,
            environment: None,
        },
    })
}

/// 管理端：最近上报的订阅记录（含用户昵称）。
#[derive(Debug, Serialize)]
pub struct AdminSubscriptionItem {
    pub user_id: String,
    pub nickname: Option<String>,
    pub product_id: String,
    pub original_transaction_id: String,
    pub purchase_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub environment: String,
    pub updated_at: String,
}

pub async fn list_recent(db: &PgPool, limit: i64) -> Result<Vec<AdminSubscriptionItem>, AppError> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT s.user_id, u.nickname, s.product_id, s.original_transaction_id,
               s.purchase_at, s.expires_at, s.revoked_at, s.environment, s.updated_at
        FROM user_subscriptions s
        LEFT JOIN users u ON u.id = s.user_id
        ORDER BY s.updated_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(
            |(user_id, nickname, product_id, otid, purchase_at, expires_at, revoked_at, environment, updated_at)| {
                AdminSubscriptionItem {
                    user_id,
                    nickname,
                    product_id,
                    original_transaction_id: otid,
                    purchase_at: purchase_at.map(|t| t.to_rfc3339()),
                    expires_at: expires_at.map(|t| t.to_rfc3339()),
                    revoked_at: revoked_at.map(|t| t.to_rfc3339()),
                    environment,
                    updated_at: updated_at.to_rfc3339(),
                }
            },
        )
        .collect())
}
