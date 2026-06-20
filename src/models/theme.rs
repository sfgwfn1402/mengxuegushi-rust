use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PoemTheme {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub description: String,
    pub poem_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub items: Vec<PoemTheme>,
}
