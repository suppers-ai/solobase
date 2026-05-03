//! `/b/userportal/profile` — profile card (avatar, name, email, edit name).
//!
//! Account actions (Change Password, Sign Out) live elsewhere — Sign Out
//! in the sidebar profile menu (`ui/sidebar.rs:115`); Change Password on
//! the security page. This page is just profile info + the rename form.

use maud::html;
use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{
        auth::USERS_COLLECTION,
        helpers::{RecordExt, ResponseBuilder},
    },
    ui::{
        components, nav_groups,
        shell::{Crumb, Topbar},
        shelled_response, SiteConfig, UserInfo,
    },
};

pub async fn profile_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_record = db::get(ctx, USERS_COLLECTION, &user_id).await.ok();
    let display_name = user_record
        .as_ref()
        .map(|r| r.str_field("name").to_string())
        .unwrap_or_default();
    let avatar_url = user_record
        .as_ref()
        .map(|r| r.str_field("avatar_url").to_string())
        .unwrap_or_default();
    let email = user.as_ref().map(|u| u.email.as_str()).unwrap_or("");

    let body = html! {
        (components::page_header("Profile", Some("Your account profile."), None))

        div .card style="margin-bottom:1.5rem" {
            div style="display:flex;align-items:center;gap:1.5rem;padding:1.5rem" {
                div .user-avatar style="width:64px;height:64px;font-size:1.5rem;flex-shrink:0" {
                    @if !avatar_url.is_empty() {
                        img src=(avatar_url) alt="Avatar"
                            style="width:100%;height:100%;border-radius:50%;object-fit:cover";
                    } @else if let Some(u) = &user {
                        (u.avatar_initial())
                    }
                }
                div style="flex:1;min-width:0" {
                    h2 style="margin:0;font-size:1.25rem" {
                        @if display_name.is_empty() { (email) } @else { (display_name) }
                    }
                    p .text-muted style="margin:0.25rem 0 0" { (email) }
                    @if let Some(u) = &user {
                        div style="margin-top:0.5rem;display:flex;gap:0.25rem;flex-wrap:wrap" {
                            @for role in &u.roles {
                                (components::status_badge(role))
                            }
                        }
                    }
                }
            }

            div style="padding:0 1.5rem 1.5rem;border-top:1px solid var(--border-color)" {
                form
                    hx-post="/b/userportal/update-profile"
                    hx-target="#content"
                    hx-swap="innerHTML"
                    style="display:flex;gap:0.5rem;align-items:end;margin-top:1rem"
                {
                    div .form-group style="flex:1;margin:0" {
                        label .form-label for="display-name" { "Display Name" }
                        input .form-input #display-name type="text" name="name"
                            value=(display_name) placeholder="Enter your name";
                    }
                    button .btn .btn-primary type="submit" { "Save" }
                }
            }
        }
    };

    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![
            Crumb {
                label: "Dashboard",
                href: Some("/b/auth/dashboard"),
            },
            Crumb {
                label: "Profile",
                href: None,
            },
        ],
        primary_action: None,
        show_palette: true,
    };
    shelled_response(
        msg,
        "Profile",
        &site_config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{anon_msg, auth_msg, output_html, output_status, TestContext};

    #[tokio::test]
    async fn anonymous_redirects_to_login() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("retrieve", "/b/userportal/profile");
        let resp = profile_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 302);
    }

    #[tokio::test]
    async fn authenticated_renders_profile_with_no_buttons_grid() {
        let ctx = TestContext::with_auth().await;
        let msg = auth_msg("retrieve", "/b/userportal/profile", "user-a");
        let resp = profile_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("Display Name"), "missing edit-name form");
        // Negative assertions: the in-page Account card (with inline Sign Out
        // form and Change Password button) is removed from the page body.
        // The sidebar always includes Sign Out / Change Password in the profile
        // menu, so we check for the content-area-specific markup instead.
        assert!(
            !html.contains("btn-secondary"),
            "in-page Account card buttons removed"
        );
        assert!(
            !html.contains("grid-template-columns:repeat(auto-fill"),
            "buttons grid should not be on profile page"
        );
    }
}
