//! Canonical sidebar groups per audience. Single source of truth for
//! both `sidebar_grouped()` callers and the ⌘K palette entries.

use super::{sidebar::NavGroup, NavItem};

fn item(label: &str, href: &str, icon: &'static str) -> NavItem {
    NavItem {
        label: label.to_string(),
        href: href.to_string(),
        icon,
    }
}

/// Admin sidebar groups. The `_current_path` arg exists for symmetry
/// with `portal()` and is forwarded to `sidebar_grouped` by the caller;
/// active-item highlighting happens there.
pub fn admin(_current_path: &str) -> Vec<NavGroup> {
    vec![
        NavGroup {
            label: Some("Workspace".to_string()),
            items: vec![
                item("Dashboard", "/b/admin/", "layout-dashboard"),
                item("Users", "/b/admin/users", "users"),
            ],
        },
        NavGroup {
            label: Some("Data".to_string()),
            items: vec![
                item("Blocks", "/b/admin/blocks", "package"),
                item("Storage", "/b/admin/storage", "hard-drive"),
                item("SQL", "/b/admin/sql", "server"),
            ],
        },
        NavGroup {
            label: Some("Communication".to_string()),
            items: vec![
                item("Messages", "/b/messages/", "file-text"),
                item("LLM", "/b/llm/", "globe"),
            ],
        },
        NavGroup {
            label: Some("System".to_string()),
            items: vec![
                item("Logs", "/b/admin/logs", "file-text"),
                item("Inspector", "/b/inspector", "shield"),
                item("Settings", "/b/admin/settings/email", "settings"),
            ],
        },
    ]
}

/// Portal sidebar groups (end-user account + apps).
pub fn portal(_current_path: &str) -> Vec<NavGroup> {
    vec![
        NavGroup {
            label: Some("Account".to_string()),
            items: vec![
                item("Profile", "/b/userportal/profile", "user"),
                item("Organizations", "/b/auth/orgs", "users"),
                item("Sessions", "/b/userportal/sessions", "shield"),
            ],
        },
        NavGroup {
            label: Some("Apps".to_string()),
            items: vec![
                item("Products", "/b/products/", "package"),
                item("Files", "/b/storage/", "folder"),
                item("Legal", "/b/legalpages/admin/privacy", "file-text"),
            ],
        },
    ]
}

/// Build palette entries for the audience matching `current_path`.
/// Admin paths get admin nav; everything else gets portal nav.
pub fn palette_entries(current_path: &str) -> Vec<crate::ui::palette::PaletteEntry> {
    use crate::ui::palette::PaletteEntry;
    let groups = if is_admin_path(current_path) {
        admin(current_path)
    } else {
        portal(current_path)
    };
    groups
        .into_iter()
        .flat_map(|g| g.items.into_iter())
        .map(|item| PaletteEntry {
            keywords: format!("{} {}", item.label.to_lowercase(), item.href),
            label: item.label,
            kind_label: "Page".to_string(),
            href: item.href,
        })
        .collect()
}

fn is_admin_path(path: &str) -> bool {
    path.starts_with("/b/admin")
        || path.starts_with("/b/inspector")
        || path.starts_with("/b/messages")
        || path.starts_with("/b/llm")
        || path.starts_with("/b/files")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_four_labeled_groups_in_spec_order() {
        let groups = admin("/b/admin/");
        let labels: Vec<&str> = groups
            .iter()
            .map(|g| g.label.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(labels, vec!["Workspace", "Data", "Communication", "System"]);
    }

    #[test]
    fn admin_workspace_has_dashboard_and_users() {
        let groups = admin("/b/admin/");
        let workspace = &groups[0];
        let labels: Vec<&str> = workspace.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["Dashboard", "Users"]);
    }

    #[test]
    fn admin_settings_points_at_email_tab_for_phase_3_route() {
        let groups = admin("/b/admin/settings/email");
        let system = groups
            .iter()
            .find(|g| g.label.as_deref() == Some("System"))
            .unwrap();
        let settings = system.items.iter().find(|i| i.label == "Settings").unwrap();
        assert_eq!(settings.href, "/b/admin/settings/email");
    }

    #[test]
    fn portal_has_account_and_apps() {
        let groups = portal("/b/userportal/profile");
        let labels: Vec<&str> = groups
            .iter()
            .map(|g| g.label.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(labels, vec!["Account", "Apps"]);
    }

    #[test]
    fn portal_account_includes_profile_orgs_sessions() {
        let groups = portal("/b/userportal/profile");
        let account = &groups[0];
        let hrefs: Vec<&str> = account.items.iter().map(|i| i.href.as_str()).collect();
        assert_eq!(
            hrefs,
            vec![
                "/b/userportal/profile",
                "/b/auth/orgs",
                "/b/userportal/sessions"
            ]
        );
    }

    #[test]
    fn portal_apps_includes_products_files_legal() {
        let groups = portal("/b/products/");
        let apps = &groups[1];
        let labels: Vec<&str> = apps.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["Products", "Files", "Legal"]);
    }

    #[test]
    fn palette_entries_for_admin_path_includes_admin_pages() {
        let entries = palette_entries("/b/admin/users");
        let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"Dashboard"));
        assert!(labels.contains(&"Users"));
        assert!(labels.contains(&"Settings"));
    }

    #[test]
    fn palette_entries_for_portal_path_includes_portal_pages() {
        let entries = palette_entries("/b/userportal/profile");
        let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"Profile"));
        assert!(labels.contains(&"Products"));
    }

    #[test]
    fn palette_entry_keywords_lowercase_label_plus_href() {
        let entries = palette_entries("/b/admin/");
        let users = entries.iter().find(|e| e.label == "Users").unwrap();
        assert!(users.keywords.contains("users"));
        assert!(users.keywords.contains("/b/admin/users"));
        assert_eq!(users.kind_label, "Page");
    }
}
