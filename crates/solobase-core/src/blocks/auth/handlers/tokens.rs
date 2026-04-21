//! PAT CRUD handlers under `/auth/tokens`.
//!
//! Tasks 8–10 in Plan A2 Cluster B:
//! * GET `/auth/tokens` — list PATs the caller owns (Task 8).
//!
//! Task 9 (POST) and Task 10 (DELETE) append to this file in later commits.
//!
//! The `id` in responses is the hex-encoded `token_hash`, since the
//! `personal_access_tokens` table uses the hash as its primary key rather
//! than a separate surrogate id.

use serde_json::{json, Value};
use wafer_core::interfaces::auth::service::{AuthError, AuthService};
use wafer_run::{
    context::Context,
    types::{Message, WaferError},
};

use super::HttpReply;
use crate::blocks::auth::repo::pats;

/// Render a byte slice as a lowercase hex string (used as the public PAT id).
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn unauthorized() -> HttpReply {
    HttpReply::new(401).json_body(&json!({ "error": "unauthorized" }))
}

/// Require an authenticated user or return an HttpReply to short-circuit.
pub(crate) async fn require_user_or_reply(
    service: &dyn AuthService,
    msg: &Message,
) -> Result<String, HttpReply> {
    match service.require_user(msg).await {
        Ok(u) => Ok(u.0),
        Err(AuthError::Unauthorized)
        | Err(AuthError::Forbidden)
        | Err(AuthError::NotFound) => Err(unauthorized()),
        Err(AuthError::ProviderDown(m)) => Err(HttpReply::new(503)
            .json_body(&json!({ "error": "provider_down", "detail": m }))),
        Err(AuthError::Internal(m)) => {
            Err(HttpReply::new(500).json_body(&json!({ "error": "internal", "detail": m })))
        }
    }
}

/// GET `/auth/tokens` — list PATs belonging to the authenticated user.
/// `token_hash` is never serialised back to the client; only the hex id is.
pub async fn list_tokens(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
) -> Result<HttpReply, WaferError> {
    let user_id = match require_user_or_reply(service, msg).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };

    let rows = pats::list_for_user(ctx, &user_id).await.map_err(|e| {
        WaferError::new(
            wafer_run::types::ErrorCode::INTERNAL,
            format!("pats list: {e}"),
        )
    })?;

    let view: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": hex(&r.token_hash),
                "name": r.name,
                "scopes": r.scopes,
                "created_at": r.created_at,
                "last_used_at": r.last_used_at,
                "expires_at": r.expires_at,
            })
        })
        .collect();

    Ok(HttpReply::new(200).json_body(&Value::Array(view)))
}

// ---------------------------------------------------------------------------
// Task 9: POST /auth/tokens
// ---------------------------------------------------------------------------

/// Parse a JSON create-token body into `(name, scopes, expires_at)`.
///
/// Returns `Err(HttpReply)` with a 422 if `name` is missing or empty — the
/// row has a `NOT NULL` constraint on `name` and blank names make the PAT
/// list unreadable.
fn parse_create_body(
    body: &[u8],
) -> Result<
    (
        String,
        Vec<wafer_core::interfaces::auth::service::TokenScope>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
    HttpReply,
> {
    use wafer_core::interfaces::auth::service::TokenScope;
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        return Err(HttpReply::new(422).json_body(&json!({ "error": "name_required" })));
    }
    let scopes: Vec<TokenScope> = v
        .get("scopes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .filter_map(|s| match s {
                    "publish" => Some(TokenScope::Publish),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let expires_at = v
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    Ok((name, scopes, expires_at))
}

/// POST `/auth/tokens` — issue a new PAT. Returns the raw token exactly
/// once in the response body; it is not retrievable again.
pub async fn create_token(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
    body: &[u8],
) -> Result<HttpReply, WaferError> {
    use crate::blocks::auth::{pat, service::hash_token};

    let user_id = match require_user_or_reply(service, msg).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let (name, scopes, expires_at) = match parse_create_body(body) {
        Ok(t) => t,
        Err(r) => return Ok(r),
    };

    let issued = pat::issue(ctx, &user_id, &name, &scopes, expires_at).await?;
    let scopes_str: Vec<&str> = scopes
        .iter()
        .map(|s| match s {
            wafer_core::interfaces::auth::service::TokenScope::Publish => "publish",
        })
        .collect();
    let payload = json!({
        "id": hex(&hash_token(&issued.raw_token)),
        "name": name,
        "scopes": scopes_str,
        "token": issued.raw_token,
        "expires_at": issued.expires_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    });
    Ok(HttpReply::new(201).json_body(&payload))
}

// ---------------------------------------------------------------------------
// Task 10: DELETE /auth/tokens/{id}
// ---------------------------------------------------------------------------

fn not_found() -> HttpReply {
    HttpReply::new(404).json_body(&json!({ "error": "not_found" }))
}

/// Decode a hex-encoded id back to its raw token-hash bytes. Returns `None`
/// if the input is not valid hex.
fn unhex(id: &str) -> Option<Vec<u8>> {
    if id.is_empty() || id.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(id.len() / 2);
    let bytes = id.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = char_to_nib(bytes[i])?;
        let lo = char_to_nib(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn char_to_nib(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// DELETE `/auth/tokens/{id}` — revoke a PAT owned by the caller.
///
/// Returns 204 on success, 404 if the id is malformed, doesn't exist, or
/// belongs to someone else. The not-found-vs-not-owned distinction is
/// intentionally collapsed so the endpoint doesn't leak "this token exists
/// on another account" to probes.
pub async fn delete_token(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
    id: &str,
) -> Result<HttpReply, WaferError> {
    let user_id = match require_user_or_reply(service, msg).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let Some(hash) = unhex(id) else {
        return Ok(not_found());
    };
    let deleted = pats::delete_by_id(ctx, &user_id, &hash).await.map_err(|e| {
        WaferError::new(
            wafer_run::types::ErrorCode::INTERNAL,
            format!("pats delete: {e}"),
        )
    })?;
    if deleted {
        Ok(HttpReply::new(204))
    } else {
        Ok(not_found())
    }
}
