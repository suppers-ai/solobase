//! Integration tests for `handlers::login::{post_login, post_logout}`.
//!
//! Layer 2 — exercises the handler against a real in-memory sqlite +
//! Argon2 crypto service, mirroring production wiring.

use solobase_core::blocks::auth::{
    config::AuthConfig,
    handlers::login::{post_login, post_logout},
    migrations,
    repo::{local_credentials, sessions, users},
    service::hash_token,
    session,
};
use wafer_core::clients::crypto as crypto_client;
use wafer_run::types::Message;

use crate::common::MigrationTestCtx;

async fn seed_user_with_password(ctx: &MigrationTestCtx, email: &str, password: &str) -> String {
    let u = users::insert(
        ctx,
        users::NewUser {
            email: email.into(),
            display_name: "T".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user");
    let hash = crypto_client::hash(ctx, password).await.expect("hash");
    local_credentials::insert(ctx, &u.id, &hash, false)
        .await
        .expect("insert credentials");
    u.id
}

#[tokio::test]
async fn right_password_returns_303_with_set_cookie() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    seed_user_with_password(&ctx, "a@b.c", "pw").await;

    let cfg = AuthConfig::from_env_for_test(&[]);
    let reply = post_login(&ctx, &cfg, br#"{"email":"a@b.c","password":"pw"}"#)
        .await
        .expect("login succeeds");
    assert_eq!(reply.status, 303);
    let sc = reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .expect("Set-Cookie header present");
    assert!(
        sc.1.starts_with("wafer_session=wafer_session_"),
        "cookie must carry the prefixed raw session token, got {}",
        sc.1
    );
    assert!(sc.1.contains("HttpOnly"));
    assert!(sc.1.contains("SameSite=Lax"));
    assert!(sc.1.contains("Path=/"));
}

#[tokio::test]
async fn wrong_password_returns_401_no_cookie() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    seed_user_with_password(&ctx, "a@b.c", "pw").await;

    let cfg = AuthConfig::from_env_for_test(&[]);
    let reply = post_login(&ctx, &cfg, br#"{"email":"a@b.c","password":"WRONG"}"#)
        .await
        .expect("login returns 401 not error");
    assert_eq!(reply.status, 401);
    assert!(
        reply
            .headers
            .iter()
            .all(|(k, _)| !k.eq_ignore_ascii_case("Set-Cookie")),
        "failed login must not emit Set-Cookie"
    );
}

#[tokio::test]
async fn unknown_email_returns_401() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let cfg = AuthConfig::from_env_for_test(&[]);
    let reply = post_login(&ctx, &cfg, br#"{"email":"nobody@x.io","password":"pw"}"#)
        .await
        .expect("login returns 401 not error");
    assert_eq!(reply.status, 401);
    // Sanity: the DUMMY_HASH that `post_login` falls back to must be parseable
    // by the argon2 service, otherwise the handler would error rather than
    // return 401 (and timing equalisation would be broken).
    use solobase_core::blocks::auth::handlers::login::DUMMY_HASH;
    let out = crypto_client::compare_hash(&ctx, "pw", DUMMY_HASH).await;
    assert!(out.is_err(), "DUMMY_HASH must parse and reject 'pw'");
}

#[tokio::test]
async fn post_logout_clears_cookie_and_deletes_session() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let uid = seed_user_with_password(&ctx, "a@b.c", "pw").await;
    let issued = session::issue_for(&ctx, &uid, 30).await.unwrap();

    let mut msg = Message::new("POST");
    msg.set_meta(
        "http.header.cookie",
        format!("wafer_session={}", issued.raw_token),
    );

    let reply = post_logout(&ctx, &msg).await.expect("logout ok");
    assert_eq!(reply.status, 204);
    let sc = reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .expect("Set-Cookie present to clear");
    assert!(sc.1.contains("Max-Age=0"));
    let still = sessions::find_by_token_hash(&ctx, &hash_token(&issued.raw_token))
        .await
        .unwrap();
    assert!(still.is_none(), "session row must be deleted on logout");
}

#[tokio::test]
async fn post_logout_without_cookie_is_idempotent() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let msg = Message::new("POST");
    let reply = post_logout(&ctx, &msg).await.expect("logout ok");
    assert_eq!(reply.status, 204);
    // Still emits the Max-Age=0 cookie so a stale cookie on the client is cleared.
    let sc = reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .expect("clear cookie emitted");
    assert!(sc.1.contains("Max-Age=0"));
}
