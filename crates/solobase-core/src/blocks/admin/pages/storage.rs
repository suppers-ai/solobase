use maud::{html, Markup};
use wafer_block::db::{ListOptions, SortField};
use wafer_core::clients::database as db;
use wafer_run::{context::Context, Message, OutputStream};
use wafer_sql_utils::{query, Backend};

use super::{admin_page, crumb};
use crate::{
    blocks::admin::STORAGE_ACCESS_LOGS_TABLE as STORAGE_ACCESS_LOGS,
    ui::{
        icons,
        shell::Topbar,
        templates::{list_page, PageHeader},
        SiteConfig, UserInfo,
    },
};

pub async fn storage_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let refresh_action = html! {
        button .btn .btn-secondary .btn-sm
            hx-get="/b/admin/storage"
            hx-target="#content"
        { (icons::refresh_cw()) " Refresh" }
    };

    let tabs_and_body = html! {
        div .tabs {
            a .tab .active
                href="/b/admin/storage"
                hx-get="/b/admin/storage"
                hx-target="#content"
                hx-push-url="true"
            { (icons::eye()) " Access Logs" }
        }

        div #storage-tab-content {
            (storage_logs_tab(ctx, msg).await)
        }
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        None,
        tabs_and_body,
        None,
    );

    admin_page(
        "Storage",
        &config,
        "/b/admin/storage",
        user.as_ref(),
        Topbar {
            crumbs: crumb("Storage"),
            primary_action: Some(refresh_action),
            subtitle: Some("Per-block storage isolation and access logs"),
            show_palette: true,
        },
        body,
        msg,
    )
}

async fn storage_logs_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
    let stmt = query::build_select_columns(
        STORAGE_ACCESS_LOGS,
        &["source_block", "operation", "path", "status", "created_at"],
        &ListOptions {
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
            limit: 100,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let logs = db::query(ctx, &stmt).await.unwrap_or_default();

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
