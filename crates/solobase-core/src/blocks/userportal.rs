use super::helpers::{parse_form_body, stamp_updated, RecordExt};
use crate::ui::{self, components, icons, sidebar::nav_icon, NavItem, SiteConfig, UserInfo};
use maud::html;
use wafer_core::clients::database::{ListOptions, SortField};
use wafer_core::clients::{config, database as db};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

const BUTTONS_COLLECTION: &str = "userportal_buttons";

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
        .collections(vec![CollectionSchema::new("userportal_buttons")
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
        .config_keys(vec![
            BlockConfigKey::new("APP_NAME", "Application display name", "Solobase"),
            BlockConfigKey::new("LOGO_URL", "Logo image URL", ""),
            BlockConfigKey::new("PRIMARY_COLOR", "Primary brand color", "#6366f1"),
            BlockConfigKey::new("ALLOW_SIGNUP", "Allow new user registration", "true"),
            BlockConfigKey::new("ENABLE_OAUTH", "Enable OAuth login", "false"),
        ])
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
        let block_rows = db::query_raw(ctx, "SELECT block_name, enabled FROM block_settings", &[])
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
            "logo_url": config::get_default(ctx, "LOGO_URL", "").await,
            "app_name": config::get_default(ctx, "APP_NAME", "Solobase").await,
            "primary_color": config::get_default(ctx, "PRIMARY_COLOR", "#6366f1").await,
            "enable_oauth": config::get_default(ctx, "ENABLE_OAUTH", "false").await,
            "allow_signup": config::get_default(ctx, "ALLOW_SIGNUP", "true").await,
            "show_powered_by": true,
            "features": {
                "files": is_enabled("suppers-ai/files"),
                "products": is_enabled("suppers-ai/products"),
                "user_products": config::get_default(ctx, "FEATURE_USER_PRODUCTS", "false").await,
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

fn admin_buttons_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "My Account".into(),
            href: "/b/userportal/".into(),
            icon: "user",
        },
        NavItem {
            label: "Manage Buttons".into(),
            href: "/b/userportal/admin/buttons".into(),
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
    let user_record = db::get(ctx, "auth_users", &user_id).await.ok();
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

    if let Err(e) = db::update(ctx, "auth_users", &user_id, data).await {
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
        &admin_buttons_nav(),
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
