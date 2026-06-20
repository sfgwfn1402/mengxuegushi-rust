use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Clone)]
pub struct IpGuard {
    inner: Arc<Mutex<HashMap<IpAddr, IpRecord>>>,
    window: Duration,
    max_requests: usize,
    ban_duration: Duration,
}

#[derive(Debug, Default)]
struct IpRecord {
    requests: VecDeque<Instant>,
    banned_until: Option<Instant>,
}

#[derive(Debug, Serialize)]
struct RateLimitError {
    code: u16,
    message: &'static str,
}

#[derive(Debug)]
pub enum IpGuardDecision {
    Allow,
    Ban { retry_after_seconds: u64 },
}

impl IpGuard {
    pub fn new(window: Duration, max_requests: usize, ban_duration: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            window,
            max_requests,
            ban_duration,
        }
    }

    pub fn check(&self, ip: IpAddr) -> IpGuardDecision {
        let now = Instant::now();
        let mut records = self.inner.lock().expect("ip guard mutex poisoned");
        let record = records.entry(ip).or_default();

        if let Some(banned_until) = record.banned_until {
            if banned_until > now {
                return IpGuardDecision::Ban {
                    retry_after_seconds: banned_until.duration_since(now).as_secs().max(1),
                };
            }
            record.banned_until = None;
            record.requests.clear();
        }

        while let Some(&front) = record.requests.front() {
            if now.duration_since(front) <= self.window {
                break;
            }
            record.requests.pop_front();
        }

        record.requests.push_back(now);

        if record.requests.len() > self.max_requests {
            let banned_until = now + self.ban_duration;
            record.banned_until = Some(banned_until);
            record.requests.clear();
            tracing::warn!(%ip, max_requests = self.max_requests, ban_seconds = self.ban_duration.as_secs(), "ip banned for too many requests");
            return IpGuardDecision::Ban {
                retry_after_seconds: self.ban_duration.as_secs(),
            };
        }

        IpGuardDecision::Allow
    }
}

pub async fn ip_guard_middleware(
    State(guard): State<IpGuard>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // 静态媒体加载量大且由小程序并发触发，不应计入 API 防刷限流，避免误封正常用户。
    let path = request.uri().path();
    if path.starts_with("/audios/")
        || path.starts_with("/images/")
        || path.starts_with("/recitations/")
        || path.starts_with("/avatars/")
        || path.starts_with("/artworks/")
    {
        return next.run(request).await;
    }

    let ip = client_ip(&request);

    if let Some(ip) = ip {
        match guard.check(ip) {
            IpGuardDecision::Allow => {}
            IpGuardDecision::Ban {
                retry_after_seconds,
            } => {
                let body = Json(RateLimitError {
                    code: StatusCode::TOO_MANY_REQUESTS.as_u16(),
                    message: "请求过于频繁，请稍后再试",
                });

                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, retry_after_seconds.to_string())],
                    body,
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

fn client_ip(request: &Request<Body>) -> Option<IpAddr> {
    // If the service is behind Nginx / a cloud proxy, X-Forwarded-For normally
    // contains: "client, proxy1, proxy2". The first valid IP is the real client.
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_for) = forwarded_for.to_str() {
            for part in forwarded_for.split(',') {
                if let Ok(ip) = IpAddr::from_str(part.trim()) {
                    return Some(ip);
                }
            }
        }
    }

    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(real_ip) = real_ip.to_str() {
            if let Ok(ip) = IpAddr::from_str(real_ip.trim()) {
                return Some(ip);
            }
        }
    }

    // Without proxy headers Axum does not expose the peer address by default.
    // In that case we skip IP banning rather than accidentally banning everyone.
    None
}
