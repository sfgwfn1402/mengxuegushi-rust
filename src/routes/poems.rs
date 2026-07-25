use axum::{extract::Path, extract::Query, extract::State, http::HeaderMap, Json};
use sqlx::Row;

use crate::{
    error::AppError,
    models::poem::{PoemListQuery, PoemListResponse},
    routes::me::current_user,
    services::poem_store,
    AppState,
};

pub async fn list_poems(
    State(state): State<AppState>,
    Query(query): Query<PoemListQuery>,
) -> Result<Json<PoemListResponse>, AppError> {
    let page = query.page();
    let page_size = query.page_size();
    let (total, items) = poem_store::list_poems(&state.db, &query).await?;

    Ok(Json(PoemListResponse {
        total,
        page,
        page_size,
        items,
    }))
}

pub async fn get_poem(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<crate::models::poem::Poem>, AppError> {
    poem_store::find_poem(&state.db, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("poem {id}")))
}

#[derive(serde::Deserialize)]
pub struct PanelPositionBody {
    pub x: f64,
    pub y: f64,
}

/// 全部诗句框位置（poem_id → {x,y}）。公开读，客户端进闯关时一次拉取。
pub async fn list_panel_positions(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = sqlx::query("SELECT poem_id, x, y FROM poem_panel_positions")
        .fetch_all(&state.db)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let mut map = serde_json::Map::new();
    for r in rows {
        let id: i32 = r.get("poem_id");
        let x: f64 = r.get("x");
        let y: f64 = r.get("y");
        map.insert(id.to_string(), serde_json::json!({ "x": x, "y": y }));
    }
    Ok(Json(serde_json::json!({ "positions": map })))
}

/// 管理员编排某首诗的诗句框位置（归一化 x,y）。
pub async fn set_panel_position(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(poem_id): Path<i32>,
    Json(body): Json<PanelPositionBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = current_user(&state, &headers).await?;
    if user.role != "admin" {
        return Err(AppError::Forbidden("admin required".to_string()));
    }
    let x = body.x.clamp(0.0, 1.0);
    let y = body.y.clamp(0.0, 1.0);
    sqlx::query(
        r#"INSERT INTO poem_panel_positions (poem_id, x, y) VALUES ($1, $2, $3)
           ON CONFLICT (poem_id) DO UPDATE SET x = $2, y = $3, updated_at = now()"#,
    )
    .bind(poem_id)
    .bind(x)
    .bind(y)
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
