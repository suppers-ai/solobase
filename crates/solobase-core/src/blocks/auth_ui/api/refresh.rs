//! POST /b/auth/api/refresh — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{config, crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{build_auth_cookie, ensure_admin_role, generate_tokens, store_refresh_token},
        repo::sessions,
        service::hash_token,
        TOKENS_TABLE, USERS_TABLE,
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

    // Verify refresh token
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

    // Validate refresh token exists in DB (prevents use of revoked tokens)
    match db::get_by_field(
        ctx,
        TOKENS_TABLE,
        "token",
        serde_json::Value::String(body.refresh_token.clone()),
    )
    .await
    {
        Ok(_) => {} // Token exists — proceed
        Err(_) => return error_response(ErrorCode::InvalidToken, "Refresh token has been revoked"),
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

    // Revoke old refresh token family and issue new
    let family = claims
        .get("family")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if !family.is_empty() {
        db::delete_by_field(
            ctx,
            TOKENS_TABLE,
            "family",
            serde_json::Value::String(family),
        )
        .await
        .ok();
    }

    // Preserve the original auth method across refresh — a token issued
    // via OAuth must remain "oauth.<provider>" forever, not silently
    // upgrade/downgrade. Default "password" handles refresh tokens
    // minted before this claim was added.
    let prior_auth_method = claims
        .get("auth_method")
        .and_then(|v| v.as_str())
        .unwrap_or("password")
        .to_string();
    let (access_token, refresh_token, new_family) =
        match generate_tokens(ctx, &user_id, &email, &roles, &prior_auth_method).await {
            Ok(t) => t,
            Err(r) => return r,
        };

    store_refresh_token(ctx, &user_id, &refresh_token, &new_family).await;

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
