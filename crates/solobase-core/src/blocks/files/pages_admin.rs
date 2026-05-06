//! SSR admin pages for the files block.

use maud::{html, Markup};
use wafer_core::clients::{
    database as db,
    database::{Filter, FilterOp, ListOptions, SortField},
};
use wafer_run::{context::Context, types::*, OutputStream};

use super::{BUCKETS_COLLECTION, OBJECTS_COLLECTION, QUOTAS_COLLECTION, SHARES_COLLECTION};
use crate::{
    blocks::helpers::RecordExt,
    ui::{
        components, icons, nav_groups,
        shell::{Crumb, Topbar},
        SiteConfig, UserInfo,
    },
};

fn files_page<'a>(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    crumb_label: &'a str,
    content: Markup,
    msg: &Message,
) -> OutputStream {
    let groups = nav_groups::admin();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: crumb_label,
            href: None,
        }],
        primary_action: None,
        show_palette: true,
    };
    crate::ui::shelled_response(msg, title, config, &groups, user, path, topbar, content)
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AdminStats {
    pub buckets: i64,
    pub files: i64,
    pub total_size_bytes: i64,
    pub shares: i64,
    pub quotas_count: i64,
}

/// Render the 4 stat cards (Buckets, Files, Total Size, Active Shares)
/// for the admin storage overview. Designed for the `list_page` template's
/// `filters` slot. Pure helper — no `Context` access.
pub fn render_admin_overview_stats(stats: &AdminStats) -> Markup {
    html! {
        div .stats-grid {
            (components::stat_card("Buckets", &stats.buckets.to_string(), icons::folder()))
            (components::stat_card("Files", &stats.files.to_string(), icons::file_text()))
            (components::stat_card("Total Size", &format_bytes(stats.total_size_bytes), icons::hard_drive()))
            (components::stat_card("Active Shares", &stats.shares.to_string(), icons::globe()))
        }
    }
}

/// Render the optional "X user(s) with custom quotas" hint card.
/// Returns an empty markup when `quotas_count == 0`. Pure helper.
pub fn render_admin_overview_quotas_hint(quotas_count: i64) -> Markup {
    if quotas_count <= 0 {
        return html! {};
    }
    html! {
        div .card style="padding:1rem" {
            p .text-muted style="font-size:0.875rem" {
                (quotas_count) " user(s) with custom quotas configured."
            }
        }
    }
}

pub async fn overview(ctx: &dyn Context, msg: &Message) -> OutputStream {
    use crate::ui::templates::{list_page, PageHeader};

    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let stats = load_admin_stats(ctx).await;

    let body = list_page(
        PageHeader {
            title: "Storage",
            subtitle: Some("File storage statistics"),
            primary_action: None,
        },
        Some(render_admin_overview_stats(&stats)),
        render_admin_overview_quotas_hint(stats.quotas_count),
        None,
    );

    files_page(
        "Storage",
        &config,
        "/b/storage/admin/",
        user.as_ref(),
        "Overview",
        body,
        msg,
    )
}

async fn load_admin_stats(ctx: &dyn Context) -> AdminStats {
    let one = ListOptions {
        limit: 1,
        ..Default::default()
    };
    let buckets = match db::list(ctx, BUCKETS_COLLECTION, &one).await {
        Ok(r) => r.total_count,
        Err(e) => {
            tracing::warn!(error = %e.message, "admin overview: bucket count failed");
            0
        }
    };

    let complete_only = ListOptions {
        filters: vec![Filter {
            field: "status".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String("complete".into()),
        }],
        limit: 1,
        ..Default::default()
    };
    let files = match db::list(ctx, OBJECTS_COLLECTION, &complete_only).await {
        Ok(r) => r.total_count,
        Err(e) => {
            tracing::warn!(error = %e.message, "admin overview: files count failed");
            0
        }
    };

    let shares = match db::list(ctx, SHARES_COLLECTION, &one).await {
        Ok(r) => r.total_count,
        Err(e) => {
            tracing::warn!(error = %e.message, "admin overview: shares count failed");
            0
        }
    };

    let quotas_count = match db::list(ctx, QUOTAS_COLLECTION, &one).await {
        Ok(r) => r.total_count,
        Err(e) => {
            tracing::warn!(error = %e.message, "admin overview: quotas count failed");
            0
        }
    };

    let total_size_bytes: i64 = match db::list(
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
    {
        Ok(r) => r.records.iter().map(|rec| rec.i64_field("size")).sum(),
        Err(e) => {
            tracing::warn!(error = %e.message, "admin overview: total size sum failed");
            0
        }
    };

    AdminStats {
        buckets,
        files,
        total_size_bytes,
        shares,
        quotas_count,
    }
}

// ---------------------------------------------------------------------------
// Buckets
// ---------------------------------------------------------------------------

pub async fn buckets(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        "Buckets",
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Shares
// ---------------------------------------------------------------------------

pub async fn shares(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        "Shares",
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Quotas
// ---------------------------------------------------------------------------

pub async fn quotas(ctx: &dyn Context, msg: &Message) -> OutputStream {
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
        "Quotas",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_admin_overview_stats_renders_four_stat_cards() {
        let stats = AdminStats {
            buckets: 3,
            files: 42,
            total_size_bytes: 2_500_000_000,
            shares: 5,
            quotas_count: 1,
        };
        let html = render_admin_overview_stats(&stats).into_string();
        assert!(html.contains(">3<"), "buckets count missing: {html}");
        assert!(html.contains(">42<"), "files count missing: {html}");
        assert!(html.contains(">5<"), "shares count missing: {html}");
        // total_size_bytes 2.5 GB → "2.3 GB" via format_bytes (or close).
        assert!(html.contains("GB"), "size humanization missing: {html}");
    }

    #[test]
    fn render_admin_overview_quotas_hint_when_present() {
        let html = render_admin_overview_quotas_hint(3).into_string();
        assert!(
            html.contains("3 user(s) with custom quotas"),
            "quotas hint missing: {html}"
        );
    }

    #[test]
    fn render_admin_overview_quotas_hint_empty_when_zero() {
        let html = render_admin_overview_quotas_hint(0).into_string();
        // Empty markup or no visible "with custom quotas" copy.
        assert!(
            !html.contains("with custom quotas"),
            "should be empty when zero: {html}"
        );
    }
}
