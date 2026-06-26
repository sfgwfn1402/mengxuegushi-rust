use sqlx::{PgPool, Row};

use crate::{
    error::AppError,
    models::event::{
        AnalyticsResponse, DailyActive, EventCount, TopPoem, TrackEventInput,
    },
};

const MAX_BATCH: usize = 50;

pub async fn insert_events(
    db: &PgPool,
    user_id: Option<&str>,
    events: &[TrackEventInput],
) -> Result<u64, AppError> {
    let mut inserted = 0u64;
    for ev in events.iter().take(MAX_BATCH) {
        let name = ev.event.trim();
        if name.is_empty() || name.len() > 64 {
            continue;
        }
        sqlx::query(
            "INSERT INTO events (event_name, user_id, page, props) VALUES ($1, $2, $3, $4)",
        )
        .bind(name)
        .bind(user_id)
        .bind(ev.page.as_deref())
        .bind(ev.props.clone())
        .execute(db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
        inserted += 1;
    }
    Ok(inserted)
}

pub async fn analytics(db: &PgPool, range_days: i64) -> Result<AnalyticsResponse, AppError> {
    let days = range_days.clamp(1, 90);
    let since = format!("{} days", days);

    // 总事件数 & 活跃用户数
    let totals = sqlx::query(
        "SELECT COUNT(*) AS total, COUNT(DISTINCT user_id) AS users
         FROM events WHERE created_at >= now() - $1::interval",
    )
    .bind(&since)
    .fetch_one(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let total_events: i64 = totals.get("total");
    let active_users: i64 = totals.get("users");

    // 各事件计数
    let rows = sqlx::query(
        "SELECT event_name, COUNT(*) AS cnt
         FROM events WHERE created_at >= now() - $1::interval
         GROUP BY event_name ORDER BY cnt DESC",
    )
    .bind(&since)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let event_counts = rows
        .into_iter()
        .map(|r| EventCount {
            event_name: r.get("event_name"),
            count: r.get("cnt"),
        })
        .collect();

    // 每日活跃
    let rows = sqlx::query(
        "SELECT to_char(date_trunc('day', created_at), 'MM-DD') AS day,
                COUNT(DISTINCT user_id) AS users, COUNT(*) AS events
         FROM events WHERE created_at >= now() - $1::interval
         GROUP BY date_trunc('day', created_at)
         ORDER BY date_trunc('day', created_at)",
    )
    .bind(&since)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let daily_active = rows
        .into_iter()
        .map(|r| DailyActive {
            day: r.get("day"),
            users: r.get("users"),
            events: r.get("events"),
        })
        .collect();

    // 最受欢迎的诗（学习/跟读/背诵/朗诵事件里的 poem_id）
    let rows = sqlx::query(
        "SELECT e.props->>'poem_id' AS poem_id, p.title AS title, COUNT(*) AS cnt
         FROM events e
         LEFT JOIN poems p ON e.props->>'poem_id' ~ '^[0-9]+$' AND p.id = (e.props->>'poem_id')::int
         WHERE e.created_at >= now() - $1::interval
           AND e.event_name IN ('poem_learn','poem_follow','poem_recite','poem_open')
           AND e.props->>'poem_id' IS NOT NULL
         GROUP BY e.props->>'poem_id', p.title
         ORDER BY cnt DESC LIMIT 10",
    )
    .bind(&since)
    .fetch_all(db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let top_poems = rows
        .into_iter()
        .map(|r| TopPoem {
            poem_id: r.get::<Option<String>, _>("poem_id").unwrap_or_default(),
            title: r.get("title"),
            count: r.get("cnt"),
        })
        .collect();

    Ok(AnalyticsResponse {
        range_days: days,
        total_events,
        active_users,
        event_counts,
        daily_active,
        top_poems,
    })
}
