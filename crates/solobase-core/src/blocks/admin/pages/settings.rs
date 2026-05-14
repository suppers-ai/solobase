//! Consolidated settings page — wraps each tab's body in `form_page`.
//! Routes:
//!   /b/admin/settings              → 308 redirect to /b/admin/settings/email
//!   /b/admin/settings/email        → email::settings_body
//!   /b/admin/settings/network      → network::settings_body
//!   /b/admin/settings/variables    → variables::settings_body
//!
//! `/b/admin/settings/permissions` is no longer routed here — it 308s to the
//! standalone `/b/admin/permissions` page (see admin/mod.rs). The Permissions
//! UI used to be a Settings sub-tab; it moved to a top-level sidebar entry
//! because WRAP grants are the platform's access-control surface and were
//! too buried inside a settings tab strip.

use wafer_run::{context::Context, types::*, OutputStream};

use super::{admin_page, crumb, email, network, variables};
use crate::ui::{
    shell::Topbar,
    templates::{form_page, FormSection, PageHeader},
    SiteConfig, UserInfo,
};

/// Render the settings page for the given tab. `tab` is one of
/// "email" / "network" / "variables"; unknown values fall back to "email".
pub async fn settings_page(ctx: &dyn Context, msg: &Message, tab: &str) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let path = msg.path().to_string();

    let active = match tab {
        "email" | "network" | "variables" => tab,
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
    ];

    let body_markup = match active {
        "email" => email::settings_body(ctx, msg).await,
        "network" => network::settings_body(ctx, msg).await,
        "variables" => variables::settings_body(ctx, msg).await,
        _ => unreachable!(),
    };

    let form_body = form_page(
        PageHeader {
            title: "",
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
            subtitle: Some(tab_title(active)),
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
        _ => "Settings",
    }
}

fn tab_description(active: &str) -> Option<&'static str> {
    match active {
        "email" => Some("Configure email delivery via Mailgun."),
        "network" => Some("Manage network access rules for blocks."),
        "variables" => Some("Configure environment variables and shared config."),
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
    }

    #[test]
    fn tab_title_permissions_falls_back_to_settings() {
        // Permissions is no longer a Settings sub-tab — it moved to the
        // top-level `/b/admin/permissions` page. If someone accidentally
        // routes `"permissions"` back through `settings_page`, the title
        // must not pretend the tab still exists.
        assert_eq!(tab_title("permissions"), "Settings");
    }

    #[test]
    fn tab_title_unknown_falls_back_to_settings() {
        assert_eq!(tab_title("unknown"), "Settings");
    }
}
