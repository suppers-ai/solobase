//! CLI login endpoints — the browser-mediated flow that hands a freshly
//! issued PAT to a separate CLI process.
//!
//! * `post_issue` — POST `/auth/cli/issue`. Authenticated by the browser
//!   session cookie. Rejects `Authorization: Bearer` explicitly: a
//!   compromised publish-scope PAT shouldn't be able to mint a fresh CLI
//!   PAT (defeats revocation). Generates a 32-byte random code, hashes it
//!   with sha256, stores the hash in `cli_exchange_codes` with a 15-minute
//!   expiry, and returns `{code, expires_at}`. The raw code is shown once.
//!
//! * `post_exchange` — POST `/auth/cli/exchange`. **Unauthenticated**: the
//!   code itself is the credential. Atomically takes the row by hash (via
//!   `DELETE ... RETURNING`), then calls `pat::issue` with the publish scope
//!   and no expiry. Returns `{token, expires_at: null}`.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wafer_core::interfaces::auth::service::{AuthError, AuthService, TokenScope};
use wafer_run::{
    context::Context,
    types::{ErrorCode, Message, WaferError},
};

use super::HttpReply;
use crate::blocks::auth::{pat, repo::cli_codes};

/// 15 minutes, in seconds. Expressed as a constant so the insert side and
/// any future test that inspects the `expires_at` serialisation agree.
pub const CLI_CODE_TTL_SECS: i64 = 15 * 60;

fn unauthorized(detail: &str) -> HttpReply {
    HttpReply::new(401).json_body(&json!({ "error": "unauthorized", "detail": detail }))
}

/// Generate a fresh 32-byte token, url-base64 encoded (no padding). 43 chars
/// long so it easily passes the "code.len() >= 32" sanity checks in tests
/// and the user-facing `/auth/cli-login` Plan D page.
fn gen_code() -> Result<String, WaferError> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| {
        WaferError::new(ErrorCode::INTERNAL, format!("getrandom for CLI code: {e}"))
    })?;
    Ok(Base64UrlUnpadded::encode_string(&bytes))
}

fn sha256_bytes(input: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(input);
    h.finalize().to_vec()
}

/// POST `/auth/cli/issue` — session-cookie only. An explicit Authorization
/// header (even one the service would otherwise accept) is rejected: the
/// whole point of this endpoint is "only the browser that just logged in
/// can mint a CLI token." Rotating a PAT into a new one would bypass the
/// user's revocation page entirely.
pub async fn post_issue(
    ctx: &dyn Context,
    service: &dyn AuthService,
    msg: &Message,
) -> Result<HttpReply, WaferError> {
    // Reject bearer credentials outright. We do this BEFORE `require_user`
    // so a forged PAT never reaches `AuthServiceImpl` and potentially
    // bumps `last_used_at` on the stolen token.
    if !msg.header("authorization").is_empty() {
        return Ok(unauthorized("use session cookie, not bearer token"));
    }

    let user_id = match service.require_user(msg).await {
        Ok(u) => u,
        Err(AuthError::Unauthorized)
        | Err(AuthError::Forbidden)
        | Err(AuthError::NotFound) => return Ok(unauthorized("authentication required")),
        Err(AuthError::ProviderDown(m)) => {
            return Ok(
                HttpReply::new(503).json_body(&json!({ "error": "provider_down", "detail": m }))
            )
        }
        Err(AuthError::Internal(m)) => {
            return Ok(
                HttpReply::new(500).json_body(&json!({ "error": "internal", "detail": m }))
            )
        }
    };

    let raw = gen_code()?;
    let hash = sha256_bytes(raw.as_bytes());
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(CLI_CODE_TTL_SECS);
    let expires_iso = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    cli_codes::insert(
        ctx,
        cli_codes::NewCode {
            code_hash: &hash,
            user_id: &user_id.0,
            expires_at: &expires_iso,
        },
    )
    .await
    .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("cli_codes insert: {e}")))?;

    Ok(HttpReply::new(200).json_body(&json!({
        "code": raw,
        "expires_at": expires_iso,
    })))
}

fn parse_exchange(body: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("code")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// POST `/auth/cli/exchange` — UNAUTHENTICATED. The short-lived code IS the
/// credential, minted server-side by `/auth/cli/issue`. Single-use by
/// construction: `cli_codes::take` uses `DELETE ... RETURNING` so even a
/// race between two concurrent CLI processes sees at most one winner.
pub async fn post_exchange(ctx: &dyn Context, body: &[u8]) -> Result<HttpReply, WaferError> {
    let Some(raw) = parse_exchange(body) else {
        return Ok(HttpReply::new(400).json_body(&json!({
            "error": "invalid_body",
            "detail": "expected {code}",
        })));
    };

    let hash = sha256_bytes(raw.as_bytes());
    let row = match cli_codes::take(ctx, &hash)
        .await
        .map_err(|e| WaferError::new(ErrorCode::INTERNAL, format!("cli_codes take: {e}")))?
    {
        Some(r) => r,
        None => {
            return Ok(
                HttpReply::new(401).json_body(&json!({
                    "error": "invalid_code",
                    "detail": "unknown or expired code",
                })),
            )
        }
    };

    let issued = pat::issue(ctx, &row.user_id, "CLI", &[TokenScope::Publish], None).await?;
    Ok(HttpReply::new(200).json_body(&json!({
        "token": issued.raw_token,
        "expires_at": Value::Null,
    })))
}
