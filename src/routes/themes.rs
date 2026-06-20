use axum::{extract::State, Json};

use crate::{error::AppError, models::theme::ThemeListResponse, services::theme_store, AppState};

pub async fn list_themes(
    State(state): State<AppState>,
) -> Result<Json<ThemeListResponse>, AppError> {
    let items = theme_store::list_themes(&state.db).await?;
    Ok(Json(ThemeListResponse { items }))
}
