use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::recitation::{LikeResponse, RecitationItem},
};

pub async fn create_recitation(
    db: &PgPool,
    user_id: &str,
    poem_id: i32,
    audio_url: &str,
    object_path: &str,
    duration_seconds: Option<i32>,
) -> Result<RecitationItem, AppError> {
    let id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    sqlx::query(
        r#"
        UPDATE user_recitations
        SET status = 'replaced', updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1 AND poem_id = $2 AND status = 'active'
        "#,
    )
    .bind(user_id)
    .bind(poem_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO user_recitations
            (id, user_id, poem_id, audio_url, object_path, duration_seconds, like_count, status)
        VALUES ($1, $2, $3, $4, $5, $6, 0, 'active')
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(poem_id)
    .bind(audio_url)
    .bind(object_path)
    .bind(duration_seconds)
    .execute(&mut *tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    tx.commit()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    get_recitation(db, &id, Some(user_id)).await
}

pub async fn get_recitation(
    db: &PgPool,
    recitation_id: &str,
    current_user_id: Option<&str>,
) -> Result<RecitationItem, AppError> {
    let item = sqlx::query_as::<_, RecitationItem>(
        r#"
        SELECT
            r.id,
            r.poem_id,
            r.user_id,
            u.nickname,
            u.avatar_url,
            ('/recitations/' || r.id || '/audio') AS audio_url,
            r.duration_seconds,
            r.like_count,
            EXISTS(
                SELECT 1 FROM user_recitation_likes l
                WHERE l.recitation_id = r.id AND l.user_id = $2
            ) AS liked_by_me,
            r.status,
            r.created_at
        FROM user_recitations r
        JOIN users u ON u.id = r.user_id
        WHERE r.id = $1 AND r.status IN ('active','submitted','public')
        "#,
    )
    .bind(recitation_id)
    .bind(current_user_id.unwrap_or(""))
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound("recitation not found".to_string()))?;

    Ok(item)
}

pub async fn list_top_by_poem(
    db: &PgPool,
    poem_id: i32,
    current_user_id: Option<&str>,
    limit: i64,
) -> Result<Vec<RecitationItem>, AppError> {
    let items = sqlx::query_as::<_, RecitationItem>(
        r#"
        SELECT
            r.id,
            r.poem_id,
            r.user_id,
            u.nickname,
            u.avatar_url,
            ('/recitations/' || r.id || '/audio') AS audio_url,
            r.duration_seconds,
            r.like_count,
            EXISTS(
                SELECT 1 FROM user_recitation_likes l
                WHERE l.recitation_id = r.id AND l.user_id = $2
            ) AS liked_by_me,
            r.status,
            r.created_at
        FROM user_recitations r
        JOIN users u ON u.id = r.user_id
        WHERE r.poem_id = $1 AND r.status = 'public'
        ORDER BY r.like_count DESC, r.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(poem_id)
    .bind(current_user_id.unwrap_or(""))
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(items)
}

pub async fn list_active_by_user(
    db: &PgPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<RecitationItem>, AppError> {
    let items = sqlx::query_as::<_, RecitationItem>(
        r#"
        SELECT
            r.id,
            r.poem_id,
            r.user_id,
            u.nickname,
            u.avatar_url,
            ('/recitations/' || r.id || '/audio') AS audio_url,
            r.duration_seconds,
            r.like_count,
            EXISTS(
                SELECT 1 FROM user_recitation_likes l
                WHERE l.recitation_id = r.id AND l.user_id = $1
            ) AS liked_by_me,
            r.status,
            r.created_at
        FROM user_recitations r
        JOIN users u ON u.id = r.user_id
        WHERE r.user_id = $1 AND r.status IN ('active','submitted','public','rejected')
        ORDER BY r.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(items)
}

pub async fn get_featured_by_poem(
    db: &PgPool,
    poem_id: i32,
    current_user_id: Option<&str>,
    min_likes: i32,
) -> Result<Option<RecitationItem>, AppError> {
    let item = sqlx::query_as::<_, RecitationItem>(
        r#"
        SELECT
            r.id,
            r.poem_id,
            r.user_id,
            u.nickname,
            u.avatar_url,
            ('/recitations/' || r.id || '/audio') AS audio_url,
            r.duration_seconds,
            r.like_count,
            EXISTS(
                SELECT 1 FROM user_recitation_likes l
                WHERE l.recitation_id = r.id AND l.user_id = $3
            ) AS liked_by_me,
            r.status,
            r.created_at
        FROM user_recitations r
        JOIN users u ON u.id = r.user_id
        WHERE r.poem_id = $1
          AND r.status = 'public'
          AND r.like_count >= $2
        ORDER BY r.like_count DESC, r.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(poem_id)
    .bind(min_likes)
    .bind(current_user_id.unwrap_or(""))
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(item)
}

pub async fn like_recitation(
    db: &PgPool,
    recitation_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let like_id = Uuid::new_v4().to_string();
    let mut tx = db
        .begin()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO user_recitation_likes (id, recitation_id, user_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (recitation_id, user_id) DO NOTHING
        "#,
    )
    .bind(&like_id)
    .bind(recitation_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if inserted > 0 {
        sqlx::query(
            r#"
            UPDATE user_recitations
            SET like_count = like_count + 1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND status = 'public'
            "#,
        )
        .bind(recitation_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let like_count = fetch_like_count_in_tx(&mut tx, recitation_id).await?;
    tx.commit()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(LikeResponse {
        liked: true,
        like_count,
    })
}

pub async fn unlike_recitation(
    db: &PgPool,
    recitation_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let mut tx = db
        .begin()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM user_recitation_likes
        WHERE recitation_id = $1 AND user_id = $2
        "#,
    )
    .bind(recitation_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if deleted > 0 {
        sqlx::query(
            r#"
            UPDATE user_recitations
            SET like_count = GREATEST(like_count - 1, 0), updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND status = 'public'
            "#,
        )
        .bind(recitation_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let like_count = fetch_like_count_in_tx(&mut tx, recitation_id).await?;
    tx.commit()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(LikeResponse {
        liked: false,
        like_count,
    })
}

async fn fetch_like_count_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recitation_id: &str,
) -> Result<i32, AppError> {
    let like_count = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT like_count FROM user_recitations
        WHERE id = $1 AND status = 'public'
        "#,
    )
    .bind(recitation_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound("recitation not found".to_string()))?;

    Ok(like_count)
}

pub async fn get_object_path(db: &PgPool, recitation_id: &str) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT object_path FROM user_recitations
        WHERE id = $1 AND status IN ('active','submitted','public')
        "#,
    )
    .bind(recitation_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound("recitation not found".to_string()))
}

pub async fn soft_delete_recitation(
    db: &PgPool,
    recitation_id: &str,
    user_id: &str,
) -> Result<bool, AppError> {
    let rows = sqlx::query(
        r#"
        UPDATE user_recitations
        SET status = 'deleted', updated_at = CURRENT_TIMESTAMP
        WHERE id = $1 AND user_id = $2 AND status IN ('active','submitted','public','rejected')
        "#,
    )
    .bind(recitation_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(rows > 0)
}

pub async fn set_submission_status(
    db: &PgPool,
    recitation_id: &str,
    user_id: &str,
    status: &str,
) -> Result<bool, AppError> {
    let rows = sqlx::query(
        "UPDATE user_recitations SET status = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND user_id = $2 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(recitation_id)
    .bind(user_id)
    .bind(status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();
    Ok(rows > 0)
}

pub async fn admin_set_status(
    db: &PgPool,
    recitation_id: &str,
    status: &str,
) -> Result<bool, AppError> {
    let rows = sqlx::query(
        "UPDATE user_recitations SET status = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(recitation_id)
    .bind(status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();
    Ok(rows > 0)
}

pub async fn get_status(db: &PgPool, recitation_id: &str) -> Result<Option<String>, AppError> {
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM user_recitations WHERE id = $1",
    )
    .bind(recitation_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(status)
}

pub async fn list_admin_recitations(
    db: &PgPool,
    page: u32,
    page_size: u32,
    status_filter: Option<&str>,
) -> Result<(i64, Vec<RecitationItem>), AppError> {
    let offset = ((page - 1) as i64) * (page_size as i64);
    let limit = page_size as i64;

    let (total, items): (i64, Vec<RecitationItem>) = match status_filter {
        Some(status) => {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_recitations WHERE status = $1",
            )
            .bind(status)
            .fetch_one(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let items = sqlx::query_as::<_, RecitationItem>(
                r#"
                SELECT
                    r.id,
                    r.poem_id,
                    r.user_id,
                    u.nickname,
                    u.avatar_url,
                    ('/recitations/' || r.id || '/audio') AS audio_url,
                    r.duration_seconds,
                    r.like_count,
                    FALSE AS liked_by_me,
                    r.status,
                    r.created_at
                FROM user_recitations r
                JOIN users u ON u.id = r.user_id
                WHERE r.status = $1
                ORDER BY
                    CASE r.status WHEN 'submitted' THEN 0 ELSE 1 END,
                    r.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            (total, items)
        }
        None => {
            // Default "全部" view excludes private (active) works — those are user
            // drafts and do not need admin review.
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_recitations WHERE status IN ('submitted','public','rejected')",
            )
            .fetch_one(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let items = sqlx::query_as::<_, RecitationItem>(
                r#"
                SELECT
                    r.id,
                    r.poem_id,
                    r.user_id,
                    u.nickname,
                    u.avatar_url,
                    ('/recitations/' || r.id || '/audio') AS audio_url,
                    r.duration_seconds,
                    r.like_count,
                    FALSE AS liked_by_me,
                    r.status,
                    r.created_at
                FROM user_recitations r
                JOIN users u ON u.id = r.user_id
                WHERE r.status IN ('submitted','public','rejected')
                ORDER BY
                    CASE r.status WHEN 'submitted' THEN 0 ELSE 1 END,
                    r.created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            (total, items)
        }
    };

    Ok((total, items))
}

#[allow(dead_code)]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}
