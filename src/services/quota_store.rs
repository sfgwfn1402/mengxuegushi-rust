//! 每日限额（服务器口径）：新学闸口、创作记账、用量查询。
//! 「今日」按 Asia/Shanghai 自然日切分（用户群是国内亲子）。
//! 免费额度与客户端 DailyQuota 保持一致：新学 1 首/天 + 创作 1 次/天，会员不限。

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use super::subscription_store;

pub const FREE_DAILY_NEW_POEMS: i64 = 1;
pub const FREE_DAILY_CREATIONS: i64 = 1;

#[derive(Debug, Serialize)]
pub struct QuotaStatus {
    pub is_premium: bool,
    pub allowed: Option<bool>,          // 仅 start-poem 接口返回：本次是否放行
    pub reason: Option<String>,         // premium | learned | already_today | counted | quota_exceeded
    pub today_new_poem_ids: Vec<i32>,   // 今天已开始的新诗 id（服务器口径）
    pub new_poems_total: i64,
    pub creations_used: i64,
    pub creations_total: i64,
}

/// 今天已开始的「新诗」id 集合（服务器流水口径）。
async fn today_new_poem_ids(db: &PgPool, user_id: &str) -> Result<Vec<i32>, AppError> {
    let rows = sqlx::query_as::<_, (i32,)>(
        r#"
        SELECT DISTINCT ref_id FROM quota_events
        WHERE user_id = $1 AND kind = 'start_poem' AND ref_id IS NOT NULL
          AND (created_at AT TIME ZONE 'Asia/Shanghai')::date
            = (NOW() AT TIME ZONE 'Asia/Shanghai')::date
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn today_creation_count(db: &PgPool, user_id: &str) -> Result<i64, AppError> {
    let (n,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM quota_events
        WHERE user_id = $1 AND kind = 'creation'
          AND (created_at AT TIME ZONE 'Asia/Shanghai')::date
            = (NOW() AT TIME ZONE 'Asia/Shanghai')::date
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(n)
}

async fn is_poem_learned(db: &PgPool, user_id: &str, poem_id: i32) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_poem_progress
            WHERE user_id = $1 AND poem_id = $2 AND learned = TRUE
        )
        "#,
    )
    .bind(user_id)
    .bind(poem_id)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(exists)
}

async fn status(
    db: &PgPool,
    user_id: &str,
    allowed: Option<bool>,
    reason: Option<&str>,
) -> Result<QuotaStatus, AppError> {
    let premium = subscription_store::entitlement(db, user_id).await?.is_premium;
    Ok(QuotaStatus {
        is_premium: premium,
        allowed,
        reason: reason.map(|s| s.to_string()),
        today_new_poem_ids: today_new_poem_ids(db, user_id).await?,
        new_poems_total: FREE_DAILY_NEW_POEMS,
        creations_used: today_creation_count(db, user_id).await?,
        creations_total: FREE_DAILY_CREATIONS,
    })
}

/// 新学闸口：会员/已学会/今天已开始过 → 放行不记账；否则今天未满 1 首则记账放行，满了拒绝。
/// 全部判定在服务端完成，客户端只上报 poem_id。
pub async fn start_poem(db: &PgPool, user_id: &str, poem_id: i32) -> Result<QuotaStatus, AppError> {
    if subscription_store::entitlement(db, user_id).await?.is_premium {
        return status(db, user_id, Some(true), Some("premium")).await;
    }
    if is_poem_learned(db, user_id, poem_id).await? {
        return status(db, user_id, Some(true), Some("learned")).await;
    }
    let ids = today_new_poem_ids(db, user_id).await?;
    if ids.contains(&poem_id) {
        return status(db, user_id, Some(true), Some("already_today")).await;
    }
    if ids.len() as i64 >= FREE_DAILY_NEW_POEMS {
        return status(db, user_id, Some(false), Some("quota_exceeded")).await;
    }
    sqlx::query(
        "INSERT INTO quota_events (id, user_id, kind, ref_id) VALUES ($1, $2, 'start_poem', $3)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(poem_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    status(db, user_id, Some(true), Some("counted")).await
}

/// 发布成功记一次创作（客户端在朗诵/诗配画发布成功后调用）。
pub async fn record_creation(db: &PgPool, user_id: &str) -> Result<QuotaStatus, AppError> {
    sqlx::query(
        "INSERT INTO quota_events (id, user_id, kind, ref_id) VALUES ($1, $2, 'creation', NULL)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    status(db, user_id, None, None).await
}

/// 用量查询：客户端启动/进入创作页时刷新缓存用。
pub async fn get_status(db: &PgPool, user_id: &str) -> Result<QuotaStatus, AppError> {
    status(db, user_id, None, None).await
}

/// DEBUG：清掉本人今天的限额流水（仅管理员，配合客户端开发者控制台反复测试）。
pub async fn debug_reset_today(db: &PgPool, user_id: &str) -> Result<QuotaStatus, AppError> {
    sqlx::query(
        r#"
        DELETE FROM quota_events
        WHERE user_id = $1
          AND (created_at AT TIME ZONE 'Asia/Shanghai')::date
            = (NOW() AT TIME ZONE 'Asia/Shanghai')::date
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    status(db, user_id, None, None).await
}
