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
    let params =
        Params::new(4096, 2, 1, None).map_err(|e| format!("argon2 params: {e}"))?;
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|e| format!("rng error: {e}"))?;
    let salt =
        SaltString::encode_b64(&salt_bytes).map_err(|e| format!("salt encode: {e}"))?;
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

fn hmac_sha256(data: &[u8], key: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn base64_url_encode(input: &[u8]) -> String {
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
    let sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    let sig_b64 = base64_url_encode(&sig);

    format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
}

/// Verify a JWT signature and check expiry. Returns the claims on success.
pub fn jwt_verify(
    token: &str,
    secret: &str,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT format".into());
    }

    // Verify signature
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    let actual_sig = base64_url_decode(parts[2])?;
    if expected_sig != actual_sig {
        return Err("invalid JWT signature".into());
    }

    // Decode payload
    let payload = base64_url_decode(parts[1])?;
    let claims: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&payload).map_err(|e| format!("invalid JWT claims: {e}"))?;

    // Check expiration
    if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if exp < now {
            return Err("JWT expired".into());
        }
    }

    Ok(claims)
}

/// Generate cryptographically random bytes.
pub fn random_bytes(n: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| format!("rng error: {e}"))?;
    Ok(buf)
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
pub fn extract_auth_meta(
    auth_header: &str,
    jwt_secret: &str,
    msg: &mut wafer_run::types::Message,
) {
    use wafer_run::meta::*;

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return,
    };

    let claims = match jwt_verify(token, jwt_secret) {
        Ok(c) => c,
        Err(_) => return,
    };

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
