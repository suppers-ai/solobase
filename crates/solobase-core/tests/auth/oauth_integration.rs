//! Block-integration tests for the OAuth start + callback handlers.
//!
//! Uses [`FakeGithub`](super::fake_github::FakeGithub) as the provider
//! double — same shape the future registry tests will reuse.

use std::{collections::HashMap, sync::Arc};

use solobase_core::blocks::auth::{
    config::AuthConfig,
    handlers::{
        oauth::{get_callback, get_start},
        oauth_state::{parse_cookie_value, COOKIE_NAME},
    },
    migrations,
    providers::{registry::ProviderRegistry, OAuthProvider, ProviderProfile},
    repo::{provider_links, users},
};

use crate::{common::MigrationTestCtx, fake_github::FakeGithub};

fn registry_with_fake(fake: Arc<FakeGithub>) -> ProviderRegistry {
    let mut m: HashMap<&'static str, Arc<dyn OAuthProvider>> = HashMap::new();
    m.insert("github", fake);
    ProviderRegistry::from_map(m)
}

/// Grab the first `Set-Cookie` header whose name matches `name` and return
/// the raw cookie value (the part after `name=`, before the first `;`).
fn cookie_from(
    reply: &solobase_core::blocks::auth::handlers::HttpReply,
    name: &str,
) -> Option<String> {
    for (k, v) in &reply.headers {
        if !k.eq_ignore_ascii_case("Set-Cookie") {
            continue;
        }
        if let Some(rest) = v.strip_prefix(&format!("{name}=")) {
            return Some(rest.split(';').next().unwrap_or("").to_string());
        }
    }
    None
}

fn profile(provider_ref: &str, email: &str) -> ProviderProfile {
    ProviderProfile {
        provider_ref: provider_ref.into(),
        login: "alice".into(),
        email: Some(email.into()),
        email_verified: true,
        display_name: "Alice".into(),
        avatar_url: None,
        access_token: format!("tok-{provider_ref}"),
    }
}

#[tokio::test]
async fn start_sets_state_cookie_and_redirects() {
    let fake = Arc::new(FakeGithub::new("github"));
    let reg = registry_with_fake(fake);
    let reply = get_start(&reg, "github", Some("/dashboard")).await.unwrap();
    assert_eq!(reply.status, 302);
    let loc = reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Location"))
        .expect("Location")
        .1
        .clone();
    assert!(loc.starts_with("https://fake/authorize"), "loc: {loc}");

    let raw = cookie_from(&reply, COOKIE_NAME).expect("state cookie");
    let payload = parse_cookie_value(&raw).expect("decode state");
    assert_eq!(payload.next.as_deref(), Some("/dashboard"));
    assert!(!payload.state.is_empty());
    assert!(!payload.pkce_verifier.is_empty());
}

#[tokio::test]
async fn start_unknown_provider_is_404() {
    let reg = ProviderRegistry::empty();
    let reply = get_start(&reg, "nope", None).await.unwrap();
    assert_eq!(reply.status, 404);
}

#[tokio::test]
async fn callback_happy_path_creates_user_and_issues_session() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let fake = Arc::new(FakeGithub::new("github"));
    fake.register_code("good-code", profile("42", "alice@example.com"));
    let reg = registry_with_fake(Arc::clone(&fake));
    let cfg = AuthConfig::from_env_for_test(&[]);

    // Prime the state cookie the way /start would.
    let start = get_start(&reg, "github", Some("/after")).await.unwrap();
    let cookie_raw = cookie_from(&start, COOKIE_NAME).unwrap();
    let payload = parse_cookie_value(&cookie_raw).unwrap();

    let reply = get_callback(
        &ctx,
        &cfg,
        &reg,
        "github",
        "good-code",
        &payload.state,
        &cookie_raw,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 303);
    let loc = reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Location"))
        .unwrap()
        .1
        .clone();
    assert_eq!(loc, "/after");

    // Both cookies present: new session + cleared state.
    assert!(reply
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Set-Cookie") && v.starts_with("wafer_session=")));
    assert!(reply
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Set-Cookie")
            && v.starts_with("wafer_oauth_state=")
            && v.contains("Max-Age=0")));

    // Link and user row exist.
    let link = provider_links::find_by_provider_ref(&ctx, "github", "42")
        .await
        .unwrap()
        .unwrap();
    let user = users::find_by_id(&ctx, &link.user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(link.access_token, "tok-42");
}

#[tokio::test]
async fn callback_mismatched_state_is_400_and_clears_cookie() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let fake = Arc::new(FakeGithub::new("github"));
    fake.register_code("c", profile("42", "alice@example.com"));
    let reg = registry_with_fake(Arc::clone(&fake));
    let cfg = AuthConfig::from_env_for_test(&[]);

    let start = get_start(&reg, "github", None).await.unwrap();
    let cookie_raw = cookie_from(&start, COOKIE_NAME).unwrap();

    let reply = get_callback(
        &ctx,
        &cfg,
        &reg,
        "github",
        "c",
        "not-the-state",
        &cookie_raw,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 400);
    assert!(reply
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Set-Cookie")
            && v.contains("wafer_oauth_state=")
            && v.contains("Max-Age=0")));
}

#[tokio::test]
async fn callback_provider_exchange_error_is_503() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");
    let fake = Arc::new(FakeGithub::new("github"));
    // Intentionally do NOT register "missing" → exchange returns Upstream.
    let reg = registry_with_fake(Arc::clone(&fake));
    let cfg = AuthConfig::from_env_for_test(&[]);

    let start = get_start(&reg, "github", None).await.unwrap();
    let cookie_raw = cookie_from(&start, COOKIE_NAME).unwrap();
    let payload = parse_cookie_value(&cookie_raw).unwrap();

    let reply = get_callback(
        &ctx,
        &cfg,
        &reg,
        "github",
        "missing",
        &payload.state,
        &cookie_raw,
    )
    .await
    .unwrap();
    assert_eq!(reply.status, 503);
}

#[tokio::test]
async fn callback_existing_email_links_to_existing_user() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    // Seed a user with the email the fake will report.
    let existing = users::insert(
        &ctx,
        users::NewUser {
            email: "alice@example.com".into(),
            display_name: "Pre-existing Alice".into(),
            avatar_url: None,
            role: "user".into(),
        },
    )
    .await
    .unwrap();

    let fake = Arc::new(FakeGithub::new("github"));
    fake.register_code("c", profile("42", "alice@example.com"));
    let reg = registry_with_fake(Arc::clone(&fake));
    let cfg = AuthConfig::from_env_for_test(&[]);

    let start = get_start(&reg, "github", None).await.unwrap();
    let cookie_raw = cookie_from(&start, COOKIE_NAME).unwrap();
    let payload = parse_cookie_value(&cookie_raw).unwrap();

    let reply = get_callback(&ctx, &cfg, &reg, "github", "c", &payload.state, &cookie_raw)
        .await
        .unwrap();
    assert_eq!(reply.status, 303);

    let link = provider_links::find_by_provider_ref(&ctx, "github", "42")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(link.user_id, existing.id);
}

#[tokio::test]
async fn callback_unverified_email_is_403() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("migrations");

    let fake = Arc::new(FakeGithub::new("github"));
    fake.register_code(
        "c",
        ProviderProfile {
            provider_ref: "77".into(),
            login: "shady".into(),
            email: Some("shady@example.com".into()),
            email_verified: false,
            display_name: "Shady".into(),
            avatar_url: None,
            access_token: "tok".into(),
        },
    );
    let reg = registry_with_fake(Arc::clone(&fake));
    let cfg = AuthConfig::from_env_for_test(&[]);

    let start = get_start(&reg, "github", None).await.unwrap();
    let cookie_raw = cookie_from(&start, COOKIE_NAME).unwrap();
    let payload = parse_cookie_value(&cookie_raw).unwrap();

    let reply = get_callback(&ctx, &cfg, &reg, "github", "c", &payload.state, &cookie_raw)
        .await
        .unwrap();
    assert_eq!(reply.status, 403);
    // State cookie cleared on rejection.
    assert!(reply
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Set-Cookie")
            && v.contains("wafer_oauth_state=")
            && v.contains("Max-Age=0")));
}
