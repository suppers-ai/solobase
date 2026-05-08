//! POST /b/auth/api/bootstrap — bootstrap admin token redemption.
//!
//! The first-boot bootstrap flow can install a sha256(token) row into
//! `suppers_ai__auth__bootstrap_tokens` (24h expiry) instead of creating an
//! admin user directly — see [`crate::blocks::auth::bootstrap`]. This
//! handler is the redemption side: holder posts the raw token + chosen
//! email/password, we verify the token row, create the admin user via the
//! same code path the env-var bootstrap uses, consume the token row, and
//! mint a session cookie identical to the one [`super::login`] would issue.
//!
//! Request body is `application/x-www-form-urlencoded` (the GET page
//! submits a plain HTML form — no JS).

use sha2::{Digest, Sha256};
use wafer_run::{context::Context, InputStream, OutputStream};

use crate::blocks::{
    auth::{
        bootstrap,
        helpers::{build_auth_cookie, generate_tokens, store_refresh_token},
        repo::{bootstrap_tokens, sessions, users},
        service::hash_token,
    },
    helpers::{err_bad_request, err_internal, err_unauthorized, parse_form_body, ResponseBuilder},
};

pub async fn handle(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let form = parse_form_body(&raw);
    let token = match form.get("token") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return err_bad_request("missing token"),
    };
    let email = match form.get("email") {
        Some(e) if !e.is_empty() => e.trim().to_lowercase(),
        _ => return err_bad_request("missing email"),
    };
    let password = match form.get("password") {
        Some(p) if !p.is_empty() => p.clone(),
        _ => return err_bad_request("missing password"),
    };
    if password.len() < 8 {
        return err_bad_request("password must be at least 8 characters");
    }

    let token_hash = Sha256::digest(token.as_bytes()).to_vec();

    // 1. Verify the token row exists and hasn't expired.
    match bootstrap_tokens::is_valid(ctx, &token_hash).await {
        Ok(true) => {}
        Ok(false) => return err_unauthorized("invalid or expired bootstrap token"),
        Err(e) => return err_internal(&format!("bootstrap_tokens lookup: {e}")),
    }

    // 2. Create the admin user via the same code path bootstrap-on-init uses.
    //    Reusing this keeps the legacy companion columns (`name`, `disabled`,
    //    `deleted_at`) and the local_credentials row consistent with the
    //    env-var path.
    if let Err(e) = bootstrap::bootstrap_with_email_password(ctx, &email, &password).await {
        return err_internal(&format!("create admin: {e}"));
    }

    // 3. Consume the token row. Best-effort: the admin user already exists,
    //    so a delete failure here just leaves a stale row that will expire on
    //    its own.
    if let Err(e) = bootstrap_tokens::delete_by_hash(ctx, &token_hash).await {
        tracing::warn!(
            "bootstrap_tokens::delete_by_hash after redemption failed (admin already created): {e}"
        );
    }

    // 4. Look up the just-created user so we have its id for session minting.
    let user = match users::find_by_email(ctx, &email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return err_internal("bootstrap created admin but find_by_email returned no row")
        }
        Err(e) => return err_internal(&format!("users::find_by_email after bootstrap: {e}")),
    };

    // 5. Mint a session — same pattern as auth_ui::api::login::handle.
    let roles = vec!["admin".to_string()];
    let (access_token, refresh_token, family) =
        match generate_tokens(ctx, &user.id, &email, &roles, "password").await {
            Ok(t) => t,
            Err(r) => return r,
        };
    store_refresh_token(ctx, &user.id, &refresh_token, &family).await;
    if let Err(e) = sessions::create_for_user(ctx, &user.id, hash_token(&access_token), 1).await {
        tracing::warn!(
            user_id = %user.id,
            "failed to persist session row for bootstrap redemption: {e}"
        );
    }

    // 6. Set the auth cookie + redirect to the dashboard. The form is a
    //    plain HTML POST (no JS), so a 302 with Set-Cookie is the right
    //    completion signal.
    let cookie = build_auth_cookie(&access_token, 86400, ctx).await;
    ResponseBuilder::new()
        .status(302)
        .set_cookie(&cookie)
        .set_header("Location", "/b/auth/dashboard")
        .empty()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::test_support::TestContext;

    /// Register a real crypto block on the test context — bootstrap admin
    /// creation goes through `crypto::hash` for the password, and session
    /// minting goes through `crypto::sign`/`random_bytes`. Without this the
    /// handler trips on `block 'wafer-run/crypto' not registered`.
    async fn ctx_with_crypto() -> TestContext {
        let mut ctx = TestContext::with_auth().await;
        let svc = Arc::new(wafer_block_crypto::service::Argon2JwtCryptoService::new(
            "test-jwt-secret".to_string(),
        ));
        let crypto_block: Arc<dyn wafer_run::block::Block> =
            Arc::new(wafer_core::service_blocks::crypto::CryptoBlock::new(svc));
        ctx.register_block("wafer-run/crypto", crypto_block);
        ctx
    }

    #[tokio::test]
    async fn redeems_valid_token_creates_admin_and_consumes_row() {
        let ctx = ctx_with_crypto().await;

        // Seed a bootstrap token row (sha256 of "test-token-xyz").
        let raw = "test-token-xyz";
        let hash = Sha256::digest(raw.as_bytes()).to_vec();
        let expires = (chrono::Utc::now() + chrono::Duration::hours(24))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        bootstrap_tokens::insert(&ctx, hash.clone(), &expires)
            .await
            .unwrap();

        // POST the form.
        let form = format!("token={raw}&email=admin@example.com&password=test1234");
        let input = InputStream::from_bytes(form.into_bytes());
        let _ = handle(&ctx, input).await;

        // Admin user got created.
        let user = users::find_by_email(&ctx, "admin@example.com")
            .await
            .unwrap()
            .expect("admin user created");
        assert_eq!(user.role, "admin");

        // Bootstrap-token row consumed.
        assert!(!bootstrap_tokens::is_valid(&ctx, &hash).await.unwrap());
    }

    #[tokio::test]
    async fn rejects_invalid_token() {
        let ctx = ctx_with_crypto().await;
        let form = "token=wrong&email=admin@example.com&password=test1234";
        let input = InputStream::from_bytes(form.as_bytes().to_vec());
        let _ = handle(&ctx, input).await;
        // No admin user was created.
        assert!(users::find_by_email(&ctx, "admin@example.com")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn rejects_short_password() {
        let ctx = ctx_with_crypto().await;
        // Even with a valid token row, the handler must reject password <8 chars.
        let raw = "another-token";
        let hash = Sha256::digest(raw.as_bytes()).to_vec();
        let expires = (chrono::Utc::now() + chrono::Duration::hours(24))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        bootstrap_tokens::insert(&ctx, hash.clone(), &expires)
            .await
            .unwrap();

        let form = format!("token={raw}&email=admin@example.com&password=short");
        let input = InputStream::from_bytes(form.into_bytes());
        let _ = handle(&ctx, input).await;

        // No admin user, token row still valid (not consumed on rejection).
        assert!(users::find_by_email(&ctx, "admin@example.com")
            .await
            .unwrap()
            .is_none());
        assert!(bootstrap_tokens::is_valid(&ctx, &hash).await.unwrap());
    }
}
