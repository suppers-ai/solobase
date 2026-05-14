//! POST /b/auth/api/refresh — relocated from auth/login.rs in Task 5.
//!
//! Token-rotation flow for refresh JWTs. Implements the family-rotation
//! reuse-detection pattern (SEC-039):
//!
//! 1. Hash the incoming refresh token, look up the row by `token_hash` (SEC-032).
//! 2. If the row is `revoked = 1`, that token was already rotated away — a
//!    legitimate client would only have the *current* token. Revoke the
//!    entire family.
//! 3. If the row is live, mark it revoked and insert a new row under the
//!    same family ID with `generation + 1`. Return the new access + refresh
//!    pair.

use wafer_core::clients::{config, crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{
            build_auth_cookie, ensure_admin_role, expected_issuer, generate_tokens,
            store_refresh_token,
        },
        repo::{sessions, tokens},
        service::hash_token,
        USERS_TABLE,
    },
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, RecordExt, ResponseBuilder},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct RefreshReq {
        refresh_token: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: RefreshReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Verify the JWT signature/expiry. A valid signature alone is not enough
    // — the row lookup below is the source of truth for "this token has not
    // been used or revoked yet".
    let claims = match crypto::verify(ctx, &body.refresh_token).await {
        Ok(c) => c,
        Err(_) => {
            return error_response(ErrorCode::InvalidToken, "Invalid or expired refresh token")
        }
    };

    let user_id = claims
        .get("user_id")
        .or_else(|| claims.get("sub"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if user_id.is_empty() {
        return error_response(ErrorCode::InvalidToken, "Invalid refresh token");
    }

    let token_type = claims.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if token_type != "refresh" {
        return error_response(ErrorCode::InvalidToken, "Not a refresh token");
    }

    // [SEC-038] Require the iss claim to match this deployment. A refresh
    // token minted against a different SOLOBASE_SHARED__FRONTEND_URL value
    // (e.g. a leaked staging secret) must not refresh into a production
    // access token.
    let expected_iss = expected_issuer(ctx).await;
    let iss = claims.get("iss").and_then(|v| v.as_str()).unwrap_or("");
    if iss != expected_iss {
        return error_response(ErrorCode::InvalidToken, "Invalid or expired refresh token");
    }

    // SEC-032: look up the row by SHA-256 hash of the JWT — the raw token
    // is never stored, only its hash.
    let row = match tokens::find_by_token(ctx, &body.refresh_token).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            // Signature was valid but no row exists — token was rotated and
            // its tombstone has since been wiped, or this is a forged
            // refresh token whose family we never minted. Either way, no
            // family to revoke; just refuse.
            return error_response(ErrorCode::InvalidToken, "Refresh token has been revoked");
        }
        Err(e) => {
            tracing::warn!("refresh: token lookup failed: {e}");
            return error_response(ErrorCode::InvalidToken, "Invalid refresh token");
        }
    };

    if row.revoked {
        // SEC-039: a revoked token surfaced. If the family still has a live
        // row, an attacker is replaying a stolen token after legitimate
        // rotation. Burn the whole family.
        if let Ok(true) = tokens::family_has_live_row(ctx, &row.family).await {
            tracing::warn!(
                user_id = %row.user_id,
                family = %row.family,
                "refresh: token reuse detected; revoking entire family"
            );
            let _ = tokens::revoke_family(ctx, &row.family).await;
        }
        return error_response(ErrorCode::InvalidToken, "Refresh token has been revoked");
    }

    // Get user and verify account is still active
    let user = match db::get(ctx, USERS_TABLE, &user_id).await {
        Ok(u) => u,
        Err(_) => return error_response(ErrorCode::NotAuthenticated, "User not found"),
    };

    if user.bool_field("disabled") {
        return error_response(ErrorCode::AccountDisabled, "Account is disabled");
    }

    let require_verification =
        config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
    if (require_verification == "true" || require_verification == "1")
        && !user.bool_field("email_verified")
    {
        return error_response(ErrorCode::EmailNotVerified, "Email not verified");
    }

    let email = user.str_field("email").to_string();
    let roles = ensure_admin_role(ctx, &user_id, &email).await;

    // Preserve the original auth method across refresh — a token issued
    // via OAuth must remain "oauth.<provider>" forever, not silently
    // upgrade/downgrade. Default "password" handles refresh tokens
    // minted before this claim was added.
    let prior_auth_method = claims
        .get("auth_method")
        .and_then(|v| v.as_str())
        .unwrap_or("password")
        .to_string();

    // Mint the new access JWT via the shared helper. We discard its refresh
    // JWT because `generate_tokens` allocates a fresh family for it — we
    // need the new refresh JWT to carry the *preserved* family so reuse
    // detection survives across rotation (SEC-039).
    let (access_token, _discarded_refresh, _discarded_new_family) =
        match generate_tokens(ctx, &user_id, &email, &roles, &prior_auth_method).await {
            Ok(t) => t,
            Err(r) => return r,
        };
    let refresh_token =
        match resign_refresh_with_family(ctx, &user_id, &prior_auth_method, &row.family).await {
            Ok(t) => t,
            Err(r) => return r,
        };

    // Atomic-ish rotation: mark old row revoked, insert new row under the
    // same family with generation+1. If the second step fails after the
    // first succeeds the user simply gets logged out — a recoverable UX
    // outcome — but we never leave two live rows in a family.
    if let Err(e) = tokens::revoke_by_id(ctx, &row.id).await {
        tracing::warn!("refresh: failed to revoke prior token row: {e}");
        return error_response(ErrorCode::InvalidToken, "Could not rotate refresh token");
    }
    store_refresh_token(
        ctx,
        &user_id,
        &refresh_token,
        &row.family,
        row.generation + 1,
    )
    .await;

    if let Err(e) = sessions::create_for_user(ctx, &user_id, hash_token(&access_token), 1).await {
        tracing::warn!(
            user_id = %user_id,
            "failed to persist session row for JWT refresh: {e}"
        );
    }

    let access_lifetime = crate::blocks::auth::helpers::access_token_lifetime_secs(ctx).await;
    let cookie = build_auth_cookie(&access_token, access_lifetime, ctx).await;

    ResponseBuilder::new()
        .set_cookie(&cookie)
        .json(&serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "token_type": "Bearer",
            "expires_in": access_lifetime
        }))
}

/// Mint a refresh JWT with a caller-chosen `family` claim.
///
/// `generate_tokens` always allocates a fresh family. On rotation we need to
/// preserve the prior family so the JWT's `family` claim agrees with the DB
/// row, which is the anchor for SEC-039 reuse detection. Rather than reshape
/// the shared helper (it has four other callers — login, signup, OAuth
/// callback, bootstrap — that legitimately want a new family), we re-sign
/// with the desired family here.
async fn resign_refresh_with_family(
    ctx: &dyn Context,
    user_id: &str,
    auth_method: &str,
    family: &str,
) -> std::result::Result<String, OutputStream> {
    use std::{collections::HashMap, time::Duration};

    use crate::blocks::auth::REFRESH_TOKEN_TTL_SECS;

    let mut refresh_claims = HashMap::new();
    refresh_claims.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.to_string()),
    );
    refresh_claims.insert(
        "sub".to_string(),
        serde_json::Value::String(user_id.to_string()),
    );
    refresh_claims.insert(
        "type".to_string(),
        serde_json::Value::String("refresh".to_string()),
    );
    refresh_claims.insert(
        "family".to_string(),
        serde_json::Value::String(family.to_string()),
    );
    refresh_claims.insert(
        "auth_method".to_string(),
        serde_json::Value::String(auth_method.to_string()),
    );

    crypto::sign(
        ctx,
        &refresh_claims,
        Duration::from_secs(REFRESH_TOKEN_TTL_SECS),
    )
    .await
    .map_err(OutputStream::error)
}
