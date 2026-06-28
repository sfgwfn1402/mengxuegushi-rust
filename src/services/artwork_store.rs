use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        artwork::{ArtworkItem, DeleteArtworkResponse},
        recitation::LikeResponse,
    },
};

pub async fn create_artwork(
    db: &PgPool,
    user_id: &str,
    poem_id: i32,
    title: &str,
    description: Option<&str>,
    image_url: &str,
    object_path: &str,
) -> Result<ArtworkItem, AppError> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO poem_artworks (id, user_id, poem_id, title, description, image_url, object_path)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(poem_id)
    .bind(title)
    .bind(description)
    .bind(image_url)
    .bind(object_path)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    get_artwork(db, &id, Some(user_id)).await
}

pub async fn list_recent(
    db: &PgPool,
    current_user_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ArtworkItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
               a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
               EXISTS(SELECT 1 FROM poem_artwork_likes l WHERE l.artwork_id = a.id AND l.user_id = $1) AS liked_by_me
        FROM poem_artworks a
        JOIN users u ON u.id = a.user_id
        JOIN poems p ON p.id = a.poem_id
        WHERE a.status = 'public'
        ORDER BY a.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(current_user_id.unwrap_or(""))
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    rows.into_iter().map(row_to_artwork).collect()
}

// 某用户的公开诗配画
pub async fn list_public_by_user(
    db: &PgPool,
    target_user_id: &str,
    current_user_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ArtworkItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
               a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
               EXISTS(SELECT 1 FROM poem_artwork_likes l WHERE l.artwork_id = a.id AND l.user_id = $1) AS liked_by_me
        FROM poem_artworks a
        JOIN users u ON u.id = a.user_id
        JOIN poems p ON p.id = a.poem_id
        WHERE a.user_id = $4
          AND (a.status = 'public' OR ($1 = $4 AND a.status IN ('active','submitted','rejected')))
        ORDER BY a.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(current_user_id.unwrap_or(""))
    .bind(limit)
    .bind(offset)
    .bind(target_user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    rows.into_iter().map(row_to_artwork).collect()
}

pub async fn list_mine(
    db: &PgPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<ArtworkItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
               a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
               EXISTS(SELECT 1 FROM poem_artwork_likes l WHERE l.artwork_id = a.id AND l.user_id = $1) AS liked_by_me
        FROM poem_artworks a
        JOIN users u ON u.id = a.user_id
        JOIN poems p ON p.id = a.poem_id
        WHERE a.status IN ('active','submitted','public','rejected') AND a.user_id = $1
        ORDER BY a.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    rows.into_iter().map(row_to_artwork).collect()
}

pub async fn get_artwork(
    db: &PgPool,
    artwork_id: &str,
    current_user_id: Option<&str>,
) -> Result<ArtworkItem, AppError> {
    let row = sqlx::query(
        r#"
        SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
               a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
               EXISTS(SELECT 1 FROM poem_artwork_likes l WHERE l.artwork_id = a.id AND l.user_id = $2) AS liked_by_me
        FROM poem_artworks a
        JOIN users u ON u.id = a.user_id
        JOIN poems p ON p.id = a.poem_id
        WHERE a.id = $1 AND a.status IN ('active','submitted','public')
        "#,
    )
    .bind(artwork_id)
    .bind(current_user_id.unwrap_or(""))
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    row_to_artwork(row)
}

pub async fn get_object_path(db: &PgPool, artwork_id: &str) -> Result<String, AppError> {
    sqlx::query_scalar("SELECT object_path FROM poem_artworks WHERE id = $1 AND status IN ('active','submitted','public')")
        .bind(artwork_id)
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("artwork {artwork_id}")))
}

// 编辑诗配画的标题/配文；改完重新进入审核(submitted)
pub async fn update_artwork(
    db: &PgPool,
    artwork_id: &str,
    user_id: &str,
    title: &str,
    description: &str,
) -> Result<ArtworkItem, AppError> {
    let affected = sqlx::query(
        "UPDATE poem_artworks SET title = $3, description = $4, status = 'submitted', updated_at = CURRENT_TIMESTAMP
         WHERE id = $1 AND user_id = $2 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(artwork_id)
    .bind(user_id)
    .bind(title)
    .bind(description)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::BadRequest("不可编辑（非本人或状态不允许）".to_string()));
    }
    get_artwork(db, artwork_id, Some(user_id)).await
}

pub async fn soft_delete(
    db: &PgPool,
    artwork_id: &str,
    user_id: &str,
) -> Result<DeleteArtworkResponse, AppError> {
    let affected = sqlx::query(
        "UPDATE poem_artworks SET status = 'deleted', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND user_id = $2 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(artwork_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(DeleteArtworkResponse {
        deleted: affected > 0,
    })
}

pub async fn set_submission_status(
    db: &PgPool,
    artwork_id: &str,
    user_id: &str,
    status: &str,
) -> Result<DeleteArtworkResponse, AppError> {
    let affected = sqlx::query(
        "UPDATE poem_artworks SET status = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND user_id = $2 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(artwork_id)
    .bind(user_id)
    .bind(status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(DeleteArtworkResponse {
        deleted: affected > 0,
    })
}

pub async fn admin_set_status(
    db: &PgPool,
    artwork_id: &str,
    status: &str,
) -> Result<DeleteArtworkResponse, AppError> {
    let affected = sqlx::query(
        "UPDATE poem_artworks SET status = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status IN ('active','submitted','public','rejected')",
    )
    .bind(artwork_id)
    .bind(status)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    Ok(DeleteArtworkResponse {
        deleted: affected > 0,
    })
}

pub async fn get_status(db: &PgPool, artwork_id: &str) -> Result<Option<String>, AppError> {
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM poem_artworks WHERE id = $1",
    )
    .bind(artwork_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(status)
}

pub async fn list_admin_artworks(
    db: &PgPool,
    page: u32,
    page_size: u32,
    status_filter: Option<&str>,
) -> Result<(i64, Vec<crate::models::artwork::ArtworkItem>), AppError> {
    use sqlx::Row;

    let offset = ((page - 1) as i64) * (page_size as i64);
    let limit = page_size as i64;

    let count_sql = match status_filter {
        Some(_) => "SELECT COUNT(*) FROM poem_artworks WHERE status = $1",
        None => "SELECT COUNT(*) FROM poem_artworks WHERE status IN ('submitted','public','rejected')",
    };

    let total: i64 = match status_filter {
        Some(status) => sqlx::query_scalar(count_sql)
            .bind(status)
            .fetch_one(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?,
        None => sqlx::query_scalar(count_sql)
            .fetch_one(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?,
    };

    let rows = match status_filter {
        Some(status) => sqlx::query(
            r#"
            SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
                   a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
                   FALSE AS liked_by_me
            FROM poem_artworks a
            JOIN users u ON u.id = a.user_id
            JOIN poems p ON p.id = a.poem_id
            WHERE a.status = $1
            ORDER BY
                CASE a.status WHEN 'submitted' THEN 0 ELSE 1 END,
                a.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?,
        None => sqlx::query(
            r#"
            SELECT a.id, a.poem_id, a.user_id, u.nickname, u.avatar_url, p.title AS poem_title,
                   a.title, a.description, a.image_url, a.like_count, a.status, a.created_at,
                   FALSE AS liked_by_me
            FROM poem_artworks a
            JOIN users u ON u.id = a.user_id
            JOIN poems p ON p.id = a.poem_id
            WHERE a.status IN ('submitted','public','rejected')
            ORDER BY
                CASE a.status WHEN 'submitted' THEN 0 ELSE 1 END,
                a.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?,
    };

    let items = rows
        .into_iter()
        .map(|row| {
            use crate::models::artwork::ArtworkItem;
            ArtworkItem {
                id: row.get("id"),
                poem_id: row.get("poem_id"),
                user_id: row.get("user_id"),
                nickname: row.get("nickname"),
                avatar_url: row.get("avatar_url"),
                poem_title: row.get("poem_title"),
                title: row.get("title"),
                description: row.get("description"),
                image_url: row.get("image_url"),
                like_count: row.get("like_count"),
                liked_by_me: row.get("liked_by_me"),
                status: row.get("status"),
                created_at: row.get("created_at"),
            }
        })
        .collect();

    Ok((total, items))
}

pub async fn like_artwork(
    db: &PgPool,
    artwork_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let mut tx = db
        .begin()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO poem_artwork_likes (artwork_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id, artwork_id) DO NOTHING
        "#,
    )
    .bind(artwork_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if inserted > 0 {
        sqlx::query(
            "UPDATE poem_artworks SET like_count = like_count + 1, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status = 'public'",
        )
        .bind(artwork_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let like_count = fetch_like_count_in_tx(&mut tx, artwork_id).await?;
    tx.commit()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(LikeResponse {
        liked: true,
        like_count,
    })
}

pub async fn unlike_artwork(
    db: &PgPool,
    artwork_id: &str,
    user_id: &str,
) -> Result<LikeResponse, AppError> {
    let mut tx = db
        .begin()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let deleted =
        sqlx::query("DELETE FROM poem_artwork_likes WHERE artwork_id = $1 AND user_id = $2")
            .bind(artwork_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();

    if deleted > 0 {
        sqlx::query(
            "UPDATE poem_artworks SET like_count = GREATEST(like_count - 1, 0), updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status = 'public'",
        )
        .bind(artwork_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let like_count = fetch_like_count_in_tx(&mut tx, artwork_id).await?;
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
    artwork_id: &str,
) -> Result<i32, AppError> {
    sqlx::query_scalar::<_, i32>(
        "SELECT like_count FROM poem_artworks WHERE id = $1 AND status = 'public'",
    )
    .bind(artwork_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("artwork {artwork_id}")))
}

fn row_to_artwork(row: sqlx::postgres::PgRow) -> Result<ArtworkItem, AppError> {
    Ok(ArtworkItem {
        id: row.get("id"),
        poem_id: row.get("poem_id"),
        user_id: row.get("user_id"),
        nickname: row.get("nickname"),
        avatar_url: row.get("avatar_url"),
        poem_title: row.get("poem_title"),
        title: row.get("title"),
        description: row.get("description"),
        image_url: row.get("image_url"),
        like_count: row.get("like_count"),
        liked_by_me: row.get("liked_by_me"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    })
}
