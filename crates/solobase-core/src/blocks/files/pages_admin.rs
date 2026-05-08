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

/// Tabs navigation across the storage-admin sub-pages
/// (Overview / Buckets / Shares / Quotas). `active` matches the
/// crumb label so the active tab can be highlighted.
fn admin_tabs(active: &str) -> Markup {
    let items: &[(&str, &str)] = &[
        ("Overview", "/b/storage/admin/"),
        ("Buckets", "/b/storage/admin/buckets"),
        ("Shares", "/b/storage/admin/shares"),
        ("Quotas", "/b/storage/admin/quotas"),
    ];
    html! {
        div .tabs {
            @for (label, href) in items {
                a class={ "tab" @if *label == active { " active" } } href=(href) { (label) }
            }
        }
    }
}

fn files_page<'a>(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    crumb_label: &'a str,
    subtitle: Option<&'a str>,
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
        subtitle,
        show_palette: true,
    };
    let body_with_tabs = html! {
        (admin_tabs(crumb_label))
        (content)
    };
    crate::ui::shelled_response(msg, title, config, &groups, user, path, topbar, body_with_tabs)
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
            title: "",
            subtitle: None,
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
        Some("File storage statistics"),
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

#[derive(Clone, Debug)]
pub struct AdminBucketRow {
    pub name: String,
    pub owner_short: String,
    pub public: bool,
    pub created_at_short: String,
}

/// Render the admin Buckets table (or empty state).
pub fn render_admin_buckets_table(rows: &[AdminBucketRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state { p { "No buckets" } }
        };
    }
    html! {
        table .data-table {
            thead { tr {
                th { "Name" }
                th { "Owner" }
                th { "Public" }
                th { "Created" }
            } }
            tbody {
                @for r in rows {
                    tr data-bucket=(r.name) {
                        td data-label="Name" .font-medium { (r.name) }
                        td data-label="Owner" .text-muted .text-sm { (r.owner_short) }
                        td data-label="Public" {
                            (components::status_badge(if r.public { "public" } else { "private" }))
                        }
                        td data-label="Created" .text-muted .text-sm { (r.created_at_short) }
                    }
                }
            }
        }
    }
}

pub async fn buckets(ctx: &dyn Context, msg: &Message) -> OutputStream {
    use crate::ui::templates::{list_page, PageHeader};

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

    let rows: Vec<AdminBucketRow> = match db::list(ctx, BUCKETS_COLLECTION, &opts).await {
        Ok(list) => list
            .records
            .into_iter()
            .map(|r| AdminBucketRow {
                name: r.str_field("name").to_string(),
                owner_short: r
                    .str_field("created_by")
                    .get(..8)
                    .unwrap_or("—")
                    .to_string(),
                public: r.str_field("public") == "true",
                created_at_short: r
                    .str_field("created_at")
                    .get(..10)
                    .unwrap_or("")
                    .to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e.message, "admin bucket list failed");
            Vec::new()
        }
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        None,
        render_admin_buckets_table(&rows),
        None,
    );

    files_page(
        "Buckets",
        &config,
        "/b/storage/admin/buckets",
        user.as_ref(),
        "Buckets",
        Some("All storage buckets"),
        body,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Shares
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AdminShareRow {
    pub token_short: String,
    pub bucket: String,
    pub key: String,
    pub access_count: i64,
    pub max_access_count: Option<i64>,
    pub expires_short: Option<String>,
    pub owner_short: String,
}

/// Render the admin Shares table (or empty state). Token displayed as
/// short prefix in a `<code>` block; access count includes optional
/// "/ N" divisor when a max is set; "Never" renders for unset expires.
pub fn render_admin_shares_table(rows: &[AdminShareRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state { p { "No active shares" } }
        };
    }
    html! {
        table .data-table {
            thead { tr {
                th { "Token" }
                th { "Bucket" }
                th { "File" }
                th { "Access Count" }
                th { "Expires" }
                th { "Created By" }
            } }
            tbody {
                @for r in rows {
                    tr data-share-token=(r.token_short) {
                        td data-label="Token" .text-sm { code { (r.token_short) "..." } }
                        td data-label="Bucket" .font-medium { (r.bucket) }
                        td data-label="File" .text-sm { (r.key) }
                        td data-label="Access Count" .text-sm {
                            (r.access_count)
                            @if let Some(max) = r.max_access_count {
                                @if max > 0 { " / " (max) }
                            }
                        }
                        td data-label="Expires" .text-muted .text-sm {
                            @if let Some(exp) = &r.expires_short { (exp) } @else { "Never" }
                        }
                        td data-label="Created By" .text-muted .text-sm { (r.owner_short) }
                    }
                }
            }
        }
    }
}

pub async fn shares(ctx: &dyn Context, msg: &Message) -> OutputStream {
    use crate::ui::templates::{list_page, PageHeader};

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

    let rows: Vec<AdminShareRow> = match db::list(ctx, SHARES_COLLECTION, &opts).await {
        Ok(list) => list
            .records
            .into_iter()
            .map(|r| {
                let max_str = r.str_field("max_access_count");
                let max = if max_str.is_empty() {
                    None
                } else {
                    max_str.parse::<i64>().ok().filter(|n| *n > 0)
                };
                let exp_str = r.str_field("expires_at");
                let expires_short = if exp_str.is_empty() {
                    None
                } else {
                    Some(exp_str.get(..10).unwrap_or(exp_str).to_string())
                };
                AdminShareRow {
                    token_short: r.str_field("token").get(..12).unwrap_or("—").to_string(),
                    bucket: r.str_field("bucket").to_string(),
                    key: r.str_field("key").to_string(),
                    access_count: r.i64_field("access_count"),
                    max_access_count: max,
                    expires_short,
                    owner_short: r
                        .str_field("created_by")
                        .get(..8)
                        .unwrap_or("—")
                        .to_string(),
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e.message, "admin shares list failed");
            Vec::new()
        }
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        None,
        render_admin_shares_table(&rows),
        None,
    );

    files_page(
        "Shares",
        &config,
        "/b/storage/admin/shares",
        user.as_ref(),
        "Shares",
        Some("Public file share links"),
        body,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Quotas
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AdminQuotaRow {
    pub user_short: String,
    pub max_storage_bytes: i64,
    pub max_file_size_bytes: i64,
    pub max_files_per_bucket: i64,
}

/// Render the admin Storage Quotas table (or empty state). Bytes
/// columns humanize via `format_bytes`; user_id is truncated to the
/// first 8 chars in the loader. Pure helper.
pub fn render_admin_quotas_table(rows: &[AdminQuotaRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state {
                p { "No custom quotas. Default: 1 GB storage, 100 MB file size, 10,000 files per bucket." }
            }
        };
    }
    html! {
        table .data-table {
            thead { tr {
                th { "User" }
                th { "Max Storage" }
                th { "Max File Size" }
                th { "Max Files/Bucket" }
            } }
            tbody {
                @for r in rows {
                    tr {
                        td data-label="User" .text-sm { (r.user_short) }
                        td data-label="Max Storage" .text-sm { (format_bytes(r.max_storage_bytes)) }
                        td data-label="Max File Size" .text-sm { (format_bytes(r.max_file_size_bytes)) }
                        td data-label="Max Files/Bucket" .text-sm { (r.max_files_per_bucket) }
                    }
                }
            }
        }
    }
}

pub async fn quotas(ctx: &dyn Context, msg: &Message) -> OutputStream {
    use crate::ui::templates::{list_page, PageHeader};

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

    let rows: Vec<AdminQuotaRow> = match db::list(ctx, QUOTAS_COLLECTION, &opts).await {
        Ok(list) => list
            .records
            .into_iter()
            .map(|r| AdminQuotaRow {
                user_short: r.str_field("user_id").get(..8).unwrap_or("—").to_string(),
                max_storage_bytes: r.i64_field("max_storage_bytes"),
                max_file_size_bytes: r.i64_field("max_file_size_bytes"),
                max_files_per_bucket: r.i64_field("max_files_per_bucket"),
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e.message, "admin quotas list failed");
            Vec::new()
        }
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        None,
        render_admin_quotas_table(&rows),
        None,
    );

    files_page(
        "Quotas",
        &config,
        "/b/storage/admin/quotas",
        user.as_ref(),
        "Quotas",
        Some("Per-user storage limits"),
        body,
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

    #[test]
    fn render_admin_buckets_table_empty_state() {
        let html = render_admin_buckets_table(&[]).into_string();
        assert!(html.contains("No buckets"), "missing empty hint: {html}");
    }

    #[test]
    fn render_admin_buckets_table_renders_rows() {
        let rows = vec![
            AdminBucketRow {
                name: "photos".into(),
                owner_short: "admin_1".into(),
                public: true,
                created_at_short: "2026-05-06".into(),
            },
            AdminBucketRow {
                name: "docs".into(),
                owner_short: "user_42".into(),
                public: false,
                created_at_short: "2026-05-05".into(),
            },
        ];
        let html = render_admin_buckets_table(&rows).into_string();
        assert!(html.contains(">photos<"), "name missing: {html}");
        assert!(html.contains(">docs<"));
        assert!(html.contains("admin_1"));
        // status_badge renders class names containing "public" / "private".
        assert!(html.contains("public"));
        assert!(html.contains("private"));
        assert!(html.contains("2026-05-06"));
    }

    #[test]
    fn render_admin_shares_table_empty_state() {
        let html = render_admin_shares_table(&[]).into_string();
        assert!(html.contains("No active shares"), "missing empty: {html}");
    }

    #[test]
    fn render_admin_shares_table_renders_rows() {
        let rows = vec![AdminShareRow {
            token_short: "tok12345abc1".into(),
            bucket: "photos".into(),
            key: "a.png".into(),
            access_count: 4,
            max_access_count: Some(10),
            expires_short: Some("2026-06-06".into()),
            owner_short: "admin_1".into(),
        }];
        let html = render_admin_shares_table(&rows).into_string();
        assert!(html.contains("tok12345abc1"));
        assert!(html.contains(">photos<"));
        assert!(html.contains(">a.png<"));
        // access_count and max rendered together as "4 / 10"
        assert!(
            html.contains("4 / 10"),
            "access count + max missing: {html}"
        );
        // max_access_count rendered as "/ 10"
        assert!(html.contains("/ 10"));
        assert!(html.contains("2026-06-06"));
        assert!(html.contains("admin_1"));
    }

    #[test]
    fn render_admin_shares_table_no_expires_renders_never() {
        let rows = vec![AdminShareRow {
            token_short: "abc".into(),
            bucket: "b".into(),
            key: "k".into(),
            access_count: 0,
            max_access_count: None,
            expires_short: None,
            owner_short: "u".into(),
        }];
        let html = render_admin_shares_table(&rows).into_string();
        assert!(
            html.contains("Never"),
            "missing 'Never' for null expires: {html}"
        );
        // No "/ N" segment when max_access_count is None.
        assert!(
            !html.contains("/ "),
            "should not show max divisor when None: {html}"
        );
    }

    #[test]
    fn render_admin_quotas_table_empty_state() {
        let html = render_admin_quotas_table(&[]).into_string();
        assert!(
            html.contains("No custom quotas"),
            "missing empty hint: {html}"
        );
        // Default values surfaced in the empty state copy.
        assert!(html.contains("1 GB"), "missing 1 GB default copy: {html}");
    }

    #[test]
    fn render_admin_quotas_table_renders_rows() {
        let rows = vec![AdminQuotaRow {
            user_short: "user_1".into(),
            max_storage_bytes: 5_000_000_000,
            max_file_size_bytes: 100_000_000,
            max_files_per_bucket: 1000,
        }];
        let html = render_admin_quotas_table(&rows).into_string();
        assert!(html.contains("user_1"), "user missing: {html}");
        // 5 GB ≈ "4.7 GB" via format_bytes humanization.
        assert!(html.contains("GB"), "GB unit missing: {html}");
        // 100 MB → "95.4 MB".
        assert!(html.contains("MB"), "MB unit missing: {html}");
        // max_files_per_bucket as integer in its own cell.
        assert!(
            html.contains(">1000<"),
            "files-per-bucket count missing: {html}"
        );
    }
}
