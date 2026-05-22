use maud::{html, PreEscaped};
use wafer_block::db::{ListOptions, SortField};
use wafer_core::clients::{config, database as db};
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use super::helpers::{self, parse_form_body, stamp_updated, RecordExt};
use crate::{
    blocks::helpers::{err_bad_request, err_forbidden, err_internal, err_not_found, ok_json},
    ui::{
        self, components, icons, nav_groups,
        shell::{Crumb, Topbar},
        sidebar::nav_icon,
        SiteConfig, UserInfo,
    },
};

pub(crate) mod migrations;
mod pages;

const TABLE: &str = "suppers_ai__userportal__buttons";

/// Known icon names available for button configuration.
const ICON_OPTIONS: &[(&str, &str)] = &[
    ("package", "Package"),
    ("shopping-cart", "Shopping Cart"),
    ("folder", "Folder"),
    ("key", "Key"),
    ("server", "Server"),
    ("globe", "Globe"),
    ("users", "Users"),
    ("user", "User"),
    ("settings", "Settings"),
    ("shield", "Shield"),
    ("file-text", "File"),
    ("bar-chart", "Chart"),
    ("dollar-sign", "Dollar"),
    ("dashboard", "Dashboard"),
];

pub struct UserPortalBlock;

impl UserPortalBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UserPortalBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for UserPortalBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::AuthLevel;

        BlockInfo::new(
            "suppers-ai/userportal",
            "0.0.1",
            "http-handler@v1",
            "User profile and account hub with admin-configurable navigation buttons",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/database".into(), "wafer-run/config".into()])
        .collections(vec![CollectionSchema::new(TABLE)
            .field("label", "string")
            .field_default("icon", "string", "package")
            .field("path", "string")
            .field_default("sort_order", "int", "0")])
        .category(wafer_run::BlockCategory::Feature)
        .description("User-facing profile page with editable display name, admin-configurable navigation buttons, and portal configuration endpoint.")
        .endpoints(vec![
            BlockEndpoint::get("/b/userportal/").summary("Portal home (apps + orgs)").auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/userportal/profile").summary("Profile page").auth(AuthLevel::Authenticated),
            BlockEndpoint::post("/b/userportal/update-profile").summary("Update profile").auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/userportal/sessions").summary("Active sessions").auth(AuthLevel::Authenticated),
            BlockEndpoint::delete("/b/userportal/sessions/:hash").summary("Revoke session").auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/userportal/security").summary("Account security").auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/userportal/config").summary("Portal configuration"),
            BlockEndpoint::get("/b/userportal/admin/buttons").summary("Manage portal buttons").auth(AuthLevel::Admin),
            BlockEndpoint::post("/b/userportal/admin/buttons").summary("Create button").auth(AuthLevel::Admin),
        ])
        .config_keys(vec![])
        .admin_url("/b/userportal/admin/settings")
        .can_disable(true)
        .default_enabled(false)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::authenticated("/"),
            wafer_run::UiRoute::authenticated("/profile"),
            wafer_run::UiRoute::authenticated("/sessions"),
            wafer_run::UiRoute::authenticated("/security"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let path = msg.path().to_string();
        let action = msg.action().to_string();

        if !path.starts_with("/b/userportal") {
            return self.handle_config(ctx).await;
        }

        let sub = path
            .strip_prefix("/b/userportal")
            .unwrap_or("/")
            .to_string();

        // Admin routes — require admin role
        if sub.starts_with("/admin/") {
            if !helpers::is_admin(&msg) {
                return crate::ui::forbidden_response(&msg);
            }
            return self.handle_admin(ctx, msg, input, &action, &sub).await;
        }

        match (action.as_str(), sub.as_str()) {
            ("retrieve", "" | "/") => pages::dashboard::dashboard_page(ctx, &msg).await,
            ("retrieve", "/profile") => pages::profile::profile_page(ctx, &msg).await,
            ("create", "/update-profile") => handle_update_profile(ctx, &msg, input).await,
            ("retrieve", "/sessions") => pages::sessions::sessions_page(ctx, &msg).await,
            ("retrieve", "/security") => pages::security::security_page(ctx, &msg).await,
            ("delete", s) if s.starts_with("/sessions/") => {
                pages::sessions::handle_revoke(ctx, &msg, s).await
            }
            ("retrieve", "/config") => self.handle_config(ctx).await,
            ("retrieve", "/internal/list-buttons") => self.handle_list_buttons(ctx).await,
            _ => err_not_found("not found"),
        }
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if matches!(event.event_type, LifecycleType::Init) {
            migrations::apply(ctx).await.map_err(|e| {
                WaferError::new(
                    wafer_run::ErrorCode::Internal,
                    format!("userportal migrations: {e}"),
                )
            })?;
        }
        Ok(())
    }
}

impl UserPortalBlock {
    /// Internal cross-block action — returns the configured portal buttons as
    /// a JSON array. Not user-routable. Consumed by the auth block's dashboard
    /// page via `ctx.call_block` to avoid raw cross-block SQL.
    async fn handle_list_buttons(&self, ctx: &dyn Context) -> OutputStream {
        let records = load_buttons(ctx).await;
        let arr: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "label": r.str_field("label"),
                    "icon": r.str_field("icon"),
                    "path": r.str_field("path"),
                    "sort_order": r.data.get("sort_order").cloned().unwrap_or(serde_json::Value::Null),
                })
            })
            .collect();
        ok_json(&serde_json::Value::Array(arr))
    }

    async fn handle_config(&self, ctx: &dyn Context) -> OutputStream {
        let settings = ctx
            .config_get(crate::features::BLOCK_SETTINGS_CONFIG_KEY)
            .map(crate::features::BlockSettings::from_config_json)
            .unwrap_or_else(|| crate::features::BlockSettings::from_map(Default::default()));

        let is_enabled = |name: &str| -> bool {
            use crate::features::FeatureConfig;
            settings.is_block_enabled(name)
        };

        let config_val = serde_json::json!({
            "logo_url": config::get_default(ctx, "SOLOBASE_SHARED__LOGO_URL", "https://solobase.dev/images/logo_long.png").await,
            "app_name": config::get_default(ctx, "SOLOBASE_SHARED__APP_NAME", "Solobase").await,
            "primary_color": config::get_default(ctx, "SOLOBASE_SHARED__PRIMARY_COLOR", "#6366f1").await,
            "enable_oauth": config::get_default(ctx, "SOLOBASE_SHARED__ENABLE_OAUTH", "false").await,
            "allow_signup": config::get_default(ctx, "SOLOBASE_SHARED__ALLOW_SIGNUP", "true").await,
            "show_powered_by": true,
            "features": {
                "files": is_enabled("suppers-ai/files"),
                "products": is_enabled("suppers-ai/products"),
                "user_products": config::get_default(ctx, "SOLOBASE_SHARED__ALLOW_USER_PRODUCTS", "false").await,
                "legal_pages": is_enabled("suppers-ai/legalpages"),
                "userportal": is_enabled("suppers-ai/userportal"),
            }
        });
        ok_json(&config_val)
    }

    async fn handle_admin(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
        action: &str,
        sub: &str,
    ) -> OutputStream {
        match (action, sub) {
            ("retrieve", "/admin/settings") => admin_settings_page(ctx, &msg).await,
            ("create", "/admin/settings") => handle_save_settings(ctx, input).await,
            ("retrieve", "/admin/buttons") => admin_buttons_page(ctx, &msg).await,
            ("create", "/admin/buttons") => handle_create_button(ctx, input).await,
            ("retrieve", s) if s.starts_with("/admin/buttons/") && s.ends_with("/edit") => {
                let id = s
                    .strip_prefix("/admin/buttons/")
                    .and_then(|s| s.strip_suffix("/edit"))
                    .unwrap_or("");
                if id.is_empty() {
                    return err_not_found("not found");
                }
                handle_edit_button_form(ctx, id).await
            }
            ("update", s) if s.starts_with("/admin/buttons/") => {
                let id = s.strip_prefix("/admin/buttons/").unwrap_or("");
                if id.is_empty() {
                    return err_not_found("not found");
                }
                handle_update_button(ctx, input, id).await
            }
            ("delete", s) if s.starts_with("/admin/buttons/") => {
                let id = s.strip_prefix("/admin/buttons/").unwrap_or("");
                if id.is_empty() {
                    return err_not_found("not found");
                }
                handle_delete_button(ctx, id).await
            }
            _ => err_not_found("not found"),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn render_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    crumb_label: &'static str,
    content: maud::Markup,
    msg: &Message,
) -> OutputStream {
    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: crumb_label,
            href: None,
        }],
        primary_action: None,
        subtitle: None,
        show_palette: true,
    };
    crate::ui::shelled_response(msg, title, config, &groups, user, path, topbar, content)
}

async fn load_buttons(ctx: &dyn Context) -> Vec<wafer_core::clients::database::Record> {
    db::list(
        ctx,
        TABLE,
        &ListOptions {
            sort: vec![SortField {
                field: "sort_order".into(),
                desc: false,
            }],
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .map(|r| r.records)
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// User-facing: Update profile
// ---------------------------------------------------------------------------

async fn handle_update_profile(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden("Not authenticated");
    }

    let raw = input.collect_to_bytes().await;
    let body = parse_form_body(&raw);
    let name = body.get("name").map(|s| s.as_str()).unwrap_or("");

    let mut data = std::collections::HashMap::new();
    data.insert("name".to_string(), serde_json::json!(name));
    stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, crate::blocks::auth::USERS_TABLE, &user_id, data).await {
        // Pass the full WaferError (code + meta + message) so the
        // helper logs structured info instead of just the rendered string.
        return err_internal("Failed to update profile", e);
    }

    // Plain form POST → 303 See Other so the browser follows up with a GET
    // and the back/forward stack stays clean.
    crate::blocks::helpers::ResponseBuilder::new()
        .status(303)
        .set_header("Location", "/b/userportal/profile")
        .body(Vec::new(), "text/plain")
}

// ---------------------------------------------------------------------------
// Admin: Buttons management page
// ---------------------------------------------------------------------------

async fn admin_buttons_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let buttons = load_buttons(ctx).await;

    let content = html! {
        (components::page_header(
            "Portal Buttons",
            Some("Configure navigation buttons shown on the user profile page"),
            None,
        ))

        // Add button form
        div .card style="margin-bottom:1.5rem;padding:1.25rem" {
            h3 style="margin:0 0 1rem;font-size:1rem" { "Add Button" }
            form
                hx-post="/b/userportal/admin/buttons"
                hx-target="#buttons-table"
                hx-swap="outerHTML"
                style="display:grid;grid-template-columns:1fr 1fr 1fr auto auto;gap:0.75rem;align-items:end"
            {
                div .form-group style="margin:0" {
                    label .form-label for="label" { "Label" }
                    input .form-input #label type="text" name="label"
                        placeholder="e.g. My Products" required;
                }
                div .form-group style="margin:0" {
                    label .form-label for="path" { "Path" }
                    input .form-input #path type="text" name="path"
                        placeholder="e.g. /b/products/mine" required;
                }
                div .form-group style="margin:0" {
                    label .form-label for="icon" { "Icon" }
                    select .form-input #icon name="icon" {
                        @for &(value, display) in ICON_OPTIONS {
                            option value=(value) { (display) }
                        }
                    }
                }
                div .form-group style="margin:0" {
                    label .form-label for="sort_order" { "Order" }
                    input .form-input #sort_order type="number" name="sort_order"
                        value="0" style="width:5rem";
                }
                button .btn .btn-primary type="submit" style="white-space:nowrap" {
                    (icons::plus()) " Add"
                }
            }
        }

        // Buttons table
        (render_buttons_table(&buttons))
    };

    render_page(
        "Portal Buttons",
        &site_config,
        "/b/userportal/admin/buttons",
        user.as_ref(),
        "Portal Buttons",
        content,
        msg,
    )
}

fn render_buttons_table(buttons: &[wafer_core::clients::database::Record]) -> maud::Markup {
    html! {
        div #buttons-table {
            @if buttons.is_empty() {
                (components::empty_state(
                    icons::package(),
                    "No buttons configured",
                    "Add a button above to show navigation links on the user profile page.",
                    None,
                ))
            } @else {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Label" }
                                th { "Icon" }
                                th { "Path" }
                                th { "Order" }
                                th style="width:6rem" { "Actions" }
                            }
                        }
                        tbody {
                            @for btn in buttons {
                                tr {
                                    td .font-medium { (btn.str_field("label")) }
                                    td {
                                        span .nav-icon style="display:inline-flex" {
                                            (nav_icon(btn.str_field("icon")))
                                        }
                                        " "
                                        span .text-muted .text-sm { (btn.str_field("icon")) }
                                    }
                                    td { code { (btn.str_field("path")) } }
                                    td { (btn.i64_field("sort_order")) }
                                    td {
                                        div style="display:flex;gap:0.25rem" {
                                            button .btn .btn-ghost .btn-sm
                                                hx-get=(format!("/b/userportal/admin/buttons/{}/edit", btn.id))
                                                hx-target=(format!("#edit-modal-{}", btn.id))
                                                hx-swap="innerHTML"
                                                title="Edit"
                                            {
                                                (icons::edit())
                                            }
                                            button .btn .btn-ghost .btn-sm
                                                hx-delete=(format!("/b/userportal/admin/buttons/{}", btn.id))
                                                hx-target="#buttons-table"
                                                hx-swap="outerHTML"
                                                hx-confirm="Delete this button?"
                                                title="Delete"
                                                style="color:var(--danger)"
                                            {
                                                (icons::trash())
                                            }
                                        }
                                        div id=(format!("edit-modal-{}", btn.id)) {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Admin: Button CRUD handlers
// ---------------------------------------------------------------------------

async fn handle_create_button(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body = parse_form_body(&raw);

    let label = body.get("label").map(|s| s.as_str()).unwrap_or("").trim();
    let path = body.get("path").map(|s| s.as_str()).unwrap_or("").trim();
    let icon = body
        .get("icon")
        .map(|s| s.as_str())
        .unwrap_or("package")
        .trim();
    let sort_order: i64 = body
        .get("sort_order")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if label.is_empty() || path.is_empty() {
        return err_bad_request("Label and path are required");
    }

    let mut data = super::helpers::json_map(serde_json::json!({
        "label": label,
        "path": path,
        "icon": icon,
        "sort_order": sort_order,
    }));
    super::helpers::stamp_created(&mut data);

    if let Err(e) = db::create(ctx, TABLE, data).await {
        return err_internal("Failed to create button", e.message);
    }

    // Re-render buttons table
    let buttons = load_buttons(ctx).await;
    ui::html_response(render_buttons_table(&buttons))
}

/// Validate that `id` is safe to interpolate into inline HTML/JS strings
/// (DOM IDs, `getElementById` arguments, etc.). Rejects anything outside
/// `[A-Za-z0-9_-]` and any string longer than 64 chars or empty.
///
/// Used by SEC-058: the userportal edit-button form inlines the record ID
/// into a `script` block via `PreEscaped`, so it must not contain quotes,
/// angle brackets, backslashes, or any other JS-syntax-significant chars.
fn is_safe_dom_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

async fn handle_edit_button_form(ctx: &dyn Context, id: &str) -> OutputStream {
    // SEC-058: the modal auto-show script below uses `PreEscaped(format!(...))`
    // to inject the record ID into inline JS. A record returned from
    // `db::get` should always have a well-formed ID (UUIDv7), but defensively
    // reject any caller-supplied path segment that isn't strict
    // `[a-zA-Z0-9_-]{1,64}` so the JS string interpolation can never be
    // poisoned even if a future code path looks the record up by a
    // non-validated identifier.
    if !is_safe_dom_id(id) {
        return err_not_found("Button not found");
    }
    let record = match db::get(ctx, TABLE, id).await {
        Ok(r) => r,
        Err(_) => return err_not_found("Button not found"),
    };

    let current_icon = record.str_field("icon");

    let markup = html! {
        (components::modal(&format!("edit-btn-{id}"), "Edit Button", html! {
            form
                hx-put=(format!("/b/userportal/admin/buttons/{id}"))
                hx-target="#buttons-table"
                hx-swap="outerHTML"
                style="display:flex;flex-direction:column;gap:0.75rem"
            {
                div .form-group style="margin:0" {
                    label .form-label { "Label" }
                    input .form-input type="text" name="label"
                        value=(record.str_field("label")) required;
                }
                div .form-group style="margin:0" {
                    label .form-label { "Path" }
                    input .form-input type="text" name="path"
                        value=(record.str_field("path")) required;
                }
                div .form-group style="margin:0" {
                    label .form-label { "Icon" }
                    select .form-input name="icon" {
                        @for &(value, display) in ICON_OPTIONS {
                            option value=(value) selected[value == current_icon] { (display) }
                        }
                    }
                }
                div .form-group style="margin:0" {
                    label .form-label { "Order" }
                    input .form-input type="number" name="sort_order"
                        value=(record.i64_field("sort_order"));
                }
                div style="display:flex;gap:0.5rem;justify-content:flex-end" {
                    button .btn .btn-secondary type="button"
                        onclick=(format!("document.getElementById('edit-btn-{id}').style.display='none'"))
                    { "Cancel" }
                    button .btn .btn-primary type="submit" { "Save" }
                }
            }
        }))
        // Auto-show the modal
        script { (maud::PreEscaped(format!(
            "document.getElementById('edit-btn-{id}').style.display='flex';"
        ))) }
    };

    ui::html_response(markup)
}

async fn handle_update_button(ctx: &dyn Context, input: InputStream, id: &str) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body = parse_form_body(&raw);

    let label = body.get("label").map(|s| s.as_str()).unwrap_or("").trim();
    let path = body.get("path").map(|s| s.as_str()).unwrap_or("").trim();
    let icon = body
        .get("icon")
        .map(|s| s.as_str())
        .unwrap_or("package")
        .trim();
    let sort_order: i64 = body
        .get("sort_order")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if label.is_empty() || path.is_empty() {
        return err_bad_request("Label and path are required");
    }

    let mut data = super::helpers::json_map(serde_json::json!({
        "label": label,
        "path": path,
        "icon": icon,
        "sort_order": sort_order,
    }));
    stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, TABLE, id, data).await {
        return err_internal("Failed to update button", e.message);
    }

    // Re-render buttons table
    let buttons = load_buttons(ctx).await;
    ui::html_response(render_buttons_table(&buttons))
}

async fn handle_delete_button(ctx: &dyn Context, id: &str) -> OutputStream {
    if let Err(e) = db::delete(ctx, TABLE, id).await {
        return err_internal("Failed to delete button", e.message);
    }

    let buttons = load_buttons(ctx).await;
    ui::html_response(render_buttons_table(&buttons))
}

// ---------------------------------------------------------------------------
// Admin: Branding Settings
// ---------------------------------------------------------------------------

const PORTAL_SETTINGS_KEYS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "SOLOBASE_SHARED__APP_NAME",
        "Application Name",
        "Display name shown in headers, emails, and login pages.",
        "Solobase",
        "text",
    ),
    (
        "SOLOBASE_SHARED__LOGO_URL",
        "Logo URL",
        "URL of the logo image shown in the header and login pages.",
        "https://solobase.dev/images/logo_long.png",
        "text",
    ),
    (
        "SOLOBASE_SHARED__LOGO_ICON_URL",
        "Logo Icon URL",
        "Small icon version of the logo (used in favicons and compact views).",
        "https://solobase.dev/images/logo.png",
        "text",
    ),
    (
        "SOLOBASE_SHARED__AUTH_LOGO_URL",
        "Auth Logo URL",
        "Override logo for login/signup pages. Falls back to Logo URL if empty.",
        "",
        "text",
    ),
    (
        "SOLOBASE_SHARED__FAVICON_URL",
        "Favicon URL",
        "URL of the favicon for browser tabs.",
        "",
        "text",
    ),
    (
        "SOLOBASE_SHARED__PRIMARY_COLOR",
        "Primary Color",
        "Primary brand color used for buttons, links, and accents.",
        "#6366f1",
        "color",
    ),
];

async fn admin_settings_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let mut values = Vec::new();
    for &(key, label, help, default, input_type) in PORTAL_SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value, input_type));
    }

    let content = html! {
        (components::page_header("Branding Settings", Some("Customize your application appearance"), None))

        form #settings-form onsubmit="return submitPortalSettings(event)" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::settings()) " Branding"
            }

            @for (key, label, help, default, ref value, input_type) in &values {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(key) { (label) }
                    @if *input_type == "color" {
                        div style="display:flex;align-items:center;gap:0.75rem" {
                            input .form-input #(key) name=(key) type="text" value=(value) style="flex:1";
                            input type="color" value=(value)
                                style="width:40px;height:36px;border:1px solid #e2e8f0;border-radius:6px;cursor:pointer;padding:2px"
                                onchange={"document.getElementById('" (key) "').value=this.value"};
                        }
                    } @else {
                        input .form-input #(key) name=(key) type="text" value=(value) placeholder=(default);
                    }
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                }
            }

            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }

        script { (PreEscaped(r#"
function submitPortalSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name]').forEach(function(el) { data[el.name] = el.value; });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/userportal/admin/settings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) })
    .then(function(r) { return r.json(); })
    .then(function(d) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' } })); })
    .catch(function(err) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: 'Error: ' + err.message, type: 'error' } })); })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#)) }
    };

    render_page(
        "Settings",
        &site_config,
        "/b/userportal/admin/settings",
        user.as_ref(),
        "Settings",
        content,
        msg,
    )
}

async fn handle_save_settings(ctx: &dyn Context, input: InputStream) -> OutputStream {
    let raw = input.collect_to_bytes().await;
    let body: std::collections::HashMap<String, String> = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => {
            return ok_json(&serde_json::json!({"error": format!("Invalid request: {e}")}));
        }
    };
    for &(key, _, _, _, _) in PORTAL_SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }
    ok_json(&serde_json::json!({"message": "Settings saved"}))
}

#[cfg(test)]
mod cross_block_tests {
    use std::collections::HashMap;

    use serde_json::json;
    use wafer_core::clients::database as db;
    use wafer_run::block::Block;

    use super::*;
    use crate::test_support::{anon_msg, output_json, TestContext};

    fn button_data(
        label: &str,
        icon: &str,
        path: &str,
        sort_order: i64,
    ) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("label".to_string(), json!(label));
        m.insert("icon".to_string(), json!(icon));
        m.insert("path".to_string(), json!(path));
        m.insert("sort_order".to_string(), json!(sort_order));
        m
    }

    #[tokio::test]
    async fn list_buttons_action_returns_json_array_in_sort_order() {
        let ctx = TestContext::with_userportal().await;

        // Seed two buttons through the userportal-owned `buttons` table.
        db::create(
            &ctx,
            TABLE,
            button_data("Solobase", "shield", "/b/admin/", 0),
        )
        .await
        .expect("seed first button");
        db::create(
            &ctx,
            TABLE,
            button_data("Inspector", "search", "/b/inspector/ui", 1),
        )
        .await
        .expect("seed second button");

        let block = UserPortalBlock;
        let msg = anon_msg("retrieve", "/b/userportal/internal/list-buttons");
        let resp = block
            .handle(&ctx, msg, wafer_run::InputStream::empty())
            .await;
        let parsed = output_json(resp).await;

        let arr = parsed.as_array().expect("response is JSON array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["label"], "Solobase");
        assert_eq!(arr[0]["icon"], "shield");
        assert_eq!(arr[0]["path"], "/b/admin/");
        assert_eq!(arr[1]["label"], "Inspector");
        assert_eq!(arr[1]["icon"], "search");
    }

    #[tokio::test]
    async fn list_buttons_action_returns_empty_array_when_none_configured() {
        let ctx = TestContext::new().await;
        let block = UserPortalBlock;
        let msg = anon_msg("retrieve", "/b/userportal/internal/list-buttons");
        let resp = block
            .handle(&ctx, msg, wafer_run::InputStream::empty())
            .await;
        let parsed = output_json(resp).await;
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/userportal", UserPortalBlock);
