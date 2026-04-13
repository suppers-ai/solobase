//! Shared crypto — argon2 password hashing + HMAC-SHA256 JWT.
//!
//! Works on both native and wasm32 targets (uses `getrandom` for RNG).

use std::collections::HashMap;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Password hashing (argon2id)
// ---------------------------------------------------------------------------

/// Hash a password with argon2id. Uses lower-cost params suitable for
/// constrained environments (Workers). Native deployments may want to
/// increase these.
pub fn hash_password(password: &str) -> Result<String, String> {
    use argon2::{password_hash::SaltString, Argon2, Params, PasswordHasher};

    // 4 MiB memory, 2 iterations, 1 lane — fast enough for Workers
    let params = Params::new(4096, 2, 1, None).map_err(|e| format!("argon2 params: {e}"))?;
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|e| format!("rng error: {e}"))?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|e| format!("salt encode: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("argon2 hash: {e}"))
}

/// Verify a password against an argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// ---------------------------------------------------------------------------
// HMAC-SHA256 JWT
// ---------------------------------------------------------------------------

fn hmac_sha256(data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub fn base64_url_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(input)
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| format!("invalid base64: {e}"))
}

/// Sign a JWT with HMAC-SHA256.
pub fn jwt_sign(
    claims: &HashMap<String, serde_json::Value>,
    expiry: Duration,
    secret: &str,
) -> String {
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::seconds(expiry.as_secs() as i64);

    let mut payload = claims.clone();
    payload.insert("iat".to_string(), serde_json::json!(now.timestamp()));
    payload.insert("exp".to_string(), serde_json::json!(exp.timestamp()));

    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let header_b64 = base64_url_encode(header.as_bytes());
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();
    let payload_b64 = base64_url_encode(payload_json.as_bytes());

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let sig = match hmac_sha256(signing_input.as_bytes(), secret.as_bytes()) {
        Ok(s) => s,
        Err(_) => return String::new(), // Signing failure — return empty (unusable) token
    };
    let sig_b64 = base64_url_encode(&sig);

    format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
}

/// Verify a JWT signature and check expiry. Returns the claims on success.
pub fn jwt_verify(token: &str, secret: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT format".into());
    }

    // Verify signature (constant-time comparison to prevent timing attacks)
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes())?;
    let actual_sig = base64_url_decode(parts[2])?;
    if !constant_time_eq(&expected_sig, &actual_sig) {
        return Err("invalid JWT signature".into());
    }

    // Decode payload
    let payload = base64_url_decode(parts[1])?;
    let claims: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&payload).map_err(|e| format!("invalid JWT claims: {e}"))?;

    // Require and check expiration — tokens without exp are rejected
    let exp = claims
        .get("exp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "JWT missing exp claim".to_string())?;
    let now = chrono::Utc::now().timestamp();
    if exp < now {
        return Err("JWT expired".into());
    }

    Ok(claims)
}

/// Generate cryptographically random bytes.
pub fn random_bytes(n: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| format!("rng error: {e}"))?;
    Ok(buf)
}

/// Constant-time comparison to prevent timing attacks.
/// Returns true if both slices are equal in length and content.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Per-block JWT key derivation (must match wafer-block-crypto's HKDF)
// ---------------------------------------------------------------------------

/// Derive a per-block JWT signing key from the master secret using HKDF-SHA256.
/// This matches the derivation in `wafer-block-crypto::Argon2JwtCryptoService`.
fn derive_block_jwt_key(master_secret: &str, block_id: &str) -> String {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hk = Hkdf::<Sha256>::new(None, master_secret.as_bytes());
    let info = format!("wafer-jwt|{block_id}");
    let mut okm = [0u8; 32];
    hk.expand(info.as_bytes(), &mut okm).expect("HKDF expand");
    okm.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Auth meta extraction
// ---------------------------------------------------------------------------

/// Extract JWT claims from an `Authorization: Bearer <token>` header and
/// set auth meta fields on the message.
///
/// Sets: `auth.user_id`, `auth.user_email`, `auth.user_roles`
///
/// Silently does nothing if the token is invalid (the request continues
/// as unauthenticated).
pub fn extract_auth_meta(auth_header: &str, jwt_secret: &str, msg: &mut wafer_run::types::Message) {
    use wafer_run::meta::*;

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return,
    };

    // The auth block signs JWTs with a per-block derived key (HKDF from the
    // master secret + block ID). Try the derived key first, fall back to
    // the master secret for tokens signed without block derivation.
    let derived_secret = derive_block_jwt_key(jwt_secret, "suppers-ai/auth");
    let claims = match jwt_verify(token, &derived_secret) {
        Ok(c) => c,
        Err(_) => match jwt_verify(token, jwt_secret) {
            Ok(c) => c,
            Err(_) => return,
        },
    };

    // Only accept "access" tokens for authentication (reject refresh tokens)
    let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if token_type == "refresh" {
        return; // Refresh tokens must not be used as Bearer tokens
    }

    if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
        msg.set_meta(META_AUTH_USER_ID, sub);
    }
    if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
        msg.set_meta(META_AUTH_USER_EMAIL, email);
    }

    // Roles: check for "roles" array or legacy "role" string
    let roles = if let Some(roles_arr) = claims.get("roles").and_then(|v| v.as_array()) {
        roles_arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(",")
    } else if let Some(role) = claims.get("role").and_then(|v| v.as_str()) {
        role.to_string()
    } else {
        String::new()
    };
    msg.set_meta(META_AUTH_USER_ROLES, &roles);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_password() {
        let hash = hash_password("correcthorsebatterystaple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correcthorsebatterystaple", &hash));
        assert!(!verify_password("wrongpassword", &hash));
    }

    #[test]
    fn verify_password_rejects_garbage_hash() {
        assert!(!verify_password("anything", "not-a-hash"));
        assert!(!verify_password("anything", ""));
    }

    #[test]
    fn jwt_sign_and_verify_roundtrip() {
        let secret = "test-secret-key";
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::json!("user-123"));
        claims.insert("email".to_string(), serde_json::json!("test@example.com"));

        let token = jwt_sign(&claims, Duration::from_secs(3600), secret);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        let verified = jwt_verify(&token, secret).unwrap();
        assert_eq!(verified["sub"], "user-123");
        assert_eq!(verified["email"], "test@example.com");
        assert!(verified.contains_key("iat"));
        assert!(verified.contains_key("exp"));
    }

    #[test]
    fn jwt_verify_rejects_wrong_secret() {
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::json!("user-123"));
        let token = jwt_sign(&claims, Duration::from_secs(3600), "secret-a");

        let err = jwt_verify(&token, "secret-b").unwrap_err();
        assert_eq!(err, "invalid JWT signature");
    }

    #[test]
    fn jwt_verify_rejects_expired_token() {
        // Build a token with an exp in the past by manipulating claims directly
        let secret = "secret";
        let past = chrono::Utc::now().timestamp() - 60; // 1 minute ago
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::json!("user-123"));
        claims.insert("exp".to_string(), serde_json::json!(past));
        claims.insert("iat".to_string(), serde_json::json!(past - 3600));

        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let header_b64 = base64_url_encode(header.as_bytes());
        let payload_json = serde_json::to_string(&claims).unwrap();
        let payload_b64 = base64_url_encode(payload_json.as_bytes());
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes()).unwrap();
        let sig_b64 = base64_url_encode(&sig);
        let token = format!("{}.{}.{}", header_b64, payload_b64, sig_b64);

        let err = jwt_verify(&token, secret).unwrap_err();
        assert_eq!(err, "JWT expired");
    }

    #[test]
    fn jwt_verify_rejects_malformed_token() {
        assert!(jwt_verify("not.a.valid.jwt", "secret").is_err());
        assert!(jwt_verify("only-one-part", "secret").is_err());
        assert!(jwt_verify("two.parts", "secret").is_err());
        assert!(jwt_verify("", "secret").is_err());
    }

    #[test]
    fn jwt_verify_rejects_tampered_payload() {
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::json!("user-123"));
        let token = jwt_sign(&claims, Duration::from_secs(3600), "secret");

        let parts: Vec<&str> = token.split('.').collect();
        // Replace payload with different content
        let tampered_payload = base64_url_encode(b"{\"sub\":\"admin\",\"exp\":9999999999}");
        let tampered = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

        assert!(jwt_verify(&tampered, "secret").is_err());
    }

    #[test]
    fn random_bytes_returns_correct_length() {
        let bytes = random_bytes(32).unwrap();
        assert_eq!(bytes.len(), 32);

        let bytes2 = random_bytes(32).unwrap();
        assert_ne!(bytes, bytes2, "two calls should produce different output");
    }

    #[test]
    fn random_bytes_zero_length() {
        let bytes = random_bytes(0).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn hmac_sha256_deterministic() {
        let a = hmac_sha256(b"hello", b"key").unwrap();
        let b = hmac_sha256(b"hello", b"key").unwrap();
        assert_eq!(a, b);

        let c = hmac_sha256(b"hello", b"different-key").unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn jwt_verify_rejects_missing_exp() {
        let secret = "secret";
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::json!("user-123"));
        // Manually build a token without exp
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let header_b64 = base64_url_encode(header.as_bytes());
        let payload_json = serde_json::to_string(&claims).unwrap();
        let payload_b64 = base64_url_encode(payload_json.as_bytes());
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes()).unwrap();
        let sig_b64 = base64_url_encode(&sig);
        let token = format!("{}.{}.{}", header_b64, payload_b64, sig_b64);

        let err = jwt_verify(&token, secret).unwrap_err();
        assert_eq!(err, "JWT missing exp claim");
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(constant_time_eq(b"", b""));
    }
}
