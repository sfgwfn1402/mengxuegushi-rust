use axum::{extract::Path, extract::Query, extract::State, Json};

use crate::{
    error::AppError,
    models::poem::{PoemListQuery, PoemListResponse},
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
