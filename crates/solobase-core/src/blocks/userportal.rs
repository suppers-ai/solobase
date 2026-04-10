use super::helpers::{parse_form_body, stamp_updated, RecordExt};
use crate::ui::{self, components, icons, sidebar::nav_icon, NavItem, SiteConfig, UserInfo};
use maud::{html, PreEscaped};
use wafer_core::clients::database::{ListOptions, SortField};
use wafer_core::clients::{config, database as db};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

const BUTTONS_COLLECTION: &str = "suppers_ai__userportal__buttons";

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
        .collections(vec![CollectionSchema::new(BUTTONS_COLLECTION)
            .field("label", "string")
            .field_default("icon", "string", "package")
            .field("path", "string")
            .field_default("sort_order", "int", "0")])
        .category(wafer_run::BlockCategory::Feature)
        .description("User-facing profile page with editable display name, admin-configurable navigation buttons, and portal configuration endpoint.")
        .endpoints(vec![
            BlockEndpoint::get(
                "/b/userportal/",
                "User profile page",
                AuthLevel::Authenticated,
            ),
            BlockEndpoint::post(
                "/b/userportal/update-profile",
                "Update profile",
                AuthLevel::Authenticated,
            ),
            BlockEndpoint::get(
                "/b/userportal/config",
                "Portal configuration",
                AuthLevel::Public,
            ),
            BlockEndpoint::get(
                "/b/userportal/admin/buttons",
                "Manage portal buttons",
                AuthLevel::Admin,
            ),
            BlockEndpoint::post(
                "/b/userportal/admin/buttons",
                "Create button",
                AuthLevel::Admin,
            ),
        ])
        .config_keys(vec![])
        .admin_url("/b/userportal/admin/settings")
        .can_disable(true)
        .default_enabled(false)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![wafer_run::UiRoute::authenticated("/")]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();
        let action = msg.action().to_string();

        if !path.starts_with("/b/userportal") {
            return self.handle_config(ctx, msg).await;
        }

        let sub = path.strip_prefix("/b/userportal").unwrap_or("/");

        // Admin routes — require admin role
        if sub.starts_with("/admin/") {
            if !msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin")
            {
                return crate::ui::forbidden_response(msg);
            }
            return self.handle_admin(ctx, msg, &action, sub).await;
        }

        match (action.as_str(), sub) {
            ("retrieve", "" | "/") => profile_page(ctx, msg).await,
            ("create", "/update-profile") => handle_update_profile(ctx, msg).await,
            ("retrieve", "/config") => self.handle_config(ctx, msg).await,
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

impl UserPortalBlock {
    async fn handle_config(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let block_rows = db::list_all(ctx, crate::blocks::admin::BLOCK_SETTINGS_COLLECTION, vec![])
            .await
            .unwrap_or_default();

        let is_enabled = |name: &str| -> bool {
            block_rows
                .iter()
                .find(|r| r.data.get("block_name").and_then(|v| v.as_str()) == Some(name))
                .and_then(|r| r.data.get("enabled").and_then(|v| v.as_i64()))
                .map(|v| v != 0)
                .unwrap_or(true)
        };

        let config_val = serde_json::json!({
            "logo_url": config::get_default(ctx, "SOLOBASE_SHARED__LOGO_URL", "").await,
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
                "projects": is_enabled("suppers-ai/projects"),
                "userportal": is_enabled("suppers-ai/userportal"),
            }
        });
        json_respond(msg, &config_val)
    }

    async fn handle_admin(
        &self,
        ctx: &dyn Context,
        msg: &mut Message,
        action: &str,
        sub: &str,
    ) -> Result_ {
        match (action, sub) {
            ("retrieve", "/admin/settings") => admin_settings_page(ctx, msg).await,
            ("create", "/admin/settings") => handle_save_settings(ctx, msg).await,
            ("retrieve", "/admin/buttons") => admin_buttons_page(ctx, msg).await,
            ("create", "/admin/buttons") => handle_create_button(ctx, msg).await,
            ("retrieve", s) if s.starts_with("/admin/buttons/") && s.ends_with("/edit") => {
                let id = s
                    .strip_prefix("/admin/buttons/")
                    .and_then(|s| s.strip_suffix("/edit"))
                    .unwrap_or("");
                if id.is_empty() {
                    return err_not_found(msg, "not found");
                }
                handle_edit_button_form(ctx, msg, id).await
            }
            ("update", s) if s.starts_with("/admin/buttons/") => {
                let id = s.strip_prefix("/admin/buttons/").unwrap_or("");
                if id.is_empty() {
                    return err_not_found(msg, "not found");
                }
                handle_update_button(ctx, msg, id).await
            }
            ("delete", s) if s.starts_with("/admin/buttons/") => {
                let id = s.strip_prefix("/admin/buttons/").unwrap_or("");
                if id.is_empty() {
                    return err_not_found(msg, "not found");
                }
                handle_delete_button(ctx, msg, id).await
            }
            _ => err_not_found(msg, "not found"),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn portal_nav() -> Vec<NavItem> {
    vec![NavItem {
        label: "My Account".into(),
        href: "/b/userportal/".into(),
        icon: "user",
    }]
}

fn admin_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "My Account".into(),
            href: "/b/userportal/".into(),
            icon: "user",
        },
        NavItem {
            label: "Manage Buttons".into(),
            href: "/b/userportal/admin/buttons".into(),
            icon: "package",
        },
        NavItem {
            label: "Settings".into(),
            href: "/b/userportal/admin/settings".into(),
            icon: "settings",
        },
    ]
}

fn render_page(
    title: &str,
    config: &SiteConfig,
    nav: &[NavItem],
    path: &str,
    user: Option<&UserInfo>,
    content: maud::Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(title, config, nav, user, path, content, is_fragment);
    ui::html_response(msg, markup)
}

async fn load_buttons(ctx: &dyn Context) -> Vec<wafer_core::clients::database::Record> {
    db::list(
        ctx,
        BUTTONS_COLLECTION,
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
// User-facing: Profile page
// ---------------------------------------------------------------------------

async fn profile_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();

    // Load user details
    let user_record = db::get(ctx, crate::blocks::auth::USERS_COLLECTION, &user_id).await.ok();
    let display_name = user_record
        .as_ref()
        .map(|r| r.str_field("name").to_string())
        .unwrap_or_default();
    let avatar_url = user_record
        .as_ref()
        .map(|r| r.str_field("avatar_url").to_string())
        .unwrap_or_default();
    let email = user.as_ref().map(|u| u.email.as_str()).unwrap_or("");

    // Load configurable buttons
    let buttons = load_buttons(ctx).await;

    let content = html! {
        (components::page_header("My Account", Some("Manage your profile and settings"), None))

        // Profile card
        div .card style="margin-bottom:1.5rem" {
            div style="display:flex;align-items:center;gap:1.5rem;padding:1.5rem" {
                // Avatar
                div .user-avatar style="width:64px;height:64px;font-size:1.5rem;flex-shrink:0" {
                    @if !avatar_url.is_empty() {
                        img src=(avatar_url) alt="Avatar" style="width:100%;height:100%;border-radius:50%;object-fit:cover";
                    } @else if let Some(u) = &user {
                        (u.avatar_initial())
                    }
                }
                div style="flex:1;min-width:0" {
                    h2 style="margin:0;font-size:1.25rem" {
                        @if display_name.is_empty() { (email) } @else { (display_name) }
                    }
                    p .text-muted style="margin:0.25rem 0 0" { (email) }
                    @if let Some(u) = &user {
                        div style="margin-top:0.5rem;display:flex;gap:0.25rem;flex-wrap:wrap" {
                            @for role in &u.roles {
                                (components::status_badge(role))
                            }
                        }
                    }
                }
            }

            // Edit name form
            div style="padding:0 1.5rem 1.5rem;border-top:1px solid var(--border-color)" {
                form
                    hx-post="/b/userportal/update-profile"
                    hx-target="#content"
                    hx-swap="innerHTML"
                    style="display:flex;gap:0.5rem;align-items:end;margin-top:1rem"
                {
                    div .form-group style="flex:1;margin:0" {
                        label .form-label for="display-name" { "Display Name" }
                        input .form-input #display-name type="text" name="name"
                            value=(display_name) placeholder="Enter your name";
                    }
                    button .btn .btn-primary type="submit" { "Save" }
                }
            }
        }

        // Action buttons grid
        @if !buttons.is_empty() {
            div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:1rem;margin-bottom:1.5rem" {
                @for btn in &buttons {
                    @let label = btn.str_field("label");
                    @let icon = btn.str_field("icon");
                    @let path = btn.str_field("path");
                    a .card href=(path)
                        style="padding:1.25rem;text-decoration:none;color:inherit;display:flex;align-items:center;gap:0.75rem;transition:box-shadow 0.15s"
                    {
                        span .nav-icon {
                            (nav_icon(if icon.is_empty() { "package" } else { icon }))
                        }
                        span .font-medium { (label) }
                    }
                }
            }
        }

        // Account actions
        div .card style="padding:1.25rem" {
            h3 style="margin:0 0 1rem;font-size:1rem" { "Account" }
            div style="display:flex;flex-direction:column;gap:0.5rem" {
                a .btn .btn-secondary href="/b/auth/change-password" {
                    (icons::key()) " Change Password"
                }
                form action="/b/auth/api/logout" method="post" {
                    button .btn .btn-ghost type="submit" style="width:100%;color:var(--danger)" {
                        (icons::log_out()) " Sign Out"
                    }
                }
            }
        }
    };

    render_page(
        "My Account",
        &site_config,
        &portal_nav(),
        "/b/userportal/",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// User-facing: Update profile
// ---------------------------------------------------------------------------

async fn handle_update_profile(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return err_forbidden(msg, "Not authenticated");
    }

    let body = parse_form_body(&msg.data);
    let name = body.get("name").map(|s| s.as_str()).unwrap_or("");

    let mut data = std::collections::HashMap::new();
    data.insert("name".to_string(), serde_json::json!(name));
    stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, crate::blocks::auth::USERS_COLLECTION, &user_id, data).await {
        return err_internal(msg, &format!("Failed to update profile: {}", e.message));
    }

    // Re-render the profile page (htmx will swap content)
    profile_page(ctx, msg).await
}

// ---------------------------------------------------------------------------
// Admin: Buttons management page
// ---------------------------------------------------------------------------

async fn admin_buttons_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        &admin_nav(),
        "/b/userportal/admin/buttons",
        user.as_ref(),
        content,
        msg,
    )
}

fn render_buttons_table(buttons: &[wafer_core::clients::database::Record]) -> maud::Markup {
    html! {
        div #buttons-table {
            @if buttons.is_empty() {
                (components::empty_state(
                    "No buttons configured",
                    "Add a button above to show navigation links on the user profile page.",
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

async fn handle_create_button(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body = parse_form_body(&msg.data);

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
        return err_bad_request(msg, "Label and path are required");
    }

    let mut data = super::helpers::json_map(serde_json::json!({
        "label": label,
        "path": path,
        "icon": icon,
        "sort_order": sort_order,
    }));
    super::helpers::stamp_created(&mut data);

    if let Err(e) = db::create(ctx, BUTTONS_COLLECTION, data).await {
        return err_internal(msg, &format!("Failed to create button: {}", e.message));
    }

    // Re-render buttons table
    let buttons = load_buttons(ctx).await;
    ui::html_response(msg, render_buttons_table(&buttons))
}

async fn handle_edit_button_form(ctx: &dyn Context, msg: &mut Message, id: &str) -> Result_ {
    let record = match db::get(ctx, BUTTONS_COLLECTION, id).await {
        Ok(r) => r,
        Err(_) => return err_not_found(msg, "Button not found"),
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

    ui::html_response(msg, markup)
}

async fn handle_update_button(ctx: &dyn Context, msg: &mut Message, id: &str) -> Result_ {
    let body = parse_form_body(&msg.data);

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
        return err_bad_request(msg, "Label and path are required");
    }

    let mut data = super::helpers::json_map(serde_json::json!({
        "label": label,
        "path": path,
        "icon": icon,
        "sort_order": sort_order,
    }));
    stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, BUTTONS_COLLECTION, id, data).await {
        return err_internal(msg, &format!("Failed to update button: {}", e.message));
    }

    // Re-render buttons table
    let buttons = load_buttons(ctx).await;
    ui::html_response(msg, render_buttons_table(&buttons))
}

async fn handle_delete_button(ctx: &dyn Context, msg: &mut Message, id: &str) -> Result_ {
    if let Err(e) = db::delete(ctx, BUTTONS_COLLECTION, id).await {
        return err_internal(msg, &format!("Failed to delete button: {}", e.message));
    }

    let buttons = load_buttons(ctx).await;
    ui::html_response(msg, render_buttons_table(&buttons))
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
        "",
        "text",
    ),
    (
        "SOLOBASE_SHARED__LOGO_ICON_URL",
        "Logo Icon URL",
        "Small icon version of the logo (used in favicons and compact views).",
        "",
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

async fn admin_settings_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        &admin_nav(),
        "/b/userportal/admin/settings",
        user.as_ref(),
        content,
        msg,
    )
}

async fn handle_save_settings(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: std::collections::HashMap<String, String> = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };
    for &(key, _, _, _, _) in PORTAL_SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }
    json_respond(msg, &serde_json::json!({"message": "Settings saved"}))
}
