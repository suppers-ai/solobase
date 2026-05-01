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
        let system = groups.iter().find(|g| g.label.as_deref() == Some("System")).unwrap();
        let settings = system.items.iter().find(|i| i.label == "Settings").unwrap();
        assert_eq!(settings.href, "/b/admin/settings/email");
    }
}
