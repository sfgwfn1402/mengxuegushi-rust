use serde::Serialize;

use super::{poem::Poem, recitation::RecitationItem, theme::PoemTheme};

#[derive(Debug, Serialize)]
pub struct HomeRecommendationsResponse {
    pub reason: String,
    pub theme: Option<PoemTheme>,
    pub items: Vec<Poem>,
}

#[derive(Debug, Serialize)]
pub struct HomePoemResponse {
    pub item: Option<Poem>,
}

#[derive(Debug, Serialize)]
pub struct PopularRecitationItem {
    pub recitation: RecitationItem,
    pub poem_title: String,
    pub poem_author: String,
    pub poem_dynasty: String,
}

#[derive(Debug, Serialize)]
pub struct PopularRecitationsResponse {
    pub items: Vec<PopularRecitationItem>,
}

#[derive(Debug, Serialize)]
pub struct CommunityStatsResponse {
    pub learners: i64,
    pub today_lit: i64,
    pub total_lit: i64,
}
