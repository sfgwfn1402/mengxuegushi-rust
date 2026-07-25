use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use crate::{
    error::AppError,
    models::home::{
        CommunityStatsResponse, HomePoemResponse, HomeRecommendationsResponse,
        OnlineGlobeResponse, OnlineGlobeUser, PopularRecitationsResponse,
    },
    routes::me::current_user,
    services::home_store,
    AppState,
};

// 公开接口，无需登录：客户端启动时拉取的功能开关配置。
// 二级开关：discover_enabled=发现作品展示（先审后发，风险低），moments_enabled=动态社区互动（UGC）。
// community_enabled 供只认单开关的旧版客户端用，取最保守值（两个子开关都开才为 true），
// 保证"只开发现、关动态"时旧版不会把整个社区（含动态）放出来。
pub async fn app_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "community_enabled": state.config.discover_enabled && state.config.moments_enabled,
        "discover_enabled": state.config.discover_enabled,
        "moments_enabled": state.config.moments_enabled,
    }))
}

// 空昵称兜底：按 user id 稳定哈希，生成一个可爱的中文昵称（同一用户每次都一样）。
fn fallback_nickname(id: &str) -> String {
    const SUR: [&str; 12] = ["小", "阿", "萌", "诗", "童", "梦", "云", "月", "星", "糖", "团", "米"];
    const GIV: [&str; 20] = [
        "清", "羽", "萱", "若", "宁", "安", "然", "之", "夏", "冬", "晓", "冉", "瑶", "乐", "可",
        "朵", "白", "蓝", "橙", "青",
    ];
    // FNV-1a 稳定哈希
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let s = SUR[(h % SUR.len() as u64) as usize];
    let g1 = GIV[((h / 13) % GIV.len() as u64) as usize];
    if h % 2 == 0 {
        format!("{s}{g1}")
    } else {
        let g2 = GIV[((h / 251) % GIV.len() as u64) as usize];
        format!("{s}{g1}{g2}")
    }
}

// 公开接口，无需登录：首页 3D 地球「在线小诗人」。
pub async fn online_globe(
    State(state): State<AppState>,
) -> Result<Json<OnlineGlobeResponse>, AppError> {
    let (count, rows) = home_store::online_globe(&state.db).await?;
    let users = rows
        .into_iter()
        .map(|(id, nickname, avatar)| {
            let name = nickname
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| fallback_nickname(&id));
            OnlineGlobeUser { id, name, avatar }
        })
        .collect();
    Ok(Json(OnlineGlobeResponse { count, users }))
}

// 公开接口，无需登录：首页人气展示
pub async fn community_stats(
    State(state): State<AppState>,
) -> Result<Json<CommunityStatsResponse>, AppError> {
    let (learners, today_lit, total_lit) = home_store::community_stats(&state.db).await?;
    Ok(Json(CommunityStatsResponse {
        learners,
        today_lit,
        total_lit,
    }))
}

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
