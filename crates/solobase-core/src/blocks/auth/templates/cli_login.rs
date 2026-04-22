//! `GET /auth/cli` template — authenticated landing page that issues
//! one-time codes for `wafer login`.
//!
//! Clicking the button POSTs to `/auth/cli/issue` via htmx; the fragment
//! returned is rendered by [`super::cli_code_fragment`] and swapped into
//! `#code-panel`.

use maud::{html, Markup};

use crate::blocks::auth::{
    templates::base::{layout, LayoutProps},
    view_models::CliLoginViewModel,
};

pub fn render(vm: &CliLoginViewModel) -> Markup {
    let body = html! {
        h1 { "CLI login" }
        p {
            "Click the button below to issue a one-time code. Paste it into \
             the terminal where `wafer login` is waiting."
        }
        button
            type="button"
            hx-post="/auth/cli/issue"
            hx-target="#code-panel"
            hx-swap="innerHTML"
        { "Issue CLI token" }
        div id="code-panel" style="margin-top: 1.5rem;" {}
    };
    layout(LayoutProps {
        title: "CLI login",
        user: Some(&vm.user),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::auth::view_models::NavUser;

    #[test]
    fn renders_issue_button_with_htmx_attrs() {
        let vm = CliLoginViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"hx-post="/auth/cli/issue""#));
        assert!(out.contains(r##"hx-target="#code-panel""##));
        assert!(out.contains(r#"hx-swap="innerHTML""#));
        assert!(out.contains(r#"<div id="code-panel""#));
        assert!(out.contains("<button"));
    }

    #[test]
    fn nav_shows_dashboard_link_for_signed_in_user() {
        let vm = CliLoginViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<a href="/auth/dashboard">Dashboard</a>"#));
    }
}
