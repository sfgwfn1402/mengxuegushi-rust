use sqlx::{PgPool, Row};

use crate::{
    error::AppError,
    models::{
        moment::{DeleteMomentResponse, MomentComment, MomentItem, UserProfile},
        recitation::LikeResponse,
    },
};

fn row_to_moment(row: sqlx::postgres::PgRow) -> Result<MomentItem, AppError> {
    let id: String = row.get("id");
    let paths: serde_json::Value = row.try_get("object_paths").unwrap_or(serde_json::json!([]));
    let object_paths: Vec<String> = paths
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let n = object_paths.len();
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
        object_paths,
        like_count: row.get("like_count"),
        liked_by_me: row.get("liked_by_me"),
        comment_count: row.try_get("comment_count").unwrap_or(0),
        followed_by_me: row.try_get("followed_by_me").unwrap_or(false),
        location: row.try_get("location").unwrap_or(None),
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
    location: Option<&str>,
) -> Result<MomentItem, AppError> {
    let first = object_paths.first().cloned().unwrap_or_default();
    let paths_json = serde_json::json!(object_paths);
    sqlx::query(
        r#"
        INSERT INTO moments (id, user_id, content, image_url, object_path, object_paths, location, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'submitted')
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(content)
    .bind(format!("/api/moments/{id}/image/0"))
    .bind(&first)
    .bind(&paths_json)
    .bind(location)
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
               m.like_count, m.comment_count, m.location, m.status, m.created_at,
               EXISTS(SELECT 1 FROM moment_likes l WHERE l.moment_id = m.id AND l.user_id = $1) AS liked_by_me,
               EXISTS(SELECT 1 FROM user_follows f WHERE f.follower_id = $1 AND f.followee_id = m.user_id) AS followed_by_me
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
               m.like_count, m.comment_count, m.location, m.status, m.created_at, FALSE AS liked_by_me, FALSE AS followed_by_me
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
               m.like_count, m.comment_count, m.location, m.status, m.created_at,
               EXISTS(SELECT 1 FROM moment_likes l WHERE l.moment_id = m.id AND l.user_id = $2) AS liked_by_me,
               EXISTS(SELECT 1 FROM user_follows f WHERE f.follower_id = $2 AND f.followee_id = m.user_id) AS followed_by_me
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


// 编辑动态：仅限本人、且状态为 驳回/审核中/已公开。更新后重新进入审核(submitted)。
// 返回 (更新后动态, 被移除的旧对象键)，供路由删除 MinIO 文件。
pub async fn update_moment(
    db: &PgPool,
    moment_id: &str,
    user_id: &str,
    content: &str,
    object_paths: &[String],
) -> Result<(MomentItem, Vec<String>), AppError> {
    let old = owned_object_paths(db, moment_id, user_id).await?;
    let first = object_paths.first().cloned().unwrap_or_default();
    let paths_json = serde_json::json!(object_paths);
    let affected = sqlx::query(
        r#"
        UPDATE moments
        SET content = $3, image_url = $4, object_path = $5, object_paths = $6, status = 'submitted'
        WHERE id = $1 AND user_id = $2 AND status IN ('rejected','submitted','public')
        "#,
    )
    .bind(moment_id)
    .bind(user_id)
    .bind(content)
    .bind(format!("/api/moments/{moment_id}/image/0"))
    .bind(&first)
    .bind(&paths_json)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::BadRequest("不可编辑（非本人或状态不允许）".to_string()));
    }
    // 被移除的旧图(old - new)
    let removed: Vec<String> = old
        .into_iter()
        .filter(|p| !object_paths.iter().any(|n| n == p))
        .collect();
    let item = get_moment(db, moment_id, Some(user_id)).await?;
    Ok((item, removed))
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
                       m.like_count, m.comment_count, m.location, m.status, m.created_at, FALSE AS liked_by_me, FALSE AS followed_by_me
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
                       m.like_count, m.comment_count, m.location, m.status, m.created_at, FALSE AS liked_by_me, FALSE AS followed_by_me
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

fn row_to_comment(row: sqlx::postgres::PgRow) -> Result<MomentComment, AppError> {
    Ok(MomentComment {
        id: row.get("id"),
        moment_id: row.get("moment_id"),
        user_id: row.get("user_id"),
        nickname: row.get("nickname"),
        avatar_url: row.get("avatar_url"),
        parent_id: row.get("parent_id"),
        reply_to_nickname: row.get("reply_to_nickname"),
        content: row.get("content"),
        created_at: row.get("created_at"),
    })
}

// 该动态是否可评论（公开，或本人审核中的）
async fn moment_commentable(db: &PgPool, moment_id: &str) -> Result<bool, AppError> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT status FROM moments WHERE id = $1 AND status IN ('public','submitted')")
            .bind(moment_id)
            .fetch_optional(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(exists.is_some())
}

pub async fn list_comments(
    db: &PgPool,
    moment_id: &str,
) -> Result<Vec<MomentComment>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT c.id, c.moment_id, c.user_id, u.nickname, u.avatar_url,
               c.parent_id, c.reply_to_nickname, c.content, c.created_at
        FROM moment_comments c
        JOIN users u ON u.id = c.user_id
        WHERE c.moment_id = $1 AND c.status = 'public'
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(moment_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    rows.into_iter().map(row_to_comment).collect()
}

pub async fn create_comment(
    db: &PgPool,
    id: &str,
    moment_id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    reply_to_nickname: Option<&str>,
    content: &str,
) -> Result<MomentComment, AppError> {
    if !moment_commentable(db, moment_id).await? {
        return Err(AppError::NotFound(format!("moment {moment_id}")));
    }
    // 回复必须挂在本动态已存在的顶层评论下，否则视为顶层评论
    let parent: Option<String> = match parent_id {
        Some(pid) if !pid.is_empty() => {
            sqlx::query_scalar(
                "SELECT id FROM moment_comments WHERE id = $1 AND moment_id = $2 AND parent_id IS NULL AND status = 'public'",
            )
            .bind(pid)
            .bind(moment_id)
            .fetch_optional(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
        }
        _ => None,
    };

    sqlx::query(
        r#"
        INSERT INTO moment_comments (id, moment_id, user_id, parent_id, reply_to_nickname, content, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'public')
        "#,
    )
    .bind(id)
    .bind(moment_id)
    .bind(user_id)
    .bind(&parent)
    .bind(reply_to_nickname.filter(|s| !s.is_empty()))
    .bind(content)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    sqlx::query("UPDATE moments SET comment_count = comment_count + 1 WHERE id = $1")
        .bind(moment_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let row = sqlx::query(
        r#"
        SELECT c.id, c.moment_id, c.user_id, u.nickname, u.avatar_url,
               c.parent_id, c.reply_to_nickname, c.content, c.created_at
        FROM moment_comments c JOIN users u ON u.id = c.user_id
        WHERE c.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    row_to_comment(row)
}

// 用户公开主页资料（昵称/头像 + 关注·粉丝·作品数 + 我是否已关注）
pub async fn get_user_profile(
    db: &PgPool,
    user_id: &str,
    current_user_id: Option<&str>,
) -> Result<UserProfile, AppError> {
    let row = sqlx::query(
        r#"
        SELECT u.nickname, u.avatar_url,
               (SELECT COUNT(*) FROM user_follows WHERE follower_id = $1) AS following_count,
               (SELECT COUNT(*) FROM user_follows WHERE followee_id = $1) AS follower_count,
               (SELECT COUNT(*) FROM moments WHERE user_id = $1 AND status = 'public') AS moment_count,
               EXISTS(SELECT 1 FROM user_follows WHERE follower_id = $2 AND followee_id = $1) AS followed_by_me
        FROM users u WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .bind(current_user_id.unwrap_or(""))
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;

    Ok(UserProfile {
        user_id: user_id.to_string(),
        nickname: row.get("nickname"),
        avatar_url: row.get("avatar_url"),
        following_count: row.get("following_count"),
        follower_count: row.get("follower_count"),
        moment_count: row.get("moment_count"),
        followed_by_me: row.get("followed_by_me"),
    })
}

// 某用户的公开动态
pub async fn list_user_moments(
    db: &PgPool,
    target_user_id: &str,
    current_user_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MomentItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT m.id, m.user_id, u.nickname, u.avatar_url, m.content, m.image_url, m.object_paths,
               m.like_count, m.comment_count, m.location, m.status, m.created_at,
               EXISTS(SELECT 1 FROM moment_likes l WHERE l.moment_id = m.id AND l.user_id = $2) AS liked_by_me,
               EXISTS(SELECT 1 FROM user_follows f WHERE f.follower_id = $2 AND f.followee_id = m.user_id) AS followed_by_me
        FROM moments m
        JOIN users u ON u.id = m.user_id
        WHERE m.user_id = $1 AND m.status = 'public'
        ORDER BY m.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(target_user_id)
    .bind(current_user_id.unwrap_or(""))
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    rows.into_iter().map(row_to_moment).collect()
}

// 关注 / 取关，返回最新关注状态
pub async fn follow_user(
    db: &PgPool,
    follower_id: &str,
    followee_id: &str,
) -> Result<bool, AppError> {
    if follower_id == followee_id {
        return Err(AppError::BadRequest("不能关注自己".to_string()));
    }
    sqlx::query(
        "INSERT INTO user_follows (follower_id, followee_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(follower_id)
    .bind(followee_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(true)
}

pub async fn unfollow_user(
    db: &PgPool,
    follower_id: &str,
    followee_id: &str,
) -> Result<bool, AppError> {
    sqlx::query("DELETE FROM user_follows WHERE follower_id = $1 AND followee_id = $2")
        .bind(follower_id)
        .bind(followee_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(false)
}

// 删除自己的评论：顶层评论连同其楼中楼一并删除
pub async fn delete_comment(
    db: &PgPool,
    moment_id: &str,
    comment_id: &str,
    user_id: &str,
) -> Result<DeleteMomentResponse, AppError> {
    let affected = sqlx::query(
        r#"
        UPDATE moment_comments SET status = 'deleted'
        WHERE moment_id = $2 AND user_id = $3 AND status = 'public'
          AND (id = $1 OR parent_id = $1)
        "#,
    )
    .bind(comment_id)
    .bind(moment_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if affected > 0 {
        sqlx::query("UPDATE moments SET comment_count = GREATEST(comment_count - $2, 0) WHERE id = $1")
            .bind(moment_id)
            .bind(affected as i32)
            .execute(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }
    Ok(DeleteMomentResponse { deleted: affected > 0 })
}
