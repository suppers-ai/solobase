//! SSR admin pages for the files block.

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup};
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::types::*;

const BUCKETS_COLLECTION: &str = "suppers_ai__files__buckets";
const OBJECTS_COLLECTION: &str = "suppers_ai__files__objects";
const SHARES_COLLECTION: &str = "suppers_ai__files__cloud_shares";
const QUOTAS_COLLECTION: &str = "suppers_ai__files__cloud_quotas";

fn files_admin_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Overview".into(),
            href: "/b/storage/admin/".into(),
            icon: "bar-chart",
        },
        NavItem {
            label: "Buckets".into(),
            href: "/b/storage/admin/buckets".into(),
            icon: "folder",
        },
        NavItem {
            label: "Shares".into(),
            href: "/b/storage/admin/shares".into(),
            icon: "globe",
        },
        NavItem {
            label: "Quotas".into(),
            href: "/b/storage/admin/quotas".into(),
            icon: "bar-chart",
        },
    ]
}

fn files_page(
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
        &files_admin_nav(),
        user,
        path,
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

pub async fn overview(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let one = ListOptions {
        limit: 1,
        ..Default::default()
    };

    let buckets_count = db::list(ctx, BUCKETS_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let complete_only = ListOptions {
        filters: vec![Filter {
            field: "status".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("complete".into()),
        }],
        limit: 1,
        ..Default::default()
    };
    let objects_count = db::list(ctx, OBJECTS_COLLECTION, &complete_only)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let shares_count = db::list(ctx, SHARES_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let quotas_count = db::list(ctx, QUOTAS_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);

    // Total storage size (only complete uploads)
    let total_size: i64 = db::list(
        ctx,
        OBJECTS_COLLECTION,
        &ListOptions {
            filters: vec![Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::Value::String("complete".into()),
            }],
            limit: 10000,
            ..Default::default()
        },
    )
    .await
    .map(|r| r.records.iter().map(|rec| rec.i64_field("size")).sum())
    .unwrap_or(0);

    let size_display = if total_size > 1_073_741_824 {
        format!("{:.1} GB", total_size as f64 / 1_073_741_824.0)
    } else if total_size > 1_048_576 {
        format!("{:.1} MB", total_size as f64 / 1_048_576.0)
    } else if total_size > 1024 {
        format!("{:.1} KB", total_size as f64 / 1024.0)
    } else {
        format!("{} B", total_size)
    };

    let content = html! {
        (components::page_header("Storage Overview", Some("File storage statistics"), None))
        div .stats-grid {
            (components::stat_card("Buckets", &buckets_count.to_string(), icons::folder()))
            (components::stat_card("Files", &objects_count.to_string(), icons::file_text()))
            (components::stat_card("Total Size", &size_display, icons::hard_drive()))
            (components::stat_card("Active Shares", &shares_count.to_string(), icons::globe()))
        }

        @if quotas_count > 0 {
            div .card style="margin-top:1rem;padding:1rem" {
                p .text-muted style="font-size:0.875rem" {
                    (quotas_count) " user(s) with custom quotas configured."
                }
            }
        }
    };

    files_page(
        "Storage",
        &config,
        "/b/storage/admin/",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Buckets
// ---------------------------------------------------------------------------

pub async fn buckets(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".into(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, BUCKETS_COLLECTION, &opts).await;

    let content = html! {
        (components::page_header("Buckets", Some("All storage buckets"), None))

        div #buckets-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "Owner" } th { "Public" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No buckets" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
                                        td .text-muted .text-sm { (r.str_field("created_by").get(..8).unwrap_or("—")) }
                                        td { (components::status_badge(if r.str_field("public") == "true" { "public" } else { "private" })) }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    files_page(
        "Buckets",
        &config,
        "/b/storage/admin/buckets",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Shares
// ---------------------------------------------------------------------------

pub async fn shares(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".into(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, SHARES_COLLECTION, &opts).await;

    let content = html! {
        (components::page_header("Active Shares", Some("Public file share links"), None))

        div #shares-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Token" } th { "Bucket" } th { "File" } th { "Access Count" } th { "Expires" } th { "Created By" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="6" .text-center .text-muted style="padding:2rem;" { "No active shares" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .text-sm { code { (r.str_field("token").get(..12).unwrap_or("—")) "..." } }
                                        td .font-medium { (r.str_field("bucket")) }
                                        td .text-sm { (r.str_field("key")) }
                                        td .text-sm {
                                            (r.i64_field("access_count"))
                                            @let max = r.str_field("max_access_count");
                                            @if !max.is_empty() && max != "0" {
                                                " / " (max)
                                            }
                                        }
                                        td .text-muted .text-sm {
                                            @let exp = r.str_field("expires_at");
                                            @if exp.is_empty() { "Never" } @else { (exp.get(..10).unwrap_or(exp)) }
                                        }
                                        td .text-muted .text-sm { (r.str_field("created_by").get(..8).unwrap_or("—")) }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    files_page(
        "Shares",
        &config,
        "/b/storage/admin/shares",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Quotas
// ---------------------------------------------------------------------------

pub async fn quotas(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let opts = ListOptions {
        sort: vec![SortField {
            field: "created_at".into(),
            desc: true,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, QUOTAS_COLLECTION, &opts).await;

    let content = html! {
        (components::page_header("Storage Quotas", Some("Per-user storage limits"), None))

        div #quotas-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "User" } th { "Max Storage" } th { "Max File Size" } th { "Max Files/Bucket" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" {
                                        "No custom quotas. Default: 1 GB storage, 100 MB file size, 10,000 files per bucket."
                                    } }
                                }
                                @for r in &list.records {
                                    @let max_storage = r.i64_field("max_storage_bytes");
                                    @let max_file = r.i64_field("max_file_size_bytes");
                                    tr {
                                        td .text-sm { (r.str_field("user_id").get(..8).unwrap_or("—")) }
                                        td .text-sm { (format_bytes(max_storage)) }
                                        td .text-sm { (format_bytes(max_file)) }
                                        td .text-sm { (r.i64_field("max_files_per_bucket")) }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    files_page(
        "Quotas",
        &config,
        "/b/storage/admin/quotas",
        user.as_ref(),
        content,
        msg,
    )
}

fn format_bytes(bytes: i64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
