use sqlx::{PgPool, Row};

use crate::{
    error::AppError,
    models::{
        moment::{DeleteMomentResponse, MomentItem},
        recitation::LikeResponse,
    },
};

fn row_to_moment(row: sqlx::postgres::PgRow) -> Result<MomentItem, AppError> {
    let id: String = row.get("id");
    let paths: serde_json::Value = row.try_get("object_paths").unwrap_or(serde_json::json!([]));
    let n = paths.as_array().map(|a| a.len()).unwrap_or(0);
    let images: Vec<String> = if n > 0 {
        (0..n).map(|i| format!("/api/moments/{id}/image/{i}")).collect()
    } else {
        // 兼容旧单图行
        vec![format!("/api/moments/{id}/image")]
    };
    let image_url = images.first().cloned().unwrap_or_default();
    Ok(MomentItem {
        id,
        user_id: row.get("user_id"),
        nickname: row.get("nickname"),
        avatar_url: row.get("avatar_url"),
        content: row.get("content"),
        image_url,
        images,
        like_count: row.get("like_count"),
        liked_by_me: row.get("liked_by_me"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    })
}

pub async fn create_moment(
    db: &PgPool,
    id: &str,
    user_id: &str,
    content: &str,
    object_paths: &[String],
) -> Result<MomentItem, AppError> {
    let first = object_paths.first().cloned().unwrap_or_default();
    let paths_json = serde_json::json!(object_paths);
    sqlx::query(
        r#"
        INSERT INTO moments (id, user_id, content, image_url, object_path, object_paths, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'submitted')
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(content)
    .bind(format!("/api/moments/{id}/image/0"))
    .bind(&first)
    .bind(&paths_json)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    get_moment(db, id, Some(user_id)).await
}

// 取第 idx 张图的对象键（兼容旧单图行）
pub async fn get_image_object_path(
    db: &PgPool,
    moment_id: &str,
    idx: usize,
) -> Result<String, AppError> {
    let row = sqlx::query(
        "SELECT object_path, object_paths FROM moments WHERE id = $1 AND status IN ('submitted','public')",
    )
    .bind(moment_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("moment {moment_id}")))?;

    let paths: serde_json::Value = row.try_get("object_paths").unwrap_or(serde_json::json!([]));
    if let Some(arr) = paths.as_array() {
        if let Some(p) = arr.get(idx).and_then(|v| v.as_str()) {
            return Ok(p.to_string());
        }
    }
    if idx == 0 {
        let single: String = row.get("object_path");
        if !single.is_empty() {
            return Ok(single);
        }
    }
    Err(AppError::NotFound(format!("moment image {moment_id}/{idx}")))
}

pub async fn list_public(
    db: &PgPool,
    current_user_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MomentItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
               m.like_count, m.status, m.created_at,
               EXISTS(SELECT 1 FROM moment_likes l WHERE l.moment_id = m.id AND l.user_id = $1) AS liked_by_me
        FROM moments m
        JOIN users u ON u.id = m.user_id
        WHERE m.status = 'public' OR (m.user_id = $1 AND m.status = 'submitted')
        ORDER BY m.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(current_user_id.unwrap_or(""))
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    rows.into_iter().map(row_to_moment).collect()
}

pub async fn list_mine(
    db: &PgPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<MomentItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
               m.like_count, m.status, m.created_at, FALSE AS liked_by_me
        FROM moments m
        JOIN users u ON u.id = m.user_id
        WHERE m.user_id = $1 AND m.status IN ('submitted','public','rejected')
        ORDER BY m.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    rows.into_iter().map(row_to_moment).collect()
}

pub async fn get_moment(
    db: &PgPool,
    moment_id: &str,
    current_user_id: Option<&str>,
) -> Result<MomentItem, AppError> {
    let row = sqlx::query(
        r#"
        SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
               m.like_count, m.status, m.created_at,
               EXISTS(SELECT 1 FROM moment_likes l WHERE l.moment_id = m.id AND l.user_id = $2) AS liked_by_me
        FROM moments m
        JOIN users u ON u.id = m.user_id
        WHERE m.id = $1 AND m.status IN ('submitted','public')
        "#,
    )
    .bind(moment_id)
    .bind(current_user_id.unwrap_or(""))
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    row_to_moment(row)
}


// 取本人这条动态的所有图片对象键（供删除 MinIO 文件）
pub async fn owned_object_paths(
    db: &PgPool,
    moment_id: &str,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    let row = sqlx::query("SELECT object_path, object_paths FROM moments WHERE id = $1 AND user_id = $2")
        .bind(moment_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let Some(row) = row else { return Ok(vec![]); };
    let paths: serde_json::Value = row.try_get("object_paths").unwrap_or(serde_json::json!([]));
    let mut v: Vec<String> = paths
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if v.is_empty() {
        let single: String = row.get("object_path");
        if !single.is_empty() {
            v.push(single);
        }
    }
    Ok(v)
}

pub async fn soft_delete(
    db: &PgPool,
    moment_id: &str,
    user_id: &str,
) -> Result<DeleteMomentResponse, AppError> {
    let affected = sqlx::query(
        "UPDATE moments SET status = 'deleted' WHERE id = $1 AND user_id = $2 AND status IN ('submitted','public','rejected')",
    )
    .bind(moment_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(DeleteMomentResponse {
        deleted: affected > 0,
    })
}

pub async fn admin_set_status(
    db: &PgPool,
    moment_id: &str,
    status: &str,
) -> Result<DeleteMomentResponse, AppError> {
    let affected = sqlx::query(
        "UPDATE moments SET status = $2 WHERE id = $1 AND status IN ('submitted','public','rejected')",
    )
    .bind(moment_id)
    .bind(status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(DeleteMomentResponse {
        deleted: affected > 0,
    })
}

pub async fn list_admin(
    db: &PgPool,
    page: u32,
    page_size: u32,
    status_filter: Option<&str>,
) -> Result<(i64, Vec<MomentItem>), AppError> {
    let offset = ((page.max(1) - 1) as i64) * (page_size as i64);
    let limit = page_size as i64;

    let (total, rows) = match status_filter {
        Some(status) => {
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM moments WHERE status = $1")
                .bind(status)
                .fetch_one(db)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let rows = sqlx::query(
                r#"
                SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
                       m.like_count, m.status, m.created_at, FALSE AS liked_by_me
                FROM moments m JOIN users u ON u.id = m.user_id
                WHERE m.status = $1
                ORDER BY CASE m.status WHEN 'submitted' THEN 0 ELSE 1 END, m.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
            (total, rows)
        }
        None => {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM moments WHERE status IN ('submitted','public','rejected')",
            )
            .fetch_one(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
            let rows = sqlx::query(
                r#"
                SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
                       m.like_count, m.status, m.created_at, FALSE AS liked_by_me
                FROM moments m JOIN users u ON u.id = m.user_id
                WHERE m.status IN ('submitted','public','rejected')
                ORDER BY CASE m.status WHEN 'submitted' THEN 0 ELSE 1 END, m.created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
            (total, rows)
        }
    };

    let items = rows
        .into_iter()
        .map(row_to_moment)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((total, items))
}

pub async fn like_moment(
    db: &PgPool,
    moment_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let inserted = sqlx::query(
        "INSERT INTO moment_likes (moment_id, user_id) VALUES ($1, $2) ON CONFLICT (moment_id, user_id) DO NOTHING",
    )
    .bind(moment_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if inserted > 0 {
        sqlx::query("UPDATE moments SET like_count = like_count + 1 WHERE id = $1 AND status = 'public'")
            .bind(moment_id)
            .execute(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }
    let like_count = current_like_count(db, moment_id).await?;
    Ok(LikeResponse { liked: true, like_count })
}

pub async fn unlike_moment(
    db: &PgPool,
    moment_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let deleted = sqlx::query("DELETE FROM moment_likes WHERE moment_id = $1 AND user_id = $2")
        .bind(moment_id)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .rows_affected();

    if deleted > 0 {
        sqlx::query("UPDATE moments SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1")
            .bind(moment_id)
            .execute(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }
    let like_count = current_like_count(db, moment_id).await?;
    Ok(LikeResponse { liked: false, like_count })
}

async fn current_like_count(db: &PgPool, moment_id: &str) -> Result<i32, AppError> {
    let count: Option<i32> = sqlx::query_scalar("SELECT like_count FROM moments WHERE id = $1")
        .bind(moment_id)
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(count.unwrap_or(0))
}
