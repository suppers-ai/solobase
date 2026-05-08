//! POST /b/auth/api/change-password — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{crypto, database as db};
use wafer_run::{context::Context, types::Message, InputStream, OutputStream};

use crate::blocks::{
    auth::{repo::local_credentials, TOKENS_COLLECTION, USERS_COLLECTION},
    errors::{error_response, ErrorCode},
    helpers::{err_bad_request, err_internal, err_not_found, ok_json},
};

pub async fn handle(ctx: &dyn Context, msg: &Message, input: InputStream) -> OutputStream {
    let user_id = msg.user_id();
    if user_id.is_empty() {
        return error_response(ErrorCode::NotAuthenticated, "Not authenticated");
    }

    #[derive(serde::Deserialize)]
    struct ChangePwReq {
        current_password: String,
        new_password: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: ChangePwReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    if body.new_password.len() < 8 {
        return error_response(
            ErrorCode::PasswordTooShort,
            "New password must be at least 8 characters",
        );
    }
    if body.new_password.len() > 1024 {
        return error_response(
            ErrorCode::PasswordTooLong,
            "Password must not exceed 1024 characters",
        );
    }

    // Verify user exists
    match db::get(ctx, USERS_COLLECTION, user_id).await {
        Ok(_) => {}
        Err(_) => return err_not_found("User not found"),
    };

    // Fetch existing credential row — must have one to change password.
    let cred = match local_credentials::find_by_user_id(ctx, user_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return error_response(
                ErrorCode::InvalidCredentials,
                "No password set for this account",
            )
        }
        Err(e) => return err_internal(&format!("Credential lookup failed: {e}")),
    };

    if crypto::compare_hash(ctx, &body.current_password, &cred.password_hash)
        .await
        .is_err()
    {
        return error_response(
            ErrorCode::InvalidCredentials,
            "Current password is incorrect",
        );
    }

    let new_hash = match crypto::hash(ctx, &body.new_password).await {
        Ok(h) => h,
        Err(e) => return err_internal(&format!("Hash failed: {e}")),
    };

    match local_credentials::update_password(ctx, user_id, &new_hash).await {
        Ok(_) => {
            // Revoke all refresh tokens — force re-login with new password
            db::delete_by_field(
                ctx,
                TOKENS_COLLECTION,
                "user_id",
                serde_json::Value::String(user_id.to_string()),
            )
            .await
            .ok();
            ok_json(&serde_json::json!({"message": "Password changed successfully"}))
        }
        Err(e) => err_internal(&format!("Update failed: {e}")),
    }
}
