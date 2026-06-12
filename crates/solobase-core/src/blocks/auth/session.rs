//! Session issuance helper for the `suppers-ai/auth` block.
//!
//! Generates a 32-byte random token via `wafer-run/crypto`, formats it as
//! `wafer_session_<base64url-nopad>`, hashes it with sha256, inserts a row
//! into `suppers_ai__auth__sessions`, and returns the raw token + expiry.
//!
//! Callers (login handler, CLI-exchange handler) set the `wafer_session` cookie
//! with `raw_token`; the `AuthServiceImpl::require_user` side reads it back.

use base64ct::{Base64UrlUnpadded, Encoding};
use wafer_core::clients::crypto;
use wafer_run::{context::Context, ErrorCode, WaferError};

use super::{
    repo::{self, sessions},
    service::hash_token,
};

/// Prefix on the raw session token so ops engineers can eyeball a leaked
/// secret and know what it belongs to.
pub const TOKEN_PREFIX: &str = "wafer_session_";

/// Name of the session cookie both on issuance and on extraction.
pub const COOKIE_NAME: &str = "wafer_session";

/// Returned by [`issue_for`]: the raw token (show to user once) and the
/// absolute expiry (for setting the cookie `Max-Age`).
#[derive(Debug, Clone)]
pub struct IssuedSession {
    pub raw_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Issue a new session for `user_id`.
///
/// Writes a row to `suppers_ai__auth__sessions` with `token_hash` =
/// sha256(raw). The raw token is only ever returned to the caller — never
/// logged or persisted.
pub async fn issue_for(
    ctx: &dyn Context,
    user_id: &str,
    lifetime_days: u32,
) -> Result<IssuedSession, WaferError> {
    let bytes = crypto::random_bytes(ctx, 32).await?;
    let raw_token = format!("{TOKEN_PREFIX}{}", Base64UrlUnpadded::encode_string(&bytes));
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(lifetime_days as i64);
    let expires_iso = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    sessions::insert(
        ctx,
        sessions::NewSession {
            token_hash: hash_token(&raw_token),
            user_id: user_id.to_string(),
            expires_at: expires_iso,
        },
    )
    .await
    .map_err(|e: repo::RepoError| {
        WaferError::new(ErrorCode::Internal, format!("session insert: {e}"))
    })?;

    Ok(IssuedSession {
        raw_token,
        expires_at,
    })
}
