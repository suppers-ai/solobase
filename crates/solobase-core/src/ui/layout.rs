//! Page layout components — full page wrapper, block shell with sidebar.

use maud::{html, Markup, PreEscaped, DOCTYPE};

use super::{assets, icons, sidebar, NavItem, SiteConfig, UserInfo};

/// Render a full HTML page with head (CSS + htmx) and body.
pub fn page(title: &str, config: &SiteConfig, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                title { (title) " — " (config.app_name) }
                link rel="stylesheet" href=(assets::css_url());
                @if !config.favicon_url.is_empty() {
                    link rel="icon" href=(config.favicon_url);
                }
                script src=(assets::htmx_js_url()) defer {}
            }
            body {
                (body)
                div #toast-container .toast-container {}
                script { (PreEscaped(assets::toast_js())) }
                script { (PreEscaped(assets::modal_js())) }
            }
        }
    }
}

/// The standard block shell layout: sidebar + main content area.
///
/// If `is_fragment` is true, only the inner content is returned (for htmx partials).
pub fn block_shell(
    title: &str,
    config: &SiteConfig,
    nav_items: &[NavItem],
    user: Option<&UserInfo>,
    current_path: &str,
    content: Markup,
    is_fragment: bool,
) -> Markup {
    if is_fragment {
        return content;
    }

    page(title, config, html! {
        // Mobile header
        div .mobile-header {
            button .menu-toggle onclick="toggleMobileMenu()" {
                (icons::menu())
            }
            span .mobile-title { (title) }
        }

        div .app-layout {
            // Sidebar
            div .sidebar-container {
                div .sidebar-overlay onclick="toggleMobileMenu()" {}
                div .sidebar-wrapper {
                    (sidebar::sidebar(nav_items, user, current_path, &config.logo_url, &config.logo_icon_url))
                }
            }

            // Main content
            div .main-content {
                div .content-wrapper #content {
                    (content)
                }
            }
        }

        script { (PreEscaped(assets::sidebar_js())) }
    })
}

/// A simple centered page layout (used for login, signup, etc.)
pub fn centered_page(title: &str, config: &SiteConfig, content: Markup) -> Markup {
    page(title, config, html! {
        div .login-page {
            (content)
        }
    })
}
