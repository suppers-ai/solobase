//! Sidebar component with navigation and user profile.

use maud::{html, Markup, PreEscaped};

use super::{icons, NavItem, UserInfo};

/// Render the sidebar with nav items and user section.
pub fn sidebar(
    nav_items: &[NavItem],
    user: Option<&UserInfo>,
    current_path: &str,
    logo_url: &str,
    logo_icon_url: &str,
) -> Markup {
    html! {
        nav .sidebar {
            // Logo header
            div .sidebar-header {
                a .sidebar-logo href="/" {
                    @if !logo_url.is_empty() {
                        img .sidebar-logo-long src=(logo_url) alt="Home";
                    }
                    @if !logo_icon_url.is_empty() {
                        img .sidebar-logo-icon src=(logo_icon_url) alt="Home";
                    }
                    @if logo_url.is_empty() && logo_icon_url.is_empty() {
                        span .font-semibold style="font-size: 1.25rem;" { "Home" }
                    }
                }
            }

            // Navigation
            div .sidebar-nav {
                @for item in nav_items {
                    div .nav-item {
                        a
                            .nav-link
                            .(if item.href.ends_with('/') { if current_path == item.href { "active" } else { "" } } else if current_path.starts_with(&item.href) { "active" } else { "" })
                            href=(item.href)
                        {
                            span .nav-icon {
                                (nav_icon(item.icon))
                            }
                            span .nav-text { (item.label) }
                        }
                    }
                }

                // Collapse toggle
                button .sidebar-toggle onclick="toggleSidebar()" title="Toggle sidebar" {
                    (icons::chevron_left())
                }
            }

            // User section
            @if let Some(user) = user {
                div .sidebar-user-container {
                    button .sidebar-user id="user-menu-btn" onclick="toggleProfileMenu()" {
                        div .user-avatar { (user.avatar_initial()) }
                        div .user-info {
                            div .user-email { (user.email) }
                        }
                    }

                    // Profile menu (hidden by default)
                    div .profile-menu #profile-menu style="display:none" {
                        div .profile-menu-header {
                            div .profile-menu-avatar { (user.avatar_initial()) }
                            div .profile-menu-info {
                                div .profile-menu-email { (user.email) }
                                div .profile-menu-role {
                                    (user.roles.join(", "))
                                }
                            }
                        }
                        div .profile-menu-divider {}
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

        script { (PreEscaped(r#"
function toggleProfileMenu() {
    var m = document.getElementById('profile-menu');
    m.style.display = m.style.display === 'none' ? 'block' : 'none';
}
document.addEventListener('click', function(e) {
    var m = document.getElementById('profile-menu');
    var b = document.getElementById('user-menu-btn');
    if (m && b && !b.contains(e.target) && !m.contains(e.target)) {
        m.style.display = 'none';
    }
});
"#)) }
    }
}

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
        "server" | "projects" => icons::server(),
        "folder" | "files" => icons::folder(),
        "globe" => icons::globe(),
        "bar-chart" | "stats" => icons::bar_chart(),
        "dollar-sign" => icons::dollar_sign(),
        _ => icons::package(), // fallback
    }
}
