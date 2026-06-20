pub mod admin;
pub mod artworks;
pub mod auth;
pub mod health;
pub mod me;
pub mod media;
pub mod poems;
pub mod qrcode;
pub mod recitations;
pub mod themes;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/home/today-poem", get(home::today_poem))
        .route("/home/continue-learning", get(home::continue_learning))
        .route("/home/recommendations", get(home::recommendations))
        .route("/home/popular-recitations", get(home::popular_recitations))
        .route("/works/qrcode", get(qrcode::work_qrcode))
        .route(
            "/admin/artworks/{artwork_id}/review",
            post(admin::review_artwork),
        )
        .route(
            "/admin/recitations/{recitation_id}/review",
            post(admin::review_recitation),
        )
        .route("/themes", get(themes::list_themes))
        .route("/poems", get(poems::list_poems))
        .route("/poems/{id}", get(poems::get_poem))
        .route(
            "/poems/{poem_id}/recitations/featured",
            get(recitations::featured),
        )
        .route(
            "/poems/{poem_id}/recitations/top",
            get(recitations::list_top),
        )
        .route("/poems/{poem_id}/recitations", post(recitations::upload))
        .route("/poems/{poem_id}/artworks", post(artworks::upload))
        .route("/artworks", get(artworks::list))
        .route("/artworks/{artwork_id}/image", get(artworks::image))
        .route(
            "/artworks/{artwork_id}/like",
            post(artworks::like).delete(artworks::unlike),
        )
        .route(
            "/artworks/{artwork_id}",
            get(artworks::detail).delete(artworks::delete_artwork),
        )
        .route(
            "/artworks/{artwork_id}/submit",
            post(artworks::submit_artwork).delete(artworks::withdraw_artwork),
        )
        .route(
            "/recitations/{recitation_id}/like",
            post(recitations::like).delete(recitations::unlike),
        )
        .route(
            "/recitations/{recitation_id}/audio",
            get(recitations::audio),
        )
        .route(
            "/recitations/{recitation_id}",
            get(recitations::detail).delete(recitations::delete_recitation),
        )
        .route(
            "/recitations/{recitation_id}/submit",
            post(recitations::submit_recitation).delete(recitations::withdraw_recitation),
        )
        .route("/auth/wechat-login", post(auth::wechat_login))
        .route("/auth/dev-login", post(auth::dev_login))
        .route("/me", get(me::me).post(me::update_profile))
        .route("/me/avatar", post(me::upload_avatar))
        .route("/me/stats", get(me::stats))
        .route("/me/checkin", post(me::checkin))
        .route("/me/tasks", post(me::complete_task))
        .route("/me/clear-data", post(me::clear_data))
        .route("/me/progress", get(me::list_progress))
        .route("/me/recitations", get(me::list_recitations))
        .route("/me/progress/{poem_id}", post(me::update_progress))
        .route(
            "/me/idiom-progress",
            get(me::list_idiom_progress).post(me::update_idiom_progress),
        )
        .route("/me/favorites", get(me::list_favorites))
        .route(
            "/me/favorites/{poem_id}",
            post(me::add_favorite).delete(me::remove_favorite),
        )
}
pub mod home;
