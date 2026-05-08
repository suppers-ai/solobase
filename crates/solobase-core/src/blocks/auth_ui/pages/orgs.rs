//! GET /b/auth/orgs — relocated from auth/pages/orgs.rs in Task 5.

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
pub async fn handle(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        subtitle: None,
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
                        td data-label="Provider" {
                            span .badge .badge--neutral {
                                (o.verified_via.as_deref().unwrap_or("manual"))
                            }
                        }
                        td data-label="Organization" { (o.name) }
                        td data-label="Claimed" { (o.created_at) }
                    }
                }
            }
        }
    }
}
