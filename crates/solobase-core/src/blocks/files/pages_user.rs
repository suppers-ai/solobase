//! User-facing UI pages for the suppers-ai/files block.
//!
//! Pure render helpers live alongside async handlers; helpers are
//! unit-tested directly without `Context`.

use maud::{html, Markup, PreEscaped};

use super::super::helpers::url_path_encode;

/// Aggregated bucket info as shown in the user-facing table:
/// name, public flag, created-at ISO string, and live object count.
#[derive(Clone, Debug)]
pub struct BucketRow {
    pub name: String,
    pub public: bool,
    pub created_at: String,
    pub object_count: i64,
}

/// Render the bucket-list table (or empty state).
pub fn render_buckets_table(rows: &[BucketRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state {
                p { "No buckets yet — create one to upload files." }
            }
        };
    }
    html! {
        table .data-table {
            thead { tr {
                th { "Name" }
                th { "Visibility" }
                th { "Created" }
                th { "Objects" }
            } }
            tbody {
                @for r in rows {
                    tr data-bucket=(r.name) {
                        td data-label="Name" { a href={"/b/storage/" (url_path_encode(&r.name)) "/"} { (r.name) } }
                        td data-label="Visibility" {
                            @if r.public {
                                span .badge.badge-success { "Public" }
                            } @else {
                                span .badge { "Private" }
                            }
                        }
                        td data-label="Created" { (r.created_at) }
                        td data-label="Objects" { (r.object_count) }
                    }
                }
            }
        }
    }
}

/// Render the "+ New bucket" `<dialog>` modal. The form is wired by the
/// `bucketCreateModal()` handler in `files-browser.js`: it intercepts
/// submit, POSTs to `/b/storage/api/buckets`, and on success redirects to
/// `/b/storage/{name}/`. Markup is rendered server-side so the page works
/// even before the JS bundle finishes loading (the trigger is a no-op
/// without JS — accepted v1 trade-off; the JSON API is still callable).
pub fn render_new_bucket_modal() -> Markup {
    html! {
        dialog #new-bucket-modal .modal.modal--bucket-create {
            form method="dialog" {
                h3 { "New bucket" }
                p .modal-error role="alert" hidden {}
                label {
                    span { "Name" }
                    input
                        type="text"
                        name="name"
                        required
                        minlength="3"
                        maxlength="63"
                        // S3-compatible: lowercase letters, digits, hyphens; start/end alnum.
                        pattern="[a-z0-9]([a-z0-9-]*[a-z0-9])?"
                        autocomplete="off"
                        spellcheck="false"
                        placeholder="my-bucket";
                }
                small .form-hint {
                    "3–63 characters. Lowercase letters, digits, and hyphens. Must start and end with a letter or digit."
                }
                label .checkbox-label {
                    input type="checkbox" name="public" value="1";
                    span { "Public (objects can be accessed by anonymous URL)" }
                }
                div .modal-actions {
                    button type="button" data-action="cancel" .btn.btn--ghost.btn--md { "Cancel" }
                    button type="submit" data-action="create" .btn.btn--primary.btn--md { "Create bucket" }
                }
            }
        }
    }
}

use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::ui::{
    self,
    components::{button, BtnVariant, CtrlSize},
    nav_groups,
    shell::{Crumb, Topbar},
    shelled_response,
    templates::{list_page, PageHeader},
    SiteConfig, UserInfo,
};

/// Load the calling user's buckets, decorated with live object counts.
///
/// `created_by` filtering keeps users from seeing each other's buckets.
/// Object counts come from a single GROUP BY query on the objects table
/// (one row per bucket) so we avoid the previous N+1 `db::count` per
/// bucket.
pub async fn list_buckets_for_user(ctx: &dyn Context, user_id: &str) -> Vec<BucketRow> {
    use std::collections::HashMap;

    use wafer_block::db::{Filter, FilterOp, SortField};
    use wafer_sql_utils::{
        aggregate::{build_grouped_query, AggFunc, AggregateColumn, GroupedQueryConfig},
        Backend,
    };

    use super::{BUCKETS_TABLE, OBJECTS_TABLE};

    let recs = match db::list_sorted(
        ctx,
        BUCKETS_TABLE,
        vec![Filter {
            field: "created_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.into()),
        }],
        vec![SortField {
            field: "name".into(),
            desc: false,
        }],
    )
    .await
    {
        Ok(records) => records,
        Err(e) => {
            tracing::warn!(error = %e, "files bucket list failed");
            Vec::new()
        }
    };

    // Build a list of bucket names this user owns; restrict the GROUP BY
    // to those buckets so the count matches the previous per-bucket
    // db::count semantics exactly (which counted all objects in the
    // bucket regardless of `uploaded_by`).
    let bucket_names: Vec<serde_json::Value> = recs
        .iter()
        .filter_map(|r| r.data.get("name").and_then(|v| v.as_str()))
        .map(|s| serde_json::Value::String(s.to_string()))
        .collect();
    let counts_cfg = GroupedQueryConfig {
        table: OBJECTS_TABLE.into(),
        select_columns: vec!["bucket".into()],
        aggregates: vec![AggregateColumn {
            func: AggFunc::Count,
            field: None,
            alias: "cnt".into(),
            cast_as: None,
            inner_expr: None,
        }],
        filters: vec![Filter {
            field: "bucket".into(),
            operator: FilterOp::In,
            value: serde_json::Value::Array(bucket_names),
        }],
        group_by: vec!["bucket".into()],
        order_by: vec![],
        limit: None,
    };
    let stmt = build_grouped_query(counts_cfg, Backend::Sqlite);
    let counts_by_bucket: HashMap<String, i64> = match db::query(ctx, &stmt).await {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|r| {
                let bucket = r.data.get("bucket").and_then(|v| v.as_str())?.to_string();
                let cnt = r.data.get("cnt").and_then(|v| v.as_i64()).unwrap_or(0);
                Some((bucket, cnt))
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, "files bucket object counts failed");
            HashMap::new()
        }
    };

    let mut rows: Vec<BucketRow> = Vec::with_capacity(recs.len());
    for r in recs {
        let name = r
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let public = r
            .data
            .get("public")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let created_at = r
            .data
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let object_count = counts_by_bucket.get(&name).copied().unwrap_or(0);

        rows.push(BucketRow {
            name,
            public,
            created_at,
            object_count,
        });
    }
    rows
}

/// GET `/b/storage/` — bucket list for the calling user.
pub async fn bucket_list_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    // The handle() above us already enforces auth; this guard is defensive.
    if user_id.is_empty() {
        return ui::not_found_response(msg);
    }

    let rows = list_buckets_for_user(ctx, &user_id).await;
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let new_bucket_btn = button(
        BtnVariant::Primary,
        CtrlSize::Md,
        "+ New bucket",
        PreEscaped(r#"type="button" data-action="open-new-bucket""#.to_string()),
    );

    // The table cell carries the modal markup + JS so it lives inside the
    // shelled response without needing a new template parameter.
    let js_url = crate::ui::assets::files_browser_js_url();
    let table_with_modal = html! {
        (render_buckets_table(&rows))
        (render_new_bucket_modal())
        script src=(js_url) defer {}
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        None,
        table_with_modal,
        None,
    );

    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Files",
            href: None,
        }],
        primary_action: Some(new_bucket_btn),
        subtitle: Some("Your buckets and their object counts."),
        show_palette: true,
    };
    shelled_response(
        msg,
        "Files",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

/// Object as the user sees it (key, size, modified timestamp).
#[derive(Clone, Debug)]
pub struct ObjectRow {
    pub key: String,
    pub size: i64,
    pub modified: String,
}

/// Result of grouping a flat object list by a current-prefix folder view.
pub struct FolderListing<'a> {
    pub folders: Vec<String>,
    pub files: Vec<&'a ObjectRow>,
}

/// Synthesize a folder/file split for the rows whose key starts with
/// `current_prefix`. Folder names are deduped while preserving first-seen
/// order. Files are objects with no further `/` after `current_prefix`.
///
/// Pure function; safe to unit-test without `Context`.
pub fn group_objects_by_prefix<'a>(
    objs: &'a [ObjectRow],
    current_prefix: &str,
) -> FolderListing<'a> {
    let mut folders: Vec<String> = Vec::new();
    let mut files: Vec<&ObjectRow> = Vec::new();

    for obj in objs {
        let Some(rest) = obj.key.strip_prefix(current_prefix) else {
            continue;
        };
        match rest.find('/') {
            Some(idx) => {
                let folder = &rest[..idx];
                if !folder.is_empty() && !folders.iter().any(|f| f == folder) {
                    folders.push(folder.to_string());
                }
            }
            None => {
                if !rest.is_empty() {
                    files.push(obj);
                }
            }
        }
    }

    FolderListing { folders, files }
}

/// URL-encode a prefix (folder path) by splitting on '/', encoding each segment,
/// and rejoining with '/'. Preserves the trailing slash if present.
fn url_encode_prefix(prefix: &str) -> String {
    if prefix.is_empty() {
        return String::new();
    }
    // Split on '/', encode each segment, rejoin.
    let trimmed = prefix.trim_end_matches('/');
    let parts: Vec<String> = trimmed.split('/').map(url_path_encode).collect();
    if parts.is_empty() {
        return String::new();
    }
    parts.join("/") + "/"
}

/// Folder/file table for `/b/storage/{bucket}/...` views.
///
/// Folder rows link into `/b/storage/{bucket}/{prefix}{folder}/`.
/// File rows show the filename portion (after the `current_prefix`),
/// link to the download route, and carry a `data-action-menu` kebab
/// trigger that the JS asset wires up to Share / Delete / Copy-link.
pub fn render_objects_table(
    bucket: &str,
    current_prefix: &str,
    listing: &FolderListing<'_>,
) -> Markup {
    if listing.folders.is_empty() && listing.files.is_empty() {
        return html! {
            div .empty-state {
                p { "This folder is empty — drag files here to upload." }
            }
        };
    }

    html! {
        table .data-table {
            thead { tr {
                th { input type="checkbox" .bulk-select-all data-bulk-toggle; }
                th { "Name" }
                th { "Size" }
                th { "Modified" }
                th {} // kebab column
            } }
            tbody {
                @for folder in &listing.folders {
                    tr .row--folder {
                        td {} // bulk-select disabled on folders
                        td data-label="Name" {
                            a href={"/b/storage/" (url_path_encode(bucket)) "/" (url_encode_prefix(current_prefix)) (url_path_encode(folder)) "/"} {
                                "📁 " (folder)
                            }
                        }
                        td data-label="Size" { "—" }
                        td data-label="Modified" { "—" }
                        td {}
                    }
                }
                @for f in &listing.files {
                    @let filename = f.key.strip_prefix(current_prefix).unwrap_or(&f.key);
                    @let download_href = format!(
                        "/b/storage/api/buckets/{}/objects/{}",
                        url_path_encode(bucket),
                        f.key.split('/').map(url_path_encode).collect::<Vec<_>>().join("/"),
                    );
                    tr data-object-key=(f.key) {
                        td { input type="checkbox" .bulk-select data-key=(f.key); }
                        td data-label="Name" {
                            a href=(download_href) { (filename) }
                        }
                        td data-label="Size" { (f.size) }
                        // Wrap the timestamp in <time> so the visual-baseline
                        // mask `[data-relative-time], .relative-time, time`
                        // catches it. The raw string is kept as the element
                        // body for screen readers and the `datetime` attr
                        // gives machine-readable parseable form.
                        td data-label="Modified" { time datetime=(f.modified) { (f.modified) } }
                        td {
                            button .kebab-trigger
                                type="button"
                                data-action-menu
                                data-bucket=(bucket)
                                data-key=(f.key)
                                aria-label={"Actions for " (filename)}
                            { "⋯" }
                        }
                    }
                }
            }
        }
    }
}

/// Render breadcrumb crumbs for the page body (below the topbar).
///
/// This is distinct from the shell `Topbar { crumbs: vec![Crumb {...}] }`
/// system: the topbar shows the page-level chrome ("Files > {bucket}"),
/// and this in-body breadcrumb shows the current folder path within the
/// bucket. The bucket and each prefix segment except the last are
/// clickable; the last segment is plain text. Returned `Markup` is a
/// `<nav class="breadcrumbs">` block.
pub fn render_breadcrumbs(bucket: &str, current_prefix: &str) -> Markup {
    let segments: Vec<&str> = current_prefix
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let last_idx = segments.len();
    let encoded_bucket = url_path_encode(bucket);

    html! {
        nav .breadcrumbs aria-label="Folder" {
            a href="/b/storage/" { "Files" }
            span .breadcrumbs__sep { " / " }
            @if segments.is_empty() {
                span { (bucket) }
            } @else {
                a href={"/b/storage/" (encoded_bucket) "/"} { (bucket) }
                @for (i, seg) in segments.iter().enumerate() {
                    span .breadcrumbs__sep { " / " }
                    @if i + 1 == last_idx {
                        span { (seg) }
                    } @else {
                        @let cumulative: String = segments[..=i].iter().map(|s| url_path_encode(s)).collect::<Vec<_>>().join("/");
                        a href={"/b/storage/" (encoded_bucket) "/" (cumulative) "/"} { (seg) }
                    }
                }
            }
        }
    }
}

async fn user_owns_bucket(ctx: &dyn Context, user_id: &str, bucket: &str) -> bool {
    use wafer_block::db::{Filter, FilterOp};

    use super::BUCKETS_TABLE;

    let filters = vec![
        Filter {
            field: "name".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(bucket.into()),
        },
        Filter {
            field: "created_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.into()),
        },
    ];
    match db::list_all(ctx, BUCKETS_TABLE, filters).await {
        Ok(records) => !records.is_empty(),
        Err(e) => {
            tracing::warn!(error = %e, bucket = %bucket, "bucket-ownership check failed");
            false
        }
    }
}

async fn list_objects_in_bucket(ctx: &dyn Context, bucket: &str) -> Vec<ObjectRow> {
    use wafer_block::db::{Filter, FilterOp, ListOptions, SortField};

    use super::OBJECTS_TABLE;

    let opts = ListOptions {
        filters: vec![Filter {
            field: "bucket".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(bucket.into()),
        }],
        sort: vec![SortField {
            field: "key".into(),
            desc: false,
        }],
        limit: 1000,
        ..Default::default()
    };

    match db::list(ctx, OBJECTS_TABLE, &opts).await {
        Ok(rl) => rl
            .records
            .into_iter()
            .map(|r| ObjectRow {
                key: r
                    .data
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                size: r
                    .data
                    .get("size")
                    .and_then(|v| {
                        v.as_i64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0),
                modified: r
                    .data
                    .get("uploaded_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, bucket = %bucket, "object list failed");
            Vec::new()
        }
    }
}

/// Render the bootstrap JSON in a script tag, escaping `<` to prevent
/// `</script>` sequences from terminating the JSON-typed script element.
/// The escaped `<` (`<`) is valid JSON and decodes back to `<` when
/// the browser reads it via `JSON.parse`.
fn render_bootstrap_script(bucket: &str, current_prefix: &str) -> Markup {
    let bootstrap = serde_json::json!({
        "bucket": bucket,
        "currentPrefix": current_prefix,
    });
    let bootstrap_json = serde_json::to_string(&bootstrap)
        .unwrap_or_else(|_| "{}".to_string())
        .replace('<', "\\u003c");
    let js_url = crate::ui::assets::files_browser_js_url();
    html! {
        script type="application/json" id="files-browser-bootstrap" {
            (PreEscaped(bootstrap_json))
        }
        script src=(js_url) defer {}
    }
}

/// GET `/b/storage/{bucket}/[{prefix}/]` — object listing with synthesized
/// folder navigation. 404s if the bucket doesn't exist for this user
/// (cross-user isolation enforced by the `created_by` filter on lookup).
pub async fn object_list_page(
    ctx: &dyn Context,
    msg: &Message,
    bucket: &str,
    current_prefix: &str,
) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return crate::ui::not_found_response(msg);
    }
    if !user_owns_bucket(ctx, &user_id, bucket).await {
        return crate::ui::not_found_response(msg);
    }

    let all_objects = list_objects_in_bucket(ctx, bucket).await;
    let listing = group_objects_by_prefix(&all_objects, current_prefix);

    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let title = if current_prefix.is_empty() {
        bucket.to_string()
    } else {
        format!("{bucket} / {}", current_prefix.trim_end_matches('/'))
    };

    let table = render_objects_table(bucket, current_prefix, &listing);
    let table_with_js = html! {
        // Hidden file input that the topbar Upload button triggers via
        // [data-action="open-upload"]. Multi-select so users can pick
        // many files at once. Same upload endpoint as drag-drop.
        input #file-upload-input type="file" multiple style="display: none";
        (table)
        (render_bootstrap_script(bucket, current_prefix))
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        Some(render_breadcrumbs(bucket, current_prefix)),
        table_with_js,
        None,
    );

    let groups = nav_groups::portal();
    let upload_btn = crate::ui::components::button(
        crate::ui::components::BtnVariant::Primary,
        crate::ui::components::CtrlSize::Sm,
        "+ Upload",
        maud::PreEscaped(r#"type="button" data-action="open-upload""#.to_string()),
    );
    let topbar = Topbar {
        crumbs: vec![
            Crumb {
                label: "Files",
                href: Some("/b/storage/"),
            },
            Crumb {
                label: bucket,
                href: None,
            },
        ],
        primary_action: Some(upload_btn),
        subtitle: Some("Drag files here to upload, or use the Upload button."),
        show_palette: true,
    };
    shelled_response(
        msg,
        &title,
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

#[derive(Clone, Debug)]
pub struct QuotaInfo {
    pub used_bytes: i64,
    pub limit_bytes: i64,
}

#[derive(Clone, Debug)]
pub struct ShareRow {
    pub token: String,
    pub bucket: String,
    pub key: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub access_count: i64,
}

fn quota_pct(used: i64, limit: i64) -> i64 {
    if limit <= 0 {
        return 0;
    }
    ((used.max(0) as f64 / limit as f64) * 100.0).round() as i64
}

pub fn render_quota_card(q: &QuotaInfo) -> Markup {
    let pct = quota_pct(q.used_bytes, q.limit_bytes);
    let warn = pct >= 90;
    html! {
        div class={ "quota-card" @if warn { " quota-warning" } } {
            h3 { "Storage quota" }
            p {
                (q.used_bytes) " / " (q.limit_bytes) " bytes"
                " · " (pct) "%"
            }
            div .quota-bar { div .quota-bar__fill style={"width: " (pct) "%"} {} }
        }
    }
}

pub fn render_shares_table(rows: &[ShareRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state { p { "No active shares yet." } }
        };
    }
    html! {
        table .data-table {
            thead { tr {
                th { "Token" }
                th { "Source" }
                th { "Created" }
                th { "Expires" }
                th { "Accesses" }
                th {}
            } }
            tbody {
                @for r in rows {
                    tr data-share-token=(r.token) {
                        td data-label="Token" { code { (r.token) } }
                        td data-label="Source" { (r.bucket) "/" (r.key) }
                        td data-label="Created" { (r.created_at) }
                        td data-label="Expires" {
                            @if let Some(exp) = &r.expires_at { (exp) } @else { "—" }
                        }
                        td data-label="Accesses" { (r.access_count) }
                        td {
                            button .kebab-trigger
                                type="button"
                                data-action-menu
                                data-token=(r.token)
                                aria-label={"Actions for share " (r.token)}
                            { "⋯" }
                        }
                    }
                }
            }
        }
    }
}

async fn list_shares_for_user(ctx: &dyn Context, user_id: &str) -> Vec<ShareRow> {
    use wafer_block::db::{Filter, FilterOp, SortField};

    use super::SHARES_TABLE;
    match db::list_sorted(
        ctx,
        SHARES_TABLE,
        vec![Filter {
            field: "created_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.into()),
        }],
        vec![SortField {
            field: "created_at".into(),
            desc: true,
        }],
    )
    .await
    {
        Ok(records) => records
            .into_iter()
            .map(|r| ShareRow {
                token: r
                    .data
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                bucket: r
                    .data
                    .get("bucket")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                key: r
                    .data
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                created_at: r
                    .data
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                expires_at: r
                    .data
                    .get("expires_at")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                access_count: r
                    .data
                    .get("access_count")
                    .and_then(|v| {
                        v.as_i64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0),
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, "shares list failed");
            Vec::new()
        }
    }
}

async fn load_quota_info(ctx: &dyn Context, user_id: &str) -> QuotaInfo {
    use wafer_block::db::{Filter, FilterOp};
    use wafer_run::types::ErrorCode;

    use super::{OBJECTS_TABLE, QUOTAS_TABLE};

    let limit = match db::get_by_field(
        ctx,
        QUOTAS_TABLE,
        "user_id",
        serde_json::Value::String(user_id.into()),
    )
    .await
    {
        Ok(r) => r
            .data
            .get("max_storage_bytes")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(1_073_741_824),
        Err(e) if e.code == ErrorCode::NotFound => 1_073_741_824,
        Err(e) => {
            tracing::warn!(error = %e, "quota lookup failed");
            1_073_741_824
        }
    };

    // used_bytes = SUM(size) over the user's objects. v1 in-process sum.
    let used_filters = vec![Filter {
        field: "uploaded_by".into(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id.into()),
    }];
    let used = match db::list_all(ctx, OBJECTS_TABLE, used_filters).await {
        Ok(recs) => recs
            .iter()
            .filter_map(|r| {
                r.data.get("size").and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
            })
            .sum(),
        Err(e) => {
            tracing::warn!(error = %e, "used-bytes lookup failed");
            0
        }
    };

    QuotaInfo {
        used_bytes: used,
        limit_bytes: limit,
    }
}

/// GET `/b/cloudstorage/` — share list with quota card.
pub async fn cloudstorage_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let user_id = msg.user_id().to_string();
    if user_id.is_empty() {
        return crate::ui::not_found_response(msg);
    }

    let shares = list_shares_for_user(ctx, &user_id).await;
    let quota = load_quota_info(ctx, &user_id).await;

    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let shares_with_js = html! {
        (render_shares_table(&shares))
        (render_bootstrap_script("", ""))
    };

    let body = list_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        Some(render_quota_card(&quota)),
        shares_with_js,
        None,
    );

    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Shares",
            href: None,
        }],
        primary_action: None,
        subtitle: Some("Public links you've created and your storage quota."),
        show_palette: true,
    };
    shelled_response(
        msg,
        "Shares",
        &config,
        &groups,
        user.as_ref(),
        msg.path(),
        topbar,
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str, public: bool, count: i64) -> BucketRow {
        BucketRow {
            name: name.into(),
            public,
            created_at: "2026-05-06T10:00:00Z".into(),
            object_count: count,
        }
    }

    #[test]
    fn render_buckets_table_empty_state() {
        let html = render_buckets_table(&[]).into_string();
        assert!(
            html.contains("No buckets yet"),
            "missing empty hint: {html}"
        );
    }

    #[test]
    fn render_buckets_table_renders_rows() {
        let rows = vec![sample("photos", true, 12), sample("docs", false, 0)];
        let html = render_buckets_table(&rows).into_string();
        assert!(html.contains(">photos<"));
        assert!(html.contains(">docs<"));
        assert!(html.contains("Public"));
        assert!(html.contains("Private"));
        assert!(html.contains(">12<"));
        assert!(html.contains(r#"href="/b/storage/photos/""#));
    }

    #[test]
    fn render_buckets_table_escapes_special_chars_in_bucket_name() {
        // Maud auto-escapes both the text content and the href attribute
        // value, so a bucket name with `&` should render as `a&amp;b` in
        // both places. This guards against a future refactor that bypasses
        // maud's escaping (e.g. PreEscaped).
        let rows = vec![sample("a&b", false, 0)];
        let html = render_buckets_table(&rows).into_string();
        assert!(
            html.contains("a&amp;b"),
            "name should be HTML-escaped: {html}"
        );
        assert!(
            !html.contains(">a&b<") && !html.contains(r#"href="/b/storage/a&b/""#),
            "raw `&` leaked into HTML: {html}"
        );
    }

    #[test]
    fn render_buckets_table_url_encodes_bucket_name_in_href() {
        let rows = vec![sample("my files", false, 0)];
        let html = render_buckets_table(&rows).into_string();
        assert!(
            html.contains(r#"href="/b/storage/my%20files/""#),
            "bucket href should URL-encode space: {html}"
        );
        // Display text remains raw (HTML-escaped by maud).
        assert!(html.contains(">my files<"), "display text wrong: {html}");
    }

    #[test]
    fn group_objects_by_prefix_empty() {
        let g = group_objects_by_prefix(&[], "");
        assert!(g.folders.is_empty());
        assert!(g.files.is_empty());
    }

    #[test]
    fn group_objects_by_prefix_root_files_only() {
        let objs = vec![
            ObjectRow {
                key: "a.png".into(),
                size: 1,
                modified: "2026-05-06T10:00:00Z".into(),
            },
            ObjectRow {
                key: "b.txt".into(),
                size: 2,
                modified: "2026-05-06T11:00:00Z".into(),
            },
        ];
        let g = group_objects_by_prefix(&objs, "");
        assert!(g.folders.is_empty());
        assert_eq!(g.files.len(), 2);
        assert_eq!(g.files[0].key, "a.png");
    }

    #[test]
    fn group_objects_by_prefix_synthesizes_folder() {
        let objs = vec![
            ObjectRow {
                key: "a.png".into(),
                size: 1,
                modified: "x".into(),
            },
            ObjectRow {
                key: "nested/b.png".into(),
                size: 2,
                modified: "x".into(),
            },
            ObjectRow {
                key: "nested/c.png".into(),
                size: 3,
                modified: "x".into(),
            },
        ];
        let g = group_objects_by_prefix(&objs, "");
        assert_eq!(g.folders, vec!["nested".to_string()]);
        assert_eq!(g.files.len(), 1);
        assert_eq!(g.files[0].key, "a.png");
    }

    #[test]
    fn group_objects_by_prefix_filters_by_current_prefix() {
        let objs = vec![
            ObjectRow {
                key: "a.png".into(),
                size: 1,
                modified: "x".into(),
            },
            ObjectRow {
                key: "nested/b.png".into(),
                size: 2,
                modified: "x".into(),
            },
            ObjectRow {
                key: "nested/sub/c.png".into(),
                size: 3,
                modified: "x".into(),
            },
        ];
        let g = group_objects_by_prefix(&objs, "nested/");
        assert_eq!(g.folders, vec!["sub".to_string()]);
        assert_eq!(g.files.len(), 1);
        assert_eq!(g.files[0].key, "nested/b.png");
    }

    #[test]
    fn group_objects_by_prefix_dedups_folder_names() {
        let objs = vec![
            ObjectRow {
                key: "x/a".into(),
                size: 0,
                modified: "x".into(),
            },
            ObjectRow {
                key: "x/b".into(),
                size: 0,
                modified: "x".into(),
            },
        ];
        let g = group_objects_by_prefix(&objs, "");
        assert_eq!(g.folders, vec!["x".to_string()]);
    }

    #[test]
    fn render_objects_table_empty_state() {
        let listing = FolderListing {
            folders: Vec::new(),
            files: Vec::new(),
        };
        let html = render_objects_table("photos", "", &listing).into_string();
        assert!(
            html.contains("This folder is empty"),
            "missing empty hint: {html}"
        );
    }

    #[test]
    fn render_objects_table_with_files_and_folders() {
        let f1 = ObjectRow {
            key: "a.png".into(),
            size: 1024,
            modified: "2026-05-06T10:00:00Z".into(),
        };
        let listing = FolderListing {
            folders: vec!["nested".into()],
            files: vec![&f1],
        };
        let html = render_objects_table("photos", "", &listing).into_string();
        // folder row with "📁" icon + link into the prefix
        assert!(html.contains("nested"), "folder name missing: {html}");
        assert!(
            html.contains(r#"href="/b/storage/photos/nested/""#),
            "folder href wrong: {html}"
        );
        // file row: filename portion only, no leading prefix
        assert!(html.contains(">a.png<"), "filename missing: {html}");
        assert!(html.contains("1024"), "size missing");
        // kebab menu trigger
        assert!(html.contains(r#"data-action-menu"#), "kebab missing");
        assert!(
            html.contains(r#"data-bucket="photos""#),
            "kebab data-bucket missing/wrong: {html}"
        );
        assert!(
            html.contains(r#"data-key="a.png""#),
            "kebab data-key missing/wrong: {html}"
        );
    }

    #[test]
    fn render_objects_table_filename_strips_prefix() {
        let f1 = ObjectRow {
            key: "nested/sub/c.png".into(),
            size: 0,
            modified: "x".into(),
        };
        let listing = FolderListing {
            folders: Vec::new(),
            files: vec![&f1],
        };
        let html = render_objects_table("photos", "nested/sub/", &listing).into_string();
        // The file row label is just the filename portion.
        assert!(html.contains(">c.png<"), "filename portion missing: {html}");
        // The download link still uses the full key.
        assert!(
            html.contains(r#"href="/b/storage/api/buckets/photos/objects/nested/sub/c.png""#),
            "download href wrong: {html}"
        );
    }

    #[test]
    fn render_objects_table_url_encodes_key_with_spaces() {
        let f1 = ObjectRow {
            key: "report Q2.pdf".into(),
            size: 0,
            modified: "x".into(),
        };
        let listing = FolderListing {
            folders: Vec::new(),
            files: vec![&f1],
        };
        let html = render_objects_table("photos", "", &listing).into_string();
        assert!(
            html.contains(r#"href="/b/storage/api/buckets/photos/objects/report%20Q2.pdf""#),
            "download href not URL-encoded: {html}"
        );
        // Display text remains the raw filename (HTML-escaped by maud).
        assert!(
            html.contains(">report Q2.pdf<"),
            "filename text wrong: {html}"
        );
    }

    #[test]
    fn render_objects_table_url_encodes_prefix_with_spaces() {
        let f1 = ObjectRow {
            key: "my files/sub/c.png".into(),
            size: 0,
            modified: "x".into(),
        };
        let listing = FolderListing {
            folders: vec!["sub".into()],
            files: vec![&f1],
        };
        let html = render_objects_table("photos", "my files/", &listing).into_string();
        // Folder href should encode the prefix's space.
        assert!(
            html.contains(r#"href="/b/storage/photos/my%20files/sub/""#),
            "folder href should URL-encode prefix space: {html}"
        );
    }

    #[test]
    fn render_breadcrumbs_root_only() {
        let html = render_breadcrumbs("photos", "").into_string();
        // bucket name visible, no extra crumbs.
        assert!(html.contains("photos"));
        assert!(!html.contains("nested"));
    }

    #[test]
    fn render_breadcrumbs_includes_each_segment() {
        let html = render_breadcrumbs("photos", "nested/sub/").into_string();
        // Each crumb except the last has a clickable link;
        // the last segment is non-link text.
        assert!(html.contains("photos"));
        assert!(html.contains(r#"href="/b/storage/photos/nested/""#));
        assert!(html.contains(">sub<"));
        // Last segment ("sub") must NOT be a link.
        assert!(
            !html.contains(r#"href="/b/storage/photos/nested/sub/""#),
            "last segment should be plain text, not a link: {html}"
        );
    }

    #[test]
    fn render_quota_card_under_quota() {
        let q = QuotaInfo {
            used_bytes: 100_000,
            limit_bytes: 1_000_000,
        };
        let html = render_quota_card(&q).into_string();
        assert!(html.contains("100"), "used count missing");
        assert!(
            html.contains("10%") || html.contains("10 %"),
            "percent missing"
        );
        assert!(
            !html.contains("quota-warning"),
            "should not be warning class"
        );
    }

    #[test]
    fn render_quota_card_near_quota() {
        let q = QuotaInfo {
            used_bytes: 950_000,
            limit_bytes: 1_000_000,
        };
        let html = render_quota_card(&q).into_string();
        assert!(
            html.contains("quota-warning"),
            "should mark near-quota: {html}"
        );
    }

    #[test]
    fn render_shares_table_empty() {
        let html = render_shares_table(&[]).into_string();
        assert!(html.contains("No active shares"));
    }

    #[test]
    fn render_shares_table_with_rows() {
        let rows = vec![ShareRow {
            token: "abc12345".into(),
            bucket: "photos".into(),
            key: "a.png".into(),
            created_at: "2026-05-06T10:00:00Z".into(),
            expires_at: Some("2026-06-06T10:00:00Z".into()),
            access_count: 4,
        }];
        let html = render_shares_table(&rows).into_string();
        assert!(html.contains("abc12345"));
        assert!(html.contains("photos"));
        assert!(html.contains("a.png"));
        assert!(html.contains(">4<"), "access count missing");
    }
}

#[cfg(test)]
mod integration_tests {
    use std::collections::HashMap;

    use serde_json::json;
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::files::{BUCKETS_TABLE, OBJECTS_TABLE, QUOTAS_TABLE, SHARES_TABLE},
        test_support::{admin_msg, output_html, TestContext},
    };

    /// Seed two buckets + two objects in `photos`, none in `docs`.
    async fn seed_two_buckets(ctx: &TestContext, owner: &str) {
        for (name, public) in [("photos", true), ("docs", false)] {
            let mut row: HashMap<String, serde_json::Value> = HashMap::new();
            row.insert("name".into(), json!(name));
            row.insert("public".into(), json!(public));
            row.insert("created_by".into(), json!(owner));
            db::create(ctx, BUCKETS_TABLE, row)
                .await
                .expect("seed bucket");
        }
        for key in ["a.png", "nested/b.png"] {
            let mut row: HashMap<String, serde_json::Value> = HashMap::new();
            row.insert("bucket".into(), json!("photos"));
            row.insert("key".into(), json!(key));
            row.insert("size".into(), json!(1024));
            row.insert("uploaded_by".into(), json!(owner));
            db::create(ctx, OBJECTS_TABLE, row)
                .await
                .expect("seed object");
        }
    }

    #[tokio::test]
    async fn bucket_list_page_renders_user_buckets() {
        let ctx = TestContext::with_files().await;
        let owner = "admin_1"; // admin_msg's default user_id
        seed_two_buckets(&ctx, owner).await;

        let msg = admin_msg("retrieve", "/b/storage/");
        let resp = bucket_list_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(body.contains("Files"), "missing page header: {body}");
        assert!(body.contains(">photos<"), "missing bucket: {body}");
        assert!(body.contains(">docs<"), "missing bucket: {body}");
        assert!(
            body.contains(r#"data-label="Objects">2<"#),
            "photos should show 2 objects: {body}"
        );
        assert!(
            body.contains(r#"data-label="Objects">0<"#),
            "docs should show 0 objects: {body}"
        );
    }

    #[tokio::test]
    async fn bucket_list_page_empty_state_for_fresh_user() {
        let ctx = TestContext::with_files().await;

        let msg = admin_msg("retrieve", "/b/storage/");
        let resp = bucket_list_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(body.contains("Files"), "missing page header");
        assert!(body.contains("No buckets yet"), "missing empty state");
    }

    #[tokio::test]
    async fn bucket_list_page_hides_other_users_buckets() {
        let ctx = TestContext::with_files().await;
        // Seed admin_1's buckets.
        seed_two_buckets(&ctx, "admin_1").await;
        // Seed one bucket for a different user.
        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
        row.insert("name".into(), json!("secrets"));
        row.insert("created_by".into(), json!("other_user"));
        db::create(&ctx, BUCKETS_TABLE, row)
            .await
            .expect("seed cross-user bucket");

        let msg = admin_msg("retrieve", "/b/storage/"); // user_id = "admin_1"
        let body = output_html(bucket_list_page(&ctx, &msg).await).await;
        assert!(
            !body.contains(">secrets<"),
            "cross-user bucket leaked: {body}"
        );
    }

    #[tokio::test]
    async fn bucket_list_page_renders_new_bucket_button() {
        let ctx = TestContext::with_files().await;
        let msg = admin_msg("retrieve", "/b/storage/");
        let body = output_html(bucket_list_page(&ctx, &msg).await).await;

        // Primary-action lives in the Topbar slot now (see ui(pages) commit
        // that moved page-header content into the topbar).
        assert!(
            body.contains("topbar__action"),
            "topbar action slot missing: {body}"
        );
        assert!(
            body.contains("+ New bucket"),
            "new-bucket button label missing: {body}"
        );
        assert!(
            body.contains(r#"data-action="open-new-bucket""#),
            "new-bucket trigger attribute missing: {body}"
        );
    }

    #[tokio::test]
    async fn bucket_list_page_renders_new_bucket_modal() {
        let ctx = TestContext::with_files().await;
        let msg = admin_msg("retrieve", "/b/storage/");
        let body = output_html(bucket_list_page(&ctx, &msg).await).await;

        // Modal markup is server-rendered next to the table.
        assert!(
            body.contains(r#"id="new-bucket-modal""#),
            "modal element missing: {body}"
        );
        assert!(
            body.contains(r#"name="name""#),
            "name input missing: {body}"
        );
        assert!(
            body.contains(r#"name="public""#),
            "public toggle missing: {body}"
        );
        assert!(
            body.contains("Create bucket"),
            "submit button missing: {body}"
        );
        // JS bundle is included with the cache-busting hash URL.
        assert!(
            body.contains("/b/static/files-browser-"),
            "files-browser.js script tag missing: {body}"
        );
    }

    #[test]
    fn render_new_bucket_modal_validates_name_pattern_client_side() {
        // The pattern is used by the browser for native validation; assert
        // it's present so we don't regress accidentally to no client-side
        // validation. Server-side validation lives in storage.rs.
        let html = render_new_bucket_modal().into_string();
        assert!(
            html.contains(r#"pattern="[a-z0-9]([a-z0-9-]*[a-z0-9])?""#),
            "client-side pattern attribute missing: {html}"
        );
        assert!(
            html.contains(r#"minlength="3""#) && html.contains(r#"maxlength="63""#),
            "length constraints missing: {html}"
        );
    }

    #[tokio::test]
    async fn object_list_page_root_renders_files_and_folders() {
        let ctx = TestContext::with_files().await;
        seed_two_buckets(&ctx, "admin_1").await;

        let msg = admin_msg("retrieve", "/b/storage/photos/");
        let resp = object_list_page(&ctx, &msg, "photos", "").await;
        let body = output_html(resp).await;

        assert!(body.contains(">a.png<"), "root file missing: {body}");
        assert!(
            body.contains("📁 nested"),
            "synthesized folder missing: {body}"
        );
        // Breadcrumb has only the bucket segment, no prefix segments.
        assert!(
            body.contains(r#"href="/b/storage/""#),
            "Files crumb link missing: {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_with_prefix_strips_filename() {
        let ctx = TestContext::with_files().await;
        seed_two_buckets(&ctx, "admin_1").await;

        let msg = admin_msg("retrieve", "/b/storage/photos/nested/");
        let resp = object_list_page(&ctx, &msg, "photos", "nested/").await;
        let body = output_html(resp).await;

        // Filename portion of "nested/b.png" is just "b.png".
        assert!(body.contains(">b.png<"), "filename missing: {body}");
        assert!(!body.contains(">nested/b.png<"), "raw key leaked: {body}");
    }

    #[tokio::test]
    async fn object_list_page_404_for_unknown_bucket() {
        let ctx = TestContext::with_files().await;
        let mut msg = admin_msg("retrieve", "/b/storage/missing/");
        msg.set_meta("http.header.accept", "text/html");
        let resp = object_list_page(&ctx, &msg, "missing", "").await;
        let body = output_html(resp).await;
        assert!(
            body.contains("Not found") || body.contains("404"),
            "expected 404: {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_404_for_other_users_bucket() {
        // Cross-user isolation: a bucket owned by another user must 404,
        // not render its contents.
        let ctx = TestContext::with_files().await;
        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
        row.insert("name".into(), json!("secrets"));
        row.insert("created_by".into(), json!("other_user"));
        db::create(&ctx, BUCKETS_TABLE, row).await.expect("seed");

        let mut msg = admin_msg("retrieve", "/b/storage/secrets/");
        msg.set_meta("http.header.accept", "text/html");
        let resp = object_list_page(&ctx, &msg, "secrets", "").await;
        let body = output_html(resp).await;
        assert!(
            body.contains("Not found") || body.contains("404"),
            "expected 404 for cross-user bucket: {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_renders_empty_state_for_empty_bucket() {
        let ctx = TestContext::with_files().await;
        // seed_two_buckets seeds `docs` with no objects.
        seed_two_buckets(&ctx, "admin_1").await;

        let msg = admin_msg("retrieve", "/b/storage/docs/");
        let resp = object_list_page(&ctx, &msg, "docs", "").await;
        let body = output_html(resp).await;

        assert!(
            body.contains("This folder is empty"),
            "expected empty-state copy: {body}"
        );
    }

    #[tokio::test]
    async fn cloudstorage_page_renders_shares_and_quota() {
        let ctx = TestContext::with_files().await;

        // Seed a share + a quota row owned by admin_1.
        let mut share: HashMap<String, serde_json::Value> = HashMap::new();
        share.insert("token".into(), json!("tok123abc"));
        share.insert("bucket".into(), json!("photos"));
        share.insert("key".into(), json!("a.png"));
        share.insert("created_by".into(), json!("admin_1"));
        share.insert("access_count".into(), json!(2));
        db::create(&ctx, SHARES_TABLE, share)
            .await
            .expect("seed share");

        let mut quota: HashMap<String, serde_json::Value> = HashMap::new();
        quota.insert("user_id".into(), json!("admin_1"));
        quota.insert("max_storage_bytes".into(), json!(1_073_741_824i64));
        db::create(&ctx, QUOTAS_TABLE, quota)
            .await
            .expect("seed quota");

        let msg = admin_msg("retrieve", "/b/cloudstorage/");
        let resp = cloudstorage_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(body.contains("Shares"));
        assert!(body.contains("tok123abc"));
        assert!(body.contains("photos"));
        assert!(body.contains("a.png"));
        assert!(body.contains(">2<"), "access count cell: {body}");
    }

    #[tokio::test]
    async fn cloudstorage_page_hides_other_users_shares() {
        let ctx = TestContext::with_files().await;
        // Seed admin_1's share.
        let mut mine: HashMap<String, serde_json::Value> = HashMap::new();
        mine.insert("token".into(), json!("mine"));
        mine.insert("bucket".into(), json!("photos"));
        mine.insert("key".into(), json!("a.png"));
        mine.insert("created_by".into(), json!("admin_1"));
        db::create(&ctx, SHARES_TABLE, mine)
            .await
            .expect("seed mine");
        // Seed another user's share.
        let mut theirs: HashMap<String, serde_json::Value> = HashMap::new();
        theirs.insert("token".into(), json!("theirs"));
        theirs.insert("bucket".into(), json!("secrets"));
        theirs.insert("key".into(), json!("k"));
        theirs.insert("created_by".into(), json!("other_user"));
        db::create(&ctx, SHARES_TABLE, theirs)
            .await
            .expect("seed theirs");

        let msg = admin_msg("retrieve", "/b/cloudstorage/");
        let body = output_html(cloudstorage_page(&ctx, &msg).await).await;
        assert!(body.contains("mine"), "own share missing: {body}");
        assert!(!body.contains("theirs"), "other-user share leaked: {body}");
    }

    #[tokio::test]
    async fn object_list_page_includes_files_browser_js() {
        let ctx = TestContext::with_files().await;
        seed_two_buckets(&ctx, "admin_1").await;

        let msg = admin_msg("retrieve", "/b/storage/photos/");
        let resp = object_list_page(&ctx, &msg, "photos", "").await;
        let body = output_html(resp).await;

        assert!(
            body.contains(r#"id="files-browser-bootstrap""#),
            "bootstrap carrier missing: {body}"
        );
        assert!(
            body.contains(r#""bucket":"photos""#),
            "bootstrap bucket missing: {body}"
        );
        assert!(
            body.contains(r#""currentPrefix":"""#) || body.contains(r#""currentPrefix": """#),
            "bootstrap currentPrefix missing: {body}"
        );
        assert!(
            body.contains("/b/static/files-browser-"),
            "files-browser.js script tag missing: {body}"
        );
    }

    #[tokio::test]
    async fn cloudstorage_page_includes_files_browser_js() {
        let ctx = TestContext::with_files().await;

        let msg = admin_msg("retrieve", "/b/cloudstorage/");
        let resp = cloudstorage_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(
            body.contains(r#"id="files-browser-bootstrap""#),
            "bootstrap carrier missing: {body}"
        );
        assert!(
            body.contains("/b/static/files-browser-"),
            "files-browser.js script tag missing: {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_shows_actual_size_from_text_columns() {
        // SQLite TEXT columns store integers as strings (see MEMORY.md
        // wafer-wrap-table-naming). The renderer must coerce both shapes.
        let ctx = TestContext::with_files().await;
        let mut bucket: HashMap<String, serde_json::Value> = HashMap::new();
        bucket.insert("name".into(), json!("photos"));
        bucket.insert("created_by".into(), json!("admin_1"));
        db::create(&ctx, BUCKETS_TABLE, bucket)
            .await
            .expect("seed bucket");

        let mut obj: HashMap<String, serde_json::Value> = HashMap::new();
        obj.insert("bucket".into(), json!("photos"));
        obj.insert("key".into(), json!("a.png"));
        // Note: json!(2048) is a JSON number, but the SQLite backend will
        // round-trip it as a string. The fallback in list_objects_in_bucket
        // must accept both shapes.
        obj.insert("size".into(), json!(2048));
        obj.insert("uploaded_by".into(), json!("admin_1"));
        db::create(&ctx, OBJECTS_TABLE, obj)
            .await
            .expect("seed obj");

        let msg = admin_msg("retrieve", "/b/storage/photos/");
        let body = output_html(object_list_page(&ctx, &msg, "photos", "").await).await;
        // The Size column should show 2048, not 0.
        assert!(
            body.contains(r#"data-label="Size">2048<"#),
            "size cell should be 2048 (got via TEXT fallback): {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_escapes_script_close_in_bootstrap() {
        // A bucket name containing `</script>` would prematurely close the
        // <script type="application/json"> bootstrap carrier. The render
        // path must escape `<` so that no `</script>` appears in the JSON.
        let ctx = TestContext::with_files().await;
        let mut bucket: HashMap<String, serde_json::Value> = HashMap::new();
        bucket.insert("name".into(), json!("foo</script>bar"));
        bucket.insert("created_by".into(), json!("admin_1"));
        db::create(&ctx, BUCKETS_TABLE, bucket).await.expect("seed");

        let msg = admin_msg("retrieve", "/b/storage/foo</script>bar/");
        let body = output_html(object_list_page(&ctx, &msg, "foo</script>bar", "").await).await;

        // The dangerous substring must NOT appear in the rendered HTML.
        // The escaped form `</script>` is the safe representation.
        assert!(
            !body.contains("</script>foo") && !body.contains("foo</script>bar\""),
            "bootstrap is broken by unescaped </script>: {body}"
        );
        // The escaped form should appear (defensive — proves the escape ran).
        assert!(
            body.contains("\\u003c/script\\u003e") || body.contains("\\u003c/script>"),
            "expected escaped </script> sequence in bootstrap: {body}"
        );
    }
}
