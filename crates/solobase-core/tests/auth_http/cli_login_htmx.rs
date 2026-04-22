//! End-to-end htmx flow for the CLI-login page.
//!
//! 1. Browser (session cookie) POSTs `/auth/cli/issue` with `HX-Request: true`
//!    → response is an HTML fragment (not JSON) containing the one-time code.
//! 2. CLI exchanges the code on `/auth/cli/exchange` → receives a PAT.
//! 3. Replay of the same code → 401 (`cli_codes::take` is atomic).
//!
//! Also asserts that a *plain* POST to `/auth/cli/issue` (no hx-request /
//! Accept header) still returns JSON, preserving the existing API contract
//! for CLI clients that never touch the htmx path.

use reqwest::StatusCode;

use crate::common::HttpHarness;

fn extract_code_from_fragment(html: &str) -> String {
    let anchor = html.find(r#"class="pat-code""#).expect("pat-code div");
    let rest = &html[anchor..];
    let open = rest.find('>').expect("open tag") + 1;
    let close = rest[open..].find("</div>").expect("close div");
    rest[open..open + close].trim().to_string()
}

#[tokio::test]
async fn htmx_issue_returns_html_fragment_and_cli_can_exchange() {
    let h = HttpHarness::start_with_auth().await;
    let session = h
        .seed_user_and_session("bob@example.com", "longenough")
        .await;
    let client = reqwest::Client::new();

    // Step 1: htmx-style POST /auth/cli/issue
    let fragment = client
        .post(h.url("/auth/cli/issue"))
        .header("cookie", format!("wafer_session={session}"))
        .header("hx-request", "true")
        .send()
        .await
        .expect("POST /auth/cli/issue");
    assert_eq!(fragment.status(), StatusCode::OK);
    let ct = fragment
        .headers()
        .get("content-type")
        .expect("content-type")
        .to_str()
        .expect("ascii")
        .to_string();
    assert!(ct.starts_with("text/html"), "got {ct}");
    let html = fragment.text().await.expect("body");
    assert!(!html.contains("<html"), "should be a fragment: {html}");
    assert!(
        html.contains(r#"class="pat-code""#),
        "pat-code div missing: {html}"
    );

    let code = extract_code_from_fragment(&html);
    assert!(!code.is_empty(), "empty code: {html}");

    // Step 2: CLI exchanges the code
    let exchanged = client
        .post(h.url("/auth/cli/exchange"))
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .expect("POST /auth/cli/exchange");
    assert_eq!(exchanged.status(), StatusCode::OK);
    let body: serde_json::Value = exchanged.json().await.expect("json");
    let token = body["token"].as_str().expect("token");
    assert!(token.starts_with("wafer_pat_"), "got {token}");

    // Step 3: replay is rejected (one-time use)
    let replay = client
        .post(h.url("/auth/cli/exchange"))
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .expect("POST /auth/cli/exchange replay");
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn plain_post_cli_issue_still_returns_json() {
    let h = HttpHarness::start_with_auth().await;
    let session = h
        .seed_user_and_session("carol@example.com", "longenough")
        .await;
    let client = reqwest::Client::new();

    let resp = client
        .post(h.url("/auth/cli/issue"))
        .header("cookie", format!("wafer_session={session}"))
        .send()
        .await
        .expect("POST /auth/cli/issue");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .expect("content-type")
        .to_str()
        .expect("ascii")
        .to_string();
    assert!(
        ct.starts_with("application/json"),
        "plain call should return JSON, got {ct}"
    );
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(body["code"].is_string());
    assert!(body["expires_at"].is_string());
}

#[tokio::test]
async fn accept_html_header_also_triggers_fragment_path() {
    let h = HttpHarness::start_with_auth().await;
    let session = h
        .seed_user_and_session("dan@example.com", "longenough")
        .await;
    let client = reqwest::Client::new();

    let resp = client
        .post(h.url("/auth/cli/issue"))
        .header("cookie", format!("wafer_session={session}"))
        .header("accept", "text/html")
        .send()
        .await
        .expect("POST /auth/cli/issue with Accept: text/html");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("body");
    assert!(
        body.contains(r#"class="pat-code""#),
        "expected fragment: {body}"
    );
}
