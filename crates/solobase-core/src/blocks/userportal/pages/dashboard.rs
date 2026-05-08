//! `/b/userportal/` — portal home.
//!
//! Anonymous → 302 to `/b/auth/login`. Authenticated → renders the
//! configured apps grid (from this block's own `buttons` collection)
//! and the user's claimed orgs (read directly from auth/repo/orgs).

use maud::{html, Markup};
use wafer_core::clients::database::Record;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{
        auth::repo::orgs,
        helpers::{RecordExt, ResponseBuilder},
    },
    ui::{
        nav_groups,
        shell::{Crumb, Topbar},
        shelled_response,
        sidebar::nav_icon,
        SiteConfig, UserInfo,
    },
};

struct DashboardButton {
    label: String,
    icon: String,
    path: String,
}

/// GET `/b/userportal/`. Anonymous users redirected to login.
pub async fn dashboard_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let buttons = load_buttons(ctx).await;
    let orgs_list = orgs::list_for_user(ctx, &user_id).await.unwrap_or_default();

    let user = UserInfo::from_message(msg);
    let welcome_subject = user
        .as_ref()
        .map(|u| u.email.clone())
        .unwrap_or_else(|| "there".to_string());
    let title_owned = format!("Welcome back, {welcome_subject}");

    let apps_card = render_apps_card(&buttons);
    let orgs_card = render_orgs_card(&orgs_list);

    let body = crate::ui::templates::dashboard_page(
        crate::ui::templates::PageHeader {
            title: title_owned.as_str(),
            subtitle: Some("Your apps and connected organizations."),
            primary_action: None,
        },
        Vec::new(),
        orgs_card,
        html! { div {} },
        None,
        Some(apps_card),
    );

    let config = SiteConfig::load(ctx).await;
    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Dashboard",
            href: None,
        }],
        subtitle: None,
        primary_action: None,
        show_palette: true,
    };
    shelled_response(
        msg,
        "Dashboard",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

async fn load_buttons(ctx: &dyn Context) -> Vec<DashboardButton> {
    super::super::load_buttons(ctx)
        .await
        .into_iter()
        .map(|r: Record| DashboardButton {
            label: r.str_field("label").to_string(),
            icon: r.str_field("icon").to_string(),
            path: r.str_field("path").to_string(),
        })
        .collect()
}

fn render_apps_card(buttons: &[DashboardButton]) -> Markup {
    html! {
        section .card .dashboard-apps-card {
            header .card__head { h3 .card__title { "Your apps" } }
            div .card__body {
                @if buttons.is_empty() {
                    p .text-muted {
                        "No apps configured. Ask an admin to add tiles in Portal settings."
                    }
                } @else {
                    div .dashboard-apps-grid {
                        @for b in buttons {
                            a .dashboard-app-tile href=(b.path) {
                                span .dashboard-app-tile__icon { (nav_icon(&b.icon)) }
                                span .dashboard-app-tile__label { (b.label) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_orgs_card(orgs: &[orgs::OrgRow]) -> Markup {
    html! {
        section .card .dashboard-orgs-card {
            header .card__head {
                h3 .card__title { "Your organizations" }
                a .card__head-action href="/b/auth/orgs" { "View all →" }
            }
            div .card__body {
                @if orgs.is_empty() {
                    p .text-muted {
                        "No claimed organizations. Sign in with GitHub, Google, or Microsoft to claim one."
                    }
                } @else {
                    ul .orgs-list {
                        @for o in orgs.iter().take(5) {
                            li .orgs-list-row {
                                span .orgs-list-row__provider {
                                    (o.verified_via.as_deref().unwrap_or("manual"))
                                }
                                span .orgs-list-row__name { (o.name) }
                                span .orgs-list-row__date { "claimed " (o.created_at) }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use serde_json::json;
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::{
            auth::repo::orgs::{upsert_claimed, NewClaim},
            userportal::UserPortalBlock,
        },
        test_support::{
            anon_msg, auth_msg, output_header, output_html, output_status, TestContext,
        },
    };

    async fn ctx_with_userportal() -> TestContext {
        let mut ctx = TestContext::with_auth().await;
        ctx.register_block("suppers-ai/userportal", Arc::new(UserPortalBlock));
        ctx
    }

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

    fn button_data(
        label: &str,
        icon: &str,
        path: &str,
        sort_order: i64,
    ) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("label".to_string(), json!(label));
        m.insert("icon".to_string(), json!(icon));
        m.insert("path".to_string(), json!(path));
        m.insert("sort_order".to_string(), json!(sort_order));
        m
    }

    #[tokio::test]
    async fn anonymous_redirects_to_login() {
        let ctx = ctx_with_userportal().await;
        let msg = anon_msg("retrieve", "/b/userportal/");
        let resp = dashboard_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 302);
    }

    #[tokio::test]
    async fn anonymous_redirect_sets_location_header() {
        let ctx = ctx_with_userportal().await;
        let msg = anon_msg("retrieve", "/b/userportal/");
        let resp = dashboard_page(&ctx, &msg).await;
        assert_eq!(
            output_header(resp, "Location").await.as_deref(),
            Some("/b/auth/login")
        );
    }

    #[tokio::test]
    async fn authenticated_renders_both_sections() {
        let ctx = ctx_with_userportal().await;
        seed_user(&ctx, "user-a").await;
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "acme",
                owner_user_id: "user-a",
                verified_via: "github",
                verified_ref: "gh-1",
            },
        )
        .await
        .unwrap();
        db::create(
            &ctx,
            "suppers_ai__userportal__buttons",
            button_data("Solobase", "shield", "/b/admin/", 0),
        )
        .await
        .unwrap();

        let msg = auth_msg("retrieve", "/b/userportal/", "user-a");
        let resp = dashboard_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 200);
    }

    #[tokio::test]
    async fn authenticated_includes_apps_orgs_section_titles() {
        let ctx = ctx_with_userportal().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/", "user-a");
        let resp = dashboard_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("Your apps"), "missing apps section");
        assert!(html.contains("Your organizations"), "missing orgs section");
    }

    #[tokio::test]
    async fn authenticated_with_no_orgs_shows_empty_state() {
        let ctx = ctx_with_userportal().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/", "user-a");
        let resp = dashboard_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(
            html.contains("No claimed organizations"),
            "missing empty state"
        );
    }

    #[tokio::test]
    async fn authenticated_with_orgs_renders_org_name() {
        let ctx = ctx_with_userportal().await;
        seed_user(&ctx, "user-a").await;
        upsert_claimed(
            &ctx,
            NewClaim {
                name: "acme-corp",
                owner_user_id: "user-a",
                verified_via: "github",
                verified_ref: "gh-1",
            },
        )
        .await
        .unwrap();
        let msg = auth_msg("retrieve", "/b/userportal/", "user-a");
        let resp = dashboard_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("acme-corp"), "missing org name");
    }
}
