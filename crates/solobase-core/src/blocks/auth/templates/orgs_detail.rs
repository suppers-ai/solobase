//! `GET /auth/orgs/{name}` template.
//!
//! The public section (name, reserved badge, verified-via line) always
//! renders. The Manage section only renders when
//! [`OrgsDetailViewModel::viewer_is_admin`] is true — set by the handler
//! after calling `verify_org_admin`.

use maud::{html, Markup};

use crate::blocks::auth::{
    templates::base::{layout, LayoutProps},
    view_models::OrgsDetailViewModel,
};

pub fn render(vm: &OrgsDetailViewModel) -> Markup {
    let body = html! {
        h1 { (vm.org.name) }
        p {
            @if vm.is_reserved {
                em { "reserved organisation" }
                br;
            }
            @if let Some(via) = &vm.org.verified_via {
                "verified via " (via)
                @if let Some(r) = &vm.org.verified_ref { " (" (r) ")" }
            }
        }

        @if vm.viewer_is_admin {
            section {
                h2 { "Manage" }
                p { "You are an owner of this organisation." }
                ul {
                    li { a href=(format!("/registry/orgs/{}/packages", vm.org.name)) { "Packages" } }
                    li { a href=(format!("/registry/orgs/{}/publish", vm.org.name)) { "Publish" } }
                }
            }
        }
    };
    layout(LayoutProps {
        title: &vm.org.name,
        user: vm.user.as_ref(),
        body,
    })
}

#[cfg(test)]
mod tests {
    use wafer_core::interfaces::auth::service::OrgSummary;

    use super::*;
    use crate::blocks::auth::view_models::NavUser;

    fn org(name: &str, reserved: bool) -> OrgSummary {
        OrgSummary {
            name: name.into(),
            verified_via: Some("github".into()),
            verified_ref: Some(name.into()),
            is_reserved: reserved,
        }
    }

    #[test]
    fn public_view_shows_no_admin_actions() {
        let vm = OrgsDetailViewModel {
            user: None,
            org: org("acme", false),
            viewer_is_admin: false,
            is_reserved: false,
        };
        let out = render(&vm).into_string();
        assert!(out.contains("acme"));
        assert!(out.contains("verified via github"));
        assert!(!out.contains("Manage"));
    }

    #[test]
    fn admin_view_shows_manage_section() {
        let user = NavUser {
            display_name: "Alice".into(),
            avatar_url: None,
            is_admin: false,
        };
        let vm = OrgsDetailViewModel {
            user: Some(user),
            org: org("acme", false),
            viewer_is_admin: true,
            is_reserved: false,
        };
        let out = render(&vm).into_string();
        assert!(out.contains("Manage"));
        assert!(out.contains(r#"<a href="/registry/orgs/acme/packages">Packages</a>"#));
        assert!(out.contains(r#"<a href="/registry/orgs/acme/publish">Publish</a>"#));
    }

    #[test]
    fn reserved_org_labelled() {
        let vm = OrgsDetailViewModel {
            user: None,
            org: org("wafer-run", true),
            viewer_is_admin: false,
            is_reserved: true,
        };
        let out = render(&vm).into_string();
        assert!(out.contains("reserved"));
    }

    #[test]
    fn no_user_in_nav_when_viewer_anonymous() {
        let vm = OrgsDetailViewModel {
            user: None,
            org: org("acme", false),
            viewer_is_admin: false,
            is_reserved: false,
        };
        let out = render(&vm).into_string();
        assert!(out.contains(r#"<a href="/auth/login">Sign in</a>"#));
    }
}
