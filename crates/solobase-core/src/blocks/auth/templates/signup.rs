//! `GET /auth/signup` template. Only reachable when
//! `SOLOBASE_SHARED__AUTH__SIGNUP_ENABLED` is true; the handler is
//! responsible for returning 404 when disabled.

use maud::{html, Markup};

use crate::blocks::auth::{
    templates::base::{layout, LayoutProps},
    view_models::SignupViewModel,
};

pub fn render(vm: &SignupViewModel) -> Markup {
    let min = vm.password_min_length;
    let body = html! {
        h1 { "Create account" }
        @if let Some(err) = &vm.error {
            div class="error" { (err) }
        }
        form class="auth-form" method="post" action="/auth/signup" {
            label {
                "Email"
                input type="email" name="email" required autocomplete="email";
            }
            label {
                "Password (at least " (min) " characters)"
                input type="password" name="password" required minlength=(min) autocomplete="new-password";
            }
            @if let Some(next) = &vm.next_path {
                input type="hidden" name="next" value=(next);
            }
            button type="submit" { "Create account" }
        }
        p { "Already have one? " a href="/auth/login" { "Sign in" } "." }
    };
    layout(LayoutProps {
        title: "Create account",
        user: None,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_form_with_min_length_hint() {
        let vm = SignupViewModel {
            error: None,
            password_min_length: 12,
            next_path: None,
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<form class="auth-form" method="post" action="/auth/signup">"#));
        assert!(out.contains(r#"minlength="12""#));
        assert!(out.contains("at least 12 characters"));
    }

    #[test]
    fn error_banner_shown_when_present() {
        let vm = SignupViewModel {
            error: Some("email already registered".into()),
            password_min_length: 8,
            next_path: None,
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<div class="error">email already registered</div>"#));
    }

    #[test]
    fn next_path_preserved_as_hidden_field() {
        let vm = SignupViewModel {
            error: None,
            password_min_length: 8,
            next_path: Some("/auth/dashboard".into()),
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<input type="hidden" name="next" value="/auth/dashboard">"#));
    }

    #[test]
    fn login_fallback_link_present() {
        let vm = SignupViewModel {
            error: None,
            password_min_length: 8,
            next_path: None,
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<a href="/auth/login">Sign in</a>"#));
    }
}
