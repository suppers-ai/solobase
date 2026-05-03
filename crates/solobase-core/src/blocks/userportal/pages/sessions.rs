//! `/b/userportal/sessions` — list active sessions, revoke individual ones.

use maud::{html, Markup};
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{auth::repo::sessions, helpers::ResponseBuilder},
    ui::{
        nav_groups,
        shell::{Crumb, Topbar},
        shelled_response, SiteConfig, UserInfo,
    },
};

pub async fn sessions_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let rows = sessions::list_for_user(ctx, &user_id)
        .await
        .unwrap_or_default();

    let body = crate::ui::templates::list_page(
        crate::ui::templates::PageHeader {
            title: "Active sessions",
            subtitle: Some("Sessions signed in to your account. Revoke any you don't recognize."),
            primary_action: None,
        },
        None,
        render_table(&rows),
        None,
    );

    let config = SiteConfig::load(ctx).await;
    let groups = nav_groups::portal();
    let user = UserInfo::from_message(msg);
    let topbar = Topbar {
        crumbs: vec![
            Crumb {
                label: "Dashboard",
                href: Some("/b/auth/dashboard"),
            },
            Crumb {
                label: "Sessions",
                href: None,
            },
        ],
        primary_action: None,
        show_palette: true,
    };
    shelled_response(
        msg,
        "Sessions",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn render_table(rows: &[sessions::SessionRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state { p { "No active sessions." } }
        };
    }
    // TODO(phase-4-followup): mark the current session with a badge and
    // suppress its revoke button. Requires Message to expose the request's
    // session token hash — not currently surfaced through the handler API.
    html! {
        table .data-table {
            thead {
                tr {
                    th { "Started" }
                    th { "Last used" }
                    th { "Expires" }
                    th { "" }
                }
            }
            tbody {
                @for r in rows {
                    tr .session-row {
                        td { (r.created_at) }
                        td { (r.last_used_at) }
                        td { (r.expires_at) }
                        td {
                            button .btn .btn-ghost .btn-sm
                                hx-delete=(format!("/b/userportal/sessions/{}", hex_encode(&r.token_hash)))
                                hx-target="closest tr"
                                hx-swap="outerHTML"
                            { "Revoke" }
                        }
                    }
                }
            }
        }
    }
}

/// DELETE `/b/userportal/sessions/{token_hash_hex}`. Scoped to caller's
/// user_id — refusing to revoke another user's session looks indistinguishable
/// from "no such session" (returns 200 with no body either way; htmx removes
/// the row). Returns 401 if anonymous, 400 if hex is malformed.
pub async fn handle_revoke(ctx: &dyn Context, msg: &Message, sub: &str) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(401)
            .body(b"unauthenticated".to_vec(), "text/plain");
    }
    let hex_part = sub.strip_prefix("/sessions/").unwrap_or("");
    let hash = match decode_hex(hex_part) {
        Some(h) if !h.is_empty() => h,
        _ => {
            return ResponseBuilder::new()
                .status(400)
                .body(b"bad token_hash".to_vec(), "text/plain");
        }
    };
    let _ = sessions::delete_for_user(ctx, &user_id, &hash).await;
    // Empty 200 — htmx swaps the row out via outerHTML.
    ResponseBuilder::new()
        .status(200)
        .body(Vec::new(), "text/html")
}

#[cfg(test)]
mod tests {
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::auth::repo::sessions::{insert, NewSession},
        test_support::{anon_msg, auth_msg, output_html, output_status, TestContext},
    };

    async fn seed_user(ctx: &TestContext, user_id: &str) {
        db::exec_raw(
            ctx,
            "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                serde_json::json!(user_id),
                serde_json::json!(format!("{user_id}@example.com")),
                serde_json::json!(user_id),
                serde_json::json!("user"),
                serde_json::json!("2026-01-01T00:00:00Z"),
                serde_json::json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .unwrap();
    }

    fn fake_session(user_id: &str, hash_byte: u8) -> NewSession {
        NewSession {
            token_hash: vec![hash_byte; 32],
            user_id: user_id.into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn anonymous_redirects_to_login() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("retrieve", "/b/userportal/sessions");
        let resp = sessions_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 302);
    }

    #[tokio::test]
    async fn empty_renders_empty_state() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/sessions", "user-a");
        let resp = sessions_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("No active sessions"));
    }

    #[tokio::test]
    async fn populated_renders_one_row_per_session_with_revoke() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        insert(&ctx, fake_session("user-a", 0x01)).await.unwrap();
        insert(&ctx, fake_session("user-a", 0x02)).await.unwrap();

        let msg = auth_msg("retrieve", "/b/userportal/sessions", "user-a");
        let resp = sessions_page(&ctx, &msg).await;
        let html = output_html(resp).await;

        assert!(html.contains("Active sessions"), "missing page title");
        // Two Revoke buttons (one per row).
        assert!(
            html.matches(">Revoke<").count() >= 2,
            "expected ≥2 Revoke buttons, got: {}",
            html.matches(">Revoke<").count()
        );
    }

    #[tokio::test]
    async fn revoke_anonymous_returns_401() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("delete", "/b/userportal/sessions/aabb");
        let resp = handle_revoke(&ctx, &msg, "/sessions/aabb").await;
        assert_eq!(output_status(resp).await, 401);
    }

    #[tokio::test]
    async fn revoke_malformed_hex_returns_400() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("delete", "/b/userportal/sessions/zzz", "user-a");
        let resp = handle_revoke(&ctx, &msg, "/sessions/zzz").await;
        assert_eq!(output_status(resp).await, 400);
    }

    #[tokio::test]
    async fn revoke_own_session_deletes_it() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        insert(&ctx, fake_session("user-a", 0x01)).await.unwrap();
        assert_eq!(
            sessions::list_for_user(&ctx, "user-a").await.unwrap().len(),
            1
        );

        let hex_hash: String = (0..32).map(|_| "01".to_string()).collect();
        let msg = auth_msg(
            "delete",
            &format!("/b/userportal/sessions/{hex_hash}"),
            "user-a",
        );
        let resp = handle_revoke(&ctx, &msg, &format!("/sessions/{hex_hash}")).await;
        assert_eq!(output_status(resp).await, 200);
        assert_eq!(
            sessions::list_for_user(&ctx, "user-a").await.unwrap().len(),
            0
        );
    }

    #[tokio::test]
    async fn revoke_other_users_session_is_no_op_returns_200() {
        let ctx = TestContext::with_auth().await;
        for u in ["user-a", "user-b"] {
            seed_user(&ctx, u).await;
        }
        insert(&ctx, fake_session("user-b", 0x02)).await.unwrap();

        // user-a tries to revoke user-b's session.
        let hex_hash: String = (0..32).map(|_| "02".to_string()).collect();
        let msg = auth_msg(
            "delete",
            &format!("/b/userportal/sessions/{hex_hash}"),
            "user-a",
        );
        let resp = handle_revoke(&ctx, &msg, &format!("/sessions/{hex_hash}")).await;
        assert_eq!(output_status(resp).await, 200);
        // user-b's session is still there — no leak.
        assert_eq!(
            sessions::list_for_user(&ctx, "user-b").await.unwrap().len(),
            1
        );
    }
}
