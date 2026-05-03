//! Server-side rendered UI components for solobase blocks.
//!
//! Uses maud for compile-time HTML generation and htmx for interactivity.
//! CSS and htmx JS are embedded in the binary. Images (logo, favicon) are
//! configurable via environment variables.

pub mod assets;
pub mod components;
pub mod icons;
pub mod layout;
pub mod nav_groups;
pub mod palette;
pub mod shell;
pub mod sidebar;
pub mod templates;

/// Branding/site config loaded from environment variables.
/// Passed through to layout and sidebar so every page renders consistently.
pub struct SiteConfig {
    pub app_name: String,
    pub logo_url: String,
    pub logo_icon_url: String,
    pub favicon_url: String,
    /// Extra module-type script URLs appended to every rendered page.
    /// Browser targets populate this (e.g. `/webllm-engine.js` for the
    /// page-side LLM engine); native targets leave it empty.
    pub embedded_scripts: Vec<String>,
}

impl SiteConfig {
    /// Load site config from the WAFER config system (env vars / variables table).
    pub async fn load(ctx: &dyn wafer_run::context::Context) -> Self {
        use wafer_core::clients::config;
        let scripts_raw = config::get_default(ctx, "SOLOBASE_SHARED__EMBEDDED_SCRIPTS", "").await;
        Self {
            app_name: config::get_default(ctx, "SOLOBASE_SHARED__APP_NAME", "Solobase").await,
            logo_url: config::get_default(ctx, "SOLOBASE_SHARED__LOGO_URL", "").await,
            logo_icon_url: config::get_default(ctx, "SOLOBASE_SHARED__LOGO_ICON_URL", "").await,
            favicon_url: config::get_default(ctx, "SOLOBASE_SHARED__FAVICON_URL", "").await,
            embedded_scripts: scripts_raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
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
        self.email
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase()
    }
}

/// A navigation item for the sidebar.
pub struct NavItem {
    pub label: String,
    pub href: String,
    pub icon: &'static str,
    /// When true, render as `target="_blank"` and open in a new tab from
    /// both the sidebar and the ⌘K palette. Used for cross-block links
    /// that have their own chrome (e.g. Inspector).
    pub external: bool,
}

pub use sidebar::NavGroup;

/// Check if the current request is an htmx partial request.
pub fn is_htmx(msg: &wafer_run::types::Message) -> bool {
    !msg.get_meta("http.header.hx-request").is_empty()
}

/// Respond with full HTML page or htmx fragment depending on request type.
pub fn html_response(markup: maud::Markup) -> wafer_run::OutputStream {
    crate::blocks::helpers::ResponseBuilder::new().body(
        markup.into_string().into_bytes(),
        "text/html; charset=utf-8",
    )
}

/// Render a full page wrapping `body` in `shell()` + `page()`. Caller
/// passes the audience's `nav_groups` (admin or portal) and a `Topbar`.
/// Mounts the ⌘K palette modal at the bottom of the page when
/// `topbar.show_palette` is true.
pub fn shelled_page(
    title: &str,
    config: &SiteConfig,
    groups: &[NavGroup],
    user: Option<&UserInfo>,
    current_path: &str,
    topbar: shell::Topbar<'_>,
    body: maud::Markup,
) -> maud::Markup {
    use maud::{html, PreEscaped};
    let palette_markup = if topbar.show_palette {
        palette::palette(nav_groups::palette_entries_from_groups(groups))
    } else {
        html! {}
    };
    layout::page(
        title,
        config,
        html! {
            (shell::shell(
                groups,
                user,
                current_path,
                &config.logo_url,
                &config.logo_icon_url,
                topbar,
                body,
            ))
            (palette_markup)
            script { (PreEscaped(assets::palette_js())) }
            script { (PreEscaped(assets::drawer_js())) }
        },
    )
}

/// Same as `shelled_page`, but returns an `OutputStream`. Returns the
/// raw `body` (no chrome) when the request is an htmx partial.
pub fn shelled_response(
    msg: &wafer_run::types::Message,
    title: &str,
    config: &SiteConfig,
    groups: &[NavGroup],
    user: Option<&UserInfo>,
    current_path: &str,
    topbar: shell::Topbar<'_>,
    body: maud::Markup,
) -> wafer_run::OutputStream {
    if is_htmx(msg) {
        return html_response(body);
    }
    html_response(shelled_page(
        title,
        config,
        groups,
        user,
        current_path,
        topbar,
        body,
    ))
}

/// Minimal `SiteConfig` used by the status-page helpers. They render
/// before context is available, so they can't load real config; fixed
/// branding + no embedded scripts is the right shape.
fn minimal_config() -> SiteConfig {
    SiteConfig {
        app_name: "Solobase".to_string(),
        logo_url: String::new(),
        logo_icon_url: String::new(),
        favicon_url: String::new(),
        embedded_scripts: Vec::new(),
    }
}

/// Render the styled `status_page` body wrapped in `layout::page` and
/// return it with the requested HTTP status. Used by 403/404/500 helpers.
fn status_response(
    status: u16,
    page_title: &str,
    code: &str,
    title: &str,
    body_text: &str,
    primary_action: (&str, &str),
) -> wafer_run::OutputStream {
    let config = minimal_config();
    let body = templates::status_page(
        code,
        title,
        body_text,
        Some((primary_action.0.to_string(), primary_action.1.to_string())),
    );
    let markup = layout::page(page_title, &config, body);
    crate::blocks::helpers::ResponseBuilder::new()
        .status(status)
        .body(
            markup.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
}

/// Return styled 403 for browser requests, JSON for API requests.
pub fn forbidden_response(msg: &wafer_run::types::Message) -> wafer_run::OutputStream {
    let accept = msg.get_meta("http.header.accept");
    if accept.contains("text/html") && !accept.contains("application/json") {
        status_response(
            403,
            "Forbidden",
            "403",
            "Forbidden",
            "You don't have access to this page.",
            ("Sign in", "/b/auth/login"),
        )
    } else {
        crate::blocks::helpers::err_forbidden("admin access required")
    }
}

/// Return styled 404 for browser requests, JSON for API requests.
pub fn not_found_response(msg: &wafer_run::types::Message) -> wafer_run::OutputStream {
    let accept = msg.get_meta("http.header.accept");
    if accept.contains("text/html") && !accept.contains("application/json") {
        status_response(
            404,
            "Not found",
            "404",
            "Not found",
            "We couldn't find that page.",
            ("Go home", "/"),
        )
    } else {
        crate::blocks::helpers::err_not_found("endpoint not found")
    }
}

/// Return styled 500 for browser requests, JSON for API requests.
pub fn server_error_response(msg: &wafer_run::types::Message) -> wafer_run::OutputStream {
    let accept = msg.get_meta("http.header.accept");
    if accept.contains("text/html") && !accept.contains("application/json") {
        status_response(
            500,
            "Server error",
            "500",
            "Something went wrong",
            "An unexpected error occurred. Please try again.",
            ("Go home", "/"),
        )
    } else {
        crate::blocks::helpers::err_internal("internal server error")
    }
}

/// Respond with HTML + an HX-Trigger header for toast notifications.
pub fn html_response_with_toast(
    markup: maud::Markup,
    toast_message: &str,
    toast_type: &str,
) -> wafer_run::OutputStream {
    let trigger = format!(
        r#"{{"showToast":{{"message":"{}","type":"{}"}}}}"#,
        toast_message, toast_type
    );
    crate::blocks::helpers::ResponseBuilder::new()
        .set_header("HX-Trigger", &trigger)
        .body(
            markup.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
}

#[cfg(test)]
mod tests {
    use maud::html;
    use wafer_run::types::Message;

    use super::*;
    use crate::ui::shell::{Crumb, Topbar};

    fn site_config() -> SiteConfig {
        SiteConfig {
            app_name: "TestApp".to_string(),
            logo_url: String::new(),
            logo_icon_url: String::new(),
            favicon_url: String::new(),
            embedded_scripts: Vec::new(),
        }
    }

    #[test]
    fn shelled_page_full_render_includes_html_doctype_shell_and_body() {
        let groups = nav_groups::admin();
        let topbar = Topbar {
            crumbs: vec![Crumb {
                label: "Dashboard",
                href: None,
            }],
            primary_action: None,
            show_palette: true,
        };
        let body = html! { p { "hello" } };
        let markup = shelled_page(
            "Dashboard",
            &site_config(),
            &groups,
            None,
            "/b/admin/",
            topbar,
            body,
        );
        let s = markup.into_string();
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains(r#"class="shell""#));
        assert!(s.contains(r#"id="cmdk""#)); // palette mounted
        assert!(s.contains("hello"));
    }

    #[tokio::test]
    async fn not_found_response_uses_status_template() {
        let mut msg = Message::new("http.request");
        msg.set_meta("http.header.accept", "text/html");
        let out = not_found_response(&msg);
        let buf = out.collect_buffered().await.unwrap();
        let body = String::from_utf8(buf.body).unwrap_or_default();
        assert!(
            body.contains("status-page"),
            "body should contain status-page class"
        );
        assert!(body.contains(">404<"), "body should contain 404 code");
        assert!(
            body.contains("Go home"),
            "body should contain Go home action"
        );
    }

    #[tokio::test]
    async fn forbidden_response_uses_status_template() {
        let mut msg = Message::new("http.request");
        msg.set_meta("http.header.accept", "text/html");
        let out = forbidden_response(&msg);
        let buf = out.collect_buffered().await.unwrap();
        let body = String::from_utf8(buf.body).unwrap_or_default();
        assert!(
            body.contains("status-page"),
            "body should contain status-page class"
        );
        assert!(body.contains(">403<"), "body should contain 403 code");
        assert!(
            body.contains("Sign in"),
            "body should contain Sign in action"
        );
    }

    #[tokio::test]
    async fn server_error_response_uses_status_template() {
        let mut msg = Message::new("http.request");
        msg.set_meta("http.header.accept", "text/html");
        let out = server_error_response(&msg);
        let buf = out.collect_buffered().await.unwrap();
        let body = String::from_utf8(buf.body).unwrap_or_default();
        assert!(body.contains(">500<"), "body should contain 500 code");
        assert!(
            body.contains("Something went wrong"),
            "body should contain 'Something went wrong' title"
        );
    }
}
