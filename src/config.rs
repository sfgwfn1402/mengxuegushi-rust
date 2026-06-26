use anyhow::Context;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: String,
    pub audio_dir: String,
    pub public_base_url: Option<String>,
    pub enable_dev_login: bool,
    pub wechat_app_id: Option<String>,
    pub wechat_app_secret: Option<String>,
    pub recitation_dir: String,
    pub recitation_public_base_url: String,
    pub minio_endpoint: Option<String>,
    pub minio_bucket: Option<String>,
    pub minio_access_key: Option<String>,
    pub minio_secret_key: Option<String>,
    pub minio_public_base_url: Option<String>,
    pub avatar_public_base_url: Option<String>,
    pub featured_recitation_min_likes: i32,
    pub admin_token: Option<String>,
    pub funasr_score_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .context("PORT must be a valid u16")?;

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://mengxuegushi:mengxuegushi@127.0.0.1:5432/mengxuegushi".to_string()
        });

        let audio_dir = std::env::var("AUDIO_DIR").unwrap_or_else(|_| "./audios".to_string());
        let public_base_url = std::env::var("PUBLIC_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());

        let enable_dev_login = std::env::var("ENABLE_DEV_LOGIN")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(true);

        let recitation_dir =
            std::env::var("RECITATION_DIR").unwrap_or_else(|_| "./recitations".to_string());
        let recitation_public_base_url = std::env::var("RECITATION_PUBLIC_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                public_base_url
                    .as_ref()
                    .map(|base| format!("{}/static/recitations", base.trim_end_matches('/')))
            })
            .unwrap_or_else(|| "/recitations".to_string());

        let minio_endpoint = std::env::var("MINIO_ENDPOINT")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let minio_bucket = std::env::var("MINIO_BUCKET")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let minio_access_key = std::env::var("MINIO_ACCESS_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let minio_secret_key = std::env::var("MINIO_SECRET_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let minio_public_base_url = std::env::var("MINIO_PUBLIC_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let avatar_public_base_url = std::env::var("AVATAR_PUBLIC_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());

        let featured_recitation_min_likes = std::env::var("FEATURED_RECITATION_MIN_LIKES")
            .ok()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        let admin_token = std::env::var("ADMIN_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());

        // FunASR 朗诵评分服务地址；默认同机本地服务
        let funasr_score_url = Some(
            std::env::var("FUNASR_SCORE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8181".to_string()),
        )
        .filter(|value| !value.trim().is_empty());

        Ok(Self {
            port,
            database_url,
            audio_dir,
            public_base_url,
            enable_dev_login,
            wechat_app_id: std::env::var("WECHAT_APP_ID").ok(),
            wechat_app_secret: std::env::var("WECHAT_APP_SECRET").ok(),
            recitation_dir,
            recitation_public_base_url,
            minio_endpoint,
            minio_bucket,
            minio_access_key,
            minio_secret_key,
            minio_public_base_url,
            avatar_public_base_url,
            featured_recitation_min_likes,
            admin_token,
            funasr_score_url,
        })
    }
}
