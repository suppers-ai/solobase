//! Shared HTML layout for all Plan D auth pages.
//!
//! The template is intentionally minimal: inline CSS (no external
//! stylesheet), htmx loaded from a CDN for the CLI-login fragment flow,
//! a nav bar that flips between logged-out and logged-in states, and a
//! `<main>` slot for page-specific content.

use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::blocks::auth::view_models::NavUser;

/// Props passed to [`layout`]. Kept as a struct so new props can be added
/// without breaking call sites.
pub struct LayoutProps<'a> {
    pub title: &'a str,
    pub user: Option<&'a NavUser>,
    pub body: Markup,
}

const INLINE_CSS: &str = r#"
body { font-family: system-ui, -apple-system, sans-serif; max-width: 48rem; margin: 2rem auto; padding: 0 1rem; color: #24292f; }
nav { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid #d0d7de; margin-bottom: 1.5rem; }
nav a { margin-left: 1rem; color: #0969da; text-decoration: none; }
nav a:first-child { margin-left: 0; font-weight: 600; color: #24292f; }
nav a:hover { text-decoration: underline; }
nav form { display: inline; margin-left: 1rem; }
nav button { background: none; border: none; cursor: pointer; color: #0969da; font: inherit; padding: 0; }
nav button:hover { text-decoration: underline; }
main { min-height: 60vh; }
h1 { margin-top: 0; }
.error { background: #ffebe9; border: 1px solid #ff8182; padding: 0.5rem 1rem; border-radius: 6px; margin-bottom: 1rem; color: #82071e; }
form.auth-form { display: flex; flex-direction: column; gap: 0.75rem; max-width: 24rem; }
form.auth-form label { display: flex; flex-direction: column; font-size: 0.9rem; gap: 0.25rem; }
input[type=email], input[type=password], input[type=text], select { padding: 0.5rem; font: inherit; border: 1px solid #d0d7de; border-radius: 6px; }
button[type=submit] { background: #1f883d; color: white; border: none; border-radius: 6px; padding: 0.5rem 1rem; cursor: pointer; font: inherit; }
button[type=submit]:hover { background: #1a7f37; }
.pat-code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; background: #f6f8fa; padding: 0.75rem; border-radius: 6px; word-break: break-all; border: 1px solid #d0d7de; }
table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
th, td { text-align: left; padding: 0.5rem; border-bottom: 1px solid #d0d7de; }
section { margin-bottom: 2rem; }
.oauth-buttons { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 1rem; }
.oauth-buttons a { padding: 0.5rem 1rem; background: #f6f8fa; border: 1px solid #d0d7de; border-radius: 6px; color: #24292f; text-decoration: none; }
.oauth-buttons a:hover { background: #eaeef2; }
"#;

/// Render the full HTML document. Call from every page template.
pub fn layout(props: LayoutProps<'_>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (props.title) " · suppers-ai" }
                style { (PreEscaped(INLINE_CSS)) }
                script src="https://unpkg.com/htmx.org@1.9.12" {}
            }
            body {
                nav {
                    a href="/" { "suppers-ai" }
                    div {
                        @match props.user {
                            Some(user) => {
                                a href="/auth/dashboard" { "Dashboard" }
                                @if user.is_admin {
                                    a href="/admin" { "Admin" }
                                }
                                form method="post" action="/auth/logout" {
                                    button type="submit" { "Sign out (" (user.display_name) ")" }
                                }
                            }
                            None => {
                                a href="/auth/login" { "Sign in" }
                            }
                        }
                    }
                }
                main {
                    (props.body)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use maud::html;

    use super::*;

    #[test]
    fn base_renders_title_and_body() {
        let out = layout(LayoutProps {
            title: "Sign in",
            user: None,
            body: html! { p { "hello" } },
        })
        .into_string();
        assert!(out.starts_with("<!DOCTYPE html>"), "got: {}", &out[..40]);
        assert!(out.contains("<title>Sign in · suppers-ai</title>"));
        assert!(out.contains("<main><p>hello</p></main>"));
        assert!(out.contains(r#"<a href="/auth/login">Sign in</a>"#));
    }

    #[test]
    fn base_shows_dashboard_link_when_user_present() {
        let user = NavUser {
            display_name: "Alice".into(),
            avatar_url: None,
            is_admin: false,
        };
        let out = layout(LayoutProps {
            title: "Dashboard",
            user: Some(&user),
            body: html! { "x" },
        })
        .into_string();
        assert!(out.contains(r#"<a href="/auth/dashboard">Dashboard</a>"#));
        assert!(out.contains(r#"<form method="post" action="/auth/logout">"#));
        assert!(!out.contains(r#"<a href="/auth/login">Sign in</a>"#));
    }

    #[test]
    fn admin_user_gets_admin_link() {
        let user = NavUser {
            display_name: "Admin".into(),
            avatar_url: None,
            is_admin: true,
        };
        let out = layout(LayoutProps {
            title: "Dashboard",
            user: Some(&user),
            body: html! { "x" },
        })
        .into_string();
        assert!(out.contains(r#"<a href="/admin">Admin</a>"#));
    }
}
