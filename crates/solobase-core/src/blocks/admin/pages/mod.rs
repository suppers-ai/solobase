//! SSR pages for the admin block.
//!
//! Each page queries the database directly (same patterns as the JSON handlers)
//! and renders HTML via maud.

mod blocks;
mod dashboard;
mod email;
mod logs;
mod network;
mod permissions;
mod storage;
mod users;
mod variables;

// Re-export all public functions so callers can use `pages::dashboard(...)` etc.
pub use blocks::*;
pub use dashboard::*;
pub use email::*;
pub use logs::*;
pub use network::*;
pub use permissions::*;
pub use storage::*;
pub use users::*;
pub use variables::*;

use crate::ui::{self, NavItem, SiteConfig, UserInfo};
use maud::Markup;
use wafer_run::types::*;

/// Admin nav items for the sidebar.
pub(crate) fn admin_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Dashboard".into(),
            href: "/b/admin/".into(),
            icon: "layout-dashboard",
        },
        NavItem {
            label: "Users".into(),
            href: "/b/admin/users".into(),
            icon: "users",
        },
        NavItem {
            label: "Config".into(),
            href: "/b/admin/variables".into(),
            icon: "settings",
        },
        NavItem {
            label: "Network".into(),
            href: "/b/admin/network".into(),
            icon: "network",
        },
        NavItem {
            label: "Storage".into(),
            href: "/b/admin/storage".into(),
            icon: "hard-drive",
        },
        NavItem {
            label: "Permissions".into(),
            href: "/b/admin/permissions".into(),
            icon: "shield",
        },
        NavItem {
            label: "Logs".into(),
            href: "/b/admin/logs".into(),
            icon: "file-text",
        },
        NavItem {
            label: "Email".into(),
            href: "/b/admin/email".into(),
            icon: "globe",
        },
        NavItem {
            label: "Blocks".into(),
            href: "/b/admin/blocks".into(),
            icon: "package",
        },
    ]
}

/// Wrap content in the admin shell (sidebar + layout), or return fragment for htmx.
pub(crate) fn admin_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(
        title,
        config,
        &admin_nav(),
        user,
        path,
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}
