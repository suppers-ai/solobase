//! `handle_login` writes a session row to `auth::repo::sessions` so the
//! userportal `/b/userportal/sessions` page renders meaningful data after a
//! JWT login.
//!
//! These tests exercise the production code path WITHOUT applying migrations
//! — production never applies them either; the live `users` and `sessions`
//! tables are materialised through `ensure_table` on first `db::create`. The
//! tests use the `MigrationTestCtx` for its real `wafer-run/crypto` routing
//! so password hashing and JWT signing work the same way as production.

use serde_json::json;
use solobase_core::blocks::{
    auth::{repo::sessions, service::hash_token, AuthBlock, AUTH_BLOCK_ID},
    userportal::UserPortalBlock,
};
use wafer_core::clients::{crypto, database as db};
use wafer_run::{
    block::Block,
    streams::output::{BufferedResponse, TerminalNotResponse},
    types::Message,
    InputStream, OutputStream,
};

use crate::common::MigrationTestCtx;

/// Drain an `OutputStream` to a `BufferedResponse`. Mirrors the helper in
/// `solobase-core/src/test_support.rs` (which is `#[cfg(test)]` and not
/// visible from integration tests).
async fn collect_or_panic(out: OutputStream) -> BufferedResponse {
    match out.collect_buffered().await {
        Ok(buf) => buf,
        Err(TerminalNotResponse::Error(e)) => {
            panic!("handler returned error: {} ({:?})", e.message, e.code)
        }
        Err(TerminalNotResponse::Drop) => panic!("handler dropped the request"),
        Err(TerminalNotResponse::Continue(_)) => panic!("handler returned Continue"),
        Err(TerminalNotResponse::Malformed) => panic!("handler returned malformed stream"),
    }
}

/// Seed a user via `db::create` (live TEXT-everything convention). Returns
/// the user's id. Hashes the password through the registered crypto block
/// the same way production `seed_admin_user` does. Adds an `email_verified`
/// flag so the login flow doesn't gate on verification.
async fn seed_password_user(ctx: &MigrationTestCtx, email: &str, password: &str) -> String {
    let password_hash = crypto::hash(ctx, password).await.expect("hash password");
    let mut data = std::collections::HashMap::new();
    data.insert("email".to_string(), json!(email));
    data.insert("password_hash".to_string(), json!(password_hash));
    data.insert("name".to_string(), json!(""));
    data.insert("disabled".to_string(), json!("false"));
    data.insert("email_verified".to_string(), json!("true"));
    let user = db::create(ctx, "suppers_ai__auth__users", data)
        .await
        .expect("create user");
    user.id
}

fn login_msg() -> Message {
    let mut m = Message::new("http.request");
    m.set_meta("req.action", "create");
    m.set_meta("req.resource", "/b/auth/api/login");
    m
}

async fn invoke_login(ctx: &MigrationTestCtx, email: &str, password: &str) -> String {
    let block = AuthBlock::default();
    let body = json!({"email": email, "password": password}).to_string();
    let msg = login_msg();
    let out = block
        .handle(ctx, msg, InputStream::from_bytes(body.into_bytes()))
        .await;
    let buf = collect_or_panic(out).await;
    String::from_utf8(buf.body).expect("body utf8")
}

/// Run the login handler and consume the output stream regardless of whether
/// it terminates with `Complete` or `Error` — used by the wrong-password test
/// which expects an `Unauthenticated` error stream rather than a body.
async fn invoke_login_drain(ctx: &MigrationTestCtx, email: &str, password: &str) {
    let block = AuthBlock::default();
    let body = json!({"email": email, "password": password}).to_string();
    let msg = login_msg();
    let out = block
        .handle(ctx, msg, InputStream::from_bytes(body.into_bytes()))
        .await;
    // Discard the result — we only care about the side-effects (or lack
    // thereof) on the database. An error stream is the expected outcome on
    // the wrong-password path.
    let _ = out.collect_buffered().await;
}

#[tokio::test]
async fn login_creates_one_session_row_keyed_by_access_token_hash() {
    let ctx = MigrationTestCtx::new();
    let user_id = seed_password_user(&ctx, "alice@example.com", "hunter2hunter2").await;

    let resp_body = invoke_login(&ctx, "alice@example.com", "hunter2hunter2").await;
    let resp: serde_json::Value = serde_json::from_str(&resp_body)
        .unwrap_or_else(|_| panic!("login body is not JSON: {resp_body}"));
    let access_token = resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("access_token missing from login body: {resp_body}"))
        .to_string();
    assert!(
        !access_token.is_empty(),
        "login must return a non-empty access token"
    );

    let rows = sessions::list_for_user(&ctx, &user_id)
        .await
        .expect("list sessions");
    assert_eq!(
        rows.len(),
        1,
        "exactly one session row per login, got {}: {rows:?}",
        rows.len()
    );
    assert_eq!(
        rows[0].user_id, user_id,
        "session row must reference the logged-in user"
    );
    assert_eq!(
        rows[0].token_hash,
        hash_token(&access_token),
        "session row token_hash must equal sha256(access_token)"
    );
}

#[tokio::test]
async fn invalid_credentials_do_not_create_a_session_row() {
    let ctx = MigrationTestCtx::new();
    let user_id = seed_password_user(&ctx, "bob@example.com", "correct-horse").await;

    invoke_login_drain(&ctx, "bob@example.com", "WRONG-password").await;

    let rows = sessions::list_for_user(&ctx, &user_id)
        .await
        .expect("list sessions");
    assert!(
        rows.is_empty(),
        "no session row may be written for a failed login: {rows:?}"
    );
}

#[tokio::test]
async fn two_logins_produce_two_distinct_session_rows() {
    let ctx = MigrationTestCtx::new();
    let user_id = seed_password_user(&ctx, "carol@example.com", "passw0rd-passw0rd").await;

    let _ = invoke_login(&ctx, "carol@example.com", "passw0rd-passw0rd").await;
    // Sleep so the second JWT's `iat`/`exp` claims differ — without this the
    // two access tokens are byte-identical and produce the same token_hash,
    // which the sessions table treats as the same row.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let _ = invoke_login(&ctx, "carol@example.com", "passw0rd-passw0rd").await;

    let rows = sessions::list_for_user(&ctx, &user_id)
        .await
        .expect("list sessions");
    assert_eq!(
        rows.len(),
        2,
        "two logins must produce two session rows, got {}",
        rows.len()
    );
    assert_ne!(
        rows[0].token_hash, rows[1].token_hash,
        "session rows must have distinct token_hash values"
    );
}

/// End-to-end: after login writes a session row, the userportal sessions
/// page renders one row per active session (and a Revoke button for each).
/// This is what the user sees in their browser at `/b/userportal/sessions`.
#[tokio::test]
async fn userportal_sessions_page_renders_row_after_login() {
    let ctx = MigrationTestCtx::new();
    let user_id = seed_password_user(&ctx, "diana@example.com", "diana-password").await;

    let _ = invoke_login(&ctx, "diana@example.com", "diana-password").await;

    let block = UserPortalBlock;
    let mut msg = Message::new("http.request");
    msg.set_meta("req.action", "retrieve");
    msg.set_meta("req.resource", "/b/userportal/sessions");
    msg.set_meta("auth.user_id", &user_id);
    let out = block.handle(&ctx, msg, InputStream::empty()).await;
    let buf = collect_or_panic(out).await;
    let html = String::from_utf8(buf.body).expect("body utf8");

    assert!(
        html.contains("Active sessions"),
        "page must render the Active sessions header: {html}"
    );
    assert!(
        html.contains(">Revoke<"),
        "populated page must render at least one Revoke button: {html}"
    );
    assert!(
        !html.contains("No active sessions"),
        "populated page must not show the empty state: {html}"
    );
}

/// Sanity check: `AUTH_BLOCK_ID` is the block name we'd register at runtime.
/// If this drifts, every other test in this file is testing the wrong
/// surface.
#[tokio::test]
async fn auth_block_id_is_what_we_target() {
    assert_eq!(AUTH_BLOCK_ID, "suppers-ai/auth");
}
