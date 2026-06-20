use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::{
    error::AppError,
    models::poem::{Poem, PoemListQuery, PoemThemeTag},
};

const POEMS_SEED_JSON: &str = include_str!("../../data/poems.seed.json");

pub async fn list_poems(
    db: &PgPool,
    query: &PoemListQuery,
) -> Result<(usize, Vec<Poem>), AppError> {
    let total = count_poems(db, query).await?;
    let rows = build_list_query(query)
        .build()
        .fetch_all(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let items = rows
        .into_iter()
        .map(row_to_poem)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((total, items))
}

pub async fn find_poem(db: &PgPool, id: u32) -> Result<Option<Poem>, AppError> {
    let row = sqlx::query(select_poem_sql!("WHERE id = $1"))
        .bind(id as i32)
        .fetch_optional(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    row.map(row_to_poem).transpose()
}

pub async fn seed_default_poems(
    db: &PgPool,
    public_base_url: Option<&str>,
) -> Result<(), AppError> {
    let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM poems")
        .fetch_one(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    // 数据库是主数据源。seed 只负责首次初始化空库，不能在每次启动时覆盖运营/人工修改。
    if existing_count > 0 {
        tracing::info!(
            existing_count,
            "skip poem seed because database already has poems"
        );
        return Ok(());
    }

    let poems: Vec<Poem> = serde_json::from_str(POEMS_SEED_JSON)
        .map_err(|err| AppError::Internal(format!("failed to parse poem seed: {err}")))?;

    for mut poem in poems {
        poem.audio_url = normalize_audio_url(poem.audio_url, public_base_url);
        poem.image_url = normalize_image_url(poem.image_url, public_base_url);
        insert_poem(db, poem).await?;
    }

    Ok(())
}

fn normalize_audio_url(audio_url: Option<String>, public_base_url: Option<&str>) -> Option<String> {
    let audio_url = audio_url?;
    let base = public_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if audio_url.starts_with("http://") || audio_url.starts_with("https://") {
        return Some(audio_url);
    }

    Some(format!(
        "{}/{}",
        base.trim_end_matches('/'),
        audio_url.trim_start_matches('/')
    ))
}

fn normalize_image_url(image_url: Option<String>, public_base_url: Option<&str>) -> Option<String> {
    let image_url = image_url?;
    let base = public_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if image_url.starts_with("http://") || image_url.starts_with("https://") {
        return Some(image_url);
    }

    Some(format!(
        "{}/{}",
        base.trim_end_matches('/'),
        image_url.trim_start_matches('/')
    ))
}

async fn count_poems(db: &PgPool, query: &PoemListQuery) -> Result<usize, AppError> {
    let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM poems");
    push_filters(&mut builder, query);

    let total: i64 = builder
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(total as usize)
}

fn build_list_query(query: &PoemListQuery) -> QueryBuilder<'_, Postgres> {
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            id, title, author, dynasty, content_json, pinyin, translation, story, parent_guide,
            difficulty, level, tags_json, season, audio_url, audio_version, image_url, video_available, card_unlocked,
            annotated_content_json,
            COALESCE((
                SELECT json_agg(json_build_object('id', t.id, 'name', t.name, 'emoji', t.emoji) ORDER BY t.sort_order)
                FROM poem_theme_relations ptr
                JOIN poem_themes t ON t.id = ptr.theme_id
                WHERE ptr.poem_id = poems.id AND t.enabled = TRUE
            )::text, '[]') AS themes_json,
            (
                SELECT lines_json::text
                FROM poem_follow_timings pft
                WHERE pft.poem_id = poems.id AND pft.status = 'active'
            ) AS follow_timings_json
        FROM poems
        "#,
    );

    push_filters(&mut builder, query);
    builder.push(" ORDER BY id ASC LIMIT ");
    builder.push_bind(query.page_size() as i64);
    builder.push(" OFFSET ");
    builder.push_bind(((query.page() - 1) * query.page_size()) as i64);

    builder
}

fn push_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &PoemListQuery) {
    let has_filter = query.level.is_some()
        || query.difficulty.is_some()
        || normalized_opt(&query.season).is_some()
        || normalized_opt(&query.tag).is_some()
        || normalized_opt(&query.theme).is_some()
        || normalized_opt(&query.keyword).is_some();

    if !has_filter {
        return;
    }

    builder.push(" WHERE ");
    let mut separated = builder.separated(" AND ");

    if let Some(level) = query.level {
        separated
            .push("level = ")
            .push_bind_unseparated(level as i64);
    }

    if let Some(difficulty) = query.difficulty {
        separated
            .push("difficulty = ")
            .push_bind_unseparated(difficulty as i64);
    }

    if let Some(season) = normalized_opt(&query.season) {
        separated.push("season = ").push_bind_unseparated(season);
    }

    if let Some(tag) = normalized_opt(&query.tag) {
        separated
            .push("tags_json LIKE ")
            .push_bind_unseparated(format!("%\"{}\"%", escape_like(&tag)));
    }

    if let Some(theme) = normalized_opt(&query.theme) {
        separated.push("EXISTS (SELECT 1 FROM poem_theme_relations ptr WHERE ptr.poem_id = poems.id AND ptr.theme_id = ");
        separated.push_bind_unseparated(theme);
        separated.push_unseparated(")");
    }

    if let Some(keyword) = normalized_opt(&query.keyword) {
        let keyword = format!("%{}%", escape_like(&keyword));
        separated.push("(");
        separated.push_unseparated("title LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR author LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR dynasty LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR content_json LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR translation LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR story LIKE ");
        separated.push_bind_unseparated(keyword.clone());
        separated.push_unseparated(" OR tags_json LIKE ");
        separated.push_bind_unseparated(keyword);
        separated.push_unseparated(")");
    }
}

async fn insert_poem(db: &PgPool, poem: Poem) -> Result<(), AppError> {
    let tags_json =
        serde_json::to_string(&poem.tags).map_err(|err| AppError::Internal(err.to_string()))?;
    let annotated_content_json = serde_json::to_string(&poem.annotated_content)
        .map_err(|err| AppError::Internal(err.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO poems (
            id, title, author, dynasty, content_json, pinyin, translation, story, parent_guide,
            difficulty, level, tags_json, season, audio_url, audio_version, image_url, video_available, card_unlocked,
            annotated_content_json
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        ON CONFLICT(id) DO NOTHING
        "#,
    )
    .bind(poem.id as i32)
    .bind(poem.title)
    .bind(poem.author)
    .bind(poem.dynasty)
    .bind(poem.content)
    .bind(poem.pinyin)
    .bind(poem.translation)
    .bind(poem.story)
    .bind(poem.parent_guide)
    .bind(poem.difficulty as i32)
    .bind(poem.level as i32)
    .bind(tags_json)
    .bind(poem.season)
    .bind(poem.audio_url)
    .bind(poem.audio_version)
    .bind(poem.image_url)
    .bind(poem.video_available)
    .bind(poem.card_unlocked)
    .bind(annotated_content_json)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}

fn row_to_poem(row: sqlx::postgres::PgRow) -> Result<Poem, AppError> {
    let tags_json: String = row.get("tags_json");
    let annotated_content_json: String = row.get("annotated_content_json");
    let id: i32 = row.get("id");
    let difficulty: i32 = row.get("difficulty");
    let level: i32 = row.get("level");
    let video_available: bool = row.get("video_available");
    let card_unlocked: bool = row.get("card_unlocked");

    let follow_timings_json: Option<String> = row.get("follow_timings_json");
    let follow_timings = follow_timings_json
        .map(|value| serde_json::from_str::<serde_json::Value>(&value))
        .transpose()
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Poem {
        id: id as u32,
        title: row.get("title"),
        author: row.get("author"),
        dynasty: row.get("dynasty"),
        content: row.get("content_json"),
        pinyin: row.get("pinyin"),
        translation: row.get("translation"),
        story: row.get("story"),
        parent_guide: row.get("parent_guide"),
        difficulty: difficulty as u8,
        level: level as u8,
        tags: serde_json::from_str(&tags_json)
            .map_err(|err| AppError::Internal(err.to_string()))?,
        season: row.get("season"),
        audio_url: row.get("audio_url"),
        audio_version: row.get("audio_version"),
        image_url: row.get("image_url"),
        video_available,
        card_unlocked,
        annotated_content: serde_json::from_str(&annotated_content_json)
            .map_err(|err| AppError::Internal(err.to_string()))?,
        themes: serde_json::from_str::<Vec<PoemThemeTag>>(&row.get::<String, _>("themes_json"))
            .map_err(|err| AppError::Internal(err.to_string()))?,
        follow_timings,
    })
}

fn normalized_opt(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

macro_rules! select_poem_sql {
    ($tail:literal) => {
        concat!(
            r#"
            SELECT
                id, title, author, dynasty, content_json, pinyin, translation, story, parent_guide,
                difficulty, level, tags_json, season, audio_url, audio_version, image_url, video_available, card_unlocked,
                annotated_content_json,
                COALESCE((
                    SELECT json_agg(json_build_object('id', t.id, 'name', t.name, 'emoji', t.emoji) ORDER BY t.sort_order)
                    FROM poem_theme_relations ptr
                    JOIN poem_themes t ON t.id = ptr.theme_id
                    WHERE ptr.poem_id = poems.id AND t.enabled = TRUE
                )::text, '[]') AS themes_json,
                (
                    SELECT lines_json::text
                    FROM poem_follow_timings pft
                    WHERE pft.poem_id = poems.id AND pft.status = 'active'
                ) AS follow_timings_json
            FROM poems
            "#,
            $tail
        )
    };
}

use select_poem_sql;
