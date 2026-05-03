//! Canonical sidebar groups per audience. Single source of truth for
//! both `sidebar_grouped()` callers and the ⌘K palette entries.

use super::{sidebar::NavGroup, NavItem};

fn item(label: &str, href: &str, icon: &'static str) -> NavItem {
    NavItem {
        label: label.to_string(),
        href: href.to_string(),
        icon,
        external: false,
    }
}

fn item_external(label: &str, href: &str, icon: &'static str) -> NavItem {
    NavItem {
        label: label.to_string(),
        href: href.to_string(),
        icon,
        external: true,
    }
}

/// Admin sidebar groups.
pub fn admin() -> Vec<NavGroup> {
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
                item("Database", "/b/admin/database", "server"),
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
                item_external("Inspector", "/b/inspector/ui", "shield"),
                item("Settings", "/b/admin/settings/email", "settings"),
            ],
        },
    ]
}

/// Portal sidebar groups (end-user account + apps).
pub fn portal() -> Vec<NavGroup> {
    vec![
        NavGroup {
            label: Some("Account".to_string()),
            items: vec![
                item("Profile", "/b/userportal/profile", "user"),
                item("Organizations", "/b/auth/orgs", "users"),
                item("Sessions", "/b/userportal/sessions", "shield"),
                item("Security", "/b/userportal/security", "lock"),
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

/// Flatten a slice of `NavGroup`s into palette entries. Same items the
/// sidebar shows; ⌘K uses the same source of truth so the two can never
/// drift out of sync.
pub fn palette_entries_from_groups(groups: &[NavGroup]) -> Vec<crate::ui::palette::PaletteEntry> {
    use crate::ui::palette::PaletteEntry;
    groups
        .iter()
        .flat_map(|g| g.items.iter())
        .map(|item| PaletteEntry {
            keywords: format!("{} {}", item.label.to_lowercase(), item.href),
            label: item.label.clone(),
            kind_label: "Page".to_string(),
            href: item.href.clone(),
            external: item.external,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_four_labeled_groups_in_spec_order() {
        let groups = admin();
        let labels: Vec<&str> = groups
            .iter()
            .map(|g| g.label.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(labels, vec!["Workspace", "Data", "Communication", "System"]);
    }

    #[test]
    fn admin_workspace_has_dashboard_and_users() {
        let groups = admin();
        let workspace = &groups[0];
        let labels: Vec<&str> = workspace.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["Dashboard", "Users"]);
    }

    #[test]
    fn admin_data_group_has_database_not_sql() {
        let groups = admin();
        let data = groups
            .iter()
            .find(|g| g.label.as_deref() == Some("Data"))
            .unwrap();
        let database = data.items.iter().find(|i| i.label == "Database").unwrap();
        assert_eq!(database.href, "/b/admin/database");
        assert!(
            data.items.iter().all(|i| i.label != "SQL"),
            "SQL should be renamed to Database"
        );
    }

    #[test]
    fn admin_settings_points_at_email_tab_for_phase_3_route() {
        let groups = admin();
        let system = groups
            .iter()
            .find(|g| g.label.as_deref() == Some("System"))
            .unwrap();
        let settings = system.items.iter().find(|i| i.label == "Settings").unwrap();
        assert_eq!(settings.href, "/b/admin/settings/email");
    }

    #[test]
    fn portal_has_account_and_apps() {
        let groups = portal();
        let labels: Vec<&str> = groups
            .iter()
            .map(|g| g.label.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(labels, vec!["Account", "Apps"]);
    }

    #[test]
    fn portal_account_includes_profile_orgs_sessions_security() {
        let groups = portal();
        let account = &groups[0];
        let hrefs: Vec<&str> = account.items.iter().map(|i| i.href.as_str()).collect();
        assert_eq!(
            hrefs,
            vec![
                "/b/userportal/profile",
                "/b/auth/orgs",
                "/b/userportal/sessions",
                "/b/userportal/security"
            ]
        );
    }

    #[test]
    fn portal_apps_includes_products_files_legal() {
        let groups = portal();
        let apps = &groups[1];
        let labels: Vec<&str> = apps.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["Products", "Files", "Legal"]);
    }

    #[test]
    fn palette_entries_for_admin_groups_includes_admin_pages() {
        let entries = palette_entries_from_groups(&admin());
        let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"Dashboard"));
        assert!(labels.contains(&"Users"));
        assert!(labels.contains(&"Settings"));
    }

    #[test]
    fn palette_entries_for_portal_groups_includes_portal_pages() {
        let entries = palette_entries_from_groups(&portal());
        let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
        assert!(labels.contains(&"Profile"));
        assert!(labels.contains(&"Products"));
    }

    #[test]
    fn palette_entry_keywords_lowercase_label_plus_href() {
        let entries = palette_entries_from_groups(&admin());
        let users = entries.iter().find(|e| e.label == "Users").unwrap();
        assert!(users.keywords.contains("users"));
        assert!(users.keywords.contains("/b/admin/users"));
        assert_eq!(users.kind_label, "Page");
    }

    #[test]
    fn admin_inspector_is_external_new_tab_link_to_inspector_ui() {
        let groups = admin();
        let system = groups
            .iter()
            .find(|g| g.label.as_deref() == Some("System"))
            .unwrap();
        let inspector = system
            .items
            .iter()
            .find(|i| i.label == "Inspector")
            .expect("System group must include Inspector");
        assert!(inspector.external, "Inspector should open in a new tab");
        assert_eq!(inspector.href, "/b/inspector/ui");
    }
}
