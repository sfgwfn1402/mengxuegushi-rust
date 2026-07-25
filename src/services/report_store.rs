use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::error::AppError;

/// 管理端「举报」列表项：被举报内容的预览 + 举报理由 + 同一目标被举报次数。
#[derive(Debug, Serialize)]
pub struct AdminReportItem {
    pub id: String,
    pub target_kind: String,        // moment | comment | artwork | recitation | user
    pub target_id: String,
    pub reason: Option<String>,
    pub status: String,             // pending | reviewed | dismissed
    pub created_at: String,
    pub reporter_name: Option<String>,
    pub target_preview: Option<String>,   // 正文 / 标题 / 昵称
    pub target_image: Option<String>,     // 诗配画封面(如有)
    pub target_author: Option<String>,
    pub target_author_id: Option<String>,
    pub report_count: i64,          // 同一目标被举报的总次数
}

/// 举报列表(可按状态过滤)。左连各内容表拼出预览,窗口函数算出每个目标被举报次数。
pub async fn list_admin_reports(
    db: &PgPool,
    status: Option<&str>,
) -> Result<Vec<AdminReportItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT r.id, r.target_kind, r.target_id, r.reason, r.status, r.created_at,
               ru.nickname AS reporter_name,
               COALESCE(m.content, aw.title, pr.title, mc.content, tu.nickname) AS target_preview,
               aw.image_url AS target_image,
               COALESCE(mu.nickname, awu.nickname, recu.nickname, mcu.nickname, tu.nickname) AS target_author,
               COALESCE(m.user_id, aw.user_id, rec.user_id, mc.user_id, tu.id) AS target_author_id,
               COUNT(*) OVER (PARTITION BY r.target_kind, r.target_id) AS report_count
        FROM content_reports r
        LEFT JOIN users ru ON ru.id = r.reporter_id
        LEFT JOIN moments m ON r.target_kind = 'moment' AND m.id = r.target_id
        LEFT JOIN users mu ON mu.id = m.user_id
        LEFT JOIN poem_artworks aw ON r.target_kind = 'artwork' AND aw.id = r.target_id
        LEFT JOIN users awu ON awu.id = aw.user_id
        LEFT JOIN user_recitations rec ON r.target_kind = 'recitation' AND rec.id = r.target_id
        LEFT JOIN poems pr ON pr.id = rec.poem_id
        LEFT JOIN users recu ON recu.id = rec.user_id
        LEFT JOIN moment_comments mc ON r.target_kind = 'comment' AND mc.id = r.target_id
        LEFT JOIN users mcu ON mcu.id = mc.user_id
        LEFT JOIN users tu ON r.target_kind = 'user' AND tu.id = r.target_id
        WHERE ($1::text IS NULL OR r.status = $1)
        ORDER BY r.created_at DESC
        LIMIT 300
        "#,
    )
    .bind(status)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| {
            let created_at: chrono::DateTime<chrono::Utc> =
                row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now());
            AdminReportItem {
                id: row.try_get("id").unwrap_or_default(),
                target_kind: row.try_get("target_kind").unwrap_or_default(),
                target_id: row.try_get("target_id").unwrap_or_default(),
                reason: row.try_get("reason").unwrap_or(None),
                status: row.try_get("status").unwrap_or_default(),
                created_at: created_at.to_rfc3339(),
                reporter_name: row.try_get("reporter_name").unwrap_or(None),
                target_preview: row.try_get("target_preview").unwrap_or(None),
                target_image: row.try_get("target_image").unwrap_or(None),
                target_author: row.try_get("target_author").unwrap_or(None),
                target_author_id: row.try_get("target_author_id").unwrap_or(None),
                report_count: row.try_get("report_count").unwrap_or(1),
            }
        })
        .collect();
    Ok(items)
}

/// 处理举报:
/// - `dismiss`  忽略:把该目标所有 pending 举报标记 dismissed(内容保留)。
/// - `takedown` 下架:把被举报内容状态置 rejected(动态/诗配画/朗诵),并把举报标记 reviewed。
pub async fn resolve_report(db: &PgPool, report_id: &str, action: &str) -> Result<(), AppError> {
    let row = sqlx::query("SELECT target_kind, target_id FROM content_reports WHERE id = $1")
        .bind(report_id)
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .ok_or_else(|| AppError::NotFound("report not found".to_string()))?;
    let target_kind: String = row.try_get("target_kind").unwrap_or_default();
    let target_id: String = row.try_get("target_id").unwrap_or_default();

    let new_status = match action {
        "dismiss" => "dismissed",
        "takedown" => {
            match target_kind.as_str() {
                "moment" => {
                    crate::services::moment_store::admin_set_status(db, &target_id, "rejected").await?;
                }
                "artwork" => {
                    crate::services::artwork_store::admin_set_status(db, &target_id, "rejected").await?;
                }
                "recitation" => {
                    crate::services::recitation_store::admin_set_status(db, &target_id, "rejected").await?;
                }
                _ => {} // comment / user 暂不改内容状态,仅记为已处理
            }
            "reviewed"
        }
        _ => return Err(AppError::BadRequest("invalid action".to_string())),
    };

    sqlx::query(
        "UPDATE content_reports SET status = $3 WHERE target_kind = $1 AND target_id = $2 AND status = 'pending'",
    )
    .bind(&target_kind)
    .bind(&target_id)
    .bind(new_status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}
