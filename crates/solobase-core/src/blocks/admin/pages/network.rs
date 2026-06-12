use maud::{html, Markup};
use wafer_block::db::{Filter, FilterOp, ListOptions, SortField};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, Message, OutputStream};
use wafer_sql_utils::{query, Backend};

use crate::{
    blocks::admin::REQUEST_LOGS_TABLE as REQUEST_LOGS,
    ui::{components, icons},
};

/// Render JUST the network monitoring body. The parent `settings_page`
/// handler wraps this in `form_page` + the shell.
pub async fn settings_body(ctx: &dyn Context, msg: &Message) -> Markup {
    html! {
        div .filter-bar style="margin-bottom:0.5rem" {
            button .btn .btn-secondary .btn-sm
                hx-get="/b/admin/settings/network"
                hx-target="#content"
            { (icons::refresh_cw()) " Refresh" }
        }

        div .tabs {
            a .tab .active
                href="/b/admin/settings/network"
                hx-get="/b/admin/settings/network"
                hx-target="#content"
                hx-push-url="true"
            { (icons::arrow_down_left()) " Inbound" }
        }

        div #network-tab-content {
            (network_inbound_tab(ctx, msg).await)
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

    let stmt = aggregate::build_grouped_query(
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
    let summary = db::query(ctx, &stmt).await.unwrap_or_default();

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

    let stmt = query::build_select_columns(
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
    let rows = db::query(ctx, &stmt).await.unwrap_or_default();

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
