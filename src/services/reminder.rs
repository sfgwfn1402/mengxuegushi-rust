use crate::{services::{user_store, wechat}, AppState};

/// 给"有额度、今天没打卡、今天没提醒过"的用户发学习提醒。返回成功发送条数。
pub async fn send_daily_reminders(state: &AppState) -> usize {
    let users = match user_store::users_to_remind(&state.db).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("users_to_remind failed: {}", e);
            return 0;
        }
    };
    let mut sent = 0usize;
    for (id, openid, learned) in users {
        let r = wechat::send_study_reminder(state, &openid, learned).await;
        // 总是消耗额度+标记今天已处理，避免同一坏 openid 反复重试刷屏
        let _ = user_store::mark_reminded(&state.db, &id).await;
        match r {
            Ok(_) => sent += 1,
            Err(e) => tracing::warn!("study reminder failed user={} err={}", id, e),
        }
    }
    tracing::info!("study reminders sent: {}", sent);
    sent
}
