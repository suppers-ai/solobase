//! POST /b/auth/api/forgot-password — relocated from auth/login.rs in Task 5.

use wafer_core::clients::{crypto, database as db};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::USERS_TABLE,
    helpers::{err_bad_request, err_internal, hex_encode, json_map, ok_json, sha256_hex},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct Req {
        email: String,
    }
    let raw = input.collect_to_bytes().await;
    let body: Req = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let email_lower = body.email.trim().to_lowercase();
    let safe_msg = "If that email is registered, a password reset link has been sent.";

    let user = match db::get_by_field(
        ctx,
        USERS_TABLE,
        "email",
        serde_json::Value::String(email_lower.clone()),
    )
    .await
    {
        Ok(u) => u,
        Err(_) => return ok_json(&serde_json::json!({"message": safe_msg})),
    };

    // Generate reset token (expires in 1 hour). The raw token goes in the
    // email link; only its SHA-256 hex digest is persisted, so a leak of
    // the row (admin SQL explorer, backup, log dump, any block with read
    // grant on the users table) does not become a password-reset oracle.
    let reset_token = match crypto::random_bytes(ctx, 32).await {
        Ok(bytes) => hex_encode(&bytes),
        Err(e) => return err_internal("Token generation failed", e),
    };
    let reset_token_hash = sha256_hex(reset_token.as_bytes());

    let expires = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let mut data = json_map(serde_json::json!({
        "reset_token": reset_token_hash,
        "reset_token_expires": expires
    }));
    crate::blocks::helpers::stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, USERS_TABLE, &user.id, data).await {
        return err_internal("Failed to store reset token", e);
    }

    // Send the raw token in the email; the hash lives only in the DB.
    super::send_template_email(ctx, "password_reset", &email_lower, &reset_token).await;

    ok_json(&serde_json::json!({"message": safe_msg}))
}
