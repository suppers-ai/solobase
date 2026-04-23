//! POST `/auth/login` + POST `/auth/logout` handlers.
//!
//! * `post_login` — parses a JSON `{email, password}` body, looks up the user,
//!   runs `crypto::compare_hash` (against a dummy hash when the user isn't
//!   found, so timing doesn't leak enumeration), issues a fresh session via
//!   [`session::issue_for`], and returns a 303 redirect with the
//!   `wafer_session` cookie set.
//! * `post_logout` — deletes the session row identified by the `wafer_session`
//!   cookie and returns a `Max-Age=0` cookie so the browser discards it.
//!
//! Both handlers return [`HttpReply`] values; the HTTP-adapter wrapper in
//! `handlers::mod` converts them to `OutputStream`s.

use serde_json::{json, Value};
use wafer_core::{clients::crypto, interfaces::auth::service::UserId};
use wafer_run::{
    context::Context,
    types::{ErrorCode, Message, WaferError},
};

use super::HttpReply;
use crate::blocks::auth::{
    cache::OrgAdminCache,
    config::AuthConfig,
    repo::{local_credentials, sessions, users},
    service::hash_token,
    session,
};

/// Pre-computed Argon2id hash used for timing equalisation when the user
/// isn't found. The exact password it hashes doesn't matter — we never check
/// against it; we only care that `compare_hash` does the same amount of work
/// as for a real row.
pub const DUMMY_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzYWx0$/Bh63qVyV9NsRyKTshNxdmnQefw3+grDDlPhv6xiyiw";

/// Parse the JSON body of a login request. Missing / malformed fields are
/// treated as empty strings so the handler can uniformly return 401.
fn parse_login(body: &[u8]) -> (String, String) {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let email = v
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let password = v
        .get("password")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    (email, password)
}

/// Build the `wafer_session=...` Set-Cookie header for a freshly-issued
/// session. `Secure` is intentionally always set — Plan A2 is HTTPS-only in
/// production; dev/test callers are expected to strip the flag at the HTTP
/// adapter layer if they truly need to.
fn set_cookie_header(raw_token: &str, lifetime_days: u32) -> (String, String) {
    let max_age = (lifetime_days as u64) * 24 * 60 * 60;
    (
        "Set-Cookie".into(),
        format!(
            "{name}={raw}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age}",
            name = session::COOKIE_NAME,
            raw = raw_token,
        ),
    )
}

fn clear_cookie_header() -> (String, String) {
    (
        "Set-Cookie".into(),
        format!(
            "{name}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0",
            name = session::COOKIE_NAME,
        ),
    )
}

fn unauthorized() -> HttpReply {
    HttpReply::new(401).json_body(&json!({ "error": "invalid_credentials" }))
}

/// POST `/auth/login` — email+password exchange for a session cookie.
pub async fn post_login(
    ctx: &dyn Context,
    cfg: &AuthConfig,
    body: &[u8],
) -> Result<HttpReply, WaferError> {
    let (email, password) = parse_login(body);
    let email_lower = email.trim().to_lowercase();

    let user = users::find_by_email(ctx, &email_lower)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("users lookup: {e}")))?;

    // Pick a hash to verify against. Unknown-user path still hits
    // `compare_hash` so the response latency is indistinguishable.
    let hash_to_verify: String = match &user {
        Some(u) => local_credentials::find_by_user_id(ctx, &u.id)
            .await
            .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("credentials: {e}")))?
            .map(|c| c.password_hash)
            .unwrap_or_else(|| DUMMY_HASH.to_string()),
        None => DUMMY_HASH.to_string(),
    };

    let verify = crypto::compare_hash(ctx, &password, &hash_to_verify).await;
    let Some(user) = user.filter(|_| verify.is_ok()) else {
        return Ok(unauthorized());
    };

    let issued = session::issue_for(ctx, &user.id, cfg.session_lifetime_days).await?;
    let (set_name, set_val) = set_cookie_header(&issued.raw_token, cfg.session_lifetime_days);
    Ok(HttpReply::new(303)
        .header(set_name, set_val)
        .header("Location", "/"))
}

/// POST `/auth/logout` — delete the session row identified by the
/// `wafer_session` cookie (if any), drop every `OrgAdminCache` entry for
/// the session's user, and clear the cookie on the client.
///
/// Invalidating the cache here means a user whose upstream permissions
/// were revoked during the session won't retain admin access for up to
/// the cache's TTL after signing out.
pub async fn post_logout(
    ctx: &dyn Context,
    msg: &Message,
    org_admin_cache: &OrgAdminCache,
) -> Result<HttpReply, WaferError> {
    let raw = msg.cookie(session::COOKIE_NAME);
    if !raw.is_empty() {
        let hash = hash_token(raw);
        // Look up the session owner before deleting so the cache key is
        // recoverable. Missing row is fine: best-effort.
        if let Ok(Some(row)) = sessions::find_by_token_hash(ctx, &hash).await {
            let _ = sessions::delete_by_token_hash(ctx, &hash).await;
            org_admin_cache.invalidate_user(&UserId(row.user_id));
        } else {
            // Row already gone (idempotent logout) — nothing to invalidate.
            let _ = sessions::delete_by_token_hash(ctx, &hash).await;
        }
    }
    let (cname, cval) = clear_cookie_header();
    Ok(HttpReply::new(204).header(cname, cval))
}
