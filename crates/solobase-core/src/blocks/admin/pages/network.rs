use maud::{html, Markup};
use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions, SortField};
use wafer_run::{context::Context, types::*, OutputStream};
use wafer_sql_utils::{query, value::sea_values_to_json, Backend};

use crate::{
    blocks::admin::{NETWORK_RULES_TABLE as NETWORK_RULES, REQUEST_LOGS_TABLE as REQUEST_LOGS},
    ui::{components, icons},
};

/// Render JUST the network monitoring body. The parent `settings_page`
/// handler wraps this in `form_page` + the shell.
pub async fn settings_body(ctx: &dyn Context, msg: &Message) -> Markup {
    let tab = msg.query("tab");
    let active_tab = match tab {
        "rules" => "rules",
        _ => "inbound",
    };

    html! {
        div .filter-bar style="margin-bottom:0.5rem" {
            button .btn .btn-secondary .btn-sm
                hx-get={"/b/admin/settings/network?tab=" (active_tab)}
                hx-target="#content"
            { (icons::refresh_cw()) " Refresh" }
        }

        div .tabs {
            a .tab .(if active_tab == "inbound" { "active" } else { "" })
                href="/b/admin/settings/network"
                hx-get="/b/admin/settings/network"
                hx-target="#content"
                hx-push-url="true"
            { (icons::arrow_down_left()) " Inbound" }
        }

        @if active_tab == "rules" {
            div .card .mt-4 style="background:#f0f9ff;border-color:#bae6fd" {
                p style="padding:12px;margin:0;font-size:13px" {
                    (icons::info()) " Network permissions have moved to the "
                    a href="/b/admin/permissions?subtab=network" { "Permissions" }
                    " page."
                }
            }
        }

        div #network-tab-content {
            @if active_tab == "inbound" {
                (network_inbound_tab(ctx, msg).await)
            } @else {
                // rules tab still renders content but with banner above
                (network_rules_tab(ctx, msg).await)
            }
        }
    }
}

async fn network_inbound_tab(ctx: &dyn Context, msg: &Message) -> Markup {
    use sea_query::{Alias, Expr, ExprTrait};
    use wafer_sql_utils::{
        aggregate::{self, AggFunc, AggregateColumn, GroupedQueryConfig},
        ident::DynCol,
    };

    let search = msg.query("search").to_string();

    let filters = if search.is_empty() {
        vec![]
    } else {
        vec![Filter {
            field: "path".into(),
            operator: FilterOp::Like,
            value: serde_json::json!(format!("%{search}%")),
        }]
    };

    // status_code is stored as TEXT, so the conditional SUM has to cast
    // before the comparison; mirrors the previous hand-written SQL.
    let status_code_int = Expr::col(DynCol("status_code".into())).cast_as(Alias::new("INTEGER"));

    let (sql, vals) = aggregate::build_grouped_query(
        GroupedQueryConfig {
            table: REQUEST_LOGS.to_string(),
            select_columns: vec!["method".into(), "path".into()],
            aggregates: vec![
                AggregateColumn {
                    func: AggFunc::Count,
                    field: None,
                    alias: "cnt".into(),
                    cast_as: None,
                    inner_expr: None,
                },
                AggregateColumn {
                    func: AggFunc::Avg,
                    field: Some("duration_ms".into()),
                    alias: "avg_ms".into(),
                    cast_as: Some("INTEGER".into()),
                    inner_expr: None,
                },
                AggregateColumn::case_when_sum("errors", status_code_int.gte(400)),
                AggregateColumn {
                    func: AggFunc::Max,
                    field: Some("created_at".into()),
                    alias: "last_seen".into(),
                    cast_as: None,
                    inner_expr: None,
                },
            ],
            filters,
            group_by: vec!["method".into(), "path".into()],
            order_by: vec![SortField {
                field: "cnt".into(),
                desc: true,
            }],
            limit: Some(50),
        },
        Backend::Sqlite,
    );
    let summary = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .unwrap_or_default();

    html! {
        div .filter-bar {
            (components::search_input_with_value("search", "Search by path...", "/b/admin/settings/network", "#content", &search))
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
        &[
            "status_code",
            "duration_ms",
            "client_ip",
            "user_id",
            "created_at",
        ],
        &ListOptions {
            filters: vec![
                Filter {
                    field: "method".into(),
                    operator: FilterOp::Equal,
                    value: serde_json::json!(&method),
                },
                Filter {
                    field: "path".into(),
                    operator: FilterOp::Equal,
                    value: serde_json::json!(&path),
                },
            ],
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
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
