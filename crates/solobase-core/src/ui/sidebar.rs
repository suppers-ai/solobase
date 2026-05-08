//! Sidebar component — grouped navigation with brand, groups, and user profile.

use maud::Markup;

use super::icons;

/// Map icon name strings to icon functions.
pub fn nav_icon(name: &str) -> Markup {
    match name {
        "layout-dashboard" | "dashboard" => icons::layout_dashboard(),
        "users" => icons::users(),
        "shield" => icons::shield(),
        "key" => icons::key(),
        "settings" => icons::settings(),
        "file-text" | "logs" => icons::file_text(),
        "package" | "products" => icons::package(),
        "shopping-cart" => icons::shopping_cart(),
        "server" => icons::server(),
        "folder" | "files" => icons::folder(),
        "user" | "account" => icons::user(),
        "globe" => icons::globe(),
        "robot" | "bot" => icons::robot(),
        "network" => icons::network(),
        "hard-drive" | "storage" => icons::hard_drive(),
        "bar-chart" | "stats" => icons::bar_chart(),
        "dollar-sign" => icons::dollar_sign(),
        "link" => icons::link(),
        _ => icons::package(), // fallback
    }
}

/// A group of nav items rendered with an optional uppercase label.
pub struct NavGroup {
    pub label: Option<String>,
    pub items: Vec<super::NavItem>,
}

/// Grouped sidebar — same layout as `sidebar(...)`, but items are
/// partitioned into labeled groups. The brand at top, the user pinned
/// at bottom (when `user` is `Some`).
pub fn sidebar_grouped(
    groups: &[NavGroup],
    user: Option<&crate::ui::UserInfo>,
    current_path: &str,
    logo_url: &str,
    logo_icon_url: &str,
) -> maud::Markup {
    use maud::html;

    html! {
        nav .sidebar aria-label="Primary" {
            div .sidebar__brand {
                @if !logo_icon_url.is_empty() {
                    img src=(logo_icon_url) alt="" .sidebar__brand-icon;
                }
                @if !logo_url.is_empty() {
                    img src=(logo_url) alt="Solobase" .sidebar__brand-wordmark;
                } @else {
                    span .sidebar__brand-name { "Solobase" }
                }
            }
            div .sidebar__panel {
                div .sidebar__groups {
                    @for g in groups {
                        div .sidebar__group {
                            @if let Some(l) = &g.label {
                                div .sidebar__group-label { (l) }
                            }
                            ul .sidebar__nav {
                                @for item in &g.items {
                                    @let active = current_path == item.href || current_path.starts_with(&format!("{}/", item.href));
                                    li {
                                        a href=(item.href)
                                          class={ "sidebar__nav-item" @if active { " is-active" } }
                                          aria-current=[active.then_some("page")]
                                          target=[item.external.then_some("_blank")]
                                          rel=[item.external.then_some("noopener noreferrer")] {
                                            span .sidebar__nav-icon {
                                                (nav_icon(item.icon))
                                            }
                                            span .sidebar__nav-label { (item.label) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                button .sidebar__collapse-toggle id="sidebar-collapse-btn" type="button" onclick="toggleSidebar()" aria-label="Toggle sidebar" {
                    span .sidebar__collapse-icon-expanded { (icons::chevron_left()) }
                    span .sidebar__collapse-icon-collapsed { (icons::chevron_right()) }
                }
            }
            @if let Some(u) = user {
                div .sidebar__user-container {
                    button .sidebar__user id="user-menu-btn" type="button" onclick="toggleProfileMenu()" {
                        (crate::ui::components::avatar(&u.email, crate::ui::components::CtrlSize::Sm))
                        div .sidebar__user-text {
                            div .sidebar__user-email { (u.email) }
                            div .sidebar__user-role {
                                @if u.is_admin() { "Admin" } @else { "User" }
                            }
                        }
                    }
                    div .profile-menu #profile-menu style="display:none" {
                        div .profile-menu-header {
                            div .profile-menu-avatar { (u.avatar_initial()) }
                            div .profile-menu-info {
                                div .profile-menu-email { (u.email) }
                                div .profile-menu-role { (u.roles.join(", ")) }
                            }
                        }
                        div .profile-menu-divider {}
                        a .profile-menu-item href="/b/userportal/" {
                            (icons::user())
                            span { "My Account" }
                        }
                        a .profile-menu-item href="/b/auth/change-password" {
                            (icons::settings())
                            span { "Change Password" }
                        }
                        div .profile-menu-divider {}
                        form action="/b/auth/api/logout" method="post" {
                            button .profile-menu-item .profile-menu-item-danger type="submit" {
                                (icons::log_out())
                                span { "Sign Out" }
                            }
                        }
                    }
                }
            }
        }
        script { (maud::PreEscaped(r#"
function toggleProfileMenu() {
    var m = document.getElementById('profile-menu');
    if (m) m.style.display = m.style.display === 'none' ? 'block' : 'none';
}
document.addEventListener('click', function(e) {
    var m = document.getElementById('profile-menu');
    var b = document.getElementById('user-menu-btn');
    if (m && b && !b.contains(e.target) && !m.contains(e.target)) {
        m.style.display = 'none';
    }
});
function toggleSidebar() {
    var s = document.querySelector('.sidebar');
    if (!s) return;
    s.classList.toggle('collapsed');
    try { localStorage.setItem('sidebar.collapsed', s.classList.contains('collapsed') ? '1' : '0'); } catch (e) {}
}
(function() {
    try {
        if (localStorage.getItem('sidebar.collapsed') === '1') {
            var s = document.querySelector('.sidebar');
            if (s) s.classList.add('collapsed');
        }
    } catch (e) {}
})();
"#)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::NavItem;

    fn item(label: &str, href: &str) -> NavItem {
        NavItem {
            label: label.to_string(),
            href: href.to_string(),
            icon: "circle",
            external: false,
        }
    }

    #[test]
    fn grouped_sidebar_renders_labels_and_groups() {
        let groups = vec![
            NavGroup {
                label: Some("Workspace".to_string()),
                items: vec![item("Users", "/b/admin/users")],
            },
            NavGroup {
                label: Some("Data".to_string()),
                items: vec![item("Blocks", "/b/admin/blocks")],
            },
        ];
        let s = sidebar_grouped(&groups, None, "/b/admin/users", "", "").into_string();
        assert!(s.contains(">Workspace<"));
        assert!(s.contains(">Data<"));
        assert!(s.contains("/b/admin/users"));
        assert!(s.contains(r#"aria-current="page""#));
    }

    #[test]
    fn grouped_sidebar_marks_active_via_subpath() {
        let groups = vec![NavGroup {
            label: None,
            items: vec![item("Storage", "/b/storage")],
        }];
        let s = sidebar_grouped(&groups, None, "/b/storage/files/foo.png", "", "").into_string();
        assert!(s.contains("is-active"));
    }
}
