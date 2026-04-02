//! SSR pages for the admin block.
//!
//! Each page queries the database directly (same patterns as the JSON handlers)
//! and renders HTML via maud.

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;

/// Parse URL-encoded form body (htmx default) into a HashMap.
fn parse_form_body(data: &[u8]) -> std::collections::HashMap<String, String> {
    let body = String::from_utf8_lossy(data);
    let mut map = std::collections::HashMap::new();
    for pair in body.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let key = urlencoding_decode(k);
            let value = urlencoding_decode(v);
            map.insert(key, value);
        }
    }
    map
}

fn urlencoding_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

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
            label: "Variables".into(),
            href: "/b/admin/variables".into(),
            icon: "settings",
        },
        NavItem {
            label: "Logs".into(),
            href: "/b/admin/logs".into(),
            icon: "file-text",
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
        "auth_users",
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
        "SELECT COUNT(*) as cnt FROM auth_users WHERE deleted_at IS NULL AND created_at >= ?1",
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
        "SELECT COUNT(*) as cnt FROM request_logs WHERE created_at >= ?1",
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
        "SELECT COUNT(*) as cnt FROM request_logs WHERE status = 'ERROR' AND created_at >= ?1",
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
        "SELECT AVG(duration_ms) as avg_ms FROM request_logs WHERE created_at >= ?1",
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
        "SELECT id, email, created_at FROM auth_users WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT 5",
        &[],
    ).await.unwrap_or_default();

    // Recent audit logs (last 5)
    let recent_audit = db::query_raw(ctx,
        "SELECT action, resource, user_id, created_at FROM audit_logs ORDER BY created_at DESC LIMIT 5",
        &[],
    ).await.unwrap_or_default();

    // Recent errors (last 5)
    let recent_errors = db::query_raw(ctx,
        "SELECT status_code, method, path, duration_ms, created_at FROM request_logs WHERE status = 'ERROR' OR status_code >= 400 ORDER BY created_at DESC LIMIT 5",
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
            "SELECT * FROM auth_users WHERE deleted_at IS NULL AND (email LIKE ?1 OR id LIKE ?1) ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
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
            "auth_users",
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
        let roles: Vec<String> = match db::list(ctx, "iam_user_roles", &roles_opts).await {
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

    let opts = ListOptions {
        limit: 200,
        ..Default::default()
    };
    let settings = db::list(ctx, "variables", &opts).await;

    let content = html! {
        (components::page_header("Variables", Some("Environment variables and configuration"),
            Some(html! {
                button .btn .btn-primary .btn-sm onclick="openModal('create-var')" {
                    (icons::plus()) " Add Variable"
                }
            })
        ))

        div #variables-content {
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
        "Variables",
        &config,
        "/b/admin/variables",
        user.as_ref(),
        content,
        msg,
    )
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
    let result = db::list(ctx, "iam_roles", &opts).await;

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
    let result = db::list(ctx, "api_keys", &opts).await;

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
                                                hx-post={"/b/auth/api-keys/" (record.id) "/revoke"}
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
            form hx-post="/b/auth/api-keys" hx-target="#users-tab-content" {
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
        "request_logs",
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
        "audit_logs",
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
        "middleware" => "middleware",
        "infrastructure" => "infrastructure",
        _ => "features",
    };

    let registered_blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();

    // Load block enabled/disabled state from block_settings table
    let block_settings_rows =
        db::query_raw(ctx, "SELECT block_name, enabled FROM block_settings", &[])
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

    // Build full block list: registered blocks + disabled blocks from block_settings
    // Disabled blocks get placeholder BlockInfo since they aren't in the runtime
    let mut all_blocks = registered_blocks.clone();
    for (name, enabled) in &block_enabled {
        if !*enabled && !all_blocks.iter().any(|b| &b.name == name) {
            // Create a minimal placeholder for this disabled block
            all_blocks.push(
                wafer_run::BlockInfo::new(
                    name,
                    "0.0.1",
                    "http.handler",
                    "(disabled — restart to load)",
                )
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
            a .tab .(if active_tab == "middleware" { "active" } else { "" })
                href="/b/admin/blocks?tab=middleware"
                hx-get="/b/admin/blocks?tab=middleware"
                hx-target="#content"
                hx-push-url="true"
            { (icons::shield()) " Middleware" }
            a .tab .(if active_tab == "infrastructure" { "active" } else { "" })
                href="/b/admin/blocks?tab=infrastructure"
                hx-get="/b/admin/blocks?tab=infrastructure"
                hx-target="#content"
                hx-push-url="true"
            { (icons::settings()) " Infrastructure" }
        }

        div #blocks-tab-content {
            @let category = match active_tab {
                "services" => wafer_run::BlockCategory::Service,
                "middleware" => wafer_run::BlockCategory::Middleware,
                "infrastructure" => wafer_run::BlockCategory::Infrastructure,
                _ => wafer_run::BlockCategory::Feature,
            };
            @let filtered: Vec<_> = all_blocks.iter().filter(|b| b.category == category).collect();

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
                            span style="font-size:11px;color:#94a3b8" { "v" (block.version) }
                            span style="color:#94a3b8;font-size:11px;display:flex;align-items:center;gap:2px" {
                                "Details" (icons::chevron_right())
                            }
                        }
                        div {
                            h3 style="font-size:14px;font-weight:600;color:#1e3a5f;margin:0 0 4px;padding-right:50px" { (block.name) }
                            p .text-muted .block-summary style="font-size:13px;margin:0;line-height:1.4" { (block.summary) }
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
        "SELECT enabled FROM block_settings WHERE block_name = ?1",
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
        "INSERT INTO block_settings (block_name, enabled, created_at, updated_at) \
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
        "SELECT enabled FROM block_settings WHERE block_name = ?1",
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
                    span .text-muted { "This block is currently disabled." }
                    label .toggle {
                        input type="checkbox"
                            checked[is_enabled]
                            hx-post={"/b/admin/blocks/" (encoded) "/toggle"}
                            hx-target="#content";
                        span .toggle-slider {}
                    }
                }
                p style="font-size:0.875rem;color:#94a3b8;margin-top:1rem" {
                    "Enable and restart the server to load this block and see its full details."
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
    let record = match db::get(ctx, "auth_users", user_id).await {
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
    let roles: Vec<String> = match db::list(ctx, "iam_user_roles", &roles_opts).await {
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
    if let Err(e) = db::update(ctx, "auth_users", user_id, data).await {
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
    if let Err(e) = db::update(ctx, "auth_users", user_id, data).await {
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
    if let Err(e) = db::soft_delete(ctx, "auth_users", user_id).await {
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

    if let Err(e) = db::create(ctx, "iam_roles", data).await {
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

    if let Err(e) = db::create(ctx, "variables", data).await {
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
        "variables",
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

    // Protect security-critical keys from being emptied
    const PROTECTED_KEYS: &[&str] = &["JWT_SECRET"];
    if PROTECTED_KEYS.contains(&var_key) {
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
        "variables",
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

    if let Err(e) = db::update(ctx, "variables", &record.id, data).await {
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
    if let Ok(record) = db::get(ctx, "iam_roles", role_id).await {
        if record.bool_field("is_system") {
            return wafer_run::helpers::err_forbidden(msg, "Cannot delete system role");
        }
    }

    if let Err(e) = db::delete(ctx, "iam_roles", role_id).await {
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
