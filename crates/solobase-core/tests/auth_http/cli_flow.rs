//! Layer-3 HTTP end-to-end: browser logs in → issues a CLI code → a
//! separate clean client exchanges the code for a PAT → the PAT works
//! against /auth/me.
//!
//! Two reqwest clients driving the same axum server:
//!   * `browser` — drives /auth/login and forwards the returned
//!     `wafer_session` cookie on /auth/cli/issue. The existing `reqwest`
//!     build doesn't ship the `cookies` feature so we extract the
//!     cookie pair manually (mirrors `login.rs`).
//!   * `cli` — sends no cookies at all; exchanges the code for a PAT and
//!     then calls /auth/me with `Authorization: Bearer <pat>`. This
//!     proves /auth/cli/exchange is genuinely unauthenticated.

use crate::common::HttpHarness;

/// Strip the `name=value` pair out of a `Set-Cookie` header.
fn cookie_pair(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .expect("set-cookie has name=value")
        .trim()
        .to_string()
}

/// Full browser → CLI handoff exercising the real axum → block dispatch
/// path end-to-end.
#[tokio::test]
async fn browser_issues_code_cli_exchanges_and_uses_pat() {
    let h = HttpHarness::start_with_auth().await;
    h.seed_user_with_password("cli@x.com", "hunter2hunter2")
        .await;

    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("browser reqwest");
    let cli = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("cli reqwest");

    // 1) Browser logs in, extracts `wafer_session=...` from Set-Cookie.
    let login = browser
        .post(h.url("/auth/login"))
        .json(&serde_json::json!({"email":"cli@x.com","password":"hunter2hunter2"}))
        .send()
        .await
        .expect("POST /auth/login");
    assert_eq!(login.status().as_u16(), 303, "login status");
    let set_cookie = login
        .headers()
        .get("set-cookie")
        .expect("Set-Cookie present")
        .to_str()
        .expect("Set-Cookie ASCII")
        .to_string();
    let pair = cookie_pair(&set_cookie);
    assert!(
        pair.starts_with("wafer_session="),
        "unexpected cookie name: {pair}"
    );

    // 2) Browser requests a CLI code — carries the session cookie.
    let issue = browser
        .post(h.url("/auth/cli/issue"))
        .header("Cookie", &pair)
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("POST /auth/cli/issue");
    assert_eq!(
        issue.status().as_u16(),
        200,
        "issue status; body: {}",
        issue.text().await.unwrap_or_default()
    );
    let issue_body: serde_json::Value = issue.json().await.expect("issue body JSON");
    let code = issue_body["code"]
        .as_str()
        .expect("code present")
        .to_string();
    assert!(code.len() >= 32, "code too short: {code}");

    // 3) CLI client exchanges the code — no cookies in play. We also
    //    assert the outbound request carries no Cookie header at all.
    let req = cli
        .post(h.url("/auth/cli/exchange"))
        .json(&serde_json::json!({"code": code}))
        .build()
        .expect("build exchange request");
    assert!(
        req.headers().get("cookie").is_none(),
        "CLI request must not carry a Cookie header: {:?}",
        req.headers()
    );
    let exchange = cli.execute(req).await.expect("POST /auth/cli/exchange");
    assert_eq!(exchange.status().as_u16(), 200, "exchange status");
    let exchange_body: serde_json::Value = exchange.json().await.expect("exchange body JSON");
    let pat = exchange_body["token"]
        .as_str()
        .expect("token present")
        .to_string();
    assert!(pat.starts_with("wafer_pat_"), "pat shape: {pat}");
    assert!(
        exchange_body["expires_at"].is_null(),
        "CLI PATs don't expire"
    );

    // 4) CLI uses the PAT against /auth/me.
    let me = cli
        .get(h.url("/auth/me"))
        .bearer_auth(&pat)
        .send()
        .await
        .expect("GET /auth/me");
    assert_eq!(me.status().as_u16(), 200, "me status");
    let me_body: serde_json::Value = me.json().await.expect("me body JSON");
    assert_eq!(me_body["email"], "cli@x.com");

    // 5) Single-use semantics survive the HTTP round-trip.
    let second = cli
        .post(h.url("/auth/cli/exchange"))
        .json(&serde_json::json!({"code": code}))
        .send()
        .await
        .expect("POST /auth/cli/exchange again");
    assert_eq!(second.status().as_u16(), 401, "code must be single-use");
}

#[tokio::test]
async fn cli_issue_without_cookie_returns_401_over_http() {
    let h = HttpHarness::start_with_auth().await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client");

    let resp = client
        .post(h.url("/auth/cli/issue"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("POST /auth/cli/issue");
    assert_eq!(resp.status().as_u16(), 401);
}

#[tokio::test]
async fn cli_issue_with_bearer_pat_returns_401_over_http() {
    // Belt-and-braces HTTP repro of the unit test: a publish-scope PAT
    // cannot mint a fresh CLI token even over the real HTTP surface.
    let h = HttpHarness::start_with_auth().await;
    let user_id = h
        .seed_user_with_password("cli@x.com", "hunter2hunter2")
        .await;

    // Mint a PAT directly through the block's pat helper so we exercise
    // only the /auth/cli/issue gate, not login+token-create.
    let issued = solobase_core::blocks::auth::pat::issue(
        h.ctx.as_ref(),
        &user_id,
        "test",
        &[wafer_core::interfaces::auth::service::TokenScope::Publish],
        None,
    )
    .await
    .expect("mint pat");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client");

    let resp = client
        .post(h.url("/auth/cli/issue"))
        .json(&serde_json::json!({}))
        .bearer_auth(&issued.raw_token)
        .send()
        .await
        .expect("POST /auth/cli/issue with bearer");
    assert_eq!(resp.status().as_u16(), 401, "bearer must be rejected");
}
