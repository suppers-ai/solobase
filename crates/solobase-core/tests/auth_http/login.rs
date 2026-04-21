//! Layer-3 HTTP end-to-end: `login` → `me` → `logout` → `me`.
//!
//! Uses a real axum server on an ephemeral port and drives it with
//! `reqwest`. Asserts:
//! - login returns 303 + `Set-Cookie` with `HttpOnly; Secure; SameSite=Lax`
//! - authenticated `/auth/me` returns 200 + JSON profile
//! - logout returns 204 + clearing cookie (`Max-Age=0`)
//! - the invalidated cookie gets 401 on `/auth/me`

use crate::common::HttpHarness;

#[tokio::test]
async fn full_cookie_round_trip_over_real_http() {
    let h = HttpHarness::start_with_auth().await;
    h.seed_user_with_password("a@b.c", "pw").await;

    // Disable reqwest's redirect follower — we want to assert the 303.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client");

    // 1. POST /auth/login → 303 + Set-Cookie
    let login = client
        .post(h.url("/auth/login"))
        .json(&serde_json::json!({"email":"a@b.c","password":"pw"}))
        .send()
        .await
        .expect("POST /auth/login");
    assert_eq!(login.status().as_u16(), 303, "login status");
    let set_cookie = login
        .headers()
        .get("set-cookie")
        .expect("Set-Cookie header present")
        .to_str()
        .expect("Set-Cookie ASCII")
        .to_string();
    assert!(set_cookie.contains("HttpOnly"), "Set-Cookie: {set_cookie}");
    assert!(
        set_cookie.contains("SameSite=Lax"),
        "Set-Cookie: {set_cookie}"
    );
    assert!(set_cookie.contains("Secure"), "Set-Cookie: {set_cookie}");
    let cookie_pair = set_cookie
        .split(';')
        .next()
        .expect("Set-Cookie has name=value segment")
        .trim()
        .to_string();
    assert!(
        cookie_pair.starts_with("wafer_session="),
        "unexpected cookie name: {cookie_pair}"
    );

    // 2. GET /auth/me with the session cookie → 200 + profile JSON
    let me = client
        .get(h.url("/auth/me"))
        .header("Cookie", &cookie_pair)
        .send()
        .await
        .expect("GET /auth/me");
    assert_eq!(me.status().as_u16(), 200, "me status");
    let body: serde_json::Value = me.json().await.expect("me JSON");
    assert_eq!(body["email"], "a@b.c");

    // 3. POST /auth/logout → 204 + clearing cookie
    let out = client
        .post(h.url("/auth/logout"))
        .header("Cookie", &cookie_pair)
        .send()
        .await
        .expect("POST /auth/logout");
    assert_eq!(out.status().as_u16(), 204, "logout status");
    let clear = out
        .headers()
        .get("set-cookie")
        .expect("logout emits Set-Cookie to clear")
        .to_str()
        .expect("Set-Cookie ASCII");
    assert!(clear.contains("Max-Age=0"), "clearing cookie: {clear}");

    // 4. The same cookie must be rejected after logout.
    let me2 = client
        .get(h.url("/auth/me"))
        .header("Cookie", &cookie_pair)
        .send()
        .await
        .expect("GET /auth/me post-logout");
    assert_eq!(me2.status().as_u16(), 401, "me after logout must be 401");
}

#[tokio::test]
async fn wrong_password_returns_401_over_real_http() {
    let h = HttpHarness::start_with_auth().await;
    h.seed_user_with_password("a@b.c", "pw").await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client");

    let res = client
        .post(h.url("/auth/login"))
        .json(&serde_json::json!({"email":"a@b.c","password":"WRONG"}))
        .send()
        .await
        .expect("POST /auth/login wrong pw");
    assert_eq!(res.status().as_u16(), 401);
    assert!(
        res.headers().get("set-cookie").is_none(),
        "failed login must not emit Set-Cookie"
    );
}
