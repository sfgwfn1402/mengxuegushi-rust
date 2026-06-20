use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poem {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: String,
    pub pinyin: Option<String>,
    pub translation: Option<String>,
    pub story: Option<String>,
    pub parent_guide: Option<String>,
    pub difficulty: u8,
    pub level: u8,
    pub tags: Vec<String>,
    pub season: String,
    pub audio_url: Option<String>,
    pub audio_version: Option<String>,
    pub image_url: Option<String>,
    pub video_available: bool,
    pub card_unlocked: bool,
    pub annotated_content: Vec<AnnotatedChar>,
    #[serde(default)]
    pub themes: Vec<PoemThemeTag>,
    #[serde(default)]
    pub follow_timings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoemThemeTag {
    pub id: String,
    pub name: String,
    pub emoji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedChar {
    pub char: String,
    pub pinyin: String,
    pub punct: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoemListQuery {
    pub level: Option<u8>,
    pub difficulty: Option<u8>,
    pub season: Option<String>,
    pub tag: Option<String>,
    pub theme: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PoemListResponse {
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub items: Vec<Poem>,
}

impl PoemListQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).clamp(1, 100)
    }
}
