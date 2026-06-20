use sqlx::PgPool;

use crate::{error::AppError, models::theme::PoemTheme};

pub async fn list_themes(db: &PgPool) -> Result<Vec<PoemTheme>, AppError> {
    let items = sqlx::query_as::<_, PoemTheme>(
        r#"
        SELECT
            t.id,
            t.name,
            t.emoji,
            t.description,
            COUNT(r.poem_id)::BIGINT AS poem_count
        FROM poem_themes t
        LEFT JOIN poem_theme_relations r ON r.theme_id = t.id
        WHERE t.enabled = TRUE
        GROUP BY t.id, t.name, t.emoji, t.description, t.sort_order
        HAVING COUNT(r.poem_id) > 0
        ORDER BY t.sort_order ASC, t.id ASC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(items)
}
