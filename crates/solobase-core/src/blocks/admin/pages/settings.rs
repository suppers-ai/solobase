//! Consolidated settings page — wraps each tab's body in `form_page`.
//! Routes:
//!   /b/admin/settings              → 308 redirect to /b/admin/settings/email
//!   /b/admin/settings/email        → email::settings_body
//!   /b/admin/settings/network      → network::settings_body
//!   /b/admin/settings/variables    → variables::settings_body
//!   /b/admin/settings/permissions  → permissions::settings_body

use wafer_run::{context::Context, types::*, OutputStream};

use super::{admin_page, crumb, email, network, permissions, variables};
use crate::ui::{
    shell::Topbar,
    templates::{FormSection, PageHeader, form_page},
    SiteConfig, UserInfo,
};

/// Render the settings page for the given tab. `tab` is one of
/// "email" / "network" / "variables" / "permissions"; unknown values
/// fall back to "email".
pub async fn settings_page(ctx: &dyn Context, msg: &Message, tab: &str) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let active = match tab {
        "email" | "network" | "variables" | "permissions" => tab,
        _ => "email",
    };

    let tabs = vec![
        (
            "Email".to_string(),
            "/b/admin/settings/email".to_string(),
            active == "email",
        ),
        (
            "Network".to_string(),
            "/b/admin/settings/network".to_string(),
            active == "network",
        ),
        (
            "Variables".to_string(),
            "/b/admin/settings/variables".to_string(),
            active == "variables",
        ),
        (
            "Permissions".to_string(),
            "/b/admin/settings/permissions".to_string(),
            active == "permissions",
        ),
    ];

    let body_markup = match active {
        "email" => email::settings_body(ctx, msg).await,
        "network" => network::settings_body(ctx, msg).await,
        "variables" => variables::settings_body(ctx, msg).await,
        "permissions" => permissions::settings_body(ctx, msg).await,
        _ => unreachable!(),
    };

    let form_body = form_page(
        PageHeader {
            title: "Settings",
            subtitle: None,
            primary_action: None,
        },
        Some(tabs),
        vec![FormSection {
            title: tab_title(active),
            description: tab_description(active),
            body: body_markup,
        }],
        &format!("/b/admin/settings/{}", active),
        "post",
        "Save",
    );

    admin_page(
        "Settings",
        &config,
        &path,
        user.as_ref(),
        Topbar {
            crumbs: crumb("Settings"),
            primary_action: None,
            show_palette: true,
        },
        form_body,
        msg,
    )
}

fn tab_title(active: &str) -> &'static str {
    match active {
        "email" => "Email",
        "network" => "Network",
        "variables" => "Variables",
        "permissions" => "Permissions",
        _ => "Settings",
    }
}

fn tab_description(active: &str) -> Option<&'static str> {
    match active {
        "email" => Some("Configure email delivery via Mailgun."),
        "network" => Some("Manage network access rules for blocks."),
        "variables" => Some("Configure environment variables and shared config."),
        "permissions" => {
            Some("Control which blocks can access other blocks' data, files, and services.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_title_known_tabs() {
        assert_eq!(tab_title("email"), "Email");
        assert_eq!(tab_title("network"), "Network");
        assert_eq!(tab_title("variables"), "Variables");
        assert_eq!(tab_title("permissions"), "Permissions");
    }

    #[test]
    fn tab_title_unknown_falls_back_to_settings() {
        assert_eq!(tab_title("unknown"), "Settings");
    }
}
