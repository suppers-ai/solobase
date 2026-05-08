//! GET /b/auth/dashboard — relocated from auth/pages/dashboard.rs in Task 5.
//!
//! Anonymous → 302 to `/b/auth/login`. Authenticated → renders the
//! configured apps grid (fetched from userportal via cross-block call)
//! and the user's claimed orgs (read directly from auth/repo/orgs).

use maud::{html, Markup};
use wafer_run::{
    context::Context,
    types::{Message, WaferError},
    OutputStream,
};

use crate::{
    blocks::{auth::repo::orgs, helpers::ResponseBuilder},
    ui::{
        nav_groups,
        shell::{Crumb, Topbar},
        shelled_response,
        sidebar::nav_icon,
        SiteConfig, UserInfo,
    },
};

#[derive(serde::Deserialize)]
struct DashboardButton {
    label: String,
    icon: String,
    path: String,
}

/// GET `/b/auth/dashboard`. Anonymous users redirected to login.
pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let buttons = fetch_buttons(ctx).await;
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
        primary_action: None,
        subtitle: None,
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

/// Fetch the configured portal buttons from the userportal block via
/// `ctx.call_block_buffered`. Returns an empty vec on any failure
/// (cross-block call denied, JSON parse error, etc.) — buttons are an
/// enhancement, not a requirement; an empty grid is preferable to a 500.
async fn fetch_buttons(ctx: &dyn Context) -> Vec<DashboardButton> {
    let mut msg = Message::new("http.request");
    msg.set_meta("req.action", "retrieve");
    msg.set_meta("req.resource", "/b/userportal/internal/list-buttons");
    let resp: Result<wafer_run::streams::output::BufferedResponse, WaferError> = ctx
        .call_block_buffered("suppers-ai/userportal", msg, &[])
        .await;
    let body = match resp {
        Ok(r) => r.body,
        Err(_) => return Vec::new(),
    };
    serde_json::from_slice::<Vec<DashboardButton>>(&body).unwrap_or_default()
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
