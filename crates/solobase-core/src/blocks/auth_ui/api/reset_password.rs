//! POST /b/auth/api/reset-password — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        repo::{local_credentials, tokens},
        USERS_TABLE,
    },
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, err_internal, json_map, ok_json, RecordExt},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        token: String,
        new_password: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.new_password.len() < 8 {
        return error_response(
            ErrorCode::PasswordTooShort,
            "Password must be at least 8 characters",
        );
    }
    if body.new_password.len() > 1024 {
        return error_response(
            ErrorCode::PasswordTooLong,
            "Password must not exceed 1024 characters",
        );
    }

    // Find user by reset token
    let user = match db::get_by_field(
        ctx,
        USERS_TABLE,
        "reset_token",
        serde_json::Value::String(body.token.clone()),
    )
    .await
    {
        Ok(u) => u,
        Err(_) => return error_response(ErrorCode::InvalidToken, "Invalid or expired reset token"),
    };

    // Check expiry — reject if missing or malformed (tokens must have an expiry)
    let expires = user.str_field("reset_token_expires");
    if expires.is_empty() {
        return error_response(
            ErrorCode::TokenExpired,
            "Reset token has expired. Please request a new one.",
        );
    }
    match chrono::DateTime::parse_from_rfc3339(expires) {
        Ok(exp) => {
            if chrono::Utc::now() > exp.with_timezone(&chrono::Utc) {
                return error_response(
                    ErrorCode::TokenExpired,
                    "Reset token has expired. Please request a new one.",
                );
            }
        }
        Err(_) => {
            return error_response(
                ErrorCode::TokenExpired,
                "Reset token has expired. Please request a new one.",
            );
        }
    }

    // Hash new password
    let new_hash = match crypto::hash(ctx, &body.new_password).await {
        Ok(h) => h,
        Err(e) => return err_internal(&format!("Hash failed: {e}")),
    };

    // Update credential row (typed path, no password_hash on users table).
    if let Err(e) = local_credentials::update_password(ctx, &user.id, &new_hash).await {
        return err_internal(&format!("Failed to update password: {e}"));
    }

    // Clear reset token on the users row.
    let mut data = json_map(serde_json::json!({
        "reset_token": "",
        "reset_token_expires": ""
    }));
    crate::blocks::helpers::stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, data).await {
        return err_internal(&format!("Failed to clear reset token: {e}"));
    }

    // Revoke all refresh tokens — invalidate any stolen sessions.
    // SEC-032/039: mark rows revoked (don't delete) so the reuse-detection
    // tombstones survive across the password reset.
    tokens::revoke_all_for_user(ctx, &user.id).await.ok();

    ok_json(&serde_json::json!({"message": "Password reset successfully"}))
}
