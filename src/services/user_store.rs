use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::user::{UpdateProgressRequest, User, UserPoemProgress},
};

pub async fn upsert_wechat_user(
    db: &PgPool,
    openid: &str,
    unionid: Option<&str>,
) -> Result<User, AppError> {
    let existing = find_user_by_openid(db, openid).await?;
    let id = existing
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO users (id, openid, unionid)
        VALUES ($1, $2, $3)
        ON CONFLICT(openid) DO UPDATE SET
            unionid = COALESCE(excluded.unionid, users.unionid),
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(openid)
    .bind(unionid)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    find_user_by_id(db, &id)
        .await?
        .ok_or_else(|| AppError::Internal("failed to load upserted user".to_string()))
}

pub async fn find_user_by_id(db: &PgPool, id: &str) -> Result<Option<User>, AppError> {
    let row =
        sqlx::query("SELECT id, openid, unionid, nickname, avatar_url, COALESCE(role, 'user') AS role FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(row.map(row_to_user))
}

pub async fn find_user_by_openid(db: &PgPool, openid: &str) -> Result<Option<User>, AppError> {
    let row = sqlx::query(
        "SELECT id, openid, unionid, nickname, avatar_url, COALESCE(role, 'user') AS role FROM users WHERE openid = $1",
    )
    .bind(openid)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(row.map(row_to_user))
}

pub async fn update_profile(
    db: &PgPool,
    user_id: &str,
    nickname: Option<String>,
    avatar_url: Option<String>,
) -> Result<User, AppError> {
    sqlx::query(
        r#"
        UPDATE users
        SET
            nickname = COALESCE($1, nickname),
            avatar_url = COALESCE($2, avatar_url),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $3
        "#,
    )
    .bind(nickname)
    .bind(avatar_url)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    find_user_by_id(db, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user not found".to_string()))
}

pub async fn list_progress(db: &PgPool, user_id: &str) -> Result<Vec<UserPoemProgress>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT poem_id, learned, read_count, quiz_correct_count, quiz_wrong_count, to_char(last_learned_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_learned_at
        FROM user_poem_progress
        WHERE user_id = $1
        ORDER BY poem_id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    rows.into_iter().map(row_to_progress).collect()
}

pub async fn update_progress(
    db: &PgPool,
    user_id: &str,
    poem_id: u32,
    payload: UpdateProgressRequest,
) -> Result<UserPoemProgress, AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_poem_progress (id, user_id, poem_id)
        VALUES ($1, $2, $3)
        ON CONFLICT(user_id, poem_id) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(poem_id as i32)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let learned = payload.learned;
    let read_delta = payload.read_count_delta.unwrap_or(0);
    let correct_delta = payload.quiz_correct_delta.unwrap_or(0);
    let wrong_delta = payload.quiz_wrong_delta.unwrap_or(0);

    sqlx::query(
        r#"
        UPDATE user_poem_progress
        SET
            learned = COALESCE($1, learned),
            read_count = read_count + $2,
            quiz_correct_count = quiz_correct_count + $3,
            quiz_wrong_count = quiz_wrong_count + $4,
            last_learned_at = CASE
                WHEN $5 = TRUE THEN CURRENT_TIMESTAMP
                ELSE last_learned_at
            END,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $6 AND poem_id = $7
        "#,
    )
    .bind(learned)
    .bind(read_delta as i32)
    .bind(correct_delta as i32)
    .bind(wrong_delta as i32)
    .bind(learned)
    .bind(user_id)
    .bind(poem_id as i32)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    get_progress(db, user_id, poem_id)
        .await?
        .ok_or_else(|| AppError::Internal("failed to load updated progress".to_string()))
}

pub async fn get_progress(
    db: &PgPool,
    user_id: &str,
    poem_id: u32,
) -> Result<Option<UserPoemProgress>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT poem_id, learned, read_count, quiz_correct_count, quiz_wrong_count, to_char(last_learned_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_learned_at
        FROM user_poem_progress
        WHERE user_id = $1 AND poem_id = $2
        "#,
    )
    .bind(user_id)
    .bind(poem_id as i32)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    row.map(row_to_progress).transpose()
}

pub async fn set_favorite(
    db: &PgPool,
    user_id: &str,
    poem_id: u32,
    favorite: bool,
) -> Result<(), AppError> {
    if favorite {
        sqlx::query(
            r#"
            INSERT INTO favorites (id, user_id, poem_id)
            VALUES ($1, $2, $3)
            ON CONFLICT(user_id, poem_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(poem_id as i32)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    } else {
        sqlx::query("DELETE FROM favorites WHERE user_id = $1 AND poem_id = $2")
            .bind(user_id)
            .bind(poem_id as i32)
            .execute(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    Ok(())
}

pub async fn list_favorite_poem_ids(db: &PgPool, user_id: &str) -> Result<Vec<u32>, AppError> {
    let rows =
        sqlx::query("SELECT poem_id FROM favorites WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let poem_id: i32 = row.get("poem_id");
            poem_id as u32
        })
        .collect())
}

fn row_to_user(row: sqlx::postgres::PgRow) -> User {
    User {
        id: row.get("id"),
        openid: row.get("openid"),
        unionid: row.get("unionid"),
        nickname: row.get("nickname"),
        avatar_url: row.get("avatar_url"),
        role: row.get("role"),
    }
}

fn row_to_progress(row: sqlx::postgres::PgRow) -> Result<UserPoemProgress, AppError> {
    let poem_id: i32 = row.get("poem_id");
    let learned: bool = row.get("learned");
    let read_count: i32 = row.get("read_count");
    let quiz_correct_count: i32 = row.get("quiz_correct_count");
    let quiz_wrong_count: i32 = row.get("quiz_wrong_count");

    Ok(UserPoemProgress {
        poem_id: poem_id as u32,
        learned,
        read_count: read_count as u32,
        quiz_correct_count: quiz_correct_count as u32,
        quiz_wrong_count: quiz_wrong_count as u32,
        last_learned_at: row.get("last_learned_at"),
    })
}
