//! `/b/auth/orgs` — read-only list of orgs claimed by the current user.

use maud::{html, Markup};
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{auth::repo::orgs, helpers::ResponseBuilder},
    ui::{
        nav_groups,
        shell::{Crumb, Topbar},
        shelled_response, SiteConfig, UserInfo,
    },
};

/// GET `/b/auth/orgs`. Anonymous users redirected to login.
pub async fn orgs_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let orgs_list = orgs::list_for_user(ctx, &user_id).await.unwrap_or_default();
    let body = crate::ui::templates::list_page(
        crate::ui::templates::PageHeader {
            title: "Organizations",
            subtitle: Some("Orgs you've claimed via GitHub, Google, or Microsoft sign-in."),
            primary_action: None,
        },
        None,
        render_orgs_table(&orgs_list),
        None,
    );

    let config = SiteConfig::load(ctx).await;
    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![
            Crumb {
                label: "Dashboard",
                href: Some("/b/auth/dashboard"),
            },
            Crumb {
                label: "Organizations",
                href: None,
            },
        ],
        primary_action: None,
        show_palette: true,
    };
    let user = UserInfo::from_message(msg);
    shelled_response(
        msg,
        "Organizations",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

fn render_orgs_table(orgs: &[orgs::OrgRow]) -> Markup {
    if orgs.is_empty() {
        return html! {
            div .empty-state {
                p { "No claimed organizations." }
                p .text-muted {
                    "Sign in with GitHub, Google, or Microsoft to claim one."
                }
            }
        };
    }
    html! {
        table .data-table {
            thead {
                tr {
                    th { "Provider" }
                    th { "Organization" }
                    th { "Claimed" }
                }
            }
            tbody {
                @for o in orgs {
                    tr {
                        td {
                            span .badge .badge--neutral {
                                (o.verified_via.as_deref().unwrap_or("manual"))
                            }
                        }
                        td { (o.name) }
                        td { (o.created_at) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::auth::repo::orgs::{upsert_claimed, NewClaim},
        test_support::{
            anon_msg, auth_msg, output_header, output_html, output_status, TestContext,
        },
    };

    async fn seed_user(ctx: &TestContext, user_id: &str) {
        db::exec_raw(
            ctx,
            "INSERT INTO suppers_ai__auth__users (id, email, display_name, role, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                json!(user_id),
                json!(format!("{user_id}@example.com")),
                json!(user_id),
                json!("user"),
                json!("2026-01-01T00:00:00Z"),
                json!("2026-01-01T00:00:00Z"),
            ],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn anonymous_redirects_to_login() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("retrieve", "/b/auth/orgs");
        let resp = orgs_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 302);
    }

    #[tokio::test]
    async fn anonymous_redirect_sets_location() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("retrieve", "/b/auth/orgs");
        let resp = orgs_page(&ctx, &msg).await;
        assert_eq!(
            output_header(resp, "Location").await.as_deref(),
            Some("/b/auth/login")
        );
    }

    #[tokio::test]
    async fn empty_renders_empty_state_copy() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/auth/orgs", "user-a");
        let resp = orgs_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("No claimed organizations"));
        assert!(html.contains("Sign in with GitHub"));
    }

    #[tokio::test]
    async fn populated_renders_one_row_per_org() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "alpha",
                owner_user_id: "user-a",
                verified_via: "github",
                verified_ref: "gh-1",
            },
        )
        .await
        .unwrap();
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "beta",
                owner_user_id: "user-a",
                verified_via: "google",
                verified_ref: "gg-2",
            },
        )
        .await
        .unwrap();

        let msg = auth_msg("retrieve", "/b/auth/orgs", "user-a");
        let resp = orgs_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("alpha"));
        assert!(html.contains("beta"));
        assert!(html.contains("github"));
        assert!(html.contains("google"));
    }
}
