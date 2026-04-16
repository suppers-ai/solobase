use maud::html;
use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions};
use wafer_run::{context::Context, types::*, InputStream, OutputStream};
use wafer_sql_utils::{query, upsert, value::sea_values_to_json, Backend};

use super::admin_page;
use crate::{
    blocks::admin::BLOCK_SETTINGS_COLLECTION as BLOCK_SETTINGS,
    ui::{self, components, icons, SiteConfig, UserInfo},
};

pub async fn blocks_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "services" => "services",
        "infrastructure" => "infrastructure",
        "custom" => "custom",
        _ => "features",
    };

    let registered_blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();

    // Load block enabled/disabled state from block_settings table
    let block_settings_rows = db::list_all(ctx, BLOCK_SETTINGS, vec![])
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
                "(enabled \u{2014} restart to load)"
            } else {
                "(disabled \u{2014} restart to load)"
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
                div style="display:flex;gap:8px" {
                    a .btn .btn-sm href="https://wafer.run/registry" target="_blank"
                        style="display:inline-flex;align-items:center;gap:4px;background:#8b5cf6;color:#fff;border:none"
                    {
                        (icons::arrow_up_right()) " Explore WASM blocks"
                    }
                    a .btn .btn-primary .btn-sm href="/debug/inspector/ui" target="_blank" {
                        (icons::globe()) " Open Inspector"
                    }
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
            a .tab .(if active_tab == "custom" { "active" } else { "" })
                href="/b/admin/blocks?tab=custom"
                hx-get="/b/admin/blocks?tab=custom"
                hx-target="#content"
                hx-push-url="true"
            { (icons::package()) " Custom" }
        }

        div #blocks-tab-content {
            @if active_tab == "custom" {
                (custom_tab_content())
            } @else {
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
                                    span style="color:#10b981;font-size:14px" title="Enabled" { "\u{2713}" }
                                } @else {
                                    span style="color:#94a3b8;font-size:14px" title="Disabled" { "\u{2717}" }
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

/// POST /b/admin/blocks/{name}/toggle -- toggle a block's enabled state
pub async fn handle_toggle_feature(
    ctx: &dyn Context,
    msg: &Message,
    block_name: &str,
) -> OutputStream {
    // Read current state from block_settings
    let (sql, vals) = query::build_select_columns(
        BLOCK_SETTINGS,
        &["enabled"],
        &ListOptions {
            filters: vec![Filter {
                field: "block_name".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(block_name),
            }],
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let current_enabled = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
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
    let now = chrono::Utc::now().to_rfc3339();
    let (sql, vals) = upsert::build_upsert(
        BLOCK_SETTINGS,
        &[
            ("block_name".to_string(), serde_json::json!(block_name)),
            ("enabled".to_string(), serde_json::json!(new_enabled_int)),
            ("created_at".to_string(), serde_json::json!(&now)),
            ("updated_at".to_string(), serde_json::json!(&now)),
        ],
        &["block_name"],
        &["enabled", "updated_at"],
        Backend::Sqlite,
    );
    let _ = db::exec_raw(ctx, &sql, &sea_values_to_json(vals)).await;

    let admin_id = msg.user_id().to_string();
    let ip = msg.remote_addr().to_string();
    let action = if new_enabled {
        "block.enable"
    } else {
        "block.disable"
    };
    super::super::logs::audit_log(ctx, &admin_id, action, &format!("blocks/{block_name}"), &ip)
        .await;

    // Re-render the blocks page
    blocks_page(ctx, msg).await
}

/// GET /b/admin/blocks/{name}/detail -- block detail modal content
pub async fn handle_block_detail(
    ctx: &dyn Context,
    _msg: &Message,
    block_name: &str,
) -> OutputStream {
    let blocks: Vec<wafer_run::BlockInfo> = ctx.registered_blocks();
    let block_opt = blocks.iter().find(|b| b.name == block_name);

    // Check block enabled state from block_settings
    let (sql, vals) = query::build_select_columns(
        BLOCK_SETTINGS,
        &["enabled"],
        &ListOptions {
            filters: vec![Filter {
                field: "block_name".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(block_name),
            }],
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let is_enabled = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .ok()
        .and_then(|rows| {
            rows.first()
                .and_then(|r| r.data.get("enabled").and_then(|v| v.as_i64()))
        })
        .map(|v| v != 0)
        .unwrap_or(true);

    let encoded = block_name.replace('/', "--");

    // Disabled block not in runtime -- show minimal modal with toggle
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
        return ui::html_response(markup);
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
                                    td .text-sm { code style="font-size:11px" { @if ck.default.is_empty() { "\u{2014}" } @else { (ck.default) } } }
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

    ui::html_response(markup)
}

// ---------------------------------------------------------------------------
// Custom tab helpers
// ---------------------------------------------------------------------------

fn custom_tab_content() -> maud::Markup {
    html! {
        // Install from Registry
        div .card style="margin-bottom:16px" {
            h3 style="font-size:14px;font-weight:600;margin:0 0 12px" {
                (icons::arrow_up_right()) " Install from Registry"
            }
            p .text-muted style="font-size:13px;margin:0 0 12px" {
                "Enter a manifest URL from the "
                a href="https://wafer.run/registry" target="_blank" { "WAFER registry" }
                " to install a custom WASM block."
            }
            form
                hx-post="/b/admin/custom-blocks/install"
                hx-target="#custom-blocks-list"
                hx-swap="outerHTML"
                style="display:flex;gap:8px;align-items:flex-end"
            {
                div style="flex:1" {
                    label style="font-size:12px;color:#64748b;display:block;margin-bottom:4px" {
                        "Manifest URL"
                    }
                    input .form-input type="text" name="manifest_url"
                        placeholder="https://wafer.run/registry/org/block/manifest.json"
                        style="width:100%";
                }
                button .btn .btn-primary type="submit" {
                    (icons::arrow_up_right()) " Install"
                }
            }
        }

        // Upload .wasm
        div .card style="margin-bottom:16px" {
            h3 style="font-size:14px;font-weight:600;margin:0 0 12px" {
                (icons::hard_drive()) " Upload .wasm"
            }
            p .text-muted style="font-size:13px;margin:0 0 12px" {
                "Upload a compiled .wasm block directly. The block name will be derived from the filename."
            }
            form
                hx-post="/b/admin/custom-blocks/upload"
                hx-target="#custom-blocks-list"
                hx-swap="outerHTML"
                hx-encoding="multipart/form-data"
                style="display:flex;gap:8px;align-items:flex-end"
            {
                div style="flex:1" {
                    label style="font-size:12px;color:#64748b;display:block;margin-bottom:4px" {
                        "WASM file"
                    }
                    input .form-input type="file" name="wasm_file" accept=".wasm"
                        style="width:100%";
                }
                button .btn .btn-primary type="submit" {
                    (icons::arrow_up_right()) " Upload"
                }
            }
        }

        // Installed custom blocks list (initially empty placeholder)
        div #custom-blocks-list {
            (custom_blocks_list(&[]))
        }
    }
}

/// Render the installed custom blocks table. `blocks` is a slice of
/// `(name, version, uploaded_at)` tuples coming from the backend.
pub fn custom_blocks_list(blocks: &[(&str, &str, &str)]) -> maud::Markup {
    html! {
        div #custom-blocks-list {
            h3 style="font-size:14px;font-weight:600;margin:0 0 12px" {
                (icons::package()) " Installed Custom Blocks"
            }
            @if blocks.is_empty() {
                div .card style="text-align:center;padding:32px;color:#94a3b8" {
                    p style="margin:0" { "No custom blocks installed yet." }
                    p style="margin:8px 0 0;font-size:13px" {
                        "Use the forms above to install from the registry or upload a .wasm file."
                    }
                }
            } @else {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Name" }
                                th { "Version" }
                                th { "Uploaded" }
                                th style="width:120px" { "Status" }
                                th style="width:80px" { "" }
                            }
                        }
                        tbody {
                            @for (name, version, uploaded_at) in blocks {
                                @let encoded = name.replace('/', "--");
                                tr {
                                    td .text-sm { code style="font-size:12px" { (name) } }
                                    td .text-sm .text-muted { "v" (version) }
                                    td .text-sm .text-muted { (uploaded_at) }
                                    td {
                                        label .toggle {
                                            input type="checkbox"
                                                checked
                                                hx-post={"/b/admin/blocks/" (encoded) "/toggle"}
                                                hx-target="#content";
                                            span .toggle-slider {}
                                        }
                                    }
                                    td {
                                        button .btn .btn-sm
                                            style="background:#fce4ec;color:#c62828;border:none"
                                            hx-delete={"/b/admin/custom-blocks/" (encoded)}
                                            hx-target="#custom-blocks-list"
                                            hx-swap="outerHTML"
                                            hx-confirm={"Delete custom block " (name) "?"}
                                        {
                                            (icons::trash())
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

// ---------------------------------------------------------------------------
// Custom block endpoint handlers
// ---------------------------------------------------------------------------

/// POST /b/admin/custom-blocks/install — install a block from a manifest URL
pub async fn handle_custom_block_install(
    _ctx: &dyn Context,
    _msg: &Message,
    input: InputStream,
) -> OutputStream {
    use crate::blocks::helpers::parse_form_body;
    let body = input.collect_to_bytes().await;
    let form = parse_form_body(&body);
    let manifest_url = form.get("manifest_url").cloned().unwrap_or_default();

    if manifest_url.is_empty() {
        let markup = html! {
            div #custom-blocks-list {
                div .card style="color:#c62828;padding:16px" {
                    "Error: manifest URL is required."
                }
            }
        };
        return ui::html_response(markup);
    }

    // TODO(cloud): delegate to /_internal/blocks/install when running in cloud mode.
    // For local deployment, custom blocks are auto-discovered from the blocks/ directory.
    let markup = html! {
        div #custom-blocks-list {
            div .card style="padding:16px;background:#fef3c7;color:#92400e;border-left:3px solid #f59e0b" {
                p style="margin:0 0 8px;font-weight:600" { "Local deployment" }
                p style="margin:0;font-size:13px" {
                    "Custom blocks are auto-discovered from the "
                    code { "blocks/" }
                    " directory. Use "
                    code { "wafer build" }
                    " to compile blocks locally, then restart the server."
                }
            }
        }
    };
    ui::html_response(markup)
}

/// POST /b/admin/custom-blocks/upload — upload a .wasm file
pub async fn handle_custom_block_upload(
    _ctx: &dyn Context,
    _msg: &Message,
    _input: InputStream,
) -> OutputStream {
    // TODO(cloud): parse multipart body and store to R2/D1 when running in cloud mode.
    // For local deployment, point users to the blocks/ directory workflow.
    let markup = html! {
        div #custom-blocks-list {
            div .card style="padding:16px;background:#fef3c7;color:#92400e;border-left:3px solid #f59e0b" {
                p style="margin:0 0 8px;font-weight:600" { "Local deployment" }
                p style="margin:0;font-size:13px" {
                    "Custom blocks are auto-discovered from the "
                    code { "blocks/" }
                    " directory. Place your compiled "
                    code { ".wasm" }
                    " file there and restart the server. Use "
                    code { "wafer build" }
                    " to compile blocks locally."
                }
            }
        }
    };
    ui::html_response(markup)
}

/// DELETE /b/admin/custom-blocks/{name} — delete a custom block
pub async fn handle_custom_block_delete(
    _ctx: &dyn Context,
    _msg: &Message,
    _block_name: &str,
) -> OutputStream {
    // TODO(cloud): delete from R2/D1 when running in cloud mode.
    let markup = custom_blocks_list(&[]);
    ui::html_response(markup)
}
