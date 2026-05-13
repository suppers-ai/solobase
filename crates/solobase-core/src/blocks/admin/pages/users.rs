use maud::{html, Markup, PreEscaped};
use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions, SortField};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};

use super::{admin_page, crumb};
use crate::{
    blocks::{
        admin::{ROLES_TABLE, USER_ROLES_TABLE},
        auth::{API_KEYS_TABLE as API_KEYS, USERS_TABLE as USERS},
        helpers::{
            self, err_bad_request, err_forbidden, err_internal, parse_form_body, RecordExt,
            ResponseBuilder,
        },
    },
    ui::{
        self,
        components::{self, button, pagination, BtnVariant, CtrlSize},
        icons,
        shell::Topbar,
        templates::{list_page, PageHeader},
        SiteConfig, UserInfo,
    },
};

pub async fn users_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "roles" => "roles",
        "api-keys" => "api-keys",
        _ => "users",
    };

    let tabs_markup = html! {
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
    };

    let current_uid = user
        .as_ref()
        .map(|u| u.id.as_str())
        .unwrap_or("")
        .to_string();
    let tab_content = html! {
        div #users-tab-content {
            @if active_tab == "users" {
                (users_tab(ctx, msg, &current_uid).await)
            } @else if active_tab == "roles" {
                div #iam-content { (roles_tab(ctx).await) }
            } @else {
                (api_keys_tab(ctx).await)
            }
        }
        // Invite-user modal — always present so the topbar action works
        // regardless of which tab is active. Posts back to
        // /b/admin/users (create) and swaps the users table.
        (components::modal("invite-user", "Invite user", html! {
            form hx-post="/b/admin/users" hx-target="#users-tab-content" {
                div .form-group {
                    label .form-label .required for="invite-email" { "Email" }
                    input .form-input type="email" #invite-email name="email" placeholder="user@example.com" required;
                }
                div .form-group {
                    label .form-label .required for="invite-password" { "Initial password" }
                    input .form-input type="text" #invite-password name="password" placeholder="At least 8 chars" minlength="8" required;
                }
                div .form-group {
                    label .form-label for="invite-name" { "Name" }
                    input .form-input type="text" #invite-name name="name" placeholder="Optional";
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('invite-user')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Invite" }
                }
            }
        }))
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        Some(tabs_markup),
        tab_content,
        None,
    );

    admin_page(
        "Users",
        &config,
        "/b/admin/users",
        user.as_ref(),
        Topbar {
            crumbs: crumb("Users"),
            primary_action: Some(button(
                BtnVariant::Primary,
                CtrlSize::Sm,
                "+ Invite user",
                PreEscaped(r##"onclick="openModal('invite-user')""##.to_string()),
            )),
            subtitle: Some("Manage accounts, roles, and API keys"),
            show_palette: true,
        },
        body,
        msg,
    )
}

/// Users tab content (table + search + pagination).
async fn users_tab(ctx: &dyn Context, msg: &Message, current_user_id: &str) -> Markup {
    let (page, page_size, _) = msg.pagination_params(20);
    let search = msg.query("search").to_string();

    let result = if !search.is_empty() {
        // Search by email OR id. The OR group + SELECT * shape needs
        // `build_select_with_condition` rather than the flat-filters
        // `db::paginated_list` typed client.
        use sea_query::{Cond, Expr};
        use wafer_sql_utils::{ident::DynCol, query, value::sea_values_to_json, Backend};

        let like = format!("%{search}%");
        let offset = ((page - 1) * page_size) as i64;
        let or_group = Cond::any()
            .add(Expr::col(DynCol("email".into())).like(like.clone()))
            .add(Expr::col(DynCol("id".into())).like(like.clone()));

        let (sql, vals) = query::build_select_with_condition(
            USERS,
            &ListOptions {
                filters: vec![Filter {
                    field: "deleted_at".into(),
                    operator: FilterOp::IsNull,
                    value: serde_json::Value::Null,
                }],
                sort: vec![SortField {
                    field: "created_at".into(),
                    desc: true,
                }],
                limit: page_size as i64,
                offset,
                ..Default::default()
            },
            Some(or_group),
            Backend::Sqlite,
        );
        let records = db::query_raw(ctx, &sql, &sea_values_to_json(vals)).await;
        // Wrap in RecordList format. total_count is the in-page count here;
        // the search UI doesn't paginate beyond what fits in one page.
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
        db::paginated_list(ctx, USERS, page as i64, page_size as i64, filters, sort).await
    };

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by email or user ID...", "/b/admin/users", "#content", &search))
        }

        @match &result {
            Ok(list) => {
                (users_table(&list.records, ctx, current_user_id).await)

                (pagination(list.page as u32, list.page_size as u32, list.total_count as u32, "/b/admin/users"))
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
        let role_filters = vec![Filter {
            field: "user_id".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(record.id.clone()),
        }];
        let roles: Vec<String> = match db::list_all(ctx, USER_ROLES_TABLE, role_filters).await
        {
            Ok(records) => records
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
                                    span .text-muted { "\u{2014}" }
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

/// Render a single user table row (used by enable/disable mutations).
async fn user_row_fragment(ctx: &dyn Context, user_id: &str) -> Markup {
    let record = match db::get(ctx, USERS, user_id).await {
        Ok(r) => r,
        Err(_) => return html! {},
    };

    let role_filters = vec![Filter {
        field: "user_id".into(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.to_string()),
    }];
    let roles: Vec<String> = match db::list_all(ctx, USER_ROLES_TABLE, role_filters).await {
        Ok(records) => records
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
                    span .text-muted { "\u{2014}" }
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
pub async fn handle_user_disable(ctx: &dyn Context, msg: &Message, user_id: &str) -> OutputStream {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        return err_bad_request("Cannot disable your own account");
    }
    let ip = msg.remote_addr().to_string();
    let mut data = std::collections::HashMap::new();
    data.insert("disabled".to_string(), serde_json::json!(true));
    helpers::stamp_updated(&mut data);
    if let Err(e) = db::update(ctx, USERS, user_id, data).await {
        return err_internal(&format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "user.disable",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    let row = user_row_fragment(ctx, user_id).await;
    ui::html_response_with_toast(row, "User disabled", "success")
}

/// POST /b/admin/users/{id}/enable
pub async fn handle_user_enable(ctx: &dyn Context, msg: &Message, user_id: &str) -> OutputStream {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let mut data = std::collections::HashMap::new();
    data.insert("disabled".to_string(), serde_json::json!(false));
    helpers::stamp_updated(&mut data);
    if let Err(e) = db::update(ctx, USERS, user_id, data).await {
        return err_internal(&format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "user.enable",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    let row = user_row_fragment(ctx, user_id).await;
    ui::html_response_with_toast(row, "User enabled", "success")
}

/// DELETE /b/admin/users/{id}
pub async fn handle_user_delete(ctx: &dyn Context, msg: &Message, user_id: &str) -> OutputStream {
    let admin_id = msg.user_id().to_string();
    if admin_id == user_id {
        return err_bad_request("Cannot delete your own account");
    }
    let ip = msg.remote_addr().to_string();
    if let Err(e) = db::soft_delete(ctx, USERS, user_id).await {
        return err_internal(&format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "user.delete",
        &format!("users/{user_id}"),
        &ip,
    )
    .await;
    ui::html_response_with_toast(html! {}, "User deleted", "success")
}

/// POST /b/admin/users (invite-user modal form)
pub async fn handle_user_invite(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    use wafer_core::clients::crypto;
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let bytes = input.collect_to_bytes().await;
    let body = parse_form_body(&bytes);

    let email = body
        .get("email")
        .map(String::as_str)
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let password = body.get("password").map(String::as_str).unwrap_or("");
    let name = body
        .get("name")
        .map(String::as_str)
        .unwrap_or("")
        .to_string();
    if email.is_empty() {
        return err_bad_request("Email is required");
    }
    if password.len() < 8 {
        return err_bad_request("Password must be at least 8 characters");
    }

    // Reject duplicate email — emails are unique in practice though the
    // table doesn't carry a UNIQUE constraint.
    match db::get_by_field(
        ctx,
        USERS,
        "email",
        serde_json::Value::String(email.clone()),
    )
    .await
    {
        Ok(_) => return err_bad_request("A user with that email already exists"),
        Err(e) if e.code == ErrorCode::NotFound => {}
        Err(_) => {}
    }

    let password_hash = match crypto::hash(ctx, password).await {
        Ok(h) => h,
        Err(e) => return err_internal(&format!("Failed to hash password: {e}")),
    };

    let mut data = helpers::json_map(serde_json::json!({
        "email": email,
        "password_hash": password_hash,
        "name": name,
        "disabled": false,
        "email_verified": false,
        "avatar_url": "",
        "oauth_provider": "",
        "verification_token": "",
        "reset_token": "",
        "reset_token_expires": null,
        "last_verification_sent": null,
        "last_login_at": null,
        "deleted_at": null,
    }));
    helpers::stamp_created(&mut data);

    let new_user = match db::create(ctx, USERS, data).await {
        Ok(u) => u,
        Err(e) => return err_internal(&format!("Failed: {}", e.message)),
    };

    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "user.invite",
        &format!("users/{}", new_user.id),
        &ip,
    )
    .await;

    // Return the refreshed users-tab content (table + pagination) so it
    // slots into #users-tab-content via innerHTML swap. Trigger toast +
    // close-modal client-side.
    let content = users_tab(ctx, msg, &admin_id).await;
    let trigger = r#"{"showToast":{"message":"User invited","type":"success"},"closeModal":{"id":"invite-user"}}"#;
    ResponseBuilder::new()
        .set_header("HX-Trigger", trigger)
        .body(
            content.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
}

/// POST /b/admin/iam/roles (create role from modal form)
pub async fn handle_create_role(
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let bytes = input.collect_to_bytes().await;
    let body = parse_form_body(&bytes);

    let name = body
        .get("name")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if name.is_empty() {
        return err_bad_request("Role name is required");
    }

    let mut data = std::collections::HashMap::new();
    data.insert("name".to_string(), serde_json::json!(name));
    if let Some(desc) = body.get("description") {
        data.insert("description".to_string(), serde_json::json!(desc));
    }
    helpers::stamp_created(&mut data);

    if let Err(e) = db::create(ctx, ROLES_TABLE, data).await {
        return err_internal(&format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(ctx, &admin_id, "role.create", &format!("roles/{name}"), &ip)
        .await;

    // Return the updated roles tab + close modal + toast
    let content = roles_tab(ctx).await;
    let trigger = r#"{"showToast":{"message":"Role created","type":"success"},"closeModal":{"id":"create-role"}}"#;
    ResponseBuilder::new()
        .set_header("HX-Trigger", trigger)
        .body(
            content.into_string().into_bytes(),
            "text/html; charset=utf-8",
        )
}

pub async fn handle_delete_role(ctx: &dyn Context, msg: &Message, role_id: &str) -> OutputStream {
    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    // Check if system role
    if let Ok(record) = db::get(ctx, ROLES_TABLE, role_id).await {
        if record.bool_field("is_system") {
            return err_forbidden("Cannot delete system role");
        }
    }

    if let Err(e) = db::delete(ctx, ROLES_TABLE, role_id).await {
        return err_internal(&format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "role.delete",
        &format!("roles/{role_id}"),
        &ip,
    )
    .await;

    let content = roles_tab(ctx).await;
    ui::html_response_with_toast(content, "Role deleted", "success")
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
    let result = db::list(ctx, ROLES_TABLE, &opts).await;

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
    let result = db::list(ctx, API_KEYS, &opts).await;

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
