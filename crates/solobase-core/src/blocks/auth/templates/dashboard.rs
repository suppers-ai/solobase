//! `GET /auth/dashboard` template — profile, orgs, PATs.

use maud::{html, Markup};

use crate::blocks::auth::{
    templates::base::{layout, LayoutProps},
    view_models::DashboardViewModel,
};

pub fn render(vm: &DashboardViewModel) -> Markup {
    let body = html! {
        h1 { "Dashboard" }

        section {
            h2 { "Profile" }
            p { "Signed in as " strong { (vm.email) } }
        }

        section {
            h2 { "Organisations" }
            @if vm.orgs.is_empty() {
                p {
                    "You haven't claimed any orgs yet. "
                    a href="/auth/orgs/claim" { "Claim one" }
                    "."
                }
            } @else {
                ul {
                    @for org in &vm.orgs {
                        li {
                            a href=(format!("/auth/orgs/{}", org.name)) { (org.name) }
                            @if org.is_reserved { " (reserved)" }
                            @if let Some(via) = &org.verified_via {
                                " — verified via " (via)
                            }
                        }
                    }
                }
            }
        }

        section {
            h2 { "Personal access tokens" }
            @if vm.pats.is_empty() {
                p { "No tokens issued yet." }
            } @else {
                table {
                    thead {
                        tr {
                            th { "Name" }
                            th { "Scopes" }
                            th { "Created" }
                            th { "Last used" }
                            th {}
                        }
                    }
                    tbody {
                        @for pat in &vm.pats {
                            tr {
                                td { (pat.name) }
                                td { (pat.scopes.join(", ")) }
                                td { (pat.created_at_iso) }
                                td { (pat.last_used_at_iso.as_deref().unwrap_or("—")) }
                                td {
                                    button
                                        type="button"
                                        hx-delete=(format!("/auth/tokens/{}", pat.id))
                                        hx-confirm="Revoke this token? CLI sessions using it will stop working immediately."
                                        hx-target="closest tr"
                                        hx-swap="outerHTML"
                                    { "Revoke" }
                                }
                            }
                        }
                    }
                }
            }

            h3 { "Issue new token" }
            form class="auth-form" method="post" action="/auth/tokens" {
                label {
                    "Name"
                    input type="text" name="name" required;
                }
                label {
                    "Scope"
                    select name="scope" {
                        option value="publish" { "publish" }
                    }
                }
                button type="submit" { "Issue token" }
            }
        }
    };
    layout(LayoutProps {
        title: "Dashboard",
        user: Some(&vm.user),
        body,
    })
}

#[cfg(test)]
mod tests {
    use wafer_core::interfaces::auth::service::OrgSummary;

    use super::*;
    use crate::blocks::auth::view_models::{NavUser, PatRow};

    #[test]
    fn renders_profile_orgs_and_pats() {
        let vm = DashboardViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
            email: "a@b.com".into(),
            orgs: vec![OrgSummary {
                name: "acme".into(),
                verified_via: Some("github".into()),
                verified_ref: Some("acme".into()),
                is_reserved: false,
            }],
            pats: vec![PatRow {
                id: "p1".into(),
                name: "CLI".into(),
                scopes: vec!["publish".into()],
                created_at_iso: "2026-04-21T10:00:00Z".into(),
                last_used_at_iso: None,
            }],
        };
        let out = render(&vm).into_string();
        assert!(out.contains("a@b.com"));
        assert!(out.contains(r#"<a href="/auth/orgs/acme">acme</a>"#));
        assert!(out.contains("verified via github"));
        assert!(out.contains("publish"));
        assert!(out.contains(r#"<form class="auth-form" method="post" action="/auth/tokens">"#));
        assert!(out.contains(r#"hx-delete="/auth/tokens/p1""#));
    }

    #[test]
    fn empty_orgs_shows_claim_link() {
        let vm = DashboardViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
            email: "a@b.com".into(),
            orgs: vec![],
            pats: vec![],
        };
        let out = render(&vm).into_string();
        assert!(out.contains("haven't claimed any orgs yet"));
        assert!(out.contains(r#"<a href="/auth/orgs/claim">Claim one</a>"#));
    }

    #[test]
    fn empty_pats_shows_placeholder() {
        let vm = DashboardViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
            email: "a@b.com".into(),
            orgs: vec![],
            pats: vec![],
        };
        let out = render(&vm).into_string();
        assert!(out.contains("No tokens issued yet."));
    }

    #[test]
    fn pat_last_used_dash_when_none() {
        let vm = DashboardViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
            email: "a@b.com".into(),
            orgs: vec![],
            pats: vec![PatRow {
                id: "p1".into(),
                name: "CLI".into(),
                scopes: vec!["publish".into()],
                created_at_iso: "2026-04-21T10:00:00Z".into(),
                last_used_at_iso: None,
            }],
        };
        let out = render(&vm).into_string();
        // em-dash placeholder for never-used tokens
        assert!(out.contains("—"));
    }

    #[test]
    fn reserved_org_flagged() {
        let vm = DashboardViewModel {
            user: NavUser {
                display_name: "Alice".into(),
                avatar_url: None,
                is_admin: false,
            },
            email: "a@b.com".into(),
            orgs: vec![OrgSummary {
                name: "wafer-run".into(),
                verified_via: None,
                verified_ref: None,
                is_reserved: true,
            }],
            pats: vec![],
        };
        let out = render(&vm).into_string();
        assert!(out.contains("(reserved)"));
    }
}
