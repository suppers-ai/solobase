use crate::blocks::helpers::{parse_form_body, RecordExt};
use crate::ui::{self, components, icons, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database::{self as db, ListOptions};
use wafer_run::context::Context;
use wafer_run::types::*;

use super::admin_page;
use crate::blocks::admin::VARIABLES_COLLECTION as VARIABLES;

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

/// "All Variables" tab -- flat table of all config variables from the DB.
async fn config_all_tab(ctx: &dyn Context) -> Markup {
    let opts = ListOptions {
        limit: 200,
        ..Default::default()
    };
    let settings = db::list(ctx, VARIABLES, &opts).await;

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
                                                "\u{26a0} " (warning)
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

/// "By Block" tab -- groups config variables by owning block with WRAP access info.
async fn config_by_block_tab(ctx: &dyn Context) -> Markup {
    let blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();
    let shared_vars = crate::config_vars::shared_config_vars();

    // Load all variables from DB
    let all_vars = db::list_all(ctx, VARIABLES, vec![])
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
                        " \u{2014} Admin can read/write all. "
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

        // Unowned variables section -- keys in DB not declared by any block or shared
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

/// POST /b/admin/variables -- create a new variable
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

    if let Err(e) = db::create(ctx, VARIABLES, data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
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

/// GET /b/admin/variables/{key}/edit -- return modal edit form content
pub async fn handle_edit_variable_form(
    ctx: &dyn Context,
    msg: &mut Message,
    var_key: &str,
) -> Result_ {
    let record = match db::get_by_field(
        ctx,
        VARIABLES,
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
                        "\u{26a0} " (warning)
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

/// PUT /b/admin/variables/{key} -- update variable value
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
        VARIABLES,
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

    if let Err(e) = db::update(ctx, VARIABLES, &record.id, data).await {
        return wafer_run::helpers::err_internal(msg, &format!("Failed: {}", e.message));
    }
    super::super::logs::audit_log(
        ctx,
        &admin_id,
        "variable.update",
        &format!("variables/{var_key}"),
        &ip,
    )
    .await;

    variables_page(ctx, msg).await
}
