//! POST /b/auth/api/login — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{config, crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::{
    blocks::{
        auth::{
            helpers::{ensure_admin_role, issue_tokens_and_cookie},
            repo::{local_credentials, users},
            DUMMY_HASH, USERS_TABLE,
        },
        errors::{error_response, ErrorCode},
    },
    http::{err_bad_request, err_internal, ResponseBuilder},
    util::json_map,
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

    // Find user by email via typed repo. `users::find_by_email` already
    // collapses NOT_FOUND to `Ok(None)`; any `Err` here is a real failure
    // (WRAP denial, DB outage) that must not collapse to "invalid
    // credentials" — that would mask outages and silently log users out.
    let user_row = match users::find_by_email(ctx, &email_lower).await {
        Ok(opt) => opt,
        Err(e) => return err_internal("User lookup failed", e),
    };

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

    // Use the typed UserRow we already have. `disabled` and `email_verified`
    // ride on the row, so no second `db::get` is needed.
    let user = match user_row {
        Some(u) if password_ok => u,
        _ => return error_response(ErrorCode::InvalidCredentials, "Invalid email or password"),
    };

    // [SEC-034] Disabled accounts return the SAME generic invalid-credentials
    // response as a wrong-password attempt. Surfacing "account is disabled"
    // confirms to an attacker that the email exists, gives them a target for
    // a re-enable social-engineering attack, and signals when an admin has
    // taken action on a compromised account.
    if !user.is_active() {
        return error_response(ErrorCode::InvalidCredentials, "Invalid email or password");
    }

    // Check email verification if required
    let require_verification =
        config::get_default(ctx, "SUPPERS_AI__AUTH__REQUIRE_VERIFICATION", "false").await;
    if (require_verification == "true" || require_verification == "1") && !user.email_verified {
        return error_response(ErrorCode::EmailNotVerified, "Please verify your email before logging in. Check your inbox for the verification link.");
    }

    // Get roles, granting admin role idempotently when ADMIN_EMAIL matches.
    // A WRAP denial or DB error here must not silently resolve to "no
    // roles" — that would 403 an admin or double-grant on the next login
    // (SB-3).
    let roles = match ensure_admin_role(ctx, &user.id, &email_lower).await {
        Ok(r) => r,
        Err(e) => return err_internal("Failed to resolve user roles", e),
    };

    // Mint tokens, persist the refresh + session rows, build the cookie.
    let issued =
        match issue_tokens_and_cookie(ctx, &user.id, &email_lower, &roles, "password", None, 0)
            .await
        {
            Ok(i) => i,
            Err(r) => return r,
        };

    // Update last login
    let upd = json_map(serde_json::json!({"last_login_at": crate::util::now_rfc3339()}));
    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, upd).await {
        tracing::warn!("Failed to update last login time: {e}");
    }

    ResponseBuilder::new()
        .set_cookie(&issued.cookie)
        .json(&serde_json::json!({
            "access_token": issued.access_token,
            "refresh_token": issued.refresh_token,
            "token_type": "Bearer",
            "expires_in": issued.access_lifetime,
            "user": {
                "id": user.id,
                "email": email_lower,
                "roles": roles,
                "name": user.display_name
            }
        }))
}
