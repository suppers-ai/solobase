//! Database admin page — table browser + schema viewer + SQL editor.
//!
//! Layout: two-pane.
//! * Left pane (~32%): table list with row counts + filter input.
//! * Right pane: tabs (Schema / SQL editor). Schema is the default tab.
//!
//! Backend status badge in the page header.
//!
//! Reuses `wafer_sql_utils::introspect` for table listing/columns and
//! the shared `validate_readonly_query` helper for the SQL editor.

use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::*, OutputStream};
use wafer_sql_utils::{introspect, Backend};

use super::{admin_page, crumb};
use crate::{
    ui::{
        shell::Topbar,
        templates::{list_page, PageHeader},
        SiteConfig, UserInfo,
    },
};

/// Percent-encode a string for use as a URL query value. Conservative:
/// encodes anything outside `[A-Za-z0-9_.-]`. Avoids pulling in a crate
/// for two call sites.
fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Tab the right pane is showing.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Schema,
    Sql,
}

impl Tab {
    fn from_query(q: &str) -> Self {
        match q {
            "sql" => Tab::Sql,
            _ => Tab::Schema,
        }
    }
    fn as_query(self) -> &'static str {
        match self {
            Tab::Schema => "schema",
            Tab::Sql => "sql",
        }
    }
}

/// Lightweight summary for the left-pane list.
struct TableSummary {
    name: String,
    row_count: i64,
}

async fn load_tables(ctx: &dyn Context) -> Vec<TableSummary> {
    let sql = introspect::build_list_tables(Backend::Sqlite);
    let records = db::query_raw(ctx, &sql, &[]).await.unwrap_or_default();
    let mut out = Vec::with_capacity(records.len());
    for r in &records {
        let name = r
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let count_sql = introspect::build_table_row_count(&name, Backend::Sqlite);
        let row_count = db::query_raw(ctx, &count_sql, &[])
            .await
            .ok()
            .and_then(|r| {
                r.first()
                    .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
            })
            .unwrap_or(0);
        out.push(TableSummary { name, row_count });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn backend_badge(table_count: usize) -> Markup {
    html! {
        span .badge .badge-info .text-xs title="Database backend" {
            "SQLite · " (table_count) " tables"
        }
    }
}

fn left_pane(tables: &[TableSummary], selected: Option<&str>, tab: Tab) -> Markup {
    html! {
        aside .db-pane .db-pane--left {
            div .db-pane__head {
                input #db-filter type="text"
                    placeholder="Filter tables…"
                    autocomplete="off"
                    oninput="(function(e){var q=e.target.value.toLowerCase();document.querySelectorAll('[data-db-table]').forEach(function(li){var n=li.getAttribute('data-db-table');li.style.display=n.indexOf(q)>=0?'':'none';});})(event)";
            }
            ul .db-table-list {
                @if tables.is_empty() {
                    li .db-table-list__empty .text-muted .text-sm { "No tables yet" }
                }
                @for t in tables {
                    @let active = selected == Some(t.name.as_str());
                    @let encoded_name = pct_encode(&t.name);
                    li data-db-table=(t.name.to_lowercase()) {
                        a .db-table-list__item .(if active { "is-active" } else { "" })
                            aria-current=[active.then_some("page")]
                            href={"/b/admin/database?table=" (encoded_name) "&tab=" (tab.as_query())}
                            hx-get={"/b/admin/database?table=" (encoded_name) "&tab=" (tab.as_query())}
                            hx-target="#content"
                            hx-push-url="true"
                        {
                            span .db-table-list__name { (t.name) }
                            span .db-table-list__count .text-muted .text-xs { (t.row_count) }
                        }
                    }
                }
            }
        }
    }
}

fn right_pane_tabs(selected: Option<&str>, tab: Tab) -> Markup {
    let table_qs = selected
        .map(|t| format!("&table={}", pct_encode(t)))
        .unwrap_or_default();
    html! {
        nav .tabs {
            a .tab .(if tab == Tab::Schema { "active" } else { "" })
                href={"/b/admin/database?tab=schema" (table_qs)}
                hx-get={"/b/admin/database?tab=schema" (table_qs)}
                hx-target="#content"
                hx-push-url="true"
            { "Schema" }
            a .tab .(if tab == Tab::Sql { "active" } else { "" })
                href={"/b/admin/database?tab=sql" (table_qs)}
                hx-get={"/b/admin/database?tab=sql" (table_qs)}
                hx-target="#content"
                hx-push-url="true"
            { "SQL editor" }
        }
    }
}

async fn schema_panel(ctx: &dyn Context, table: Option<&str>) -> Markup {
    let Some(name) = table else {
        return html! {
            div .empty-state {
                p { "Select a table on the left to view its schema." }
            }
        };
    };

    let (info_sql, info_args) = introspect::build_table_info(name, Backend::Sqlite);
    let columns = db::query_raw(ctx, &info_sql, &info_args)
        .await
        .unwrap_or_default();
    let count_sql = introspect::build_table_row_count(name, Backend::Sqlite);
    let row_count = db::query_raw(ctx, &count_sql, &[])
        .await
        .ok()
        .and_then(|r| {
            r.first()
                .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
        })
        .unwrap_or(0);

    html! {
        div .db-panel {
            header .db-panel__head {
                h3 { (name) }
                span .text-muted .text-sm { (row_count) " rows" }
            }
            @if columns.is_empty() {
                div .empty-state { p { "No columns introspected (table may be empty or backend doesn't support it)." } }
            } @else {
                div .table-container {
                    table .table {
                        thead {
                            tr {
                                th { "Column" }
                                th { "Type" }
                                th { "Not null" }
                                th { "PK" }
                                th { "Default" }
                            }
                        }
                        tbody {
                            @for c in &columns {
                                @let col = c.data.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                @let ty = c.data.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                @let notnull = c.data.get("notnull").and_then(|v| v.as_i64()).unwrap_or(0) == 1;
                                @let pk = c.data.get("pk").and_then(|v| v.as_i64()).unwrap_or(0) == 1;
                                @let dflt = c.data.get("dflt_value").and_then(|v| v.as_str()).unwrap_or("");
                                tr {
                                    td .font-medium { (col) }
                                    td .text-muted { (ty) }
                                    td { @if notnull { "✓" } @else { "" } }
                                    td { @if pk { "✓" } @else { "" } }
                                    td .text-muted .text-sm { (dflt) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn right_pane(ctx: &dyn Context, selected: Option<&str>, tab: Tab) -> Markup {
    html! {
        section .db-pane .db-pane--right {
            (right_pane_tabs(selected, tab))
            div .db-panel-body {
                @match tab {
                    Tab::Schema => (schema_panel(ctx, selected).await),
                    Tab::Sql => (sql_panel(selected, None, None)),
                }
            }
        }
    }
}

/// SQL editor + results placeholder (Task 5 fills this in).
fn sql_panel(_selected: Option<&str>, _query: Option<&str>, _result: Option<Markup>) -> Markup {
    html! {
        div .db-panel {
            p .text-muted { "SQL editor coming next commit." }
        }
    }
}

pub async fn database_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let tables = load_tables(ctx).await;
    let selected = msg.query("table");
    let tab = Tab::from_query(msg.query("tab"));

    let body = list_page(
        PageHeader {
            title: "Database",
            subtitle: Some("Browse tables, view schema, run read-only SQL"),
            primary_action: Some(backend_badge(tables.len())),
        },
        None,
        html! {
            div .db-layout {
                (left_pane(&tables, if selected.is_empty() { None } else { Some(selected) }, tab))
                (right_pane(ctx, if selected.is_empty() { None } else { Some(selected) }, tab).await)
            }
        },
        None,
    );

    admin_page(
        "Database",
        &config,
        "/b/admin/database",
        user.as_ref(),
        Topbar {
            crumbs: crumb("Database"),
            primary_action: None,
            show_palette: true,
        },
        body,
        msg,
    )
}
