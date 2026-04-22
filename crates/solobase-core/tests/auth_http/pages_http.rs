//! Layer-3 HTTP tests for the Plan D auth pages.
//!
//! Drives the real axum server wrapped by [`HttpHarness`] with `reqwest`,
//! asserting status codes, cookies, redirect locations, and that the maud
//! templates render the expected anchors/forms.

use reqwest::redirect::Policy;
use reqwest::StatusCode;

use crate::common::HttpHarness;

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("reqwest client")
}

#[tokio::test]
async fn get_login_returns_200_html_with_form() {
    let h = HttpHarness::start_with_auth().await;
    let resp = reqwest::get(h.url("/auth/login"))
        .await
        .expect("GET /auth/login");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .expect("content-type")
        .to_str()
        .expect("ascii")
        .to_string();
    assert!(ct.starts_with("text/html"), "got {ct}");
    let body = resp.text().await.expect("body");
    assert!(
        body.contains(r#"<form class="auth-form" method="post" action="/auth/login">"#),
        "body missing login form: {body}"
    );
}

#[tokio::test]
async fn get_signup_404_when_disabled() {
    let h = HttpHarness::start_with_auth().await;
    let resp = reqwest::get(h.url("/auth/signup"))
        .await
        .expect("GET /auth/signup");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_signup_200_when_enabled() {
    let h = HttpHarness::builder().signup_enabled(true).spawn().await;
    let resp = reqwest::get(h.url("/auth/signup"))
        .await
        .expect("GET /auth/signup");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains(r#"<form class="auth-form" method="post" action="/auth/signup">"#),
        "body missing signup form: {body}"
    );
}

#[tokio::test]
async fn post_signup_404_when_disabled() {
    let h = HttpHarness::start_with_auth().await;
    let resp = no_redirect_client()
        .post(h.url("/auth/signup"))
        .form(&[("email", "new@example.com"), ("password", "longenough")])
        .send()
        .await
        .expect("POST /auth/signup");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_signup_happy_path_creates_user_and_sets_cookie() {
    let h = HttpHarness::builder().signup_enabled(true).spawn().await;
    let resp = no_redirect_client()
        .post(h.url("/auth/signup"))
        .form(&[("email", "new@example.com"), ("password", "longenough")])
        .send()
        .await
        .expect("POST /auth/signup");
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers()
            .get("location")
            .expect("location")
            .to_str()
            .expect("ascii"),
        "/auth/dashboard"
    );
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .expect("Set-Cookie")
        .to_str()
        .expect("ascii");
    assert!(set_cookie.contains("wafer_session="), "{set_cookie}");
    assert!(set_cookie.contains("HttpOnly"), "{set_cookie}");
    assert!(set_cookie.contains("SameSite=Lax"), "{set_cookie}");
}

#[tokio::test]
async fn post_signup_short_password_400_with_banner() {
    let h = HttpHarness::builder()
        .signup_enabled(true)
        .password_min_length(10)
        .spawn()
        .await;
    let resp = no_redirect_client()
        .post(h.url("/auth/signup"))
        .form(&[("email", "x@y.com"), ("password", "short")])
        .send()
        .await
        .expect("POST /auth/signup");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = resp.text().await.expect("body");
    assert!(body.contains("at least 10"), "missing banner: {body}");
    assert!(
        body.contains(r#"class="error""#),
        "missing error div: {body}"
    );
}

#[tokio::test]
async fn post_signup_duplicate_email_409() {
    let h = HttpHarness::builder().signup_enabled(true).spawn().await;
    h.seed_user_with_password("taken@example.com", "anything12")
        .await;
    let resp = no_redirect_client()
        .post(h.url("/auth/signup"))
        .form(&[("email", "taken@example.com"), ("password", "longenough")])
        .send()
        .await
        .expect("POST /auth/signup");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains("already registered"),
        "missing banner: {body}"
    );
}

#[tokio::test]
async fn post_signup_bad_email_re_renders_form() {
    let h = HttpHarness::builder().signup_enabled(true).spawn().await;
    let resp = no_redirect_client()
        .post(h.url("/auth/signup"))
        .form(&[("email", "not-an-email"), ("password", "longenough")])
        .send()
        .await
        .expect("POST /auth/signup");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = resp.text().await.expect("body");
    assert!(body.to_lowercase().contains("email"), "banner: {body}");
}

#[tokio::test]
async fn get_dashboard_redirects_when_unauthenticated() {
    let h = HttpHarness::start_with_auth().await;
    let resp = no_redirect_client()
        .get(h.url("/auth/dashboard"))
        .send()
        .await
        .expect("GET /auth/dashboard");
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers()
            .get("location")
            .expect("location")
            .to_str()
            .expect("ascii"),
        "/auth/login?next=/auth/dashboard"
    );
}

#[tokio::test]
async fn get_dashboard_renders_profile_when_authenticated() {
    let h = HttpHarness::start_with_auth().await;
    let session = h
        .seed_user_and_session("alice@example.com", "longenough")
        .await;
    let resp = reqwest::Client::new()
        .get(h.url("/auth/dashboard"))
        .header("cookie", format!("wafer_session={session}"))
        .send()
        .await
        .expect("GET /auth/dashboard");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains("alice@example.com"),
        "profile missing: {body}"
    );
}

#[tokio::test]
async fn get_cli_login_requires_auth_redirects() {
    let h = HttpHarness::start_with_auth().await;
    let resp = no_redirect_client()
        .get(h.url("/auth/cli-login"))
        .send()
        .await
        .expect("GET /auth/cli-login");
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers()
            .get("location")
            .expect("location")
            .to_str()
            .expect("ascii"),
        "/auth/login?next=/auth/cli-login"
    );
}

#[tokio::test]
async fn get_cli_login_renders_issue_button_when_authenticated() {
    let h = HttpHarness::start_with_auth().await;
    let session = h
        .seed_user_and_session("bob@example.com", "longenough")
        .await;
    let resp = reqwest::Client::new()
        .get(h.url("/auth/cli-login"))
        .header("cookie", format!("wafer_session={session}"))
        .send()
        .await
        .expect("GET /auth/cli-login");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains(r#"hx-post="/auth/cli/issue""#),
        "missing htmx button: {body}"
    );
}

#[tokio::test]
async fn get_org_public_view_hides_manage_section() {
    let h = HttpHarness::start_with_auth().await;
    // Migration 002 seeds reserved orgs including wafer-run.
    let resp = reqwest::get(h.url("/auth/orgs/wafer-run"))
        .await
        .expect("GET /auth/orgs/wafer-run");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("body");
    assert!(body.contains("wafer-run"), "name missing: {body}");
    assert!(body.contains("reserved"), "reserved badge missing: {body}");
    assert!(!body.contains("Manage"), "manage section leaked: {body}");
}

#[tokio::test]
async fn get_org_missing_returns_404() {
    let h = HttpHarness::start_with_auth().await;
    let resp = reqwest::get(h.url("/auth/orgs/does-not-exist"))
        .await
        .expect("GET /auth/orgs/does-not-exist");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn login_page_shows_signup_link_when_enabled() {
    let h = HttpHarness::builder().signup_enabled(true).spawn().await;
    let body = reqwest::get(h.url("/auth/login"))
        .await
        .expect("GET /auth/login")
        .text()
        .await
        .expect("body");
    assert!(
        body.contains(r#"href="/auth/signup""#),
        "signup link missing: {body}"
    );
}

#[tokio::test]
async fn login_page_hides_signup_link_when_disabled() {
    let h = HttpHarness::start_with_auth().await;
    let body = reqwest::get(h.url("/auth/login"))
        .await
        .expect("GET /auth/login")
        .text()
        .await
        .expect("body");
    assert!(
        !body.contains(r#"href="/auth/signup""#),
        "signup link leaked: {body}"
    );
}
