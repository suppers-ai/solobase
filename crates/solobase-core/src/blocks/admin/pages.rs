//! SSR pages for the admin block.
//!
//! Each page queries the database directly (same patterns as the JSON handlers)
//! and renders HTML via maud.

use crate::blocks::helpers::{parse_form_body, RecordExt};
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup, PreEscaped};
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::{config, database as db};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

/// Admin nav items for the sidebar.
fn admin_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Dashboard".into(),
            href: "/b/admin/".into(),
            icon: "layout-dashboard",
        },
        NavItem {
            label: "Users".into(),
            href: "/b/admin/users".into(),
            icon: "users",
        },
        NavItem {
            label: "Config".into(),
            href: "/b/admin/variables".into(),
            icon: "settings",
        },
        NavItem {
            label: "Network".into(),
            href: "/b/admin/network".into(),
            icon: "network",
        },
        NavItem {
            label: "Storage".into(),
            href: "/b/admin/storage".into(),
            icon: "hard-drive",
        },
        NavItem {
            label: "Permissions".into(),
            href: "/b/admin/permissions".into(),
            icon: "shield",
        },
        NavItem {
            label: "Logs".into(),
            href: "/b/admin/logs".into(),
            icon: "file-text",
        },
        NavItem {
            label: "Email".into(),
            href: "/b/admin/email".into(),
            icon: "globe",
        },
        NavItem {
            label: "Blocks".into(),
            href: "/b/admin/blocks".into(),
            icon: "package",
        },
    ]
}

/// Wrap content in the admin shell (sidebar + layout), or return fragment for htmx.
fn admin_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(
        title,
        config,
        &admin_nav(),
        user,
        path,
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

pub async fn dashboard(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Total users
    let user_count = db::list(
        ctx,
        "suppers_ai__auth__users",
        &ListOptions {
            filters: vec![Filter {
                field: "deleted_at".into(),
                operator: FilterOp::IsNull,
                value: serde_json::Value::Null,
            }],
            limit: 1,
            ..Default::default()
        },
    )
    .await
    .map(|r| r.total_count)
    .unwrap_or(0);

    // New users today
    let new_users_today = db::query_raw(
        ctx,
        "SELECT COUNT(*) as cnt FROM suppers_ai__auth__users WHERE deleted_at IS NULL AND created_at >= ?1",
        &[serde_json::json!(format!("{today}T00:00:00"))],
    )
    .await
    .ok()
    .and_then(|r| {
        r.first()
            .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
    })
    .unwrap_or(0);

    // Requests today
    let requests_today = db::query_raw(
        ctx,
        "SELECT COUNT(*) as cnt FROM suppers_ai__admin__request_logs WHERE created_at >= ?1",
        &[serde_json::json!(format!("{today}T00:00:00"))],
    )
    .await
    .ok()
    .and_then(|r| {
        r.first()
            .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
    })
    .unwrap_or(0);

    // Errors today
    let errors_today = db::query_raw(
        ctx,
        "SELECT COUNT(*) as cnt FROM suppers_ai__admin__request_logs WHERE status = 'ERROR' AND created_at >= ?1",
        &[serde_json::json!(format!("{today}T00:00:00"))],
    )
    .await
    .ok()
    .and_then(|r| {
        r.first()
            .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
    })
    .unwrap_or(0);

    // Avg response time today
    let avg_ms = db::query_raw(
        ctx,
        "SELECT AVG(duration_ms) as avg_ms FROM suppers_ai__admin__request_logs WHERE created_at >= ?1",
        &[serde_json::json!(format!("{today}T00:00:00"))],
    )
    .await
    .ok()
    .and_then(|r| {
        r.first()
            .and_then(|r| r.data.get("avg_ms").and_then(|v| v.as_f64()))
    })
    .unwrap_or(0.0);

    // Recent users (last 5 logins)
    let recent_users = db::query_raw(ctx,
        "SELECT id, email, created_at FROM suppers_ai__auth__users WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT 5",
        &[],
    ).await.unwrap_or_default();

    // Recent audit logs (last 5)
    let recent_audit = db::query_raw(ctx,
        "SELECT action, resource, user_id, created_at FROM suppers_ai__admin__audit_logs ORDER BY created_at DESC LIMIT 5",
        &[],
    ).await.unwrap_or_default();

    // Recent errors (last 5)
    let recent_errors = db::query_raw(ctx,
        "SELECT status_code, method, path, duration_ms, created_at FROM suppers_ai__admin__request_logs WHERE status = 'ERROR' OR status_code >= 400 ORDER BY created_at DESC LIMIT 5",
        &[],
    ).await.unwrap_or_default();

    let content = html! {
        (components::page_header("Dashboard", Some("System overview"), None))

        // Top row — key metrics
        div .stats-grid {
            (components::stat_card("Total Users", &user_count.to_string(), icons::users()))
            (components::stat_card("New Today", &new_users_today.to_string(), icons::plus()))
            (components::stat_card("Requests Today", &requests_today.to_string(), icons::server()))
            (components::stat_card("Errors Today", &errors_today.to_string(), icons::x()))
            (components::stat_card("Avg Response", &format!("{:.0}ms", avg_ms), icons::refresh_cw()))
        }

        // Two columns: Recent Users + Recent Activity
        div style="display:grid;grid-template-columns:1fr 1fr;gap:1.5rem;margin-top:1.5rem" {
            // Recent Users
            div .card {
                div .card-header {
                    h3 .card-title { "Recent Users" }
                    a .btn .btn-ghost .btn-sm href="/b/admin/users" { "View all" }
                }
                @if recent_users.is_empty() {
                    p .text-muted .text-sm { "No users yet" }
                } @else {
                    div .table-container {
                        table .table {
                            tbody {
                                @for record in &recent_users {
                                    @let email = record.data.get("email").and_then(|v| v.as_str()).unwrap_or("");
                                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                    tr {
                                        td .text-sm { (email) }
                                        td .text-muted .text-sm .text-right { (created.get(..10).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Recent Activity (Audit Logs)
            div .card {
                div .card-header {
                    h3 .card-title { "Recent Activity" }
                    a .btn .btn-ghost .btn-sm href="/b/admin/logs?tab=audit" { "View all" }
                }
                @if recent_audit.is_empty() {
                    p .text-muted .text-sm { "No activity yet" }
                } @else {
                    div .table-container {
                        table .table {
                            tbody {
                                @for record in &recent_audit {
                                    @let action = record.data.get("action").and_then(|v| v.as_str()).unwrap_or("");
                                    @let resource = record.data.get("resource").and_then(|v| v.as_str()).unwrap_or("");
                                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                    tr {
                                        td { span .badge .badge-info .text-xs { (action) } }
                                        td .text-sm { (resource) }
                                        td .text-muted .text-sm .text-right { (created.get(..19).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recent Errors
        @if !recent_errors.is_empty() {
            div .card style="margin-top:1.5rem" {
                div .card-header {
                    h3 .card-title { "Recent Errors" }
                    a .btn .btn-ghost .btn-sm href="/b/admin/logs" { "View all" }
                }
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Status" }
                                th { "Method" }
                                th { "Path" }
                                th { "Duration" }
                                th { "Time" }
                            }
                        }
                        tbody {
                            @for record in &recent_errors {
                                @let code = record.data.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);
                                @let method = record.data.get("method").and_then(|v| v.as_str()).unwrap_or("");
                                @let path = record.data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                @let duration = record.data.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                                @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                tr {
                                    td {
                                        span .badge .(if code >= 500 { "badge-danger" } else { "badge-warning" }) { (code) }
                                    }
                                    td .text-sm .font-medium { (method.to_uppercase()) }
                                    td .text-sm { (path) }
                                    td .text-muted .text-sm { (duration) "ms" }
                                    td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    admin_page(
        "Dashboard",
        &config,
        "/b/admin/",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

pub async fn users_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "roles" => "roles",
        "api-keys" => "api-keys",
        _ => "users",
    };

    let content = html! {
        (components::page_header("Users & Access", Some("Manage accounts, roles, and API keys"), None))

        // Tabs
        div .tabs {
            a .tab .(if active_tab == "users" { "active" } else { "" })
                href="/b/admin/users"
                hx-get="/b/admin/users"
                hx-target="#content"
                hx-push-url="true"
            { (icons::users()) " Users" }
            a .tab .(if active_tab == "roles" { "active" } else { "" })
                href="/b/admin/users?tab=roles"
                hx-get="/b/admin/users?tab=roles"
                hx-target="#content"
                hx-push-url="true"
            { (icons::shield()) " Roles" }
            a .tab .(if active_tab == "api-keys" { "active" } else { "" })
                href="/b/admin/users?tab=api-keys"
                hx-get="/b/admin/users?tab=api-keys"
                hx-target="#content"
                hx-push-url="true"
            { (icons::key()) " API Keys" }
        }

        @let current_uid = user.as_ref().map(|u| u.id.as_str()).unwrap_or("");
        div #users-tab-content {
            @if active_tab == "users" {
                (users_tab(ctx, msg, current_uid).await)
            } @else if active_tab == "roles" {
                (roles_tab(ctx).await)
            } @else {
                (api_keys_tab(ctx).await)
            }
        }
    };

    admin_page(
        "Users",
        &config,
        "/b/admin/users",
        user.as_ref(),
        content,
        msg,
    )
}

/// Users tab content (table + search + pagination).
async fn users_tab(ctx: &dyn Context, msg: &mut Message, current_user_id: &str) -> Markup {
    let (page, page_size, _) = msg.pagination_params(20);
    let search = msg.query("search").to_string();

    let result = if !search.is_empty() {
        // Search by email OR id (raw SQL for OR support)
        let like = format!("%{search}%");
        let offset = ((page - 1) * page_size) as i64;
        let records = db::query_raw(
            ctx,
            "SELECT * FROM suppers_ai__auth__users WHERE deleted_at IS NULL AND (email LIKE ?1 OR id LIKE ?1) ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
            &[serde_json::json!(like), serde_json::json!(page_size), serde_json::json!(offset)],
        ).await;
        // Wrap in RecordList format
        match records {
            Ok(rows) => Ok(db::RecordList {
                total_count: rows.len() as i64,
                page: page as i64,
                page_size: page_size as i64,
                records: rows,
            }),
            Err(e) => Err(e),
        }
    } else {
        let filters = vec![Filter {
            field: "deleted_at".into(),
            operator: FilterOp::IsNull,
            value: serde_json::Value::Null,
        }];
        let sort = vec![SortField {
            field: "created_at".into(),
            desc: true,
        }];
        db::paginated_list(
            ctx,
            "suppers_ai__auth__users",
            page as i64,
            page_size as i64,
            filters,
            sort,
        )
        .await
    };

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by email or user ID...", "/b/admin/users", "#content", &search))
        }

        @match &result {
            Ok(list) => {
                (users_table(&list.records, ctx, current_user_id).await)

                @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                (components::pagination(list.page as u32, total_pages, "/b/admin/users", "#users-tab-content"))
            }
            Err(e) => {
                div .login-error { "Failed to load users: " (e.message) }
            }
        }
    }
}

/// Render the users table body. Async because it enriches each user with roles.
async fn users_table(records: &[db::Record], ctx: &dyn Context, current_user_id: &str) -> Markup {
    // Pre-fetch roles for all users
    let mut user_roles: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for record in records {
        let roles_opts = ListOptions {
            filters: vec![Filter {
                field: "user_id".into(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String(record.id.clone()),
            }],
            ..Default::default()
        };
        let roles: Vec<String> = match db::list(ctx, "suppers_ai__admin__user_roles", &roles_opts).await {
            Ok(r) => r
                .records
                .iter()
                .map(|rec| rec.str_field("role").to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => Vec::new(),
        };
        user_roles.insert(record.id.clone(), roles);
    }

    html! {
        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Email" }
                        th { "Roles" }
                        th { "Status" }
                        th { "Created" }
                        th { "Actions" }
                    }
                }
                tbody {
                    @if records.is_empty() {
                        tr {
                            td colspan="5" .text-center .text-muted style="padding: 2rem;" { "No users found" }
                        }
                    }
                    @for record in records {
                        @let email = record.str_field("email");
                        @let disabled = record.bool_field("disabled");
                        @let created = record.str_field("created_at");
                        @let roles = user_roles.get(&record.id).cloned().unwrap_or_default();
                        tr #{"user-row-" (record.id)} {
                            td { (email) }
                            td {
                                @for role in &roles {
                                    span .badge .badge-primary style="margin-right: 0.25rem;" { (role) }
                                }
                                @if roles.is_empty() {
                                    span .text-muted { "—" }
                                }
                            }
                            td {
                                @if disabled {
                                    (components::status_badge("disabled"))
                                } @else {
                                    (components::status_badge("active"))
                                }
                            }
                            td .text-muted .text-sm { (created.get(..10).unwrap_or(created)) }
                            td {
                                @let is_self = record.id == current_user_id;
                                @if is_self {
                                    span .text-muted .text-sm { "(you)" }
                                } @else {
                                    @if disabled {
                                        button .btn .btn-sm .btn-success
                                            hx-post={"/b/admin/users/" (record.id) "/enable"}
                                            hx-target={"#user-row-" (record.id)}
                                            hx-swap="outerHTML"
                                            title="Enable user"
                                        { "Enable" }
                                    } @else {
                                        button .btn .btn-sm .btn-secondary
                                            hx-post={"/b/admin/users/" (record.id) "/disable"}
                                            hx-target={"#user-row-" (record.id)}
                                            hx-swap="outerHTML"
                                            hx-confirm={"Disable " (email) "?"}
                                            title="Disable user"
                                        { "Disable" }
                                    }
                                    " "
                                    button .btn .btn-sm .btn-danger
                                        hx-delete={"/b/admin/users/" (record.id)}
                                        hx-target={"#user-row-" (record.id)}
                                        hx-swap="outerHTML"
                                        hx-confirm={"Delete " (email) "? This cannot be undone."}
                                        title="Delete user"
                                    { (icons::trash()) }
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
// Settings
// ---------------------------------------------------------------------------

pub async fn variables_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = if tab == "all" { "all" } else { "blocks" };

    let content = html! {
        (components::page_header("Config", Some("Block configuration variables and access control"),
            Some(html! {
                button .btn .btn-primary .btn-sm onclick="openModal('create-var')" {
                    (icons::plus()) " Add Variable"
                }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "blocks" { "active" } else { "" })
                href="/b/admin/variables"
                hx-get="/b/admin/variables"
                hx-target="#content"
                hx-push-url="true"
            { (icons::package()) " By Block" }
            a .tab .(if active_tab == "all" { "active" } else { "" })
                href="/b/admin/variables?tab=all"
                hx-get="/b/admin/variables?tab=all"
                hx-target="#content"
                hx-push-url="true"
            { (icons::file_text()) " All Variables" }
        }

        div #variables-content {
            @if active_tab == "all" {
                (config_all_tab(ctx).await)
            } @else {
                (config_by_block_tab(ctx).await)
            }
        }

        // Create variable modal
        (components::modal("create-var", "Add Variable", html! {
            form hx-post="/b/admin/variables" hx-target="#variables-content" {
                div .form-group {
                    label .form-label .required for="var-key" { "Key" }
                    input .form-input type="text" #var-key name="key" placeholder="e.g. MY_SETTING" required;
                }
                div .form-group {
                    label .form-label for="var-value" { "Value" }
                    input .form-input type="text" #var-value name="value" placeholder="Value";
                }
                div .form-group {
                    label .form-label for="var-desc" { "Description" }
                    input .form-input type="text" #var-desc name="description" placeholder="Optional description";
                }
                div .form-group {
                    label .form-checkbox {
                        input type="checkbox" name="sensitive" value="1";
                        " Sensitive (mask value in UI)"
                    }
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('create-var')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }))

        // Edit variable modal (content loaded dynamically via htmx)
        div .modal-overlay #edit-var-modal-overlay hidden
            onclick="if(event.target===this)closeModal('edit-var-modal-overlay')"
        {
            div .modal {
                div #edit-var-modal {}
            }
        }
    };

    admin_page(
        "Config",
        &config,
        "/b/admin/variables",
        user.as_ref(),
        content,
        msg,
    )
}

/// "All Variables" tab — flat table of all config variables from the DB.
async fn config_all_tab(ctx: &dyn Context) -> Markup {
    let opts = ListOptions {
        limit: 200,
        ..Default::default()
    };
    let settings = db::list(ctx, "suppers_ai__admin__variables", &opts).await;

    html! {
        @match &settings {
            Ok(list) => {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Key" }
                                th { "Value" }
                                th { "Description" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            @for record in &list.records {
                                @let key = record.str_field("key");
                                @let value = record.str_field("value");
                                @let description = record.str_field("description");
                                @let warning = record.str_field("warning");
                                @let sensitive = record.i64_field("sensitive") != 0;
                                tr #{"var-row-" (key)} {
                                    td .font-medium { (key) }
                                    td .text-sm {
                                        @if sensitive {
                                            code { "********" }
                                        } @else {
                                            code { (value) }
                                        }
                                    }
                                    td .text-sm {
                                        @if !description.is_empty() {
                                            span .text-muted { (description) }
                                        }
                                        @if !warning.is_empty() {
                                            div style="color:#92400e;font-size:0.75rem;margin-top:0.25rem" {
                                                "⚠ " (warning)
                                            }
                                        }
                                    }
                                    td {
                                        button .btn .btn-sm .btn-ghost
                                            hx-get={"/b/admin/variables/" (key) "/edit"}
                                            hx-target="#edit-var-modal"
                                            hx-swap="innerHTML"
                                            title="Edit"
                                        { (icons::edit()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                div .login-error { "Failed to load variables: " (e.message) }
            }
        }
    }
}

/// "By Block" tab — groups config variables by owning block with WRAP access info.
async fn config_by_block_tab(ctx: &dyn Context) -> Markup {
    let blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();
    let shared_vars = crate::config_vars::shared_config_vars();

    // Load all variables from DB
    let all_vars = db::list_all(ctx, "suppers_ai__admin__variables", vec![])
        .await
        .unwrap_or_default();

    // Build a map of key -> (value, sensitive)
    let var_map: std::collections::HashMap<String, (String, bool)> = all_vars
        .iter()
        .map(|r| {
            let key = r.str_field("key").to_string();
            let value = r.str_field("value").to_string();
            let sensitive = r.i64_field("sensitive") != 0;
            (key, (value, sensitive))
        })
        .collect();

    // Collect blocks that have config_keys
    let blocks_with_config: Vec<_> = blocks.iter()
        .filter(|b| !b.config_keys.is_empty())
        .collect();

    // Collect all known keys (block-declared + shared) to detect unowned DB vars
    let mut known_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    for block in &blocks {
        for ck in &block.config_keys {
            known_keys.insert(ck.key.clone());
        }
    }
    for sv in &shared_vars {
        known_keys.insert(sv.key.clone());
    }

    html! {
        // Shared variables section
        @if !shared_vars.is_empty() {
            div .card .mt-4 {
                div .card-header {
                    h3 .card-title {
                        span .badge .badge-warning .mr-2 { "shared" }
                        " Shared Platform Config"
                    }
                    p .text-muted style="font-size:12px" {
                        "Any block can read. Only admin can write."
                    }
                }
                div .card-body {
                    table .table {
                        thead {
                            tr {
                                th { "Key" }
                                th { "Value" }
                                th { "Default" }
                                th { "Description" }
                                th style="width:50px" {}
                            }
                        }
                        tbody {
                            @for var in &shared_vars {
                                @let (db_value, sensitive) = var_map.get(&var.key)
                                    .cloned()
                                    .unwrap_or_else(|| (String::new(), var.is_sensitive()));
                                @let has_value = !db_value.is_empty();
                                tr {
                                    td .font-medium style="font-size:13px" {
                                        code { (var.key) }
                                        @if !var.name.is_empty() {
                                            br;
                                            span .text-muted style="font-size:12px" { (var.name) }
                                        }
                                    }
                                    td style="font-size:13px" {
                                        @if sensitive {
                                            @if has_value {
                                                code { "********" }
                                            } @else {
                                                span .text-muted { "(not set)" }
                                            }
                                        } @else {
                                            @if has_value {
                                                code { (db_value) }
                                            } @else {
                                                span .text-muted { "(not set)" }
                                            }
                                        }
                                    }
                                    td style="font-size:12px" {
                                        @if !var.default.is_empty() {
                                            code .text-muted { (var.default) }
                                        } @else if var.auto_generate {
                                            span .badge .badge-info style="font-size:11px" { "auto-generated" }
                                        }
                                    }
                                    td style="font-size:12px" { (var.description) }
                                    td {
                                        button .btn .btn-sm .btn-ghost
                                            hx-get={"/b/admin/variables/" (var.key) "/edit"}
                                            hx-target="#edit-var-modal"
                                            hx-swap="innerHTML"
                                            title="Edit"
                                        { (icons::edit()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Per-block sections
        @for block in &blocks_with_config {
            div .card .mt-4 {
                div .card-header {
                    h3 .card-title {
                        span .badge .badge-info .mr-2 { (block.name) }
                        " Configuration"
                    }
                    // Show WRAP access info for this block's config
                    p .text-muted style="font-size:12px" {
                        "Owner: " code { (block.name) }
                        " — Admin can read/write all. "
                        @for grant_block in &blocks {
                            @for grant in &grant_block.grants {
                                @for ck in &block.config_keys {
                                    @if grant.resource == ck.key || grant.resource == format!("{}*", ck.key) {
                                        @if grant.grantee != block.name {
                                            span .badge .badge-secondary .mr-1 style="font-size:11px" {
                                                (grant.grantee) ": "
                                                @if grant.write { "read+write" } @else { "read" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div .card-body {
                    table .table {
                        thead {
                            tr {
                                th { "Key" }
                                th { "Value" }
                                th { "Default" }
                                th { "Description" }
                                th style="width:50px" {}
                            }
                        }
                        tbody {
                            @for var in &block.config_keys {
                                @let (db_value, sensitive) = var_map.get(&var.key)
                                    .cloned()
                                    .unwrap_or_else(|| (String::new(), var.is_sensitive()));
                                @let has_value = !db_value.is_empty();
                                tr {
                                    td .font-medium style="font-size:13px" {
                                        code { (var.key) }
                                        @if !var.name.is_empty() {
                                            br;
                                            span .text-muted style="font-size:12px" { (var.name) }
                                        }
                                    }
                                    td style="font-size:13px" {
                                        @if sensitive {
                                            @if has_value {
                                                code { "********" }
                                            } @else {
                                                span .text-muted { "(not set)" }
                                            }
                                        } @else {
                                            @if has_value {
                                                code { (db_value) }
                                            } @else {
                                                span .text-muted { "(not set)" }
                                            }
                                        }
                                    }
                                    td style="font-size:12px" {
                                        @if !var.default.is_empty() {
                                            code .text-muted { (var.default) }
                                        } @else if var.auto_generate {
                                            span .badge .badge-info style="font-size:11px" { "auto-generated" }
                                        }
                                    }
                                    td style="font-size:12px" {
                                        (var.description)
                                        @if !var.warning.is_empty() {
                                            div style="color:#92400e;font-size:11px;margin-top:2px" {
                                                "Warning: " (var.warning)
                                            }
                                        }
                                    }
                                    td {
                                        button .btn .btn-sm .btn-ghost
                                            hx-get={"/b/admin/variables/" (var.key) "/edit"}
                                            hx-target="#edit-var-modal"
                                            hx-swap="innerHTML"
                                            title="Edit"
                                        { (icons::edit()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Unowned variables section — keys in DB not declared by any block or shared
        @let unowned_vars: Vec<_> = all_vars.iter()
            .filter(|r| !known_keys.contains(r.str_field("key")))
            .collect();
        @if !unowned_vars.is_empty() {
            div .card .mt-4 {
                div .card-header {
                    h3 .card-title {
                        span .badge .badge-secondary .mr-2 { "unowned" }
                        " Unowned Variables"
                    }
                    p .text-muted style="font-size:12px" {
                        "Variables in the database not declared by any block. These may be legacy or manually created."
                    }
                }
                div .card-body {
                    table .table {
                        thead {
                            tr {
                                th { "Key" }
                                th { "Value" }
                                th { "Description" }
                                th style="width:50px" {}
                            }
                        }
                        tbody {
                            @for record in &unowned_vars {
                                @let key = record.str_field("key");
                                @let value = record.str_field("value");
                                @let description = record.str_field("description");
                                @let sensitive = record.i64_field("sensitive") != 0;
                                tr {
                                    td .font-medium style="font-size:13px" { code { (key) } }
                                    td style="font-size:13px" {
                                        @if sensitive { code { "********" } }
                                        @else { code { (value) } }
                                    }
                                    td style="font-size:12px" { (description) }
                                    td {
                                        button .btn .btn-sm .btn-ghost
                                            hx-get={"/b/admin/variables/" (key) "/edit"}
                                            hx-target="#edit-var-modal"
                                            hx-swap="innerHTML"
                                            title="Edit"
                                        { (icons::edit()) }
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

async fn roles_tab(ctx: &dyn Context) -> Markup {
    let opts = ListOptions {
        sort: vec![SortField {
            field: "name".into(),
            desc: false,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, "suppers_ai__admin__roles", &opts).await;

    html! {
        div .flex .items-center .justify-between .mb-4 {
            h3 .font-semibold { "Roles" }
            button .btn .btn-primary .btn-sm onclick="openModal('create-role')" {
                (icons::plus()) " Create Role"
            }
        }

        @match &result {
            Ok(list) => {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Name" }
                                th { "Description" }
                                th { "Type" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            @for record in &list.records {
                                @let name = record.str_field("name");
                                @let description = record.str_field("description");
                                @let is_system = record.bool_field("is_system");
                                tr {
                                    td .font-medium { (name) }
                                    td .text-muted .text-sm { (description) }
                                    td {
                                        @if is_system {
                                            span .badge .badge-info { "System" }
                                        } @else {
                                            span .badge .badge-primary { "Custom" }
                                        }
                                    }
                                    td {
                                        @if !is_system {
                                            button .btn .btn-sm .btn-danger
                                                hx-delete={"/b/admin/iam/roles/" (record.id)}
                                                hx-target="#iam-content"
                                                hx-confirm={"Delete role \"" (name) "\"?"}
                                            { (icons::trash()) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                div .login-error { "Failed to load roles: " (e.message) }
            }
        }

        // Create role modal
        (components::modal("create-role", "Create Role", html! {
            form hx-post="/b/admin/iam/roles" hx-target="#iam-content" {
                div .form-group {
                    label .form-label .required for="role-name" { "Name" }
                    input .form-input type="text" #role-name name="name" placeholder="e.g. editor" required;
                }
                div .form-group {
                    label .form-label for="role-desc" { "Description" }
                    input .form-input type="text" #role-desc name="description" placeholder="Optional description";
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('create-role')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }))
    }
}

async fn api_keys_tab(ctx: &dyn Context) -> Markup {
    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".into(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, "suppers_ai__auth__api_keys", &opts).await;

    html! {
        div .flex .items-center .justify-between .mb-4 {
            h3 .font-semibold { "API Keys" }
            button .btn .btn-primary .btn-sm onclick="openModal('create-api-key')" {
                (icons::plus()) " Create API Key"
            }
        }

        @match &result {
            Ok(list) => {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Prefix" }
                                th { "Name" }
                                th { "User" }
                                th { "Created" }
                                th { "Status" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            @if list.records.is_empty() {
                                tr {
                                    td colspan="6" .text-center .text-muted style="padding: 2rem;" { "No API keys" }
                                }
                            }
                            @for record in &list.records {
                                @let prefix = record.str_field("key_prefix");
                                @let name = record.str_field("name");
                                @let user_id = record.str_field("user_id");
                                @let created = record.str_field("created_at");
                                @let revoked = record.str_field("revoked_at");
                                tr {
                                    td { code { (prefix) "..." } }
                                    td { (name) }
                                    td .text-muted .text-sm { (user_id.get(..8).unwrap_or(user_id)) }
                                    td .text-muted .text-sm { (created.get(..10).unwrap_or(created)) }
                                    td {
                                        @if revoked.is_empty() {
                                            (components::status_badge("active"))
                                        } @else {
                                            (components::status_badge("disabled"))
                                        }
                                    }
                                    td {
                                        @if revoked.is_empty() {
                                            button .btn .btn-sm .btn-secondary
                                                hx-post={"/b/auth/api/api-keys/" (record.id) "/revoke"}
                                                hx-target="#users-tab-content"
                                                hx-confirm="Revoke this API key?"
                                            { "Revoke" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                div .login-error { "Failed to load API keys: " (e.message) }
            }
        }

        // Create API key modal
        (components::modal("create-api-key", "Create API Key", html! {
            form hx-post="/b/auth/api/api-keys" hx-target="#users-tab-content" {
                div .form-group {
                    label .form-label for="key-name" { "Name" }
                    input .form-input type="text" #key-name name="name" placeholder="e.g. CI/CD key" required;
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('create-api-key')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Create" }
                }
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

pub async fn network_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "outbound" => "outbound",
        "rules" => "rules",
        _ => "inbound",
    };

    let content = html! {
        (components::page_header(
            "Network",
            Some("Inbound and outbound request monitoring"),
            Some(html! {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/network?tab=" (active_tab)}
                    hx-target="#content"
                { (icons::refresh_cw()) " Refresh" }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "inbound" { "active" } else { "" })
                href="/b/admin/network"
                hx-get="/b/admin/network"
                hx-target="#content"
                hx-push-url="true"
            { (icons::arrow_down_left()) " Inbound" }
            a .tab .(if active_tab == "outbound" { "active" } else { "" })
                href="/b/admin/network?tab=outbound"
                hx-get="/b/admin/network?tab=outbound"
                hx-target="#content"
                hx-push-url="true"
            { (icons::arrow_up_right()) " Outbound" }
        }

        @if active_tab == "rules" {
            div .card .mt-4 style="background:#f0f9ff;border-color:#bae6fd" {
                p style="padding:12px;margin:0;font-size:13px" {
                    (icons::info()) " Network permissions have moved to the "
                    a href="/b/admin/permissions?tab=network" { "Permissions" }
                    " page."
                }
            }
        }

        div #network-tab-content {
            @if active_tab == "inbound" {
                (network_inbound_tab(ctx, msg).await)
            } @else if active_tab == "outbound" {
                (network_outbound_tab(ctx, msg).await)
            } @else {
                // rules tab still renders content but with banner above
                (network_rules_tab(ctx, msg).await)
            }
        }
    };

    admin_page(
        "Network",
        &config,
        "/b/admin/network",
        user.as_ref(),
        content,
        msg,
    )
}

async fn network_inbound_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    let search = msg.query("search").to_string();

    let (where_clause, args) = if search.is_empty() {
        (String::new(), vec![])
    } else {
        (
            " WHERE path LIKE ?1".to_string(),
            vec![serde_json::json!(format!("%{search}%"))],
        )
    };

    let summary = db::query_raw(
        ctx,
        &format!(
            "SELECT method, path, COUNT(*) as cnt, \
             CAST(AVG(duration_ms) AS INTEGER) as avg_ms, \
             SUM(CASE WHEN CAST(status_code AS INTEGER) >= 400 THEN 1 ELSE 0 END) as errors, \
             MAX(created_at) as last_seen \
             FROM suppers_ai__admin__request_logs{where_clause} \
             GROUP BY method, path ORDER BY cnt DESC LIMIT 50"
        ),
        &args,
    )
    .await
    .unwrap_or_default();

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by path...", "/b/admin/network", "#content", &search))
        }

        style { (maud::PreEscaped("
            .expand-row { cursor: pointer; }
            .expand-row:hover { background: var(--bg-secondary, #f8fafc); }
            .detail-rows td { background: var(--bg-secondary, #f8fafc); font-size: 12px; }
            .detail-rows[hidden] { display: none; }
        ")) }
        script { (maud::PreEscaped("
            function toggleDetail(rowId, url) {
                var detail = document.getElementById(rowId);
                var row = detail.closest('tr');
                if (!row.hidden) { row.hidden = true; return; }
                row.hidden = false;
                if (!detail.innerHTML) htmx.ajax('GET', url, {target: '#' + rowId, swap: 'innerHTML'});
            }
        ")) }

        div .table-container {
            table .table {
                thead {
                    tr {
                        th style="width:30px" { "" }
                        th { "Method" }
                        th { "Path" }
                        th { "Requests" }
                        th { "Avg Duration" }
                        th { "Errors" }
                        th { "Last Seen" }
                    }
                }
                tbody {
                    @if summary.is_empty() {
                        tr {
                            td colspan="7" .text-center .text-muted style="padding: 2rem;" { "No inbound requests yet" }
                        }
                    }
                    @for row in &summary {
                        @let method = row.data.get("method").and_then(|v| v.as_str()).unwrap_or("");
                        @let path = row.data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        @let cnt = row.data.get("cnt").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let avg_ms = row.data.get("avg_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let errors = row.data.get("errors").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let last_seen = row.data.get("last_seen").and_then(|v| v.as_str()).unwrap_or("");
                        @let row_id = format!("inbound-{}-{}", method, path.replace('/', "_"));
                        @let detail_url = format!("/b/admin/network/detail/inbound?method={method}&path={path}");
                        tr .expand-row
                            onclick={"toggleDetail('" (row_id) "','" (detail_url) "')"}
                        {
                            td .text-muted { (icons::chevron_right()) }
                            td .text-sm .font-medium { (method.to_uppercase()) }
                            td .text-sm { (path) }
                            td .text-sm {
                                span .badge .badge-info { (cnt) }
                            }
                            td .text-muted .text-sm { (avg_ms) "ms" }
                            td .text-sm {
                                @if errors > 0 {
                                    span .badge .badge-danger { (errors) }
                                } @else {
                                    span .text-muted { "0" }
                                }
                            }
                            td .text-muted .text-sm { (last_seen.get(..19).unwrap_or(last_seen)) }
                        }
                        tr .detail-rows hidden {
                            td colspan="7" style="padding:0" {
                                div id=(row_id) {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Htmx fragment: individual requests for a given inbound path.
pub async fn network_inbound_detail(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let method = msg.query("method").to_string();
    let path = msg.query("path").to_string();
    let offset: i64 = msg.query("offset").parse().unwrap_or(0);
    let limit: i64 = 20;

    let rows = db::query_raw(
        ctx,
        "SELECT status_code, duration_ms, client_ip, user_id, created_at \
         FROM suppers_ai__admin__request_logs WHERE method = ?1 AND path = ?2 \
         ORDER BY created_at DESC LIMIT ?3 OFFSET ?4",
        &[
            serde_json::json!(method),
            serde_json::json!(path),
            serde_json::json!(limit + 1), // fetch one extra to detect "has more"
            serde_json::json!(offset),
        ],
    )
    .await
    .unwrap_or_default();

    let has_more = rows.len() as i64 > limit;
    let display_rows = if has_more {
        &rows[..limit as usize]
    } else {
        &rows
    };

    let markup = html! {
        table .table style="margin:0" {
            thead {
                tr {
                    th { "Status" }
                    th { "Duration" }
                    th { "IP" }
                    th { "User" }
                    th { "Time" }
                }
            }
            tbody {
                @for record in display_rows {
                    @let status_code = record.data.get("status_code").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
                    @let duration = record.data.get("duration_ms").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
                    @let client_ip = record.data.get("client_ip").and_then(|v| v.as_str()).unwrap_or("");
                    @let user_id = record.data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                    tr {
                        td {
                            span .badge .(if status_code >= 500 { "badge-danger" } else if status_code >= 400 { "badge-warning" } else { "badge-success" }) {
                                (status_code)
                            }
                        }
                        td .text-muted { (duration) "ms" }
                        td .text-muted { (client_ip) }
                        td .text-muted {
                            @if !user_id.is_empty() {
                                (user_id.get(..8).unwrap_or(user_id))
                            }
                        }
                        td .text-muted { (created.get(..19).unwrap_or(created)) }
                    }
                }
            }
        }
        @if has_more {
            @let next_offset = offset + limit;
            div style="text-align:center;padding:8px" {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/network/detail/inbound?method=" (method) "&path=" (path) "&offset=" (next_offset)}
                    hx-target="closest div"
                    hx-swap="outerHTML"
                { "Load more" }
            }
        }
    };
    crate::ui::html_response(msg, markup)
}

async fn network_outbound_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    let search = msg.query("search").to_string();

    let (where_clause, args) = if search.is_empty() {
        (String::new(), vec![])
    } else {
        (
            " WHERE url LIKE ?1".to_string(),
            vec![serde_json::json!(format!("%{search}%"))],
        )
    };

    let summary = db::query_raw(
        ctx,
        &format!(
            "SELECT method, url, source_block, COUNT(*) as cnt, \
             CAST(AVG(duration_ms) AS INTEGER) as avg_ms, \
             SUM(CASE WHEN error_message != '' THEN 1 ELSE 0 END) as errors, \
             MAX(created_at) as last_seen \
             FROM suppers_ai__admin__network_request_logs{where_clause} \
             GROUP BY method, url ORDER BY cnt DESC LIMIT 50"
        ),
        &args,
    )
    .await
    .unwrap_or_default();

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by URL...", "/b/admin/network?tab=outbound", "#content", &search))
        }

        style { (maud::PreEscaped("
            .expand-row { cursor: pointer; }
            .expand-row:hover { background: var(--bg-secondary, #f8fafc); }
            .detail-rows td { background: var(--bg-secondary, #f8fafc); font-size: 12px; }
            .detail-rows[hidden] { display: none; }
        ")) }

        div .table-container {
            table .table {
                thead {
                    tr {
                        th style="width:30px" { "" }
                        th { "Method" }
                        th { "URL" }
                        th { "Block" }
                        th { "Requests" }
                        th { "Avg Duration" }
                        th { "Errors" }
                        th { "Last Seen" }
                    }
                }
                tbody {
                    @if summary.is_empty() {
                        tr {
                            td colspan="8" .text-center .text-muted style="padding: 2rem;" { "No outbound requests yet" }
                        }
                    }
                    @for (i, row) in summary.iter().enumerate() {
                        @let method = row.data.get("method").and_then(|v| v.as_str()).unwrap_or("");
                        @let url = row.data.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        @let source_block = row.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("");
                        @let cnt = row.data.get("cnt").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let avg_ms = row.data.get("avg_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let errors = row.data.get("errors").and_then(|v| v.as_i64()).unwrap_or(0);
                        @let last_seen = row.data.get("last_seen").and_then(|v| v.as_str()).unwrap_or("");
                        @let row_id = format!("outbound-{i}");
                        @let encoded_url = url.replace('&', "%26");
                        @let detail_url = format!("/b/admin/network/detail/outbound?method={method}&url={encoded_url}");
                        tr .expand-row
                            onclick={"toggleDetail('" (row_id) "','" (detail_url) "')"}
                        {
                            td .text-muted { (icons::chevron_right()) }
                            td .text-sm .font-medium { (method.to_uppercase()) }
                            td .text-sm style="max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title=(url) { (url) }
                            td .text-sm {
                                @if !source_block.is_empty() {
                                    span .badge .badge-info { (source_block) }
                                }
                            }
                            td .text-sm {
                                span .badge .badge-info { (cnt) }
                            }
                            td .text-muted .text-sm { (avg_ms) "ms" }
                            td .text-sm {
                                @if errors > 0 {
                                    span .badge .badge-danger { (errors) }
                                } @else {
                                    span .text-muted { "0" }
                                }
                            }
                            td .text-muted .text-sm { (last_seen.get(..19).unwrap_or(last_seen)) }
                        }
                        tr .detail-rows hidden {
                            td colspan="8" style="padding:0" {
                                div id=(row_id) {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Htmx fragment: individual requests for a given outbound URL.
pub async fn network_outbound_detail(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let method = msg.query("method").to_string();
    let url = msg.query("url").to_string();
    let offset: i64 = msg.query("offset").parse().unwrap_or(0);
    let limit: i64 = 20;

    let rows = db::query_raw(
        ctx,
        "SELECT status_code, duration_ms, source_block, error_message, created_at \
         FROM suppers_ai__admin__network_request_logs WHERE method = ?1 AND url = ?2 \
         ORDER BY created_at DESC LIMIT ?3 OFFSET ?4",
        &[
            serde_json::json!(method),
            serde_json::json!(url),
            serde_json::json!(limit + 1),
            serde_json::json!(offset),
        ],
    )
    .await
    .unwrap_or_default();

    let has_more = rows.len() as i64 > limit;
    let display_rows = if has_more {
        &rows[..limit as usize]
    } else {
        &rows
    };
    let encoded_url = url.replace('&', "%26");

    let markup = html! {
        table .table style="margin:0" {
            thead {
                tr {
                    th { "Status" }
                    th { "Block" }
                    th { "Duration" }
                    th { "Error" }
                    th { "Time" }
                }
            }
            tbody {
                @for record in display_rows {
                    @let status_code = record.data.get("status_code").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
                    @let duration = record.data.get("duration_ms").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
                    @let source_block = record.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("");
                    @let error_msg = record.data.get("error_message").and_then(|v| v.as_str()).unwrap_or("");
                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                    tr {
                        td {
                            @if !error_msg.is_empty() {
                                span .badge .badge-danger {
                                    @if status_code > 0 { (status_code) } @else { "ERR" }
                                }
                            } @else if status_code >= 400 {
                                span .badge .badge-warning { (status_code) }
                            } @else if status_code > 0 {
                                span .badge .badge-success { (status_code) }
                            } @else {
                                span .badge .badge-muted { "—" }
                            }
                        }
                        td {
                            @if !source_block.is_empty() {
                                span .badge .badge-info { (source_block) }
                            }
                        }
                        td .text-muted { (duration) "ms" }
                        td .text-muted style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title=(error_msg) {
                            (error_msg)
                        }
                        td .text-muted { (created.get(..19).unwrap_or(created)) }
                    }
                }
            }
        }
        @if has_more {
            @let next_offset = offset + limit;
            div style="text-align:center;padding:8px" {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/network/detail/outbound?method=" (method) "&url=" (encoded_url) "&offset=" (next_offset)}
                    hx-target="closest div"
                    hx-swap="outerHTML"
                { "Load more" }
            }
        }
    };
    crate::ui::html_response(msg, markup)
}

async fn network_rules_tab(ctx: &dyn Context, _msg: &mut Message) -> Markup {
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    let rules = db::list_all(ctx, "suppers_ai__admin__network_rules", vec![])
        .await
        .unwrap_or_default();

    html! {
        div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px" {
            p .text-muted style="margin:0" {
                "Control which URLs each block can reach. "
                strong { "Deny" } " rules block matching URLs. "
                strong { "Allow" } " rules restrict a block to only matching URLs."
            }
            button .btn .btn-primary .btn-sm
                onclick="openModal('add-rule-modal')"
            { (icons::plus()) " Add Rule" }
        }

        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Block" }
                        th { "Type" }
                        th { "URL Pattern" }
                        th { "Priority" }
                        th style="width:80px" { "" }
                    }
                }
                tbody {
                    @if rules.is_empty() {
                        tr {
                            td colspan="5" .text-center .text-muted style="padding: 2rem;" {
                                "No network rules configured. All blocks can reach any URL by default."
                            }
                        }
                    }
                    @for rule in &rules {
                        @let id = &rule.id;
                        @let rule_type = rule.data.get("rule_type").and_then(|v| v.as_str()).unwrap_or("");
                        @let pattern = rule.data.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                        @let block_name = rule.data.get("block_name").and_then(|v| v.as_str()).unwrap_or("");
                        @let priority = rule.data.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                        tr {
                            td {
                                @if block_name.is_empty() || block_name == "*" {
                                    span .badge .badge-warning { "All blocks" }
                                } @else {
                                    code { (block_name) }
                                }
                            }
                            td {
                                @if rule_type == "block" {
                                    span .badge .badge-danger { "Deny" }
                                } @else {
                                    span .badge .badge-success { "Allow" }
                                }
                            }
                            td .text-sm .font-medium style="font-family:monospace" { (pattern) }
                            td .text-muted .text-sm { (priority) }
                            td {
                                button .btn .btn-danger .btn-sm
                                    hx-delete={"/b/admin/network/rules/" (id)}
                                    hx-target="#content"
                                    hx-confirm="Delete this rule?"
                                { (icons::trash()) }
                            }
                        }
                    }
                }
            }
        }

        // Add rule modal
        (components::modal("add-rule-modal", "Add Network Rule", html! {
            form hx-post="/b/admin/network/rules" hx-target="#content" {
                div .form-group {
                    label .form-label for="block_name" { "Which block?" }
                    select .form-input name="block_name" {
                        option value="" { "All blocks" }
                        @for name in &block_names {
                            option value=(name) { (name) }
                        }
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "The block this rule applies to. Leave as \"All blocks\" for a global rule."
                    }
                }
                div .form-group {
                    label .form-label for="rule_type" { "Allow or Deny?" }
                    select .form-input name="rule_type" {
                        option value="allow" { "Allow — permit this block to reach matching URLs" }
                        option value="block" { "Deny — block this block from reaching matching URLs" }
                    }
                }
                div .form-group {
                    label .form-label for="pattern" { "URL pattern" }
                    input .form-input type="text" name="pattern"
                        placeholder="e.g. https://api.stripe.com/*" required;
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "Use " code { "*" } " as wildcard. Examples: "
                        code { "https://api.stripe.com/*" } ", "
                        code { "*.internal.corp*" }
                    }
                }
                div .form-group {
                    label .form-label for="priority" { "Priority" }
                    input .form-input type="number" name="priority" value="0";
                    p .text-muted style="font-size:12px;margin-top:4px" { "Higher priority rules are evaluated first." }
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('add-rule-modal')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Add Rule" }
                }
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

pub async fn storage_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "rules" => "rules",
        _ => "logs",
    };

    let content = html! {
        (components::page_header(
            "Storage",
            Some("Per-block storage isolation and access rules"),
            Some(html! {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/storage?tab=" (active_tab)}
                    hx-target="#content"
                { (icons::refresh_cw()) " Refresh" }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "logs" { "active" } else { "" })
                href="/b/admin/storage"
                hx-get="/b/admin/storage"
                hx-target="#content"
                hx-push-url="true"
            { (icons::eye()) " Access Logs" }
        }

        @if active_tab == "rules" {
            div .card .mt-4 style="background:#f0f9ff;border-color:#bae6fd" {
                p style="padding:12px;margin:0;font-size:13px" {
                    (icons::info()) " Storage permissions have moved to the "
                    a href="/b/admin/permissions?tab=storage" { "Permissions" }
                    " page."
                }
            }
        }

        div #storage-tab-content {
            @if active_tab == "rules" {
                // rules tab still renders content but with banner above
                (storage_rules_tab(ctx, msg).await)
            } @else {
                (storage_logs_tab(ctx, msg).await)
            }
        }
    };

    admin_page(
        "Storage",
        &config,
        "/b/admin/storage",
        user.as_ref(),
        content,
        msg,
    )
}

async fn storage_logs_tab(ctx: &dyn Context, _msg: &mut Message) -> Markup {
    let logs = db::query_raw(
        ctx,
        "SELECT source_block, operation, path, status, created_at FROM suppers_ai__admin__storage_access_logs ORDER BY created_at DESC LIMIT 100",
        &[],
    )
    .await
    .unwrap_or_default();

    html! {
        p .text-muted style="margin-bottom:16px" {
            "Recent storage access by blocks. Each block is isolated to "
            code { "/storage/{block-name}/" }
            "."
        }

        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Block" }
                        th { "Operation" }
                        th { "Path" }
                        th { "Status" }
                        th { "Time" }
                    }
                }
                tbody {
                    @if logs.is_empty() {
                        tr {
                            td colspan="5" .text-center .text-muted style="padding: 2rem;" {
                                "No storage access logs yet."
                            }
                        }
                    }
                    @for log in &logs {
                        @let source = log.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("");
                        @let op = log.data.get("operation").and_then(|v| v.as_str()).unwrap_or("");
                        @let path = log.data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        @let status = log.data.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        @let created = log.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                        tr {
                            td {
                                @if !source.is_empty() {
                                    span .badge .badge-info { (source) }
                                }
                            }
                            td .text-sm style="font-family:monospace" { (op) }
                            td .text-sm style="font-family:monospace" { (path) }
                            td .text-sm {
                                @if status.starts_with("BLOCKED") {
                                    span .badge .badge-danger { (status) }
                                } @else if status.starts_with("ERROR") {
                                    span .badge .badge-warning { (status) }
                                } @else {
                                    span .text-muted { (status) }
                                }
                            }
                            td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                        }
                    }
                }
            }
        }
    }
}

async fn storage_rules_tab(ctx: &dyn Context, _msg: &mut Message) -> Markup {
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    let rules = db::list_all(ctx, "suppers_ai__admin__storage_rules", vec![])
        .await
        .unwrap_or_default();

    // Collect block names that use storage (service blocks don't need their own namespace)
    let registered = ctx.registered_blocks();
    let storage_blocks: Vec<&str> = registered
        .iter()
        .filter(|b| b.category != wafer_run::BlockCategory::Service && !b.name.is_empty())
        .map(|b| b.name.as_str())
        .collect();

    html! {
        div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px" {
            p .text-muted style="margin:0" {
                "Each block is isolated to its own storage namespace. "
                "Add rules below to grant cross-block access."
            }
            button .btn .btn-primary .btn-sm
                onclick="openModal('add-storage-rule-modal')"
            { (icons::plus()) " Add Rule" }
        }

        // Default isolation rules (built-in)
        h3 style="font-size:14px;margin-bottom:8px;color:#6b7280" { "Default (built-in)" }
        div .table-container style="margin-bottom:24px" {
            table .table {
                thead {
                    tr {
                        th { "Type" }
                        th { "Block" }
                        th { "Storage Path" }
                        th { "Access" }
                    }
                }
                tbody {
                    @for block_name in &storage_blocks {
                        tr {
                            td { span .badge .badge-success { "Allow" } }
                            td .text-sm { span .badge .badge-info { (block_name) } }
                            td .text-sm style="font-family:monospace" { (block_name) "/*" }
                            td .text-sm { span .badge .badge-secondary { "Read/Write" } }
                        }
                    }
                }
            }
        }

        // Custom rules
        h3 style="font-size:14px;margin-bottom:8px;color:#6b7280" { "Custom cross-block rules" }
        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Block" }
                        th { "Type" }
                        th { "Storage Path" }
                        th { "Access" }
                        th { "Priority" }
                        th style="width:80px" { "" }
                    }
                }
                tbody {
                    @if rules.is_empty() {
                        tr {
                            td colspan="6" .text-center .text-muted style="padding: 2rem;" {
                                "No cross-block rules configured. Blocks can only access their own files by default."
                            }
                        }
                    }
                    @for rule in &rules {
                        @let id = &rule.id;
                        @let rule_type = rule.data.get("rule_type").and_then(|v| v.as_str()).unwrap_or("");
                        @let source = rule.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("*");
                        @let target = rule.data.get("target_path").and_then(|v| v.as_str()).unwrap_or("");
                        @let access = rule.data.get("access").and_then(|v| v.as_str()).unwrap_or("readwrite");
                        @let priority = rule.data.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                        tr {
                            td {
                                @if source == "*" || source.is_empty() {
                                    span .badge .badge-warning { "All blocks" }
                                } @else {
                                    code { (source) }
                                }
                            }
                            td {
                                @if rule_type == "block" {
                                    span .badge .badge-danger { "Deny" }
                                } @else {
                                    span .badge .badge-success { "Allow" }
                                }
                            }
                            td .text-sm style="font-family:monospace" { (target) }
                            td .text-sm {
                                @if access == "read" {
                                    span .badge .badge-info { "Read" }
                                } @else if access == "write" {
                                    span .badge .badge-warning { "Write" }
                                } @else {
                                    span .badge .badge-secondary { "Read/Write" }
                                }
                            }
                            td .text-muted .text-sm { (priority) }
                            td {
                                button .btn .btn-danger .btn-sm
                                    hx-delete={"/b/admin/storage/rules/" (id)}
                                    hx-target="#content"
                                    hx-confirm="Delete this rule?"
                                { (icons::trash()) }
                            }
                        }
                    }
                }
            }
        }

        // Add rule modal
        (components::modal("add-storage-rule-modal", "Add Storage Rule", html! {
            form hx-post="/b/admin/storage/rules" hx-target="#content" {
                div .form-group {
                    label .form-label for="source_block" { "Which block?" }
                    select .form-input name="source_block" {
                        option value="*" { "All blocks" }
                        @for name in &block_names {
                            option value=(name) { (name) }
                        }
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "The block this rule applies to."
                    }
                }
                div .form-group {
                    label .form-label for="rule_type" { "Allow or Deny?" }
                    select .form-input name="rule_type" {
                        option value="allow" { "Allow — let this block access the storage path" }
                        option value="block" { "Deny — block this block from the storage path" }
                    }
                }
                div .form-group {
                    label .form-label for="target_path" { "Which storage path?" }
                    input .form-input type="text" name="target_path"
                        placeholder="e.g. wafer-run/web/public/*" required;
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "Each block's files are stored under its name. Use " code { "*" } " as wildcard. "
                        "Examples: " code { "wafer-run/web/*" } ", " code { "suppers-ai/files/uploads/*" }
                    }
                }
                div .form-group {
                    label .form-label for="access" { "Read, write, or both?" }
                    select .form-input #access name="access" {
                        option value="readwrite" { "Read & Write" }
                        option value="read" { "Read only" }
                        option value="write" { "Write only" }
                    }
                }
                div .form-group {
                    label .form-label for="priority" { "Priority" }
                    input .form-input type="number" #priority name="priority" value="0";
                    p .text-muted style="font-size:12px;margin-top:4px" { "Higher priority rules are evaluated first" }
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('add-storage-rule-modal')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Add Rule" }
                }
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

pub async fn logs_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "audit" => "audit",
        _ => "system",
    };

    let content = html! {
        (components::page_header(
            "Logs",
            Some("System telemetry and admin audit trail"),
            Some(html! {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/logs?tab=" (active_tab)}
                    hx-target="#content"
                { (icons::refresh_cw()) " Refresh" }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "system" { "active" } else { "" })
                href="/b/admin/logs"
                hx-get="/b/admin/logs"
                hx-target="#content"
                hx-push-url="true"
            { (icons::server()) " System Logs" }
            a .tab .(if active_tab == "audit" { "active" } else { "" })
                href="/b/admin/logs?tab=audit"
                hx-get="/b/admin/logs?tab=audit"
                hx-target="#content"
                hx-push-url="true"
            { (icons::file_text()) " Audit Logs" }
        }

        div #logs-tab-content {
            @if active_tab == "system" {
                (system_logs_tab(ctx, msg).await)
            } @else {
                (audit_logs_tab(ctx, msg).await)
            }
        }
    };

    admin_page(
        "Logs",
        &config,
        "/b/admin/logs",
        user.as_ref(),
        content,
        msg,
    )
}

async fn system_logs_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    let (page, page_size, _) = msg.pagination_params(50);
    let search = msg.query("search").to_string();

    let mut filters = Vec::new();
    if !search.is_empty() {
        filters.push(Filter {
            field: "path".into(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{search}%")),
        });
    }

    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        "suppers_ai__admin__request_logs",
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by path...", "/b/admin/logs", "#content", &search))
        }

        @match &result {
            Ok(list) => {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Status" }
                                th { "Method" }
                                th { "Path" }
                                th { "Duration" }
                                th { "User" }
                                th { "Time" }
                            }
                        }
                        tbody {
                            @if list.records.is_empty() {
                                tr {
                                    td colspan="6" .text-center .text-muted style="padding: 2rem;" { "No request logs yet" }
                                }
                            }
                            @for record in &list.records {
                                @let status = record.str_field("status");
                                @let method = record.str_field("method");
                                @let path = record.str_field("path");
                                @let duration = record.i64_field("duration_ms");
                                @let user_id = record.str_field("user_id");
                                @let created = record.str_field("created_at");
                                @let status_code = record.i64_field("status_code");
                                tr {
                                    td {
                                        span .badge .(if status == "ERROR" { "badge-danger" } else if status_code >= 400 { "badge-warning" } else { "badge-success" }) {
                                            (status_code)
                                        }
                                    }
                                    td .text-sm .font-medium { (method.to_uppercase()) }
                                    td .text-sm { (path) }
                                    td .text-muted .text-sm { (duration) "ms" }
                                    td .text-muted .text-sm {
                                        @if !user_id.is_empty() {
                                            (user_id.get(..8).unwrap_or(user_id))
                                        }
                                    }
                                    td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                                }
                            }
                        }
                    }
                }

                @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                (components::pagination(list.page as u32, total_pages, "/b/admin/logs", "#content"))
            }
            Err(e) => {
                div .login-error { "Failed to load request logs: " (e.message) }
            }
        }
    }
}

async fn audit_logs_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    let (page, page_size, _) = msg.pagination_params(50);
    let search = msg.query("search").to_string();

    let mut filters = Vec::new();
    if !search.is_empty() {
        filters.push(Filter {
            field: "resource".into(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{search}%")),
        });
    }

    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        "suppers_ai__admin__audit_logs",
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by resource...", "/b/admin/logs?tab=audit", "#content", &search))
        }

        @match &result {
            Ok(list) => {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Action" }
                                th { "Resource" }
                                th { "User" }
                                th { "IP" }
                                th { "Time" }
                            }
                        }
                        tbody {
                            @if list.records.is_empty() {
                                tr {
                                    td colspan="5" .text-center .text-muted style="padding: 2rem;" { "No audit logs yet" }
                                }
                            }
                            @for record in &list.records {
                                @let action = record.str_field("action");
                                @let resource = record.str_field("resource");
                                @let user_id = record.str_field("user_id");
                                @let ip = record.str_field("ip_address");
                                @let created = record.str_field("created_at");
                                tr {
                                    td {
                                        span .badge .badge-info { (action) }
                                    }
                                    td .text-sm { (resource) }
                                    td .text-muted .text-sm { (user_id.get(..8).unwrap_or(user_id)) }
                                    td .text-muted .text-sm { (ip) }
                                    td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                                }
                            }
                        }
                    }
                }

                @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                (components::pagination(list.page as u32, total_pages, "/b/admin/logs?tab=audit", "#content"))
            }
            Err(e) => {
                div .login-error { "Failed to load audit logs: " (e.message) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

pub async fn blocks_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "services" => "services",
        "infrastructure" => "infrastructure",
        _ => "features",
    };

    let registered_blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();

    // Load block enabled/disabled state from block_settings table
    let block_settings_rows =
        db::list_all(ctx, "suppers_ai__admin__block_settings", vec![])
            .await
            .unwrap_or_default();

    let block_enabled: std::collections::HashMap<String, bool> = block_settings_rows
        .iter()
        .map(|r| {
            let name = r
                .data
                .get("block_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let enabled = r.data.get("enabled").and_then(|v| v.as_i64()).unwrap_or(1) != 0;
            (name, enabled)
        })
        .collect();

    // Build full block list: registered blocks + unloaded blocks from block_settings
    // Blocks in block_settings but not in the runtime get placeholder BlockInfo
    let mut all_blocks = registered_blocks.clone();
    for (name, enabled) in &block_enabled {
        if !all_blocks.iter().any(|b| &b.name == name) {
            let summary = if *enabled {
                "(enabled — restart to load)"
            } else {
                "(disabled — restart to load)"
            };
            all_blocks.push(
                wafer_run::BlockInfo::new(name, "0.0.1", "http.handler", summary)
                    .instance_mode(wafer_run::types::InstanceMode::Singleton)
                    .category(wafer_run::BlockCategory::Feature)
                    .can_disable(true)
                    .default_enabled(false),
            );
        }
    }

    let content = html! {
        (components::page_header("Blocks", Some("Registered WAFER blocks"),
            Some(html! {
                a .btn .btn-primary .btn-sm href="/debug/inspector/ui" target="_blank" {
                    (icons::globe()) " Open Inspector"
                }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "features" { "active" } else { "" })
                href="/b/admin/blocks"
                hx-get="/b/admin/blocks"
                hx-target="#content"
                hx-push-url="true"
            { (icons::package()) " Features" }
            a .tab .(if active_tab == "services" { "active" } else { "" })
                href="/b/admin/blocks?tab=services"
                hx-get="/b/admin/blocks?tab=services"
                hx-target="#content"
                hx-push-url="true"
            { (icons::server()) " Services" }
            a .tab .(if active_tab == "infrastructure" { "active" } else { "" })
                href="/b/admin/blocks?tab=infrastructure"
                hx-get="/b/admin/blocks?tab=infrastructure"
                hx-target="#content"
                hx-push-url="true"
            { (icons::settings()) " Infrastructure" }
        }

        div #blocks-tab-content {
            @let runtime_filter = msg.query("runtime");
            @let filtered: Vec<_> = all_blocks.iter().filter(|b| {
                let cat_match = match active_tab {
                    "services" => b.category == wafer_run::BlockCategory::Service,
                    "infrastructure" => b.category == wafer_run::BlockCategory::Infrastructure,
                    _ => b.category == wafer_run::BlockCategory::Feature,
                };
                cat_match && match runtime_filter {
                    "native" => b.runtime == wafer_run::BlockRuntime::Native,
                    "wasm" => b.runtime == wafer_run::BlockRuntime::Wasm,
                    _ => true,
                }
            }).collect();

            // Runtime filter dropdown
            div style="display:flex;justify-content:flex-end;margin-bottom:8px" {
                select .form-input style="width:auto;font-size:12px;padding:4px 8px"
                    onchange={"window.location.href='/b/admin/blocks?tab=" (active_tab) "&runtime='+this.value"}
                {
                    option value="" selected[runtime_filter.is_empty()] { "All runtimes" }
                    option value="native" selected[runtime_filter == "native"] { "Native only" }
                    option value="wasm" selected[runtime_filter == "wasm"] { "WASM only" }
                }
            }

            @if filtered.is_empty() {
                (components::empty_state("No blocks", "No blocks registered in this category"))
            }

            div .cards style="display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:8px;align-items:start" {
                style { (maud::PreEscaped("
                    .block-card-collapsed { min-height: 120px; }
                    .block-summary { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; text-overflow: ellipsis; }
                ")) }
                @for block in &filtered {
                    @let is_enabled = block_enabled.get(&block.name).copied().unwrap_or(true);

                    @let encoded_name = block.name.replace('/', "--");
                    div .card
                        style={"cursor:pointer;height:100px;display:flex;flex-direction:column;justify-content:space-between;position:relative;" (if !is_enabled { "opacity:0.5;" } else { "" })}
                        hx-get={"/b/admin/blocks/" (encoded_name) "/detail"}
                        hx-target="#block-detail-modal"
                        hx-swap="innerHTML"
                    {
                        // Top-right: status icon + version + details link
                        div style="position:absolute;top:12px;right:12px;display:flex;align-items:center;gap:6px" {
                            @if is_enabled {
                                span style="color:#10b981;font-size:14px" title="Enabled" { "✓" }
                            } @else {
                                span style="color:#94a3b8;font-size:14px" title="Disabled" { "✗" }
                            }
                            @if block.runtime == wafer_run::BlockRuntime::Wasm {
                                span .badge style="font-size:9px;padding:1px 5px;background:#8b5cf6;color:#fff" { "WASM" }
                            } @else {
                                span .badge style="font-size:9px;padding:1px 5px;background:#e2e8f0;color:#64748b" { "Native" }
                            }
                            span style="font-size:11px;color:#94a3b8" { "v" (block.version) }
                            span style="color:#94a3b8;font-size:11px;display:flex;align-items:center;gap:2px" {
                                "Details" (icons::chevron_right())
                            }
                        }
                        div {
                            h3 style="font-size:14px;font-weight:600;color:#1e3a5f;margin:0 0 4px;padding-right:50px" { (block.name) }
                            p .text-muted .block-summary style="font-size:13px;margin:0;line-height:1.4" { (block.summary) }
                        }
                        @if is_enabled && !block.admin_url.is_empty() {
                            div style="position:absolute;bottom:10px;right:12px" {
                                a .btn .btn-sm .btn-primary
                                    href=(block.admin_url)
                                    onclick="event.stopPropagation()"
                                    style="font-size:11px;padding:2px 8px"
                                { "Open" }
                            }
                        }
                    }
                }
            }
        }

        // Block detail modal (content loaded via htmx)
        div .modal-overlay #block-detail-modal-overlay hidden
            onclick="if(event.target===this)closeModal('block-detail-modal-overlay')"
        {
            div .modal style="max-width:700px;max-height:85vh;overflow-y:auto" {
                div #block-detail-modal {}
            }
        }
    };

    admin_page(
        "Blocks",
        &config,
        "/b/admin/blocks",
        user.as_ref(),
        content,
        msg,
    )
}

/// POST /b/admin/blocks/{name}/toggle — toggle a block's enabled state
pub async fn handle_toggle_feature(
    ctx: &dyn Context,
    msg: &mut Message,
    block_name: &str,
) -> Result_ {
    // Read current state from block_settings
    let current_enabled = db::query_raw(
        ctx,
        "SELECT enabled FROM suppers_ai__admin__block_settings WHERE block_name = ?1",
        &[serde_json::json!(block_name)],
    )
    .await
    .ok()
    .and_then(|rows| {
        rows.first()
            .and_then(|r| r.data.get("enabled").and_then(|v| v.as_i64()))
    })
    .map(|v| v != 0)
    .unwrap_or(true);

    let new_enabled = !current_enabled;
    let new_enabled_int = if new_enabled { 1 } else { 0 };

    // Upsert into block_settings
    let _ = db::exec_raw(
        ctx,
        "INSERT INTO suppers_ai__admin__block_settings (block_name, enabled, created_at, updated_at) \
         VALUES (?1, ?2, datetime('now'), datetime('now')) \
         ON CONFLICT (block_name) DO UPDATE SET enabled = ?2, updated_at = datetime('now')",
        &[
            serde_json::json!(block_name),
            serde_json::json!(new_enabled_int),
        ],
    )
    .await;

    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let action = if new_enabled {
        "block.enable"
    } else {
        "block.disable"
    };
    super::logs::audit_log(ctx, &admin_id, action, &format!("blocks/{block_name}"), &ip).await;

    // Re-render the blocks page
    blocks_page(ctx, msg).await
}

/// GET /b/admin/blocks/{name}/detail — block detail modal content
pub async fn handle_block_detail(
    ctx: &dyn Context,
    msg: &mut Message,
    block_name: &str,
) -> Result_ {
    let blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();
    let block_opt = blocks.iter().find(|b| b.name == block_name);

    // Check block enabled state from block_settings
    let is_enabled = db::query_raw(
        ctx,
        "SELECT enabled FROM suppers_ai__admin__block_settings WHERE block_name = ?1",
        &[serde_json::json!(block_name)],
    )
    .await
    .ok()
    .and_then(|rows| {
        rows.first()
            .and_then(|r| r.data.get("enabled").and_then(|v| v.as_i64()))
    })
    .map(|v| v != 0)
    .unwrap_or(true);

    let encoded = block_name.replace('/', "--");

    // Disabled block not in runtime — show minimal modal with toggle
    if block_opt.is_none() {
        let markup = html! {
            div .modal-header {
                h3 .modal-title { (block_name) }
                button .modal-close onclick="closeModal('block-detail-modal-overlay')" {
                    (icons::x())
                }
            }
            div .modal-body {
                div .flex .items-center .justify-between .mb-4 {
                    span .text-muted {
                        @if is_enabled {
                            "This block is enabled but not loaded. Restart the server to load it."
                        } @else {
                            "This block is currently disabled."
                        }
                    }
                    label .toggle {
                        input type="checkbox"
                            checked[is_enabled]
                            hx-post={"/b/admin/blocks/" (encoded) "/toggle"}
                            hx-target="#content";
                        span .toggle-slider {}
                    }
                }
                p style="font-size:0.875rem;color:#94a3b8;margin-top:1rem" {
                    @if is_enabled {
                        "Restart the server to see its full details."
                    } @else {
                        "Enable and restart the server to load this block and see its full details."
                    }
                }
            }
            script { (maud::PreEscaped("document.getElementById('block-detail-modal-overlay').removeAttribute('hidden');")) }
        };
        return ui::html_response(msg, markup);
    }

    let block = block_opt.unwrap();

    let markup = html! {
        div .modal-header {
            div {
                div .flex .items-center .gap-2 {
                    h3 .modal-title { (block.name) }
                    span .badge .badge-info style="font-size:11px" { "v" (block.version) }
                    span .badge style="font-size:11px;background:#f1f5f9;color:#475569" { (format!("{:?}", block.category)) }
                }
            }
            button .modal-close onclick="closeModal('block-detail-modal-overlay')" {
                (icons::x())
            }
        }
        div .modal-body {
            // Admin UI link + Block toggle (above description)
            div .flex .items-center .justify-between .mb-4 {
                div .flex .items-center .gap-2 {
                    @if is_enabled && !block.admin_url.is_empty() {
                        a .btn .btn-sm .btn-primary href=(block.admin_url) {
                            (icons::settings()) " Open Admin UI"
                        }
                    }
                }
                @if block.can_disable {
                    div .flex .items-center .gap-2 {
                        span .text-sm .text-muted { "Enabled" }
                        label .toggle {
                            @let encoded = block.name.replace('/', "--");
                            input type="checkbox"
                                checked[is_enabled]
                                hx-post={"/b/admin/blocks/" (encoded) "/toggle"}
                                hx-target="#content";
                            span .toggle-slider {}
                        }
                    }
                } @else {
                    span .text-sm .text-muted { "Always enabled (core block)" }
                }
            }

            // Description
            @if !block.description.is_empty() {
                p style="font-size:0.875rem;color:#64748b;line-height:1.6;margin-bottom:1rem" { (block.description) }
            }

            // Endpoints
            @if !block.endpoints.is_empty() {
                h4 style="font-size:0.875rem;font-weight:600;margin:1rem 0 0.5rem" { "Endpoints" }
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th style="width:70px" { "Method" }
                                th { "Path" }
                                th { "Description" }
                                th style="width:80px" { "Auth" }
                            }
                        }
                        tbody {
                            @for ep in &block.endpoints {
                                tr {
                                    td {
                                        span .badge style={"font-size:11px;" (match ep.method {
                                            wafer_run::types::HttpMethod::Get => "background:#dbeafe;color:#1d4ed8",
                                            wafer_run::types::HttpMethod::Post => "background:#dcfce7;color:#166534",
                                            wafer_run::types::HttpMethod::Patch => "background:#fef3c7;color:#92400e",
                                            wafer_run::types::HttpMethod::Delete => "background:#fce4ec;color:#c62828",
                                        })} { (ep.method) }
                                    }
                                    td .text-sm { code style="font-size:12px" { (ep.path) } }
                                    td .text-sm .text-muted { (ep.summary) }
                                    td {
                                        span .badge style={"font-size:10px;" (match ep.auth {
                                            wafer_run::types::AuthLevel::Public => "background:#dcfce7;color:#166534",
                                            wafer_run::types::AuthLevel::Admin => "background:#fce4ec;color:#c62828",
                                            wafer_run::types::AuthLevel::Authenticated => "background:#fef3c7;color:#92400e",
                                        })} { (ep.auth) }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Config Keys
            @if !block.config_keys.is_empty() {
                h4 style="font-size:0.875rem;font-weight:600;margin:1rem 0 0.5rem" { "Configuration" }
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Key" }
                                th { "Description" }
                                th { "Default" }
                            }
                        }
                        tbody {
                            @for ck in &block.config_keys {
                                tr {
                                    td { code style="font-size:12px" { (ck.key) } }
                                    td .text-sm .text-muted { (ck.description) }
                                    td .text-sm { code style="font-size:11px" { @if ck.default.is_empty() { "—" } @else { (ck.default) } } }
                                }
                            }
                        }
                    }
                }
            }

            // Technical details
            h4 style="font-size:0.875rem;font-weight:600;margin:1rem 0 0.5rem" { "Technical" }
            div style="font-size:13px;color:#64748b" {
                div .mb-2 {
                    b { "Interface: " }
                    span .badge style="font-size:11px;background:#f1f5f9;color:#475569" { (block.interface) }
                }
                @if !block.requires.is_empty() {
                    div .mb-2 {
                        b { "Requires: " }
                        @for req in &block.requires {
                            span .badge .badge-primary style="font-size:11px;margin-right:4px" { (req) }
                        }
                    }
                }
                @if !block.collections.is_empty() {
                    div .mb-2 {
                        b { "Database tables: " }
                        @for col in &block.collections {
                            span .badge style="font-size:11px;margin-right:4px;background:#f1f5f9;color:#475569" { (col.name) }
                        }
                    }
                }
            }
        }
        // Auto-open
        script { (maud::PreEscaped("document.getElementById('block-detail-modal-overlay').removeAttribute('hidden');")) }
    };

    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// htmx mutation handlers (return HTML fragments + toast triggers)
// ---------------------------------------------------------------------------

/// Render a single user table row (used by enable/disable mutations).
async fn user_row_fragment(ctx: &dyn Context, user_id: &str) -> Markup {
    let record = match db::get(ctx, "suppers_ai__auth__users", user_id).await {
        Ok(r) => r,
        Err(_) => return html! {},
    };

    let roles_opts = ListOptions {
        filters: vec![Filter {
            field: "user_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.to_string()),
        }],
        ..Default::default()
    };
    let roles: Vec<String> = match db::list(ctx, "suppers_ai__admin__user_roles", &roles_opts).await {
        Ok(r) => r
            .records
            .iter()
            .map(|rec| rec.str_field("role").to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    };

    let email = record.str_field("email");
    let disabled = record.bool_field("disabled");
    let created = record.str_field("created_at");

    html! {
        tr #{"user-row-" (record.id)} {
            td { (email) }
            td {
                @for role in &roles {
                    span .badge .badge-primary style="margin-right: 0.25rem;" { (role) }
                }
                @if roles.is_empty() {
                    span .text-muted { "—" }
                }
            }
            td {
                @if disabled {
                    (components::status_badge("disabled"))
                } @else {
                    (components::status_badge("active"))
                }
            }
            td .text-muted .text-sm { (created.get(..10).unwrap_or(created)) }
            td {
                @if disabled {
                    button .btn .btn-sm .btn-success
                        hx-post={"/b/admin/users/" (record.id) "/enable"}
                        hx-target={"#user-row-" (record.id)}
                        hx-swap="outerHTML"
                        title="Enable user"
                    { "Enable" }
                } @else {
                    button .btn .btn-sm .btn-secondary
                        hx-post={"/b/admin/users/" (record.id) "/disable"}
                        hx-target={"#user-row-" (record.id)}
                        hx-swap="outerHTML"
                        hx-confirm={"Disable " (email) "?"}
                        title="Disable user"
                    { "Disable" }
                }
                " "
                button .btn .btn-sm .btn-danger
                    hx-delete={"/b/admin/users/" (record.id)}
                    hx-target={"#user-row-" (record.id)}
                    hx-swap="outerHTML"
                    hx-confirm={"Delete " (email) "? This cannot be undone."}
                    title="Delete user"
                { (icons::trash()) }
            }
        }
    }
}

/// POST /b/admin/users/{id}/disable
pub async fn handle_user_disable(ctx: &dyn Context, msg: &mut Message, user_id: &str) -> Result_ {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        return wafer_run::helpers::err_bad_request(msg, "Cannot disable your own account");
    }
    let ip = msg.remote_addr().to_string();
    let mut data = std::collections::HashMap::new();
    data.insert("disabled".to_string(), serde_json::json!(true));
    crate::blocks::helpers::stamp_updated(&mut data);
    if let Err(e) = db::update(ctx, "suppers_ai__auth__users", user_id, data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "user.disable",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    let row = user_row_fragment(ctx, user_id).await;
    ui::html_response_with_toast(msg, row, "User disabled", "success")
}

/// POST /b/admin/users/{id}/enable
pub async fn handle_user_enable(ctx: &dyn Context, msg: &mut Message, user_id: &str) -> Result_ {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let mut data = std::collections::HashMap::new();
    data.insert("disabled".to_string(), serde_json::json!(false));
    crate::blocks::helpers::stamp_updated(&mut data);
    if let Err(e) = db::update(ctx, "suppers_ai__auth__users", user_id, data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "user.enable",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    let row = user_row_fragment(ctx, user_id).await;
    ui::html_response_with_toast(msg, row, "User enabled", "success")
}

/// DELETE /b/admin/users/{id}
pub async fn handle_user_delete(ctx: &dyn Context, msg: &mut Message, user_id: &str) -> Result_ {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        return wafer_run::helpers::err_bad_request(msg, "Cannot delete your own account");
    }
    let ip = msg.remote_addr().to_string();
    if let Err(e) = db::soft_delete(ctx, "suppers_ai__auth__users", user_id).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "user.delete",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    ui::html_response_with_toast(msg, html! {}, "User deleted", "success")
}

/// POST /b/admin/iam/roles (create role from modal form)
pub async fn handle_create_role(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let body = parse_form_body(&msg.data);

    let name = body
        .get("name")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if name.is_empty() {
        return wafer_run::helpers::err_bad_request(msg, "Role name is required");
    }

    let mut data = std::collections::HashMap::new();
    data.insert("name".to_string(), serde_json::json!(name));
    if let Some(desc) = body.get("description") {
        data.insert("description".to_string(), serde_json::json!(desc));
    }
    crate::blocks::helpers::stamp_created(&mut data);

    if let Err(e) = db::create(ctx, "suppers_ai__admin__roles", data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(ctx, &admin_id, "role.create", &format!("roles/{name}"), &ip).await;

    // Return the updated roles tab + close modal + toast
    let content = roles_tab(ctx).await;
    let trigger = r#"{"showToast":{"message":"Role created","type":"success"},"closeModal":{"id":"create-role"}}"#;
    wafer_run::helpers::ResponseBuilder::new(msg)
        .set_header("HX-Trigger", trigger)
        .body(
            content.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
}

// DELETE /b/admin/iam/roles/{id}
// ---------------------------------------------------------------------------
// Variable mutation handlers
// ---------------------------------------------------------------------------

/// POST /b/admin/variables — create a new variable
pub async fn handle_create_variable(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let body = parse_form_body(&msg.data);

    let key = body
        .get("key")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if key.is_empty() {
        return wafer_run::helpers::err_bad_request(msg, "Key is required");
    }

    let mut data = std::collections::HashMap::new();
    data.insert("key".to_string(), serde_json::json!(key));
    if let Some(v) = body.get("value") {
        data.insert("value".to_string(), serde_json::json!(v));
    }
    if let Some(v) = body.get("description") {
        data.insert("description".to_string(), serde_json::json!(v));
    }
    let sensitive = body.get("sensitive").map(|s| s.as_str()).unwrap_or("0");
    data.insert(
        "sensitive".to_string(),
        serde_json::json!(if sensitive == "1" { 1 } else { 0 }),
    );
    crate::blocks::helpers::stamp_created(&mut data);

    if let Err(e) = db::create(ctx, "suppers_ai__admin__variables", data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "variable.create",
        &format!("variables/{key}"),
        &ip,
    )
    .await;

    // Re-render the variables page (htmx will swap #content)
    variables_page(ctx, msg).await
}

/// GET /b/admin/variables/{key}/edit — return modal edit form content
pub async fn handle_edit_variable_form(
    ctx: &dyn Context,
    msg: &mut Message,
    var_key: &str,
) -> Result_ {
    let record = match db::get_by_field(
        ctx,
        "suppers_ai__admin__variables",
        "key",
        serde_json::Value::String(var_key.to_string()),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return wafer_run::helpers::err_not_found(msg, "Variable not found"),
    };

    let key = record.str_field("key").to_string();
    let sensitive = record.i64_field("sensitive") != 0;
    let value = record.str_field("value").to_string();
    let description = record.str_field("description").to_string();
    let warning = record.str_field("warning").to_string();

    let markup = html! {
        div .modal-header {
            h3 .modal-title { "Edit Variable" }
            button .modal-close onclick="closeModal('edit-var-modal-overlay')" {
                (icons::x())
            }
        }
        div .modal-body {
            form hx-put={"/b/admin/variables/" (key)} hx-target="#content" {
                div .form-group {
                    label .form-label { "Key" }
                    input .form-input type="text" value=(key) disabled;
                }
                div .form-group {
                    label .form-label for="edit-value" { "Value" }
                    @if sensitive {
                        div style="position:relative" {
                            input .form-input #edit-value
                                type="password"
                                name="value"
                                value=(value)
                                style="padding-right:3rem";
                            button .btn .btn-ghost .btn-icon
                                type="button"
                                style="position:absolute;right:0.25rem;top:50%;transform:translateY(-50%)"
                                onclick="var i=document.getElementById('edit-value');if(i.type==='password'){i.type='text';this.title='Hide'}else{i.type='password';this.title='Reveal'}"
                                title="Reveal"
                            { (icons::eye()) }
                        }
                    } @else {
                        input .form-input type="text" #edit-value name="value" value=(value);
                    }
                }
                div .form-group {
                    label .form-label for="edit-desc" { "Description" }
                    input .form-input type="text" #edit-desc name="description" value=(description);
                }
                @if !warning.is_empty() {
                    div style="background:#fef3c7;border:1px solid #f59e0b;border-radius:8px;padding:0.75rem;margin-bottom:1rem;font-size:0.813rem;color:#92400e;display:flex;align-items:center;gap:0.5rem" {
                        "⚠ " (warning)
                    }
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('edit-var-modal-overlay')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Save" }
                }
            }
        }
        // Auto-open the modal
        script { (maud::PreEscaped("document.getElementById('edit-var-modal-overlay').removeAttribute('hidden');")) }
    };

    ui::html_response(msg, markup)
}

/// PUT /b/admin/variables/{key} — update variable value
pub async fn handle_update_variable(
    ctx: &dyn Context,
    msg: &mut Message,
    var_key: &str,
) -> Result_ {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let body = parse_form_body(&msg.data);

    // Prevent setting sensitive keys (secrets/keys) to empty (would break auth)
    if var_key.ends_with("_SECRET") || var_key.ends_with("_KEY") {
        let new_value = body.get("value").map(|s| s.as_str()).unwrap_or("");
        if new_value.is_empty() {
            return wafer_run::helpers::err_bad_request(
                msg,
                &format!("Cannot set {} to an empty value", var_key),
            );
        }
    }

    // Find existing record by key
    let record = match db::get_by_field(
        ctx,
        "suppers_ai__admin__variables",
        "key",
        serde_json::Value::String(var_key.to_string()),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return wafer_run::helpers::err_not_found(msg, "Variable not found"),
    };

    let mut data = std::collections::HashMap::new();
    if let Some(v) = body.get("value") {
        data.insert("value".to_string(), serde_json::json!(v));
    }
    if let Some(v) = body.get("description") {
        data.insert("description".to_string(), serde_json::json!(v));
    }
    crate::blocks::helpers::stamp_updated(&mut data);

    if let Err(e) = db::update(ctx, "suppers_ai__admin__variables", &record.id, data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "variable.update",
        &format!("variables/{var_key}"),
        &ip,
    )
    .await;

    variables_page(ctx, msg).await
}

pub async fn handle_delete_role(ctx: &dyn Context, msg: &mut Message, role_id: &str) -> Result_ {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    // Check if system role
    if let Ok(record) = db::get(ctx, "suppers_ai__admin__roles", role_id).await {
        if record.bool_field("is_system") {
            return wafer_run::helpers::err_forbidden(msg, "Cannot delete system role");
        }
    }

    if let Err(e) = db::delete(ctx, "suppers_ai__admin__roles", role_id).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::logs::audit_log(
        ctx,
        &admin_id,
        "role.delete",
        &format!("roles/{role_id}"),
        &ip,
    )
    .await;

    let content = roles_tab(ctx).await;
    ui::html_response_with_toast(msg, content, "Role deleted", "success")
}

// ---------------------------------------------------------------------------
// Email Settings page
// ---------------------------------------------------------------------------

const EMAIL_SETTINGS_KEYS: &[(&str, &str, &str, &str, bool)] = &[
    (
        "SUPPERS_AI__EMAIL__MAILGUN_API_KEY",
        "Mailgun API Key",
        "API key from your Mailgun account.",
        "",
        true,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_DOMAIN",
        "Mailgun Domain",
        "Sending domain configured in Mailgun (e.g. mg.example.com).",
        "",
        false,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_FROM",
        "From Address",
        "Sender address for emails. Leave empty for default (noreply@domain).",
        "",
        false,
    ),
    (
        "SUPPERS_AI__EMAIL__MAILGUN_REPLY_TO",
        "Reply-To Address",
        "Reply-to address for emails. Leave empty to omit.",
        "",
        false,
    ),
];

pub async fn email_settings_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let mut values = Vec::new();
    for &(key, label, help, default, sensitive) in EMAIL_SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value, sensitive));
    }

    let content = html! {
        (components::page_header("Email Settings", Some("Configure email delivery via Mailgun"), None))

        form #settings-form onsubmit="return submitEmailSettings(event)" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::globe()) " Mailgun Configuration"
            }

            @for (key, label, help, default, ref value, sensitive) in &values {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(key) { (label) }
                    @if *sensitive {
                        div style="display:flex;align-items:center;gap:0.5rem" {
                            input .form-input #(key) name=(key) type="password" value=(value)
                                placeholder=(if value.is_empty() { "Not configured" } else { "******** (set)" })
                                style="flex:1";
                            button type="button" .btn .btn-ghost .btn-sm
                                onclick={"var i=document.getElementById('" (key) "');i.type=i.type==='password'?'text':'password'"}
                            { (icons::eye()) }
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
function submitEmailSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name]').forEach(function(el) { data[el.name] = el.value; });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/admin/email', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) })
    .then(function(r) { return r.json(); })
    .then(function(d) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' } })); })
    .catch(function(err) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: 'Error: ' + err.message, type: 'error' } })); })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#)) }
    };

    admin_page(
        "Email",
        &site_config,
        "/b/admin/email",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// WRAP Grants
// ---------------------------------------------------------------------------

pub async fn grants_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    // Redirect to the unified Permissions page
    permissions_page(ctx, msg).await
}

pub async fn permissions_page(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "database" => "database",
        "storage" => "storage",
        "network" => "network",
        _ => "all",
    };

    let content = html! {
        (components::page_header(
            "Permissions",
            Some("Control which blocks can access other blocks' data, files, and services"),
            None::<maud::Markup>,
        ))

        div .tabs {
            a .tab .(if active_tab == "all" { "active" } else { "" })
                href="/b/admin/permissions"
                hx-get="/b/admin/permissions"
                hx-target="#content"
                hx-push-url="true"
            { (icons::shield()) " All" }
            a .tab .(if active_tab == "database" { "active" } else { "" })
                href="/b/admin/permissions?tab=database"
                hx-get="/b/admin/permissions?tab=database"
                hx-target="#content"
                hx-push-url="true"
            { (icons::database()) " Database & Config" }
            a .tab .(if active_tab == "storage" { "active" } else { "" })
                href="/b/admin/permissions?tab=storage"
                hx-get="/b/admin/permissions?tab=storage"
                hx-target="#content"
                hx-push-url="true"
            { (icons::hard_drive()) " Storage" }
            a .tab .(if active_tab == "network" { "active" } else { "" })
                href="/b/admin/permissions?tab=network"
                hx-get="/b/admin/permissions?tab=network"
                hx-target="#content"
                hx-push-url="true"
            { (icons::globe()) " Network" }
        }

        div #permissions-content {
            @if active_tab == "database" {
                (permissions_database_tab(ctx, msg).await)
            } @else if active_tab == "storage" {
                (permissions_storage_tab(ctx, msg).await)
            } @else if active_tab == "network" {
                (permissions_network_tab(ctx, msg).await)
            } @else {
                (permissions_all_tab(ctx, msg).await)
            }
        }

    };

    admin_page(
        "Permissions",
        &config,
        "/b/admin/permissions",
        user.as_ref(),
        content,
        msg,
    )
}

fn grants_code_tab(ctx: &dyn Context) -> Markup {
    let blocks = ctx.registered_blocks();

    html! {
        div .card .mt-4 {
            div .card-header {
                h3 .card-title { "Grants Declared in Code" }
                p .text-muted style="font-size:13px" {
                    "These grants are declared in block source code via BlockInfo.grants and cannot be modified here."
                }
            }
            div .card-body {
                table .table {
                    thead {
                        tr {
                            th { "Block (Owner)" }
                            th { "Grantee" }
                            th { "Type" }
                            th { "Resource Pattern" }
                            th { "Access" }
                        }
                    }
                    tbody {
                        @for block in &blocks {
                            @for grant in &block.grants {
                                tr {
                                    td {
                                        span .badge .badge-info { (block.name) }
                                    }
                                    td {
                                        @if grant.grantee == "*" {
                                            span .badge .badge-warning { "* (all blocks)" }
                                        } @else {
                                            code { (grant.grantee) }
                                        }
                                    }
                                    td {
                                        @if let Some(ref rt) = grant.resource_type {
                                            span .badge .badge-info style="font-size:11px" { (rt) }
                                        } @else {
                                            span .badge .badge-secondary style="font-size:11px" { "all" }
                                        }
                                    }
                                    td {
                                        code style="font-size:12px" { (grant.resource) }
                                    }
                                    td {
                                        @if grant.write {
                                            span .badge .badge-danger { "read + write" }
                                        } @else {
                                            span .badge .badge-success { "read only" }
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
}

async fn grants_custom_tab(ctx: &dyn Context, _msg: &mut Message) -> Markup {
    let grants = db::list_all(ctx, "suppers_ai__admin__wrap_grants", vec![])
        .await
        .unwrap_or_default();

    // Collect registered block names for the grantee dropdown
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    html! {
        div .card .mt-4 {
            div .card-header .flex .items-center .justify-between {
                div {
                    h3 .card-title { "Custom Grants" }
                    p .text-muted style="font-size:13px" {
                        "Add grants for third-party or WASM blocks. These are loaded at startup alongside code-declared grants."
                    }
                }
                button .btn .btn-primary .btn-sm onclick="openModal('add-grant-modal')" {
                    (icons::plus()) " Add Grant"
                }
            }
            div .card-body {
                @if grants.is_empty() {
                    p .text-muted { "No custom grants configured." }
                } @else {
                    table .table {
                        thead {
                            tr {
                                th { "Grantee" }
                                th { "Type" }
                                th { "Resource Pattern" }
                                th { "Access" }
                                th { "Description" }
                                th style="width:60px" {}
                            }
                        }
                        tbody {
                            @for grant in &grants {
                                @let id = &grant.id;
                                @let grantee = grant.data.get("grantee").and_then(|v| v.as_str()).unwrap_or("");
                                @let resource = grant.data.get("resource").and_then(|v| v.as_str()).unwrap_or("");
                                @let write = grant.data.get("write").map(|v| v.as_i64().unwrap_or(0) != 0 || v.as_str() == Some("1")).unwrap_or(false);
                                @let rt = grant.data.get("resource_type").and_then(|v| v.as_str()).unwrap_or("");
                                @let description = grant.data.get("description").and_then(|v| v.as_str()).unwrap_or("");
                                tr {
                                    td {
                                        @if grantee == "*" {
                                            span .badge .badge-warning { "* (all blocks)" }
                                        } @else {
                                            code { (grantee) }
                                        }
                                    }
                                    td {
                                        @if rt.is_empty() {
                                            span .badge .badge-secondary style="font-size:11px" { "all" }
                                        } @else {
                                            span .badge .badge-info style="font-size:11px" { (rt) }
                                        }
                                    }
                                    td {
                                        code style="font-size:12px" { (resource) }
                                    }
                                    td {
                                        @if write {
                                            span .badge .badge-danger { "read + write" }
                                        } @else {
                                            span .badge .badge-success { "read only" }
                                        }
                                    }
                                    td style="font-size:13px" { (description) }
                                    td {
                                        button .btn .btn-danger .btn-sm
                                            hx-delete={"/b/admin/grants/rules/" (id)}
                                            hx-target="#content"
                                            hx-confirm="Delete this grant?"
                                        { (icons::trash()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build JSON data for the grant form JS
        script {
            (maud::PreEscaped("var grantBlocks = "))
            (maud::PreEscaped({
                let block_data: Vec<serde_json::Value> = blocks.iter()
                    .filter(|b| b.name.contains('/'))
                    .map(|b| {
                        let prefix = format!("{}__", b.name.replace('/', "__").replace('-', "_"));
                        let config_prefix = prefix.to_uppercase();
                        serde_json::json!({
                            "name": b.name,
                            "prefix": prefix,
                            "config_prefix": config_prefix,
                            "collections": b.collections.iter().map(|c| &c.name).collect::<Vec<_>>(),
                            "config_keys": b.config_keys.iter().map(|k| &k.key).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                serde_json::to_string(&block_data).unwrap_or_default()
            }))
            (maud::PreEscaped(r#";
            function updateGrantForm() {
                var owner = document.getElementById('grant_owner').value;
                var type = document.getElementById('resource_type').value;
                var scopeEl = document.getElementById('grant_scope');
                var specificEl = document.getElementById('specific_group');
                var resourceEl = document.getElementById('resource');
                var specificSelect = document.getElementById('specific_resource');
                if (!owner || !scopeEl) return;

                var block = grantBlocks.find(function(b) { return b.name === owner; });
                if (!block) return;

                // Update hidden resource field based on selections
                if (scopeEl.value === 'all') {
                    specificEl.style.display = 'none';
                    // Auto-fill resource pattern
                    if (type === 'config') {
                        resourceEl.value = block.config_prefix + '*';
                    } else if (type === 'storage') {
                        resourceEl.value = block.name + '/*';
                    } else if (type === 'crypto') {
                        resourceEl.value = block.name;
                    } else {
                        resourceEl.value = block.prefix + '*';
                    }
                } else {
                    specificEl.style.display = '';
                    // Populate specific resource dropdown
                    specificSelect.innerHTML = '';
                    var items = [];
                    if (type === 'db' || type === '') {
                        block.collections.forEach(function(c) { items.push(c); });
                    }
                    if (type === 'config' || type === '') {
                        block.config_keys.forEach(function(k) { items.push(k); });
                    }
                    if (items.length === 0) {
                        var opt = document.createElement('option');
                        opt.value = block.prefix + '*';
                        opt.text = 'All resources (' + block.prefix + '*)';
                        specificSelect.appendChild(opt);
                    }
                    items.forEach(function(item) {
                        var opt = document.createElement('option');
                        opt.value = item;
                        opt.text = item;
                        specificSelect.appendChild(opt);
                    });
                    resourceEl.value = specificSelect.value;
                    specificSelect.onchange = function() { resourceEl.value = this.value; };
                }
            }
            "#))
        }

        (components::modal("add-grant-modal", "Add Access Grant", html! {
            form hx-post="/b/admin/grants/rules" hx-target="#content" {
                div .form-group {
                    label .form-label for="grantee" { "Which block needs access?" }
                    select .form-input #grantee name="grantee" required {
                        option value="" disabled selected { "Select a block..." }
                        option value="*" { "All blocks" }
                        @for name in &block_names {
                            option value=(name) { (name) }
                        }
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "The block that will receive this access permission."
                    }
                }
                div .form-group {
                    label .form-label for="grant_owner" { "Access to which block's data?" }
                    select .form-input #grant_owner
                        onchange="updateGrantForm()"
                    {
                        option value="" disabled selected { "Select the data owner..." }
                        @for b in blocks.iter().filter(|b| b.name.contains('/')) {
                            option value=(b.name) { (b.name) }
                        }
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "Each block owns its own database tables, config keys, and storage. Pick the block whose data you want to share."
                    }
                }
                div .form-group {
                    label .form-label for="resource_type" { "What kind of data?" }
                    select .form-input #resource_type name="resource_type"
                        onchange="updateGrantForm()"
                    {
                        option value="" { "All (database + config + storage)" }
                        option value="db" { "Database tables" }
                        option value="config" { "Config keys" }
                        option value="storage" { "Storage files" }
                        option value="crypto" { "Crypto signing keys" }
                    }
                }
                div .form-group {
                    label .form-label for="grant_scope" { "How much access?" }
                    select .form-input #grant_scope
                        onchange="updateGrantForm()"
                    {
                        option value="all" { "All resources of this type" }
                        option value="specific" { "A specific resource" }
                    }
                }
                div .form-group #specific_group style="display:none" {
                    label .form-label for="specific_resource" { "Pick a resource" }
                    select .form-input #specific_resource {}
                }
                // Hidden field that holds the computed resource pattern
                input type="hidden" #resource name="resource";
                div .form-group {
                    label .form-label .flex .items-center .gap-2 {
                        input type="checkbox" #write name="write" value="on";
                        " Allow write access"
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "If unchecked, the block can only read the data."
                    }
                }
                div .form-group {
                    label .form-label for="description" { "Why is this needed? (optional)" }
                    input .form-input type="text" #description name="description"
                        placeholder="e.g. Analytics block needs to read user profiles";
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('add-grant-modal')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Add Grant" }
                }
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// Permissions page tab functions
// ---------------------------------------------------------------------------

/// "All" tab: combines data from DB grants, storage rules, and network rules
/// into one unified table with human-readable descriptions.
async fn permissions_all_tab(ctx: &dyn Context, _msg: &mut Message) -> Markup {
    let blocks = ctx.registered_blocks();

    // 1. Code grants (from block declarations)
    let mut all_rows: Vec<(String, String, String, String, u8)> = Vec::new(); // (type_badge, sentence, origin, sort_key, order: 0=custom, 1=code)

    for block in &blocks {
        for grant in &block.grants {
            let type_label = match &grant.resource_type {
                Some(rt) => match rt.to_string().as_str() {
                    "db" => "DB",
                    "config" => "Config",
                    "storage" => "Storage",
                    "crypto" => "Crypto",
                    "network" => "Network",
                    other => other,
                }
                .to_string(),
                None => "DB/Config".to_string(),
            };
            let grantee = if grant.grantee == "*" {
                "All blocks".to_string()
            } else {
                grant.grantee.clone()
            };
            let verb = if grant.write {
                "can read and write"
            } else {
                "can read"
            };
            let sentence = format!(
                "{} {} {}' {}",
                grantee, verb, block.name, grant.resource
            );
            all_rows.push((type_label, sentence, "code".into(), block.name.clone(), 1));
        }
    }

    // 2. Custom DB grants
    let custom_grants = db::list_all(ctx, "suppers_ai__admin__wrap_grants", vec![])
        .await
        .unwrap_or_default();
    for grant in &custom_grants {
        let grantee = grant
            .data
            .get("grantee")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let resource = grant
            .data
            .get("resource")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let write = grant
            .data
            .get("write")
            .map(|v| v.as_i64().unwrap_or(0) != 0 || v.as_str() == Some("1"))
            .unwrap_or(false);
        let rt = grant
            .data
            .get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let type_label = match rt {
            "db" => "DB",
            "config" => "Config",
            "storage" => "Storage",
            "crypto" => "Crypto",
            "" => "DB/Config",
            other => other,
        };
        let grantee_display = if grantee == "*" {
            "All blocks"
        } else {
            grantee
        };
        let verb = if write {
            "can read and write"
        } else {
            "can read"
        };
        let sentence = format!("{} {} {}", grantee_display, verb, resource);
        all_rows.push((
            type_label.to_string(),
            sentence,
            "custom".into(),
            grantee.to_string(),
            0,
        ));
    }

    // 3. Storage rules
    let storage_rules = db::list_all(ctx, "suppers_ai__admin__storage_rules", vec![])
        .await
        .unwrap_or_default();
    for rule in &storage_rules {
        let rule_type = rule
            .data
            .get("rule_type")
            .and_then(|v| v.as_str())
            .unwrap_or("allow");
        let source = rule
            .data
            .get("source_block")
            .and_then(|v| v.as_str())
            .unwrap_or("*");
        let target = rule
            .data
            .get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let access = rule
            .data
            .get("access")
            .and_then(|v| v.as_str())
            .unwrap_or("readwrite");
        let source_display = if source == "*" {
            "All blocks"
        } else {
            source
        };
        let verb = if rule_type == "block" {
            "is blocked from"
        } else {
            match access {
                "read" => "can read",
                "write" => "can write to",
                _ => "can read and write",
            }
        };
        let sentence = format!("{} {} storage path {}", source_display, verb, target);
        all_rows.push((
            "Storage".into(),
            sentence,
            "custom".into(),
            source.to_string(),
            0,
        ));
    }

    // 4. Network rules
    let network_rules = db::list_all(ctx, "suppers_ai__admin__network_rules", vec![])
        .await
        .unwrap_or_default();
    for rule in &network_rules {
        let rule_type = rule
            .data
            .get("rule_type")
            .and_then(|v| v.as_str())
            .unwrap_or("block");
        let pattern = rule
            .data
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let scope = rule
            .data
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("global");
        let block_name = rule
            .data
            .get("block_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let source_display = if scope == "global" || block_name.is_empty() {
            "All blocks".to_string()
        } else {
            block_name.to_string()
        };
        let verb = if rule_type == "block" {
            "is blocked from reaching"
        } else {
            "is allowed to reach"
        };
        let sentence = format!("{} {} {}", source_display, verb, pattern);
        all_rows.push((
            "Network".into(),
            sentence,
            "custom".into(),
            source_display.clone(),
            0,
        ));
    }

    // Sort: custom (0) before code (1), then by sort_key
    all_rows.sort_by(|a, b| a.4.cmp(&b.4).then_with(|| a.3.cmp(&b.3)));

    html! {
        div .card .mt-4 {
            div .card-body {
                @if all_rows.is_empty() {
                    p .text-muted style="padding:2rem;text-align:center" {
                        "No permissions configured yet."
                    }
                } @else {
                    table .table {
                        thead {
                            tr {
                                th style="width:110px" { "Type" }
                                th { "Permission" }
                                th style="width:80px" { "Origin" }
                            }
                        }
                        tbody {
                            @for (type_label, sentence, origin, _sort, _order) in &all_rows {
                                tr {
                                    td {
                                        @let badge_class = match type_label.as_str() {
                                            "DB" | "DB/Config" => "badge-info",
                                            "Config" => "badge-info",
                                            "Storage" => "badge-warning",
                                            "Network" => "badge-success",
                                            "Crypto" => "badge-secondary",
                                            _ => "badge-secondary",
                                        };
                                        span .badge .(badge_class) style="font-size:11px" { (type_label) }
                                    }
                                    td style="font-size:13px" { (sentence) }
                                    td {
                                        @if origin == "code" {
                                            span .badge .badge-secondary style="font-size:10px" { "code" }
                                        } @else {
                                            span .badge .badge-primary style="font-size:10px" { "custom" }
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
}

/// "Database & Config" tab: wraps the existing grants_code_tab and grants_custom_tab.
async fn permissions_database_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    html! {
        (grants_custom_tab(ctx, msg).await)
        (grants_code_tab(ctx))
    }
}

/// "Storage" tab: delegates to the existing storage_rules_tab.
async fn permissions_storage_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    storage_rules_tab(ctx, msg).await
}

/// "Network" tab: delegates to the existing network_rules_tab.
async fn permissions_network_tab(ctx: &dyn Context, msg: &mut Message) -> Markup {
    network_rules_tab(ctx, msg).await
}

#[allow(dead_code)]
/// Form for adding a database/config grant.
fn permissions_db_form(ctx: &dyn Context) -> Markup {
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    html! {
        // Re-use the JS from grants_custom_tab for dynamic owner-based form updates
        script {
            (maud::PreEscaped("var grantBlocks = "))
            (maud::PreEscaped({
                let block_data: Vec<serde_json::Value> = blocks.iter()
                    .filter(|b| b.name.contains('/'))
                    .map(|b| {
                        let prefix = format!("{}__", b.name.replace('/', "__").replace('-', "_"));
                        let config_prefix = prefix.to_uppercase();
                        serde_json::json!({
                            "name": b.name,
                            "prefix": prefix,
                            "config_prefix": config_prefix,
                            "collections": b.collections.iter().map(|c| &c.name).collect::<Vec<_>>(),
                            "config_keys": b.config_keys.iter().map(|k| &k.key).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                serde_json::to_string(&block_data).unwrap_or_default()
            }))
            (maud::PreEscaped(r#";
            function updatePermGrantForm() {
                var owner = document.getElementById('perm_grant_owner').value;
                var type = document.getElementById('perm_resource_type').value;
                var scopeEl = document.getElementById('perm_grant_scope');
                var specificEl = document.getElementById('perm_specific_group');
                var resourceEl = document.getElementById('perm_resource');
                var specificSelect = document.getElementById('perm_specific_resource');
                if (!owner || !scopeEl) return;
                var block = grantBlocks.find(function(b) { return b.name === owner; });
                if (!block) return;
                if (scopeEl.value === 'all') {
                    specificEl.style.display = 'none';
                    if (type === 'config') {
                        resourceEl.value = block.config_prefix + '*';
                    } else if (type === 'storage') {
                        resourceEl.value = block.name + '/*';
                    } else if (type === 'crypto') {
                        resourceEl.value = block.name;
                    } else {
                        resourceEl.value = block.prefix + '*';
                    }
                } else {
                    specificEl.style.display = '';
                    specificSelect.innerHTML = '';
                    var items = [];
                    if (type === 'db' || type === '') {
                        block.collections.forEach(function(c) { items.push(c); });
                    }
                    if (type === 'config' || type === '') {
                        block.config_keys.forEach(function(k) { items.push(k); });
                    }
                    if (items.length === 0) {
                        var opt = document.createElement('option');
                        opt.value = block.prefix + '*';
                        opt.text = 'All resources (' + block.prefix + '*)';
                        specificSelect.appendChild(opt);
                    }
                    items.forEach(function(item) {
                        var opt = document.createElement('option');
                        opt.value = item;
                        opt.text = item;
                        specificSelect.appendChild(opt);
                    });
                    resourceEl.value = specificSelect.value;
                    specificSelect.onchange = function() { resourceEl.value = this.value; };
                }
            }
            "#))
        }
        form hx-post="/b/admin/grants/rules" hx-target="#content" {
            div .form-group {
                label .form-label for="grantee" { "Which block needs access?" }
                select .form-input #perm_grantee name="grantee" required {
                    option value="" disabled selected { "Select a block..." }
                    option value="*" { "All blocks" }
                    @for name in &block_names {
                        option value=(name) { (name) }
                    }
                }
            }
            div .form-group {
                label .form-label for="perm_grant_owner" { "Access to which block's data?" }
                select .form-input #perm_grant_owner
                    onchange="updatePermGrantForm()"
                {
                    option value="" disabled selected { "Select the data owner..." }
                    @for b in blocks.iter().filter(|b| b.name.contains('/')) {
                        option value=(b.name) { (b.name) }
                    }
                }
            }
            div .form-group {
                label .form-label for="perm_resource_type" { "What kind of data?" }
                select .form-input #perm_resource_type name="resource_type"
                    onchange="updatePermGrantForm()"
                {
                    option value="" { "All (database + config + storage)" }
                    option value="db" { "Database tables" }
                    option value="config" { "Config keys" }
                    option value="storage" { "Storage files" }
                    option value="crypto" { "Crypto signing keys" }
                }
            }
            div .form-group {
                label .form-label for="perm_grant_scope" { "How much access?" }
                select .form-input #perm_grant_scope
                    onchange="updatePermGrantForm()"
                {
                    option value="all" { "All resources of this type" }
                    option value="specific" { "A specific resource" }
                }
            }
            div .form-group #perm_specific_group style="display:none" {
                label .form-label for="perm_specific_resource" { "Pick a resource" }
                select .form-input #perm_specific_resource {}
            }
            input type="hidden" #perm_resource name="resource";
            div .form-group {
                label .form-label .flex .items-center .gap-2 {
                    input type="checkbox" #perm_write name="write" value="on";
                    " Allow write access"
                }
            }
            div .form-group {
                label .form-label for="perm_description" { "Why is this needed? (optional)" }
                input .form-input type="text" #perm_description name="description"
                    placeholder="e.g. Analytics block needs to read user profiles";
            }
            div .form-actions {
                button .btn .btn-secondary type="button" onclick="resetPermModal(); closeModal('add-permission-modal')" { "Cancel" }
                button .btn .btn-primary type="submit" { "Add Grant" }
            }
        }
    }
}

#[allow(dead_code)]
/// Form for adding a storage rule.
fn permissions_storage_form(ctx: &dyn Context) -> Markup {
    let registered = ctx.registered_blocks();
    let storage_blocks: Vec<&str> = registered
        .iter()
        .filter(|b| b.category != wafer_run::BlockCategory::Service && !b.name.is_empty())
        .map(|b| b.name.as_str())
        .collect();

    html! {
        form hx-post="/b/admin/storage/rules" hx-target="#content" {
            div .form-group {
                label .form-label for="rule_type" { "Rule Type" }
                select .form-input name="rule_type" {
                    option value="allow" { "Allow — grant cross-block access" }
                    option value="block" { "Block — deny access to matching paths" }
                }
            }
            div .form-group {
                label .form-label for="source_block" { "Source Block" }
                select .form-input name="source_block" {
                    option value="*" { "* (any block)" }
                    @for name in &storage_blocks {
                        option value=(name) { (name) }
                    }
                }
            }
            div .form-group {
                label .form-label for="target_path" { "Target Path" }
                input .form-input type="text" name="target_path"
                    placeholder="e.g. wafer-run/web/*" required;
                p .text-muted style="font-size:12px;margin-top:4px" {
                    "Storage path pattern. e.g. " code { "wafer-run/web/*" }
                }
            }
            div .form-group {
                label .form-label for="access" { "Access Type" }
                select .form-input name="access" {
                    option value="readwrite" { "Read & Write" }
                    option value="read" { "Read only" }
                    option value="write" { "Write only" }
                }
            }
            div .form-group {
                label .form-label for="priority" { "Priority" }
                input .form-input type="number" name="priority" value="0";
                p .text-muted style="font-size:12px;margin-top:4px" { "Higher priority rules are evaluated first" }
            }
            div .form-actions {
                button .btn .btn-secondary type="button" onclick="resetPermModal(); closeModal('add-permission-modal')" { "Cancel" }
                button .btn .btn-primary type="submit" { "Add Rule" }
            }
        }
    }
}

#[allow(dead_code)]
/// Form for adding a network rule.
fn permissions_network_form() -> Markup {
    html! {
        form hx-post="/b/admin/network/rules" hx-target="#content" {
            div .form-group {
                label .form-label for="rule_type" { "Rule Type" }
                select .form-input name="rule_type" {
                    option value="block" { "Block — deny matching URLs" }
                    option value="allow" { "Allow — only permit matching URLs" }
                }
            }
            div .form-group {
                label .form-label for="pattern" { "URL Pattern" }
                input .form-input type="text" name="pattern"
                    placeholder="e.g. https://api.example.com/*" required;
                p .text-muted style="font-size:12px;margin-top:4px" {
                    "Use * as wildcard. Examples: " code { "*.internal.corp*" } ", " code { "https://api.stripe.com/*" }
                }
            }
            div .form-group {
                label .form-label for="priority" { "Priority" }
                input .form-input type="number" name="priority" value="0";
                p .text-muted style="font-size:12px;margin-top:4px" { "Higher priority rules are evaluated first" }
            }
            div .form-actions {
                button .btn .btn-secondary type="button" onclick="resetPermModal(); closeModal('add-permission-modal')" { "Cancel" }
                button .btn .btn-primary type="submit" { "Add Rule" }
            }
        }
    }
}

pub async fn handle_save_email_settings(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: std::collections::HashMap<String, String> = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };
    for &(key, _, _, _, _) in EMAIL_SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }
    json_respond(msg, &serde_json::json!({"message": "Email settings saved"}))
}
