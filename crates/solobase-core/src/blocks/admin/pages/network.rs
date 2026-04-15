use crate::ui::{components, icons, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::OutputStream;
use wafer_sql_utils::value::sea_values_to_json;
use wafer_sql_utils::{query, Backend};

use super::admin_page;
use crate::blocks::admin::{
    NETWORK_REQUEST_LOGS_COLLECTION as NETWORK_REQUEST_LOGS,
    NETWORK_RULES_COLLECTION as NETWORK_RULES,
    REQUEST_LOGS_COLLECTION as REQUEST_LOGS,
};

pub async fn network_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
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

async fn network_inbound_tab(ctx: &dyn Context, msg: &Message) -> Markup {
    let search = msg.query("search").to_string();

    let (where_clause, args) = if search.is_empty() {
        (String::new(), vec![])
    } else {
        (
            " WHERE path LIKE ?1".to_string(),
            vec![serde_json::json!(format!("%{search}%"))],
        )
    };

    // Uses SUM(CASE WHEN...) which is too complex for the grouped query builder.
    let summary = db::query_raw(
        ctx,
        &format!(
            "SELECT method, path, COUNT(*) as cnt, \
             CAST(AVG(duration_ms) AS INTEGER) as avg_ms, \
             SUM(CASE WHEN CAST(status_code AS INTEGER) >= 400 THEN 1 ELSE 0 END) as errors, \
             MAX(created_at) as last_seen \
             FROM {REQUEST_LOGS}{where_clause} \
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
pub async fn network_inbound_detail(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let method = msg.query("method").to_string();
    let path = msg.query("path").to_string();
    let offset: i64 = msg.query("offset").parse().unwrap_or(0);
    let limit: i64 = 20;

    let (sql, vals) = query::build_select_columns(
        REQUEST_LOGS,
        &["status_code", "duration_ms", "client_ip", "user_id", "created_at"],
        &ListOptions {
            filters: vec![
                Filter { field: "method".into(), operator: FilterOp::Equal, value: serde_json::json!(&method) },
                Filter { field: "path".into(), operator: FilterOp::Equal, value: serde_json::json!(&path) },
            ],
            sort: vec![SortField { field: "created_at".into(), desc: true }],
            limit: limit + 1, // fetch one extra to detect "has more"
            offset,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let rows = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
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
    crate::ui::html_response(markup)
}

async fn network_outbound_tab(ctx: &dyn Context, msg: &Message) -> Markup {
    let search = msg.query("search").to_string();

    let (where_clause, args) = if search.is_empty() {
        (String::new(), vec![])
    } else {
        (
            " WHERE url LIKE ?1".to_string(),
            vec![serde_json::json!(format!("%{search}%"))],
        )
    };

    // Uses SUM(CASE WHEN...) which is too complex for the grouped query builder.
    let summary = db::query_raw(
        ctx,
        &format!(
            "SELECT method, url, source_block, COUNT(*) as cnt, \
             CAST(AVG(duration_ms) AS INTEGER) as avg_ms, \
             SUM(CASE WHEN error_message != '' THEN 1 ELSE 0 END) as errors, \
             MAX(created_at) as last_seen \
             FROM {NETWORK_REQUEST_LOGS}{where_clause} \
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
pub async fn network_outbound_detail(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let method = msg.query("method").to_string();
    let url = msg.query("url").to_string();
    let offset: i64 = msg.query("offset").parse().unwrap_or(0);
    let limit: i64 = 20;

    let (sql, vals) = query::build_select_columns(
        NETWORK_REQUEST_LOGS,
        &["status_code", "duration_ms", "source_block", "error_message", "created_at"],
        &ListOptions {
            filters: vec![
                Filter { field: "method".into(), operator: FilterOp::Equal, value: serde_json::json!(&method) },
                Filter { field: "url".into(), operator: FilterOp::Equal, value: serde_json::json!(&url) },
            ],
            sort: vec![SortField { field: "created_at".into(), desc: true }],
            limit: limit + 1,
            offset,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let rows = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
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
                                span .badge .badge-muted { "\u{2014}" }
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
    crate::ui::html_response(markup)
}

pub(crate) async fn network_rules_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    let rules = db::list_all(ctx, NETWORK_RULES, vec![])
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
                        option value="allow" { "Allow \u{2014} permit this block to reach matching URLs" }
                        option value="block" { "Deny \u{2014} block this block from reaching matching URLs" }
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
