//! Layer-3 HTTP end-to-end: `/auth/oauth/github/{start,callback}` driven
//! over real TCP with reqwest. Wires a [`FakeGithub`] into the provider
//! registry so the callback path runs without real network.

use std::sync::Arc;

use solobase_core::blocks::auth::providers::{OAuthProvider, ProviderProfile};

use crate::common::{registry_with, HttpHarness};
use crate::fake_github::FakeGithub;

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client")
}

fn verified_profile(provider_ref: &str, email: &str) -> ProviderProfile {
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
async fn start_unknown_provider_is_404_over_real_http() {
    let h = HttpHarness::start_with_auth().await;
    let resp = reqwest::Client::new()
        .get(h.url("/auth/oauth/doesnotexist/start"))
        .send()
        .await
        .expect("GET start");
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn start_enabled_provider_redirects_302_with_state_cookie() {
    let fake: Arc<dyn OAuthProvider> = Arc::new(FakeGithub::new("github"));
    let h = HttpHarness::start_with_providers(registry_with("github", fake)).await;

    let client = no_redirect_client();
    let resp = client
        .get(h.url("/auth/oauth/github/start?next=/dashboard"))
        .send()
        .await
        .expect("GET start");
    assert_eq!(resp.status().as_u16(), 302);
    let loc = resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap();
    assert!(loc.starts_with("https://fake/authorize"), "loc: {loc}");

    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .expect("Set-Cookie header")
        .to_str()
        .unwrap();
    assert!(
        set_cookie.starts_with("wafer_oauth_state="),
        "Set-Cookie: {set_cookie}"
    );
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Lax"));
}

#[tokio::test]
async fn full_oauth_round_trip_issues_session_cookie() {
    let fake = Arc::new(FakeGithub::new("github"));
    fake.register_code("good-code", verified_profile("42", "alice@example.com"));
    let fake_dyn: Arc<dyn OAuthProvider> = Arc::clone(&fake) as Arc<dyn OAuthProvider>;
    let h = HttpHarness::start_with_providers(registry_with("github", fake_dyn)).await;

    let client = no_redirect_client();
    let start = client
        .get(h.url("/auth/oauth/github/start?next=/after"))
        .send()
        .await
        .expect("GET start");
    assert_eq!(start.status().as_u16(), 302);
    let state_cookie = start
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // Extract just the name=value pair.
    let state_pair = state_cookie.split(';').next().unwrap().trim().to_string();
    // Parse the state value the same way handlers::oauth_state does to compute
    // the matching ?state= query param.
    let raw = state_pair
        .strip_prefix("wafer_oauth_state=")
        .expect("state cookie prefix");
    let payload = solobase_core::blocks::auth::handlers::oauth_state::parse_cookie_value(raw)
        .expect("parse state cookie");

    let callback = client
        .get(h.url(&format!(
            "/auth/oauth/github/callback?code=good-code&state={}",
            urlencoding(&payload.state)
        )))
        .header("Cookie", &state_pair)
        .send()
        .await
        .expect("GET callback");
    assert_eq!(callback.status().as_u16(), 303);
    assert_eq!(
        callback
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/after"
    );
    let cookies: Vec<String> = callback
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|h| h.to_str().unwrap().to_string())
        .collect();
    assert!(
        cookies.iter().any(|c| c.starts_with("wafer_session=")),
        "session cookie: {cookies:?}"
    );
    assert!(
        cookies
            .iter()
            .any(|c| c.starts_with("wafer_oauth_state=") && c.contains("Max-Age=0")),
        "state cookie cleared: {cookies:?}"
    );
}

/// Minimal percent-encoder for URL query values. Only reserves characters we
/// might see in a base64url state (which already uses the unreserved set,
/// so this is a no-op in practice).
fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
