//! `GET /auth/login` template.
//!
//! Renders an email/password form plus one button per configured OAuth
//! provider. The signup link is only shown when the
//! `SOLOBASE_SHARED__AUTH__SIGNUP_ENABLED` var is set.

use maud::{html, Markup};

use crate::blocks::auth::{
    templates::base::{layout, LayoutProps},
    view_models::LoginViewModel,
};

/// Percent-encode the characters that are unsafe inside a URL query
/// value. Pulled in-file to avoid a new crate dependency — the auth
/// block already has an internal `helpers::urlencode` used by the
/// OAuth handlers but it lives in a private module.
fn encode_query(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

pub fn render(vm: &LoginViewModel) -> Markup {
    let next_q = vm
        .next_path
        .as_deref()
        .map(encode_query)
        .unwrap_or_default();

    let body = html! {
        h1 { "Sign in" }
        @if let Some(err) = &vm.error {
            div class="error" { (err) }
        }
        form class="auth-form" method="post" action="/auth/login" {
            label {
                "Email"
                input type="email" name="email" required autocomplete="email";
            }
            label {
                "Password"
                input type="password" name="password" required autocomplete="current-password";
            }
            @if let Some(next) = &vm.next_path {
                input type="hidden" name="next" value=(next);
            }
            button type="submit" { "Sign in" }
        }

        @if !vm.oauth_providers.is_empty() {
            hr;
            div class="oauth-buttons" {
                @for p in &vm.oauth_providers {
                    a href=(format!("/auth/oauth/{}/start?next={}", p, next_q)) {
                        "Continue with " (p)
                    }
                }
            }
        }

        @if vm.signup_enabled {
            p { "No account? " a href="/auth/signup" { "Create one" } "." }
        }
    };

    layout(LayoutProps {
        title: "Sign in",
        user: None,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_form_and_oauth_buttons() {
        let vm = LoginViewModel {
            error: None,
            signup_enabled: true,
            oauth_providers: vec!["github".into(), "google".into()],
            next_path: Some("/auth/dashboard".into()),
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<form class="auth-form" method="post" action="/auth/login">"#));
        assert!(out.contains(r#"name="email""#));
        assert!(out.contains(r#"name="password""#));
        assert!(out.contains(r#"<input type="hidden" name="next" value="/auth/dashboard">"#));
        assert!(out.contains(r#"href="/auth/oauth/github/start?next=%2Fauth%2Fdashboard""#));
        assert!(out.contains(r#"href="/auth/oauth/google/start?next=%2Fauth%2Fdashboard""#));
        assert!(out.contains(r#"href="/auth/signup""#));
    }

    #[test]
    fn omits_signup_link_when_disabled() {
        let vm = LoginViewModel {
            error: None,
            signup_enabled: false,
            oauth_providers: vec![],
            next_path: None,
        };
        let out = render(&vm).into_string();
        assert!(!out.contains(r#"href="/auth/signup""#));
    }

    #[test]
    fn error_banner_shown_when_present() {
        let vm = LoginViewModel {
            error: Some("invalid credentials".into()),
            signup_enabled: false,
            oauth_providers: vec![],
            next_path: None,
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<div class="error">invalid credentials</div>"#));
    }

    #[test]
    fn no_oauth_section_when_providers_empty() {
        let vm = LoginViewModel {
            error: None,
            signup_enabled: false,
            oauth_providers: vec![],
            next_path: None,
        };
        let out = render(&vm).into_string();
        // Only the CSS rule should match "oauth-buttons"; no rendered div.
        assert!(!out.contains(r#"<div class="oauth-buttons""#));
        assert!(!out.contains("/auth/oauth/"));
    }
}
