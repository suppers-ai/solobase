//! POST /b/auth/api/login — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{config, crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        helpers::{build_auth_cookie, ensure_admin_role, generate_tokens, store_refresh_token},
        repo::{local_credentials, sessions, users},
        service::hash_token,
        DUMMY_HASH, USERS_TABLE,
    },
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, json_map, RecordExt, ResponseBuilder},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct LoginReq {
        email: String,
        password: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: LoginReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let email_lower = body.email.trim().to_lowercase();

    // Find user by email via typed repo.
    let user_row = users::find_by_email(ctx, &email_lower).await.ok().flatten();

    // Always run Argon2 verification to prevent timing-based user enumeration.
    // If user not found or no local credentials, compare against a dummy hash
    // so the response time is indistinguishable from a wrong-password attempt.
    let stored_hash_owned: String;
    let stored_hash: &str = match &user_row {
        Some(u) => match local_credentials::find_by_user_id(ctx, &u.id).await {
            Ok(Some(cred)) => {
                stored_hash_owned = cred.password_hash;
                &stored_hash_owned
            }
            _ => DUMMY_HASH,
        },
        None => DUMMY_HASH,
    };
    let password_ok = crypto::compare_hash(ctx, &body.password, stored_hash)
        .await
        .is_ok();

    // Fetch the full DB record for the remaining fields (disabled, email_verified, etc.)
    let user = match user_row {
        Some(u) if password_ok => match db::get(ctx, USERS_TABLE, &u.id).await {
            Ok(rec) => rec,
            Err(_) => {
                return error_response(ErrorCode::InvalidCredentials, "Invalid email or password")
            }
        },
        _ => return error_response(ErrorCode::InvalidCredentials, "Invalid email or password"),
    };

    // Check if user is disabled
    if user.bool_field("disabled") {
        return error_response(ErrorCode::AccountDisabled, "Account is disabled");
    }

    // Check email verification if required
    let require_verification =
        config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
    if (require_verification == "true" || require_verification == "1")
        && !user.bool_field("email_verified")
    {
        return error_response(ErrorCode::EmailNotVerified, "Please verify your email before logging in. Check your inbox for the verification link.");
    }

    // Get roles, granting admin role idempotently when ADMIN_EMAIL matches
    let roles = ensure_admin_role(ctx, &user.id, &email_lower).await;

    // Generate tokens
    let (access_token, refresh_token, family) =
        match generate_tokens(ctx, &user.id, &email_lower, &roles, "password").await {
            Ok(t) => t,
            Err(r) => return r,
        };

    // Store refresh token
    store_refresh_token(ctx, &user.id, &refresh_token, &family).await;

    // Persist a session row so the userportal `/b/userportal/sessions`
    // page can show this login. Failure must not block login — the
    // session row is a UX signal, not a security gate (auth is still
    // entirely JWT-based today).
    if let Err(e) = sessions::create_for_user(ctx, &user.id, hash_token(&access_token), 1).await {
        tracing::warn!(
            user_id = %user.id,
            "failed to persist session row for JWT login: {e}"
        );
    }

    // Update last login
    let upd = json_map(serde_json::json!({"last_login_at": crate::blocks::helpers::now_rfc3339()}));
    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, upd).await {
        tracing::warn!("Failed to update last login time: {e}");
    }

    let cookie = build_auth_cookie(&access_token, 86400, ctx).await;

    ResponseBuilder::new()
        .set_cookie(&cookie)
        .json(&serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "token_type": "Bearer",
            "expires_in": 86400,
            "user": {
                "id": user.id,
                "email": email_lower,
                "roles": roles,
                "name": user.str_field("name")
            }
        }))
}
