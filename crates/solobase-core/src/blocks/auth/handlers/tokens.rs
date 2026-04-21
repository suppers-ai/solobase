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
