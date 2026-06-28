pub mod admin;
pub mod artworks;
pub mod auth;
pub mod feedback;
pub mod health;
pub mod me;
pub mod media;
pub mod messages;
pub mod moments;
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
        .route("/home/community-stats", get(home::community_stats))
        .route("/home/today-poem", get(home::today_poem))
        .route("/home/continue-learning", get(home::continue_learning))
        .route("/home/recommendations", get(home::recommendations))
        .route("/home/popular-recitations", get(home::popular_recitations))
        .route("/home/hot-recitation-pick", get(home::hot_recitation_random_pick))
        .route("/works/qrcode", get(qrcode::work_qrcode))
        .route("/admin/feedback", get(admin::list_feedback))
        .route(
            "/admin/feedback/{feedback_id}/status",
            post(admin::update_feedback_status),
        )
        .route("/admin/recitations", get(admin::list_recitations))
        .route(
            "/admin/recitations/{recitation_id}/review",
            post(admin::review_recitation),
        )
        .route("/admin/artworks", get(admin::list_artworks))
        .route(
            "/admin/artworks/{artwork_id}/review",
            post(admin::review_artwork),
        )
        .route("/themes", get(themes::list_themes))
        .route("/feedback", post(feedback::submit_parent_feedback))
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
        .route(
            "/poems/{poem_id}/recitations/score",
            post(recitations::score),
        )
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
        .route("/me/reminder-subscribe", post(me::subscribe_reminder))
        .route("/me/invite-info", get(me::invite_info))
        .route("/invite/inviter/{code}", get(me::inviter_name))
        .route("/events", post(me::track_events))
        .route("/admin/analytics", get(admin::analytics))
        .route("/moments", get(moments::list).post(moments::create))
        .route("/moments/mine", get(moments::list_mine))
        .route("/moments/upload-image", post(moments::upload_image))
        .route("/moments/{moment_id}/image", get(moments::image))
        .route("/moments/{moment_id}/image/{idx}", get(moments::image_idx))
        .route(
            "/moments/{moment_id}",
            axum::routing::put(moments::edit).delete(moments::delete_moment),
        )
        .route(
            "/moments/{moment_id}/like",
            post(moments::like).delete(moments::unlike),
        )
        .route(
            "/moments/{moment_id}/comments",
            get(moments::list_comments).post(moments::create_comment),
        )
        .route(
            "/moments/{moment_id}/comments/{comment_id}",
            axum::routing::delete(moments::delete_comment),
        )
        .route("/users/{user_id}/profile", get(moments::user_profile))
        .route("/users/{user_id}/moments", get(moments::user_moments))
        .route("/users/{user_id}/artworks", get(moments::user_artworks))
        .route("/users/{user_id}/recitations", get(moments::user_recitations))
        .route(
            "/users/{user_id}/follow",
            post(moments::follow_user).delete(moments::unfollow_user),
        )
        .route("/admin/moments", get(admin::list_moments))
        .route("/admin/moments/{moment_id}/review", post(admin::review_moment))
        .route("/admin/send-reminders", post(admin::send_reminders))
        .route("/me/messages/summary", get(messages::summary))
        .route("/me/messages/list", get(messages::list))
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
