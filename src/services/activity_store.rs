use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::activity::{
        CheckinResponse, CompleteTaskResponse, IdiomProgress, UpdateIdiomProgressRequest,
        UserStatsResponse,
    },
};

pub async fn get_stats(db: &PgPool, user_id: &str) -> Result<UserStatsResponse, AppError> {
    ensure_stats(db, user_id).await?;

    let row = sqlx::query(
        r#"
        SELECT
            s.stars,
            s.total_days,
            s.streak,
            COALESCE((SELECT COUNT(*) FROM user_poem_progress p WHERE p.user_id = $1 AND p.learned = TRUE), 0) AS learned_poem_count,
            COALESCE((SELECT COUNT(*) FROM user_idiom_progress i WHERE i.user_id = $1 AND i.learned = TRUE), 0) AS learned_idiom_count,
            EXISTS(SELECT 1 FROM user_checkins c WHERE c.user_id = $1 AND c.checkin_date = CURRENT_DATE) AS today_checked
        FROM user_stats s
        WHERE s.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let task_rows = sqlx::query(
        "SELECT task_id FROM user_daily_tasks WHERE user_id = $1 AND task_date = CURRENT_DATE ORDER BY task_id",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(UserStatsResponse {
        stars: get_i64(&row, "stars") as u32,
        total_days: get_i64(&row, "total_days") as u32,
        streak: get_i64(&row, "streak") as u32,
        learned_poem_count: get_i64(&row, "learned_poem_count") as u32,
        learned_idiom_count: get_i64(&row, "learned_idiom_count") as u32,
        today_checked: row.get("today_checked"),
        today_tasks_done: task_rows.into_iter().map(|r| r.get("task_id")).collect(),
    })
}

pub async fn checkin(db: &PgPool, user_id: &str) -> Result<CheckinResponse, AppError> {
    ensure_stats(db, user_id).await?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO user_checkins (id, user_id, checkin_date)
        VALUES ($1, $2, CURRENT_DATE)
        ON CONFLICT(user_id, checkin_date) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    if inserted > 0 {
        let streak = calculate_streak(db, user_id).await?;
        let total_days: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user_checkins WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(db)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

        sqlx::query(
            "UPDATE user_stats SET total_days = $1, streak = $2, updated_at = CURRENT_TIMESTAMP WHERE user_id = $3",
        )
        .bind(total_days as i32)
        .bind(streak as i32)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let stats = get_stats(db, user_id).await?;
    Ok(CheckinResponse {
        today_checked: stats.today_checked,
        total_days: stats.total_days,
        streak: stats.streak,
    })
}

pub async fn complete_task(
    db: &PgPool,
    user_id: &str,
    task_id: &str,
    stars: u32,
) -> Result<CompleteTaskResponse, AppError> {
    ensure_stats(db, user_id).await?;
    let stars = stars.min(100);

    let inserted = sqlx::query(
        r#"
        INSERT INTO user_daily_tasks (id, user_id, task_date, task_id, stars)
        VALUES ($1, $2, CURRENT_DATE, $3, $4)
        ON CONFLICT(user_id, task_date, task_id) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(task_id)
    .bind(stars as i32)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .rows_affected();

    let mut stars_added = 0;
    if inserted > 0 {
        stars_added = stars;
        sqlx::query("UPDATE user_stats SET stars = stars + $1, updated_at = CURRENT_TIMESTAMP WHERE user_id = $2")
            .bind(stars as i32)
            .bind(user_id)
            .execute(db)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
    }

    let total_stars: i32 = sqlx::query_scalar("SELECT stars FROM user_stats WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(CompleteTaskResponse {
        task_id: task_id.to_string(),
        stars_added,
        total_stars: total_stars as u32,
        completed: true,
    })
}

pub async fn list_idiom_progress(
    db: &PgPool,
    user_id: &str,
) -> Result<Vec<IdiomProgress>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT idiom_id, learned, read_count, quiz_correct_count, quiz_wrong_count,
               to_char(last_learned_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_learned_at
        FROM user_idiom_progress
        WHERE user_id = $1
        ORDER BY idiom_id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows.into_iter().map(row_to_idiom_progress).collect())
}

pub async fn update_idiom_progress(
    db: &PgPool,
    user_id: &str,
    payload: UpdateIdiomProgressRequest,
) -> Result<IdiomProgress, AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_idiom_progress (id, user_id, idiom_id)
        VALUES ($1, $2, $3)
        ON CONFLICT(user_id, idiom_id) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(payload.idiom_id as i32)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let learned = payload.learned;
    sqlx::query(
        r#"
        UPDATE user_idiom_progress
        SET
          learned = COALESCE($1, learned),
          read_count = read_count + $2,
          quiz_correct_count = quiz_correct_count + $3,
          quiz_wrong_count = quiz_wrong_count + $4,
          last_learned_at = CASE WHEN $5 = TRUE THEN CURRENT_TIMESTAMP ELSE last_learned_at END,
          updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $6 AND idiom_id = $7
        "#,
    )
    .bind(learned)
    .bind(payload.read_count_delta.unwrap_or(0) as i32)
    .bind(payload.quiz_correct_delta.unwrap_or(0) as i32)
    .bind(payload.quiz_wrong_delta.unwrap_or(0) as i32)
    .bind(learned)
    .bind(user_id)
    .bind(payload.idiom_id as i32)
    .execute(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let row = sqlx::query(
        r#"
        SELECT idiom_id, learned, read_count, quiz_correct_count, quiz_wrong_count,
               to_char(last_learned_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_learned_at
        FROM user_idiom_progress
        WHERE user_id = $1 AND idiom_id = $2
        "#,
    )
    .bind(user_id)
    .bind(payload.idiom_id as i32)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(row_to_idiom_progress(row))
}

pub async fn clear_user_data(db: &PgPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM favorites WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    sqlx::query("DELETE FROM user_poem_progress WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    sqlx::query("DELETE FROM user_idiom_progress WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    sqlx::query("DELETE FROM user_checkins WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    sqlx::query("DELETE FROM user_daily_tasks WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    sqlx::query("UPDATE user_stats SET stars = 0, total_days = 0, streak = 0, updated_at = CURRENT_TIMESTAMP WHERE user_id = $1").bind(user_id).execute(db).await.map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

async fn ensure_stats(db: &PgPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO user_stats (user_id) VALUES ($1) ON CONFLICT(user_id) DO NOTHING")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(())
}

async fn calculate_streak(db: &PgPool, user_id: &str) -> Result<u32, AppError> {
    let dates: Vec<String> = sqlx::query_scalar(
        "SELECT checkin_date::text FROM user_checkins WHERE user_id = $1 ORDER BY checkin_date DESC LIMIT 366",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if dates.is_empty() {
        return Ok(0);
    }

    // 简化：连续天数由 PostgreSQL 递归计算对当前需求过重；这里用日期字符串按天比较。
    let mut streak = 0u32;
    let mut expected = chrono::Utc::now().date_naive();
    for date in dates {
        let parsed = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|err| AppError::Internal(err.to_string()))?;
        if parsed == expected {
            streak += 1;
            expected = expected.pred_opt().unwrap_or(expected);
        } else if streak == 0 && parsed == expected.pred_opt().unwrap_or(expected) {
            expected = parsed;
            streak += 1;
            expected = expected.pred_opt().unwrap_or(expected);
        } else {
            break;
        }
    }
    Ok(streak)
}

fn row_to_idiom_progress(row: sqlx::postgres::PgRow) -> IdiomProgress {
    let idiom_id: i32 = row.get("idiom_id");
    let read_count: i32 = row.get("read_count");
    let quiz_correct_count: i32 = row.get("quiz_correct_count");
    let quiz_wrong_count: i32 = row.get("quiz_wrong_count");
    IdiomProgress {
        idiom_id: idiom_id as u32,
        learned: row.get("learned"),
        read_count: read_count as u32,
        quiz_correct_count: quiz_correct_count as u32,
        quiz_wrong_count: quiz_wrong_count as u32,
        last_learned_at: row.get("last_learned_at"),
    }
}

fn get_i64(row: &sqlx::postgres::PgRow, name: &str) -> i64 {
    row.try_get::<i64, _>(name)
        .or_else(|_| row.try_get::<i32, _>(name).map(|v| v as i64))
        .unwrap_or(0)
}
