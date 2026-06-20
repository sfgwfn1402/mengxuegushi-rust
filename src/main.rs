mod config;
mod error;
mod ip_guard;
mod models;
mod routes;
mod services;

use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
use axum::{middleware, Router};
use config::AppConfig;
use ip_guard::IpGuard;
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub db: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;
    let db = init_database(&config).await?;

    let state = AppState {
        config: config.clone(),
        http_client: reqwest::Client::new(),
        db,
    };
    // 小程序首页会并发加载 API、图片和音频；域名 HTTPS 统一入口后，120/min 过低，
    // 容易误封正常用户。静态媒体在 ip_guard 中跳过，这里只兜底限制异常 API 风暴。
    let ip_guard = IpGuard::new(Duration::from_secs(60), 600, Duration::from_secs(300));

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .route(
            "/audios/{file_name}",
            axum::routing::get(routes::media::audio),
        )
        .route(
            "/images/{file_name}",
            axum::routing::get(routes::media::image),
        )
        .route(
            "/recitations/{*path}",
            axum::routing::get(routes::media::recitation),
        )
        .route(
            "/avatars/{*path}",
            axum::routing::get(routes::media::avatar),
        )
        .route(
            "/artworks/{*path}",
            axum::routing::get(routes::artworks::media),
        )
        .route("/health", axum::routing::get(routes::health::health_check))
        .route_layer(middleware::from_fn_with_state(
            ip_guard,
            ip_guard::ip_guard_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    tracing::info!(%addr, "mengxuegushi api server started");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn init_database(config: &AppConfig) -> anyhow::Result<sqlx::PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    services::poem_store::seed_default_poems(&pool, config.public_base_url.as_deref()).await?;

    Ok(pool)
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mengxuegushi_rust=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
