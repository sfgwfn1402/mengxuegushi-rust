//! StoreKit 2 签名交易（JWS）服务端验签。
//!
//! 链路：解析 JWS header 里的 x5c 证书链 → 验证 叶子←中间←Apple Root CA G3（内置）
//! → 用叶子证书公钥验 ES256 签名 → 解码 payload 交易字段。
//! 验过才落库；bundleId 不匹配直接拒。

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, engine::general_purpose::STANDARD, Engine};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::Deserialize;
use x509_parser::prelude::*;

/// Apple Root CA - G3（https://www.apple.com/certificateauthority/AppleRootCA-G3.cer）
const APPLE_ROOT_G3_DER: &[u8] = include_bytes!("certs/AppleRootCA-G3.cer");

/// 本 App 的 bundle id，payload 不符即拒绝（防止别的 App 的交易混进来）
const EXPECTED_BUNDLE_ID: &str = "com.duwei.mengxuegushi";

#[derive(Debug, Clone)]
pub struct VerifiedTransaction {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub product_id: String,
    pub bundle_id: String,
    pub purchase_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub environment: String,
}

#[derive(Debug, Deserialize)]
struct JwsHeader {
    #[serde(default)]
    x5c: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionPayload {
    transaction_id: String,
    original_transaction_id: String,
    bundle_id: String,
    product_id: String,
    #[serde(default)]
    purchase_date: Option<i64>,
    #[serde(default)]
    expires_date: Option<i64>,
    #[serde(default)]
    revocation_date: Option<i64>,
    #[serde(default)]
    environment: Option<String>,
}

fn ms_to_ts(ms: Option<i64>) -> Option<chrono::DateTime<chrono::Utc>> {
    ms.and_then(|v| chrono::DateTime::from_timestamp_millis(v))
}

/// 用内置 Apple Root CA G3 验签并解码交易。
pub fn verify_signed_transaction(jws: &str) -> Result<VerifiedTransaction, String> {
    verify_signed_transaction_with_root(jws, APPLE_ROOT_G3_DER)
}

/// 与上一函数相同，但根证书可注入——便于单元测试用自签 CA 构造完整链路。
fn verify_signed_transaction_with_root(jws: &str, root_der: &[u8]) -> Result<VerifiedTransaction, String> {
    let parts: Vec<&str> = jws.split('.').collect();
    if parts.len() != 3 {
        return Err("JWS 格式错误（应为 header.payload.signature）".into());
    }
    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|e| format!("header base64 解码失败: {e}"))?;
    let header: JwsHeader =
        serde_json::from_slice(&header_bytes).map_err(|e| format!("header JSON 解析失败: {e}"))?;
    if header.x5c.len() < 2 {
        return Err("x5c 证书链不完整".into());
    }

    // x5c[0]=叶子, x5c[1]=中间；根用内置的 Apple Root CA G3，不信 JWS 自带的第三张（防伪造）。
    let leaf_der = STANDARD
        .decode(&header.x5c[0])
        .map_err(|e| format!("叶子证书 base64 失败: {e}"))?;
    let inter_der = STANDARD
        .decode(&header.x5c[1])
        .map_err(|e| format!("中间证书 base64 失败: {e}"))?;

    let (_, leaf) = X509Certificate::from_der(&leaf_der).map_err(|e| format!("叶子证书解析失败: {e}"))?;
    let (_, inter) =
        X509Certificate::from_der(&inter_der).map_err(|e| format!("中间证书解析失败: {e}"))?;
    let (_, root) =
        X509Certificate::from_der(root_der).map_err(|e| format!("根证书解析失败: {e}"))?;

    // 有效期检查（叶子 + 中间）
    let now = chrono::Utc::now().timestamp();
    for (name, cert) in [("叶子", &leaf), ("中间", &inter)] {
        let nb = cert.validity().not_before.timestamp();
        let na = cert.validity().not_after.timestamp();
        if now < nb || now > na {
            return Err(format!("{name}证书不在有效期内"));
        }
    }

    // 链验证：中间←根、叶子←中间
    inter
        .verify_signature(Some(root.public_key()))
        .map_err(|_| "中间证书未被 Apple 根签名".to_string())?;
    leaf.verify_signature(Some(inter.public_key()))
        .map_err(|_| "叶子证书未被中间证书签名".to_string())?;

    // JWS 签名：ES256（P-256 + SHA-256），签名值是 R||S 64 字节
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| format!("签名 base64 解码失败: {e}"))?;
    let signature =
        Signature::from_slice(&sig_bytes).map_err(|e| format!("签名格式错误（应为 64 字节 R||S）: {e}"))?;
    let key_bytes = leaf.public_key().subject_public_key.data.as_ref();
    let verifying_key = VerifyingKey::from_sec1_bytes(key_bytes)
        .map_err(|e| format!("叶子公钥解析失败: {e}"))?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    verifying_key
        .verify(signing_input.as_bytes(), &signature)
        .map_err(|_| "JWS 签名校验失败".to_string())?;

    // 解码 payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| format!("payload base64 解码失败: {e}"))?;
    let payload: TransactionPayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("payload JSON 解析失败: {e}"))?;

    if payload.bundle_id != EXPECTED_BUNDLE_ID {
        return Err(format!("bundleId 不匹配: {}", payload.bundle_id));
    }

    Ok(VerifiedTransaction {
        transaction_id: payload.transaction_id,
        original_transaction_id: payload.original_transaction_id,
        product_id: payload.product_id,
        bundle_id: payload.bundle_id,
        purchase_at: ms_to_ts(payload.purchase_date),
        expires_at: ms_to_ts(payload.expires_date),
        revoked_at: ms_to_ts(payload.revocation_date),
        environment: payload.environment.unwrap_or_else(|| "Production".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 端到端：openssl 现造 根→中间→叶子 自签链 + 手工 ES256 JWS。
    /// 验证器注入自签根，走与生产完全相同的验证路径。
    #[test]
    fn verify_full_chain_with_test_ca() {
        let dir = std::env::temp_dir().join(format!("jws-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let sh = |cmd: &str| {
            let out = std::process::Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .output()
                .unwrap();
            assert!(out.status.success(), "命令失败: {cmd}\n{}", String::from_utf8_lossy(&out.stderr));
            out.stdout
        };
        let d = dir.display();
        // 根 CA
        sh(&format!("openssl ecparam -name prime256v1 -genkey -noout -out {d}/root.key"));
        sh(&format!("openssl req -x509 -new -key {d}/root.key -sha256 -days 3650 \
                     -subj '/CN=Test Root CA' -out {d}/root.crt"));
        // 中间 CA
        sh(&format!("openssl ecparam -name prime256v1 -genkey -noout -out {d}/inter.key"));
        sh(&format!("openssl req -new -key {d}/inter.key -subj '/CN=Test Intermediate' -out {d}/inter.csr"));
        sh(&format!("openssl x509 -req -in {d}/inter.csr -CA {d}/root.crt -CAkey {d}/root.key \
                     -CAcreateserial -sha256 -days 3650 -out {d}/inter.crt"));
        // 叶子
        sh(&format!("openssl ecparam -name prime256v1 -genkey -noout -out {d}/leaf.key"));
        sh(&format!("openssl req -new -key {d}/leaf.key -subj '/CN=Test Leaf' -out {d}/leaf.csr"));
        sh(&format!("openssl x509 -req -in {d}/leaf.csr -CA {d}/inter.crt -CAkey {d}/inter.key \
                     -CAcreateserial -sha256 -days 3650 -out {d}/leaf.crt"));
        // DER + base64
        sh(&format!("openssl x509 -in {d}/root.crt -outform DER -out {d}/root.der"));
        let leaf_b64 = String::from_utf8(sh(&format!(
            "openssl x509 -in {d}/leaf.crt -outform DER | base64"))).unwrap().replace('\n', "");
        let inter_b64 = String::from_utf8(sh(&format!(
            "openssl x509 -in {d}/inter.crt -outform DER | base64"))).unwrap().replace('\n', "");
        let root_der = std::fs::read(dir.join("root.der")).unwrap();

        // 构造 JWS
        let b64u = |data: &[u8]| URL_SAFE_NO_PAD.encode(data);
        let header = serde_json::json!({"alg":"ES256","x5c":[leaf_b64, inter_b64]});
        let payload = serde_json::json!({
            "transactionId":"10001","originalTransactionId":"10000",
            "bundleId":"com.duwei.mengxuegushi","productId":"com.duwei.mengxuegushi.premium.annual",
            "purchaseDate":1753400000000i64,"expiresDate":1785000000000i64,
            "environment":"Xcode"
        });
        let signing_input = format!("{}.{}", b64u(header.to_string().as_bytes()), b64u(payload.to_string().as_bytes()));
        // openssl 签名（DER ECDSA-Sig-Value）→ 转 R||S
        std::fs::write(dir.join("msg.bin"), &signing_input).unwrap();
        sh(&format!("openssl dgst -sha256 -sign {d}/leaf.key -out {d}/sig.der {d}/msg.bin"));
        let sig_rs = String::from_utf8(sh(&format!(
            "python3 -c \"from pathlib import Path; \
             d=Path('{d}/sig.der').read_bytes(); \
             l1=d[3]; r=d[4:4+l1]; o=4+l1; l2=d[o+1]; s=d[o+2:o+2+l2]; \
             r=r.lstrip(b'\\x00').rjust(32,b'\\x00'); s=s.lstrip(b'\\x00').rjust(32,b'\\x00'); \
             import base64; print(base64.urlsafe_b64encode(r+s).decode().rstrip('='))\""))).unwrap();
        let jws = format!("{signing_input}.{}", sig_rs.trim());

        let tx = verify_signed_transaction_with_root(&jws, &root_der).expect("验签应通过");
        assert_eq!(tx.transaction_id, "10001");
        assert_eq!(tx.product_id, "com.duwei.mengxuegushi.premium.annual");
        assert_eq!(tx.environment, "Xcode");
        assert!(tx.expires_at.is_some());

        // 反例 1：篡改 payload 必挂
        let bad_payload = serde_json::json!({
            "transactionId":"10001","originalTransactionId":"10000",
            "bundleId":"com.duwei.mengxuegushi","productId":"com.duwei.mengxuegushi.premium.monthly"
        });
        let bad_input = format!("{}.{}", b64u(header.to_string().as_bytes()), b64u(bad_payload.to_string().as_bytes()));
        let bad_jws = format!("{bad_input}.{}", sig_rs.trim());
        assert!(verify_signed_transaction_with_root(&bad_jws, &root_der).is_err());

        // 反例 2：bundleId 不符必挂（签名正确也要拒）
        let payload2 = serde_json::json!({
            "transactionId":"10002","originalTransactionId":"10000",
            "bundleId":"com.other.app","productId":"x"
        });
        let input2 = format!("{}.{}", b64u(header.to_string().as_bytes()), b64u(payload2.to_string().as_bytes()));
        std::fs::write(dir.join("msg2.bin"), &input2).unwrap();
        sh(&format!("openssl dgst -sha256 -sign {d}/leaf.key -out {d}/sig2.der {d}/msg2.bin"));
        let sig2 = String::from_utf8(sh(&format!(
            "python3 -c \"from pathlib import Path; d=Path('{d}/sig2.der').read_bytes(); \
             l1=d[3]; r=d[4:4+l1]; o=4+l1; l2=d[o+1]; s=d[o+2:o+2+l2]; \
             r=r.lstrip(b'\\x00').rjust(32,b'\\x00'); s=s.lstrip(b'\\x00').rjust(32,b'\\x00'); \
             import base64; print(base64.urlsafe_b64encode(r+s).decode().rstrip('='))\""))).unwrap();
        let jws2 = format!("{input2}.{}", sig2.trim());
        let err = verify_signed_transaction_with_root(&jws2, &root_der).unwrap_err();
        assert!(err.contains("bundleId"), "应因 bundleId 拒绝，实际: {err}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
