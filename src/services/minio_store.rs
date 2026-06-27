use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, HOST};
use sha2::{Digest, Sha256};

use crate::{config::AppConfig, error::AppError};

type HmacSha256 = Hmac<Sha256>;

fn has_minio_config(config: &AppConfig) -> bool {
    config.minio_endpoint.is_some()
        && config.minio_bucket.is_some()
        && config.minio_access_key.is_some()
        && config.minio_secret_key.is_some()
}

pub fn enabled(config: &AppConfig) -> bool {
    has_minio_config(config)
}

pub async fn put_object(
    config: &AppConfig,
    object_key: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<(), AppError> {
    if !has_minio_config(config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }

    let endpoint = config
        .minio_endpoint
        .as_ref()
        .unwrap()
        .trim_end_matches('/');
    let bucket = config.minio_bucket.as_ref().unwrap();
    let access_key = config.minio_access_key.as_ref().unwrap();
    let secret_key = config.minio_secret_key.as_ref().unwrap();
    let region = "us-east-1";
    let service = "s3";

    let url = format!("{endpoint}/{bucket}/{object_key}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|err| AppError::Internal(format!("invalid minio url: {err}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::Internal("minio endpoint missing host".to_string()))?;
    let host = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };

    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = hex::encode(Sha256::digest(&bytes));
    let canonical_uri = format!("/{bucket}/{}", uri_encode_path(object_key));
    let canonical_headers = format!(
        "content-type:{content_type}\nhost:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n"
    );
    let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";
    let canonical_request =
        format!("PUT\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let signing_key = signing_key(secret_key, &date_stamp, region, service)?;
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        HOST,
        HeaderValue::from_str(&host).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(content_type).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-date",
        HeaderValue::from_str(&amz_date).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-content-sha256",
        HeaderValue::from_str(&payload_hash).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "authorization",
        HeaderValue::from_str(&authorization).map_err(|err| AppError::Internal(err.to_string()))?,
    );

    let client = reqwest::Client::new();
    let response = client
        .put(url)
        .headers(headers)
        .body(bytes)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("minio upload failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "minio upload failed: {status} {body}"
        )));
    }

    Ok(())
}

pub async fn get_object(
    config: &AppConfig,
    object_key: &str,
) -> Result<(Vec<u8>, String), AppError> {
    if !has_minio_config(config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }

    let endpoint = config
        .minio_endpoint
        .as_ref()
        .unwrap()
        .trim_end_matches('/');
    let bucket = config.minio_bucket.as_ref().unwrap();
    let access_key = config.minio_access_key.as_ref().unwrap();
    let secret_key = config.minio_secret_key.as_ref().unwrap();
    let region = "us-east-1";
    let service = "s3";
    let url = format!("{endpoint}/{bucket}/{object_key}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|err| AppError::Internal(format!("invalid minio url: {err}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::Internal("minio endpoint missing host".to_string()))?;
    let host = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = hex::encode(Sha256::digest(b""));
    let canonical_uri = format!("/{bucket}/{}", uri_encode_path(object_key));
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request =
        format!("GET\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let signing_key = signing_key(secret_key, &date_stamp, region, service)?;
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        HOST,
        HeaderValue::from_str(&host).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-date",
        HeaderValue::from_str(&amz_date).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-content-sha256",
        HeaderValue::from_str(&payload_hash).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "authorization",
        HeaderValue::from_str(&authorization).map_err(|err| AppError::Internal(err.to_string()))?,
    );

    let response = reqwest::Client::new()
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("minio get failed: {err}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "minio get failed: {status} {body}"
        )));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| AppError::Internal(format!("minio read failed: {err}")))?
        .to_vec();
    Ok((bytes, content_type))
}

pub async fn delete_object(config: &AppConfig, object_key: &str) -> Result<(), AppError> {
    if !has_minio_config(config) {
        return Err(AppError::Internal("minio config missing".to_string()));
    }
    let endpoint = config.minio_endpoint.as_ref().unwrap().trim_end_matches('/');
    let bucket = config.minio_bucket.as_ref().unwrap();
    let access_key = config.minio_access_key.as_ref().unwrap();
    let secret_key = config.minio_secret_key.as_ref().unwrap();
    let region = "us-east-1";
    let service = "s3";
    let url = format!("{endpoint}/{bucket}/{object_key}");
    let parsed = reqwest::Url::parse(&url)
        .map_err(|err| AppError::Internal(format!("invalid minio url: {err}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::Internal("minio endpoint missing host".to_string()))?;
    let host = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = hex::encode(Sha256::digest(b""));
    let canonical_uri = format!("/{bucket}/{}", uri_encode_path(object_key));
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request =
        format!("DELETE\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let signing_key = signing_key(secret_key, &date_stamp, region, service)?;
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        HOST,
        HeaderValue::from_str(&host).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-date",
        HeaderValue::from_str(&amz_date).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "x-amz-content-sha256",
        HeaderValue::from_str(&payload_hash).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    headers.insert(
        "authorization",
        HeaderValue::from_str(&authorization).map_err(|err| AppError::Internal(err.to_string()))?,
    );
    let response = reqwest::Client::new()
        .delete(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("minio delete failed: {err}")))?;
    // 204/200 都算成功；404(已不存在)也视为成功
    if !response.status().is_success() && response.status().as_u16() != 404 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("minio delete failed: {status} {body}")));
    }
    Ok(())
}

fn signing_key(
    secret_key: &str,
    date: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>, AppError> {
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    hmac_sha256(&k_service, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|err| AppError::Internal(format!("hmac init failed: {err}")))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn uri_encode_path(path: &str) -> String {
    path.split('/')
        .map(|part| url_encode(part))
        .collect::<Vec<_>>()
        .join("/")
}

fn url_encode(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}
