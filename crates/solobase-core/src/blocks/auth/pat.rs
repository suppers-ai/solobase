//! Personal access token issuance helper for the `suppers-ai/auth` block.
//!
//! Generates a 32-byte random token via `wafer-run/crypto`, formats it as
//! `wafer_pat_<base64url-nopad>`, hashes it with sha256, and inserts a row
//! into `suppers_ai__auth__personal_access_tokens`. The raw token is
//! returned exactly once; all subsequent lookups go through `token_hash`.

use base64ct::{Base64UrlUnpadded, Encoding};
use wafer_core::{clients::crypto, interfaces::auth::service::TokenScope};
use wafer_run::{
    context::Context,
    types::{ErrorCode, WaferError},
};

use super::{
    repo::{self, pats},
    service::hash_token,
};

/// Prefix on the raw PAT so ops engineers can eyeball a leaked secret and
/// know what it belongs to.
pub const TOKEN_PREFIX: &str = "wafer_pat_";

/// Returned by [`issue`]: the raw token (show to user once) and the
/// optional absolute expiry.
#[derive(Debug, Clone)]
pub struct IssuedPat {
    pub raw_token: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Map a [`TokenScope`] variant to the canonical string stored in the
/// `scopes` TEXT column.
fn scope_str(s: TokenScope) -> &'static str {
    match s {
        TokenScope::Publish => "publish",
    }
}

/// Issue a new personal access token for `user_id`.
///
/// `name` is the human-friendly label shown in the user's PAT list. `scopes`
/// is the set of operations the token is allowed to perform. `expires_at`
/// is optional — `None` means "never expires".
pub async fn issue(
    ctx: &dyn Context,
    user_id: &str,
    name: &str,
    scopes: &[TokenScope],
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<IssuedPat, WaferError> {
    let bytes = crypto::random_bytes(ctx, 32).await?;
    let raw_token = format!("{TOKEN_PREFIX}{}", Base64UrlUnpadded::encode_string(&bytes));

    let scopes_str: Vec<String> = scopes
        .iter()
        .copied()
        .map(scope_str)
        .map(str::to_owned)
        .collect();
    let expires_iso = expires_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string());

    pats::insert(
        ctx,
        pats::NewPat {
            token_hash: hash_token(&raw_token),
            user_id: user_id.to_string(),
            name: name.to_string(),
            scopes: scopes_str,
            expires_at: expires_iso,
        },
    )
    .await
    .map_err(|e: repo::RepoError| {
        WaferError::new(ErrorCode::INTERNAL, format!("pat insert: {e}"))
    })?;

    Ok(IssuedPat {
        raw_token,
        expires_at,
    })
}
