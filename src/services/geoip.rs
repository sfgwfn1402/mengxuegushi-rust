use std::str::FromStr;
use std::net::IpAddr;
use std::time::Duration;

use axum::http::HeaderMap;

// 从代理头里取真实客户端 IP（Nginx 转发：x-forwarded-for: client, proxy...）
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        for part in xff.split(',') {
            let p = part.trim();
            if IpAddr::from_str(p).is_ok() {
                return Some(p.to_string());
            }
        }
    }
    if let Some(real) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let p = real.trim();
        if IpAddr::from_str(p).is_ok() {
            return Some(p.to_string());
        }
    }
    None
}

// 省份全称 → 简称（辽宁省 → 辽宁），尽量贴近常见 IP 属地展示
fn shorten_province(pro: &str) -> String {
    let p = pro.trim();
    let specials = [
        ("内蒙古", "内蒙古"),
        ("广西", "广西"),
        ("宁夏", "宁夏"),
        ("新疆", "新疆"),
        ("西藏", "西藏"),
        ("香港", "香港"),
        ("澳门", "澳门"),
    ];
    for (key, short) in specials {
        if p.starts_with(key) {
            return short.to_string();
        }
    }
    p.trim_end_matches('省')
        .trim_end_matches('市')
        .to_string()
}

// best-effort：调用 pconline 解析 IP 属地（GBK JSON），失败返回 None。
// 仅取省级，海外/解析失败则不展示。
pub async fn resolve_location(ip: &str) -> Option<String> {
    let ip = ip.trim();
    if ip.is_empty() || ip.starts_with("127.") || ip.starts_with("10.") || ip.starts_with("192.168.") {
        return None;
    }
    // ip-api.com：免费、UTF-8 JSON、lang=zh-CN 直接返回中文省份（regionName）。
    // 免费版为 HTTP、限速 45 次/分，对社区发帖量足够；高并发可换离线 ip2region。
    let url = format!("http://ip-api.com/json/{ip}?lang=zh-CN&fields=status,regionName");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;
    let text = client.get(&url).send().await.ok()?.text().await.ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    if json.get("status").and_then(|v| v.as_str()) != Some("success") {
        return None;
    }
    let pro = json.get("regionName").and_then(|v| v.as_str()).unwrap_or("").trim();
    if pro.is_empty() {
        return None;
    }
    let short = shorten_province(pro);
    if short.is_empty() {
        None
    } else {
        Some(short)
    }
}
