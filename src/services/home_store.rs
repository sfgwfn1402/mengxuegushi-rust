use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{
        home::PopularRecitationItem,
        poem::{Poem, PoemListQuery},
        recitation::RecitationItem,
        theme::PoemTheme,
    },
    services::{poem_store, recitation_store},
};

pub async fn today_unlearned_poem(db: &PgPool, user_id: &str) -> Result<Option<Poem>, AppError> {
    let poem_id = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT p.id
        FROM poems p
        LEFT JOIN user_poem_progress upp
            ON upp.poem_id = p.id AND upp.user_id = $1
        WHERE COALESCE(upp.learned, FALSE) = FALSE
        ORDER BY p.id ASC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let poem_id = match poem_id {
        Some(id) => id as u32,
        None => sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM poems
            ORDER BY id ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .map(|id| id as u32)
        .unwrap_or(0),
    };

    if poem_id == 0 {
        return Ok(None);
    }

    poem_store::find_poem(db, poem_id).await
}

pub async fn continue_learning_poem(db: &PgPool, user_id: &str) -> Result<Option<Poem>, AppError> {
    let poem_id = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT poem_id
        FROM user_poem_progress
        WHERE user_id = $1 AND last_learned_at IS NOT NULL
        ORDER BY last_learned_at DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    match poem_id {
        Some(id) => poem_store::find_poem(db, id as u32).await,
        None => Ok(None),
    }
}

pub async fn recommend_poems(
    db: &PgPool,
    user_id: Option<&str>,
    limit: u32,
) -> Result<(String, Option<PoemTheme>, Vec<Poem>), AppError> {
    let theme = if let Some(user_id) = user_id {
        favorite_theme_for_user(db, user_id).await?
    } else {
        None
    };

    let (reason, query) = if let Some(theme) = &theme {
        (
            format!("因为你最近喜欢「{}」", theme.name),
            PoemListQuery {
                theme: Some(theme.id.clone()),
                page: Some(1),
                page_size: Some(limit),
                level: None,
                difficulty: None,
                season: None,
                tag: None,
                keyword: None,
            },
        )
    } else {
        (
            "先从这些适合启蒙的古诗开始吧".to_string(),
            PoemListQuery {
                level: Some(1),
                page: Some(1),
                page_size: Some(limit),
                difficulty: None,
                season: None,
                tag: None,
                theme: None,
                keyword: None,
            },
        )
    };

    let (_, mut items) = poem_store::list_poems(db, &query).await?;

    if let Some(user_id) = user_id {
        let learned_ids = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT poem_id FROM user_poem_progress
            WHERE user_id = $1 AND learned = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
        let learned_ids: std::collections::HashSet<u32> =
            learned_ids.into_iter().map(|id| id as u32).collect();
        items.retain(|poem| !learned_ids.contains(&poem.id));
    }

    if items.len() < limit as usize {
        let (_, fallback) = poem_store::list_poems(
            db,
            &PoemListQuery {
                level: Some(1),
                page: Some(1),
                page_size: Some(limit),
                difficulty: None,
                season: None,
                tag: None,
                theme: None,
                keyword: None,
            },
        )
        .await?;
        for poem in fallback {
            if items.len() >= limit as usize {
                break;
            }
            if !items.iter().any(|item| item.id == poem.id) {
                items.push(poem);
            }
        }
    }

    items.truncate(limit as usize);
    Ok((reason, theme, items))
}

async fn favorite_theme_for_user(
    db: &PgPool,
    user_id: &str,
) -> Result<Option<PoemTheme>, AppError> {
    let item = sqlx::query_as::<_, PoemTheme>(
        r#"
        WITH recent_poems AS (
            SELECT poem_id, updated_at AS happened_at FROM user_poem_progress
            WHERE user_id = $1 AND updated_at >= CURRENT_TIMESTAMP - INTERVAL '2 days'
            UNION ALL
            SELECT poem_id, created_at AS happened_at FROM favorites
            WHERE user_id = $1 AND created_at >= CURRENT_TIMESTAMP - INTERVAL '2 days'
            UNION ALL
            SELECT poem_id, created_at AS happened_at FROM user_recitations
            WHERE user_id = $1 AND created_at >= CURRENT_TIMESTAMP - INTERVAL '2 days'
        ), theme_scores AS (
            SELECT ptr.theme_id, COUNT(*) AS score
            FROM recent_poems rp
            JOIN poem_theme_relations ptr ON ptr.poem_id = rp.poem_id
            GROUP BY ptr.theme_id
            ORDER BY score DESC, MAX(rp.happened_at) DESC
            LIMIT 1
        )
        SELECT
            t.id,
            t.name,
            t.emoji,
            t.description,
            COUNT(r.poem_id)::BIGINT AS poem_count
        FROM theme_scores ts
        JOIN poem_themes t ON t.id = ts.theme_id
        LEFT JOIN poem_theme_relations r ON r.theme_id = t.id
        WHERE t.enabled = TRUE
        GROUP BY t.id, t.name, t.emoji, t.description, t.sort_order, ts.score
        ORDER BY ts.score DESC, t.sort_order ASC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(item)
}

pub async fn popular_recitations(
    db: &PgPool,
    current_user_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PopularRecitationItem>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT r.id, p.title, p.author, p.dynasty
        FROM user_recitations r
        JOIN poems p ON p.id = r.poem_id
        WHERE r.status = 'public'
        ORDER BY r.like_count DESC, r.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut items = Vec::new();
    for (recitation_id, poem_title, poem_author, poem_dynasty) in rows {
        let recitation: RecitationItem =
            recitation_store::get_recitation(db, &recitation_id, current_user_id).await?;
        items.push(PopularRecitationItem {
            recitation,
            poem_title,
            poem_author,
            poem_dynasty,
        });
    }

    Ok(items)
}
