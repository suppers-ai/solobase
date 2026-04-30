//! New shell — replaces `layout::block_shell` body. Renders sidebar
//! (left) + topbar (top of content) + body (the rest). Pages declare
//! `Topbar` inputs; this module owns the chrome.

use maud::{html, Markup};

use super::{
    sidebar::{sidebar_grouped, NavGroup},
    NavItem, UserInfo,
};

/// One breadcrumb segment.
pub struct Crumb<'a> {
    pub label: &'a str,
    pub href: Option<&'a str>,
}

/// Topbar inputs declared by each page.
pub struct Topbar<'a> {
    pub crumbs: Vec<Crumb<'a>>,
    pub primary_action: Option<Markup>,
    /// Whether to render the ⌘K palette trigger (every shelled page = true).
    pub show_palette: bool,
}

impl<'a> Default for Topbar<'a> {
    fn default() -> Self {
        Self {
            crumbs: Vec::new(),
            primary_action: None,
            show_palette: true,
        }
    }
}

fn render_topbar(t: &Topbar<'_>) -> Markup {
    // Skip rendering entirely when nothing was declared — avoids an empty
    // stripe on pages that don't need a topbar.
    if t.crumbs.is_empty() && t.primary_action.is_none() && !t.show_palette {
        return html! {};
    }
    html! {
        header .topbar {
            nav .topbar__crumbs aria-label="Breadcrumb" {
                ol {
                    @for (i, c) in t.crumbs.iter().enumerate() {
                        @let last = i + 1 == t.crumbs.len();
                        li {
                            @match c.href {
                                Some(h) if !last => a href=(h) { (c.label) },
                                _ => span aria-current=[last.then_some("page")] { (c.label) },
                            }
                        }
                    }
                }
            }
            div .topbar__right {
                @if t.show_palette {
                    button .topbar__palette type="button"
                        data-action="palette-open"
                        aria-keyshortcuts="Meta+K Control+K"
                        aria-label="Open command palette" {
                        span { "Quick jump" }
                        kbd { "⌘K" }
                    }
                }
                @if let Some(a) = &t.primary_action {
                    div .topbar__action { (a.clone()) }
                }
            }
        }
    }
}

/// Renders sidebar + topbar + body in the standard 12-col grid.
///
/// `nav_groups` partitions the sidebar (Workspace / Data / System for admin,
/// Account / Apps for portal). `user` is pinned at the sidebar bottom.
pub fn shell(
    nav_groups: &[NavGroup],
    user: Option<&UserInfo>,
    current_path: &str,
    logo_url: &str,
    logo_icon_url: &str,
    topbar: Topbar<'_>,
    body: Markup,
) -> Markup {
    html! {
        div .shell {
            (sidebar_grouped(nav_groups, user, current_path, logo_url, logo_icon_url))
            div .shell__main {
                (render_topbar(&topbar))
                div .shell__body { (body) }
            }
        }
    }
}

/// Backward-compat helper used by the existing `layout::block_shell`.
/// Wraps a flat `Vec<NavItem>` into one unlabeled `NavGroup`.
pub fn one_group(items: Vec<NavItem>) -> Vec<NavGroup> {
    vec![NavGroup { label: None, items }]
}

#[cfg(test)]
mod tests {
    use maud::html;

    use super::*;
    use crate::ui::NavItem;

    fn item(label: &str, href: &str) -> NavItem {
        NavItem {
            label: label.to_string(),
            href: href.to_string(),
            icon: "circle",
        }
    }

    #[test]
    fn shell_with_breadcrumb_and_palette_button() {
        let groups = one_group(vec![item("Users", "/b/admin/users")]);
        let topbar = Topbar {
            crumbs: vec![
                Crumb {
                    label: "Workspace",
                    href: Some("/b/admin"),
                },
                Crumb {
                    label: "Users",
                    href: None,
                },
            ],
            primary_action: None,
            show_palette: true,
        };
        let body = html! { p { "page body" } };
        let s = shell(&groups, None, "/b/admin/users", "", "", topbar, body).into_string();
        assert!(s.contains("topbar__crumbs"));
        assert!(s.contains(">Workspace<"));
        assert!(s.contains(r#"aria-current="page""#));
        assert!(s.contains(r#"data-action="palette-open""#));
        assert!(s.contains("page body"));
    }

    #[test]
    fn shell_can_omit_palette() {
        let groups = one_group(vec![item("X", "/x")]);
        let mut tb = Topbar::default();
        tb.show_palette = false;
        // Need at least one declared input to render the topbar at all.
        tb.crumbs = vec![Crumb {
            label: "X",
            href: None,
        }];
        let s = shell(&groups, None, "/x", "", "", tb, html! {}).into_string();
        assert!(!s.contains("topbar__palette"));
        assert!(s.contains("topbar__crumbs"));
    }

    #[test]
    fn shell_renders_no_topbar_when_all_inputs_empty() {
        let groups = one_group(vec![item("X", "/x")]);
        let mut tb = Topbar::default();
        tb.show_palette = false;
        let s = shell(&groups, None, "/x", "", "", tb, html! { "body" }).into_string();
        // No topbar element at all when there's nothing to render in it.
        assert!(!s.contains(r#"class="topbar""#));
        assert!(s.contains(">body<") || s.contains(">body</"));
    }
}
