use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use sqlx::Row;

use crate::{error::AppError, routes::me::current_user, AppState};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

// 各类互动消息的数量，用于顶部入口的小红点/计数
pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    // 徽章显示“未读数”：只数各分类上次已读时间之后新增的互动。
    let row = sqlx::query(
        r#"
        WITH marks AS (SELECT kind, read_at FROM message_read_marks WHERE user_id = $1)
        SELECT
          ((SELECT COUNT(*) FROM moment_likes ml JOIN moments m ON m.id = ml.moment_id
              WHERE m.user_id = $1 AND ml.user_id <> $1
                AND ml.created_at > COALESCE((SELECT read_at FROM marks WHERE kind = 'like'), 'epoch'))
          +(SELECT COUNT(*) FROM poem_artwork_likes al JOIN poem_artworks a ON a.id = al.artwork_id
              WHERE a.user_id = $1 AND al.user_id <> $1
                AND al.created_at > COALESCE((SELECT read_at FROM marks WHERE kind = 'like'), 'epoch'))
          +(SELECT COUNT(*) FROM user_recitation_likes rl JOIN user_recitations r ON r.id = rl.recitation_id
              WHERE r.user_id = $1 AND rl.user_id <> $1
                AND rl.created_at > COALESCE((SELECT read_at FROM marks WHERE kind = 'like'), 'epoch')))::BIGINT AS like_count,
          (SELECT COUNT(*) FROM user_follows
              WHERE followee_id = $1
                AND created_at > COALESCE((SELECT read_at FROM marks WHERE kind = 'follow'), 'epoch'))::BIGINT AS follow_count,
          (SELECT COUNT(*) FROM moment_comments c JOIN moments m ON m.id = c.moment_id
              WHERE m.user_id = $1 AND c.user_id <> $1 AND c.status = 'public'
                AND c.created_at > COALESCE((SELECT read_at FROM marks WHERE kind = 'comment'), 'epoch'))::BIGINT AS comment_count
        "#,
    )
    .bind(&user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(serde_json::json!({
        "like_count": row.get::<i64, _>("like_count"),
        "follow_count": row.get::<i64, _>("follow_count"),
        "comment_count": row.get::<i64, _>("comment_count"),
    })))
}

/// 标记某类互动消息为已读（点进列表时调用），把该分类的徽章清零。
pub async fn mark_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    let kind = q.kind.as_deref().unwrap_or("");
    if !["like", "follow", "comment"].contains(&kind) {
        return Err(AppError::BadRequest("invalid kind".to_string()));
    }
    sqlx::query(
        r#"
        INSERT INTO message_read_marks (user_id, kind, read_at) VALUES ($1, $2, now())
        ON CONFLICT (user_id, kind) DO UPDATE SET read_at = now()
        "#,
    )
    .bind(&user.id)
    .bind(kind)
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    let kind = q.kind.as_deref().unwrap_or("like");
    let items = match kind {
        "follow" => follow_list(&state, &user.id).await?,
        "comment" => comment_list(&state, &user.id).await?,
        _ => like_list(&state, &user.id).await?,
    };
    Ok(Json(serde_json::json!({ "items": items })))
}

// 我赞过的内容（动态/诗配画/朗诵）
pub async fn my_likes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT * FROM (
          SELECT 'moment' AS kind, m.id AS id, LEFT(m.content, 30) AS title,
                 ('/api/moments/' || m.id || '/image/0') AS image, mu.nickname AS author,
                 mu.avatar_url AS author_avatar, m.like_count AS like_count, ml.created_at
          FROM moment_likes ml JOIN moments m ON m.id = ml.moment_id JOIN users mu ON mu.id = m.user_id
          WHERE ml.user_id = $1 AND m.status = 'public'
          UNION ALL
          SELECT 'artwork', a.id, p.title, a.image_url, au.nickname, au.avatar_url, a.like_count, al.created_at
          FROM poem_artwork_likes al JOIN poem_artworks a ON a.id = al.artwork_id
          JOIN poems p ON p.id = a.poem_id JOIN users au ON au.id = a.user_id
          WHERE al.user_id = $1 AND a.status = 'public'
          UNION ALL
          SELECT 'recitation', r.id, p.title, ('/images/poem-' || p.id || '.jpg'), ru.nickname, ru.avatar_url, r.like_count, rl.created_at
          FROM user_recitation_likes rl JOIN user_recitations r ON r.id = rl.recitation_id
          JOIN poems p ON p.id = r.poem_id JOIN users ru ON ru.id = r.user_id
          WHERE rl.user_id = $1 AND r.status = 'public'
        ) t ORDER BY created_at DESC LIMIT 100
        "#,
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let ts: chrono::DateTime<chrono::Utc> = r.get("created_at");
            serde_json::json!({
                "kind": r.get::<String, _>("kind"),
                "id": r.get::<String, _>("id"),
                "title": r.get::<Option<String>, _>("title"),
                "image": r.get::<Option<String>, _>("image"),
                "author": r.get::<Option<String>, _>("author"),
                "author_avatar": r.get::<Option<String>, _>("author_avatar"),
                "like_count": r.get::<i32, _>("like_count"),
                "created_at": ts.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "items": items })))
}

// 我评论过的（带所在动态）
pub async fn my_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT c.id, c.content, c.created_at, c.moment_id,
               mu.nickname AS moment_author, LEFT(m.content, 30) AS moment_text
        FROM moment_comments c
        JOIN moments m ON m.id = c.moment_id
        JOIN users mu ON mu.id = m.user_id
        WHERE c.user_id = $1 AND c.status = 'public'
        ORDER BY c.created_at DESC LIMIT 100
        "#,
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let ts: chrono::DateTime<chrono::Utc> = r.get("created_at");
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "content": r.get::<String, _>("content"),
                "moment_id": r.get::<String, _>("moment_id"),
                "moment_author": r.get::<Option<String>, _>("moment_author"),
                "moment_text": r.get::<Option<String>, _>("moment_text"),
                "created_at": ts.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "items": items })))
}

async fn follow_list(state: &AppState, uid: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT u.id AS user_id, u.nickname, u.avatar_url, f.created_at,
               EXISTS(SELECT 1 FROM user_follows ff WHERE ff.follower_id = $1 AND ff.followee_id = u.id) AS followed_by_me
        FROM user_follows f JOIN users u ON u.id = f.follower_id
        WHERE f.followee_id = $1
        ORDER BY f.created_at DESC LIMIT 100
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let ts: chrono::DateTime<chrono::Utc> = r.get("created_at");
            serde_json::json!({
                "kind": "follow",
                "user_id": r.get::<String, _>("user_id"),
                "nickname": r.get::<Option<String>, _>("nickname"),
                "avatar_url": r.get::<Option<String>, _>("avatar_url"),
                "followed_by_me": r.get::<bool, _>("followed_by_me"),
                "created_at": ts.to_rfc3339(),
            })
        })
        .collect())
}

async fn comment_list(state: &AppState, uid: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT c.id, c.content, c.created_at, c.moment_id,
               u.id AS user_id, u.nickname, u.avatar_url,
               LEFT(m.content, 30) AS target,
               EXISTS(SELECT 1 FROM user_follows ff WHERE ff.follower_id = c.user_id AND ff.followee_id = $1) AS is_follower,
               CASE WHEN COALESCE(jsonb_array_length(m.object_paths), 0) > 0
                    THEN '/api/moments/' || m.id || '/image/0' ELSE NULL END AS thumb
        FROM moment_comments c
        JOIN users u ON u.id = c.user_id
        JOIN moments m ON m.id = c.moment_id
        WHERE m.user_id = $1 AND c.user_id <> $1 AND c.status = 'public'
        ORDER BY c.created_at DESC LIMIT 100
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let ts: chrono::DateTime<chrono::Utc> = r.get("created_at");
            serde_json::json!({
                "kind": "comment",
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "nickname": r.get::<Option<String>, _>("nickname"),
                "avatar_url": r.get::<Option<String>, _>("avatar_url"),
                "content": r.get::<String, _>("content"),
                "moment_id": r.get::<String, _>("moment_id"),
                "target": r.get::<Option<String>, _>("target"),
                "is_follower": r.get::<bool, _>("is_follower"),
                "thumb": r.get::<Option<String>, _>("thumb"),
                "created_at": ts.to_rfc3339(),
            })
        })
        .collect())
}

async fn like_list(state: &AppState, uid: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT * FROM (
          SELECT u.nickname, u.avatar_url, ml.user_id, 'moment' AS target_kind, m.id AS target_id, LEFT(m.content, 24) AS target,
                 CASE WHEN COALESCE(jsonb_array_length(m.object_paths), 0) > 0 THEN '/api/moments/' || m.id || '/image/0' ELSE NULL END AS thumb,
                 EXISTS(SELECT 1 FROM user_follows ff WHERE ff.follower_id = ml.user_id AND ff.followee_id = $1) AS is_follower,
                 ml.created_at
          FROM moment_likes ml JOIN moments m ON m.id = ml.moment_id JOIN users u ON u.id = ml.user_id
          WHERE m.user_id = $1 AND ml.user_id <> $1
          UNION ALL
          SELECT u.nickname, u.avatar_url, al.user_id, 'artwork', a.id, a.title,
                 a.image_url,
                 EXISTS(SELECT 1 FROM user_follows ff WHERE ff.follower_id = al.user_id AND ff.followee_id = $1),
                 al.created_at
          FROM poem_artwork_likes al JOIN poem_artworks a ON a.id = al.artwork_id JOIN users u ON u.id = al.user_id
          WHERE a.user_id = $1 AND al.user_id <> $1
          UNION ALL
          SELECT u.nickname, u.avatar_url, rl.user_id, 'recitation', r.id, p.title,
                 '/images/poem-' || p.id || '.jpg',
                 EXISTS(SELECT 1 FROM user_follows ff WHERE ff.follower_id = rl.user_id AND ff.followee_id = $1),
                 rl.created_at
          FROM user_recitation_likes rl JOIN user_recitations r ON r.id = rl.recitation_id JOIN poems p ON p.id = r.poem_id JOIN users u ON u.id = rl.user_id
          WHERE r.user_id = $1 AND rl.user_id <> $1
        ) t
        ORDER BY created_at DESC LIMIT 100
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let ts: chrono::DateTime<chrono::Utc> = r.get("created_at");
            serde_json::json!({
                "kind": "like",
                "target_kind": r.get::<String, _>("target_kind"),
                "user_id": r.get::<String, _>("user_id"),
                "nickname": r.get::<Option<String>, _>("nickname"),
                "avatar_url": r.get::<Option<String>, _>("avatar_url"),
                "target_id": r.get::<String, _>("target_id"),
                "target": r.get::<Option<String>, _>("target"),
                "is_follower": r.get::<bool, _>("is_follower"),
                "thumb": r.get::<Option<String>, _>("thumb"),
                "created_at": ts.to_rfc3339(),
            })
        })
        .collect())
}
