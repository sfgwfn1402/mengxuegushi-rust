use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use crate::{
    error::AppError,
    models::home::{HomePoemResponse, HomeRecommendationsResponse, PopularRecitationsResponse},
    routes::me::current_user,
    services::home_store,
    AppState,
};

pub async fn today_poem(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HomePoemResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let item = home_store::today_unlearned_poem(&state.db, &user.id).await?;
    Ok(Json(HomePoemResponse { item }))
}

pub async fn continue_learning(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HomePoemResponse>, AppError> {
    let user = current_user(&state, &headers).await?;
    let item = home_store::continue_learning_poem(&state.db, &user.id).await?;
    Ok(Json(HomePoemResponse { item }))
}

pub async fn recommendations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HomeRecommendationsResponse>, AppError> {
    let user = current_user(&state, &headers).await.ok();
    let (reason, theme, items) =
        home_store::recommend_poems(&state.db, user.as_ref().map(|u| u.id.as_str()), 4).await?;
    Ok(Json(HomeRecommendationsResponse {
        reason,
        theme,
        items,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PopularRecitationsQuery {
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

pub async fn popular_recitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PopularRecitationsQuery>,
) -> Result<Json<PopularRecitationsResponse>, AppError> {
    let user = current_user(&state, &headers).await.ok();
    let limit = query.limit.unwrap_or(3).clamp(1, 50);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;
    let items = home_store::popular_recitations(
        &state.db,
        user.as_ref().map(|u| u.id.as_str()),
        limit,
        offset,
    )
    .await?;
    Ok(Json(PopularRecitationsResponse { items }))
}

/// 人气朗诵：最新100条 → 点赞前30 → 随机10条
pub async fn hot_recitation_random_pick(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PopularRecitationsResponse>, AppError> {
    let user = current_user(&state, &headers).await.ok();
    let items = home_store::hot_recitation_random_pick(
        &state.db,
        user.as_ref().map(|u| u.id.as_str()),
    )
    .await?;
    Ok(Json(PopularRecitationsResponse { items }))
}
