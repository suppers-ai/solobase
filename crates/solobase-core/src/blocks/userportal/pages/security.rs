//! `/b/userportal/security` — change password + linked OAuth providers.
//!
//! TODO(phase-4-followup): add an Email verification section once the
//! users table grows an `email_verified` column. Today there's no
//! persistent verified-status to display, so the section is omitted
//! rather than rendered as a placeholder.

use maud::html;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::{
    blocks::{auth::repo::provider_links, helpers::ResponseBuilder},
    ui::{
        nav_groups,
        shell::{Crumb, Topbar},
        shelled_response, SiteConfig, UserInfo,
    },
};

pub async fn security_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return ResponseBuilder::new()
            .status(302)
            .set_header("Location", "/b/auth/login")
            .body(Vec::new(), "text/plain");
    }

    let links = provider_links::list_for_user(ctx, &user_id)
        .await
        .unwrap_or_default();

    let body = html! {
        section .security-section .card {
            header .card__head { h2 .card__title { "Password" } }
            div .card__body {
                form
                    hx-post="/b/auth/api/change-password"
                    hx-target="#change-pw-result"
                    hx-swap="innerHTML"
                {
                    div .form-group {
                        label .form-label for="current-password" { "Current password" }
                        input .form-input #current-password type="password"
                            name="current_password" required;
                    }
                    div .form-group {
                        label .form-label for="new-password" { "New password" }
                        input .form-input #new-password type="password"
                            name="new_password" required;
                    }
                    div #change-pw-result {}
                    button .btn .btn-primary type="submit" { "Change password" }
                }
            }
        }
        section .security-section .card {
            header .card__head { h2 .card__title { "Linked accounts" } }
            div .card__body {
                @if links.is_empty() {
                    p .text-muted {
                        "No external accounts linked. Sign in with GitHub, Google, or Microsoft to link one."
                    }
                } @else {
                    ul .linked-providers-list {
                        @for l in &links {
                            li .linked-provider {
                                span .linked-provider__name { (l.provider) }
                                span .linked-provider__login { (l.provider_login) }
                                span .linked-provider__date { "linked " (l.linked_at) }
                            }
                        }
                    }
                }
            }
        }
    };

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
                label: "Security",
                href: None,
            },
        ],
        primary_action: None,
        show_palette: true,
    };
    shelled_response(
        msg,
        "Security",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

#[cfg(test)]
mod tests {
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::auth::repo::provider_links::{upsert, NewLink},
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

    #[tokio::test]
    async fn anonymous_redirects_to_login() {
        let ctx = TestContext::with_auth().await;
        let msg = anon_msg("retrieve", "/b/userportal/security");
        let resp = security_page(&ctx, &msg).await;
        assert_eq!(output_status(resp).await, 302);
    }

    #[tokio::test]
    async fn renders_two_sections() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/security", "user-a");
        let resp = security_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("Password"), "missing Password section title");
        assert!(
            html.contains("Linked accounts"),
            "missing Linked accounts section title"
        );
        // Negative: no email verification section in this iteration.
        assert!(!html.contains("Email verification"));
    }

    #[tokio::test]
    async fn change_password_form_posts_to_existing_api() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/security", "user-a");
        let resp = security_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("/b/auth/api/change-password"));
        assert!(html.contains("name=\"current_password\""));
        assert!(html.contains("name=\"new_password\""));
    }

    #[tokio::test]
    async fn linked_accounts_empty_state() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        let msg = auth_msg("retrieve", "/b/userportal/security", "user-a");
        let resp = security_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("No external accounts linked"));
    }

    #[tokio::test]
    async fn linked_accounts_render_when_present() {
        let ctx = TestContext::with_auth().await;
        seed_user(&ctx, "user-a").await;
        upsert(
            &ctx,
            NewLink {
                provider: "github",
                provider_ref: "gh-1",
                user_id: "user-a",
                provider_login: "alice",
                access_token: "tok",
            },
        )
        .await
        .unwrap();
        let msg = auth_msg("retrieve", "/b/userportal/security", "user-a");
        let resp = security_page(&ctx, &msg).await;
        let html = output_html(resp).await;
        assert!(html.contains("github"));
        assert!(html.contains("alice"));
    }
}
