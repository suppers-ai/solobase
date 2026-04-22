//! `post_logout` must drop every `OrgAdminCache` entry for the session's
//! user so a revoked-upstream admin doesn't retain privileges until the
//! cache TTL expires.

use solobase_core::blocks::auth::{
    cache::OrgAdminCache,
    handlers::login::post_logout,
    migrations,
    repo::{sessions, users},
    service::hash_token,
    session,
};
use wafer_core::interfaces::auth::service::UserId;
use wafer_run::types::Message;

use crate::common::MigrationTestCtx;

#[tokio::test]
async fn logout_invalidates_org_admin_cache_for_that_user() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    // Seed a user + session.
    let u = users::insert(
        &ctx,
        users::NewUser {
            email: "u@x.com".into(),
            display_name: "U".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user");
    let other = users::insert(
        &ctx,
        users::NewUser {
            email: "o@x.com".into(),
            display_name: "O".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .expect("insert user");

    let issued = session::issue_for(&ctx, &u.id, 30).await.unwrap();

    // Warm the cache: both users have entries.
    let cache = OrgAdminCache::default();
    let uid_u = UserId(u.id.clone());
    let uid_other = UserId(other.id.clone());
    cache.insert(&uid_u, "github", "acme", true);
    cache.insert(&uid_u, "github", "widgets", true);
    cache.insert(&uid_other, "github", "acme", true);

    // Pre-logout: all three entries present.
    assert_eq!(cache.get(&uid_u, "github", "acme"), Some(true));
    assert_eq!(cache.get(&uid_u, "github", "widgets"), Some(true));
    assert_eq!(cache.get(&uid_other, "github", "acme"), Some(true));

    // Logout the first user.
    let mut msg = Message::new("POST");
    msg.set_meta(
        "http.header.cookie",
        format!("wafer_session={}", issued.raw_token),
    );
    let reply = post_logout(&ctx, &msg, &cache).await.expect("logout ok");
    assert_eq!(reply.status, 204);

    // Session row deleted.
    let gone = sessions::find_by_token_hash(&ctx, &hash_token(&issued.raw_token))
        .await
        .unwrap();
    assert!(gone.is_none());

    // Cache entries for the logged-out user are gone; the other user's
    // entries are untouched.
    assert!(cache.get(&uid_u, "github", "acme").is_none());
    assert!(cache.get(&uid_u, "github", "widgets").is_none());
    assert_eq!(cache.get(&uid_other, "github", "acme"), Some(true));
}

#[tokio::test]
async fn logout_without_cookie_does_not_touch_cache() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let cache = OrgAdminCache::default();
    let uid = UserId("u1".into());
    cache.insert(&uid, "github", "acme", true);

    let msg = Message::new("POST");
    post_logout(&ctx, &msg, &cache).await.expect("logout ok");

    // No cookie → no user to invalidate → the cache entry survives.
    assert_eq!(cache.get(&uid, "github", "acme"), Some(true));
}

#[tokio::test]
async fn logout_with_unknown_cookie_does_not_touch_cache() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let cache = OrgAdminCache::default();
    let uid = UserId("u1".into());
    cache.insert(&uid, "github", "acme", true);

    let mut msg = Message::new("POST");
    msg.set_meta(
        "http.header.cookie",
        "wafer_session=no-such-session".to_string(),
    );
    post_logout(&ctx, &msg, &cache).await.expect("logout ok");

    // Cookie present but no matching session → can't recover the user →
    // cache is left alone.
    assert_eq!(cache.get(&uid, "github", "acme"), Some(true));
}
