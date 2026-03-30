//! Server-side rendered UI components for solobase blocks.
//!
//! Uses maud for compile-time HTML generation and htmx for interactivity.
//! CSS and htmx JS are embedded in the binary. Images (logo, favicon) are
//! configurable via environment variables.

pub mod assets;
pub mod components;
pub mod icons;
pub mod layout;
pub mod sidebar;

/// Branding/site config loaded from environment variables.
/// Passed through to layout and sidebar so every page renders consistently.
pub struct SiteConfig {
    pub app_name: String,
    pub logo_url: String,
    pub logo_icon_url: String,
    pub favicon_url: String,
}

impl SiteConfig {
    /// Load site config from the WAFER config system (env vars / variables table).
    pub async fn load(ctx: &dyn wafer_run::context::Context) -> Self {
        use wafer_core::clients::config;
        Self {
            app_name: config::get_default(ctx, "APP_NAME", "Solobase").await,
            logo_url: config::get_default(ctx, "LOGO_URL", "").await,
            logo_icon_url: config::get_default(ctx, "LOGO_ICON_URL", "").await,
            favicon_url: config::get_default(ctx, "FAVICON_URL", "").await,
        }
    }
}

/// User info available during rendering (extracted from auth metadata).
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
}

impl UserInfo {
    /// Create from message auth metadata.
    pub fn from_message(msg: &wafer_run::types::Message) -> Option<Self> {
        let id = msg.get_meta("auth.user_id");
        if id.is_empty() {
            return None;
        }
        let email = msg.get_meta("auth.user_email").to_string();
        let roles: Vec<String> = msg
            .get_meta("auth.user_roles")
            .split(',')
            .filter(|r| !r.trim().is_empty())
            .map(|r| r.trim().to_string())
            .collect();
        Some(Self {
            id: id.to_string(),
            email,
            roles,
        })
    }

    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r == "admin")
    }

    /// First letter of email, uppercased, for avatar.
    pub fn avatar_initial(&self) -> char {
        self.email.chars().next().unwrap_or('?').to_ascii_uppercase()
    }
}

/// A navigation item for the sidebar.
pub struct NavItem {
    pub label: String,
    pub href: String,
    pub icon: &'static str,
}

/// Check if the current request is an htmx partial request.
pub fn is_htmx(msg: &wafer_run::types::Message) -> bool {
    !msg.get_meta("http.header.hx-request").is_empty()
}

/// Respond with full HTML page or htmx fragment depending on request type.
pub fn html_response(
    msg: &mut wafer_run::types::Message,
    markup: maud::Markup,
) -> wafer_run::types::Result_ {
    wafer_run::helpers::ResponseBuilder::new(msg)
        .body(markup.into_string().into_bytes(), "text/html; charset=utf-8")
}

/// Render a styled 404 page.
pub fn not_found_page() -> maud::Markup {
    use maud::{html, DOCTYPE};
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                title { "Not Found" }
                link rel="stylesheet" href=(assets::css_url());
            }
            body {
                div .login-page {
                    div .login-container style="text-align:center" {
                        div style="font-size:4rem;font-weight:700;color:var(--text-muted);margin-bottom:0.5rem" { "404" }
                        h1 style="font-size:1.25rem;font-weight:600;margin:0 0 0.5rem" { "Page not found" }
                        p .login-subtitle style="margin-bottom:1.5rem" {
                            "The page you're looking for doesn't exist."
                        }
                        a .login-button href="/" style="display:inline-block;width:auto;padding:.625rem 1.25rem;text-decoration:none" {
                            "Go Home"
                        }
                    }
                }
            }
        }
    }
}

/// Return styled 404 for browser requests, JSON for API requests.
pub fn not_found_response(msg: &mut wafer_run::types::Message) -> wafer_run::types::Result_ {
    let accept = msg.get_meta("http.header.accept");
    if accept.contains("text/html") && !accept.contains("application/json") {
        wafer_run::helpers::ResponseBuilder::new(msg)
            .status(404)
            .body(not_found_page().into_string().into_bytes(), "text/html; charset=utf-8")
    } else {
        wafer_run::helpers::err_not_found(msg, "endpoint not found")
    }
}

/// Respond with HTML + an HX-Trigger header for toast notifications.
pub fn html_response_with_toast(
    msg: &mut wafer_run::types::Message,
    markup: maud::Markup,
    toast_message: &str,
    toast_type: &str,
) -> wafer_run::types::Result_ {
    let trigger = format!(r#"{{"showToast":{{"message":"{}","type":"{}"}}}}"#, toast_message, toast_type);
    wafer_run::helpers::ResponseBuilder::new(msg)
        .set_header("HX-Trigger", &trigger)
        .body(markup.into_string().into_bytes(), "text/html; charset=utf-8")
}
