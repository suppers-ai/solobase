use crate::ui::{components, icons, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::OutputStream;

use super::admin_page;
use super::network::network_rules_tab;
use super::storage::storage_rules_tab;
use crate::blocks::admin::{
    NETWORK_RULES_COLLECTION as NETWORK_RULES,
    STORAGE_RULES_COLLECTION as STORAGE_RULES,
    WRAP_GRANTS_COLLECTION as WRAP_GRANTS,
};

pub async fn grants_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    // Redirect to the unified Permissions page
    permissions_page(ctx, msg).await
}

pub async fn permissions_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
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

pub(crate) async fn grants_custom_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
    let grants = db::list_all(ctx, WRAP_GRANTS, vec![])
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
async fn permissions_all_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
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
    let custom_grants = db::list_all(ctx, WRAP_GRANTS, vec![])
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
    let storage_rules = db::list_all(ctx, STORAGE_RULES, vec![])
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
    let network_rules = db::list_all(ctx, NETWORK_RULES, vec![])
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
async fn permissions_database_tab(ctx: &dyn Context, msg: &Message) -> Markup {
    html! {
        (grants_custom_tab(ctx, msg).await)
        (grants_code_tab(ctx))
    }
}

/// "Storage" tab: delegates to the existing storage_rules_tab.
async fn permissions_storage_tab(ctx: &dyn Context, msg: &Message) -> Markup {
    storage_rules_tab(ctx, msg).await
}

/// "Network" tab: delegates to the existing network_rules_tab.
async fn permissions_network_tab(ctx: &dyn Context, msg: &Message) -> Markup {
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
                    option value="allow" { "Allow \u{2014} grant cross-block access" }
                    option value="block" { "Block \u{2014} deny access to matching paths" }
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
                    option value="block" { "Block \u{2014} deny matching URLs" }
                    option value="allow" { "Allow \u{2014} only permit matching URLs" }
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
