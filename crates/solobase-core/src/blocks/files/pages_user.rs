//! User-facing UI pages for the suppers-ai/files block.
//!
//! Pure render helpers live alongside async handlers; helpers are
//! unit-tested directly without `Context`.

use maud::{html, Markup};

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
                        td data-label="Name" { a href={"/b/storage/" (r.name) "/"} { (r.name) } }
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

use wafer_core::clients::database as db;
use wafer_run::{context::Context, types::Message, OutputStream};

use crate::ui::{
    self, nav_groups,
    shell::{Crumb, Topbar},
    shelled_response,
    templates::{list_page, PageHeader},
    SiteConfig, UserInfo,
};

/// Load the calling user's buckets, decorated with live object counts.
///
/// `created_by` filtering keeps users from seeing each other's buckets.
/// Two N+1-shaped concerns: (1) `ListOptions::default()` has `limit: 0`
/// which means no LIMIT clause — a full scan of the buckets collection
/// for this user. (2) Object counts loop one `db::count` per bucket.
/// Both are acceptable for v1 — per-user bucket count is normally small.
/// If this gets hot, fold into a single aggregate query via
/// `wafer-sql-utils::aggregate` (do **not** use raw SQL — CLAUDE.md).
pub async fn list_buckets_for_user(ctx: &dyn Context, user_id: &str) -> Vec<BucketRow> {
    use super::{BUCKETS_COLLECTION, OBJECTS_COLLECTION};
    use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

    let opts = ListOptions {
        filters: vec![Filter {
            field: "created_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id.into()),
        }],
        sort: vec![SortField {
            field: "name".into(),
            desc: false,
        }],
        ..Default::default()
    };

    let recs = match db::list(ctx, BUCKETS_COLLECTION, &opts).await {
        Ok(rl) => rl.records,
        Err(e) => {
            tracing::warn!(error = %e, "files bucket list failed");
            Vec::new()
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

        let obj_filters = vec![Filter {
            field: "bucket".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(name.clone()),
        }];
        let object_count = match db::count(ctx, OBJECTS_COLLECTION, &obj_filters).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(error = %e, bucket = %name, "files bucket object count failed");
                0
            }
        };

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

    let body = list_page(
        PageHeader {
            title: "Files",
            subtitle: Some("Your buckets and their object counts."),
            primary_action: None, // bucket creation modal arrives in Task 9 JS
        },
        None,
        render_buckets_table(&rows),
        None,
    );

    let groups = nav_groups::portal();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Files",
            href: None,
        }],
        primary_action: None,
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
                            a href={"/b/storage/" (url_path_encode(bucket)) "/" (current_prefix) (url_path_encode(folder)) "/"} {
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
                        td data-label="Modified" { (f.modified) }
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
    use super::{BUCKETS_COLLECTION};
    use wafer_core::clients::database::{Filter, FilterOp, ListOptions};

    let opts = ListOptions {
        filters: vec![
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
        ],
        ..Default::default()
    };
    match db::list(ctx, BUCKETS_COLLECTION, &opts).await {
        Ok(rl) => !rl.records.is_empty(),
        Err(e) => {
            tracing::warn!(error = %e, bucket = %bucket, "bucket-ownership check failed");
            false
        }
    }
}

async fn list_objects_in_bucket(ctx: &dyn Context, bucket: &str) -> Vec<ObjectRow> {
    use super::{OBJECTS_COLLECTION};
    use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};

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

    match db::list(ctx, OBJECTS_COLLECTION, &opts).await {
        Ok(rl) => rl
            .records
            .into_iter()
            .map(|r| ObjectRow {
                key: r.data.get("key").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                size: r.data.get("size").and_then(|v| v.as_i64()).unwrap_or(0),
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

    let body = list_page(
        PageHeader {
            title: bucket,
            subtitle: Some("Drag files here to upload."),
            primary_action: None,
        },
        Some(render_breadcrumbs(bucket, current_prefix)),
        render_objects_table(bucket, current_prefix, &listing),
        None,
    );

    let groups = nav_groups::portal();
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
        primary_action: None,
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
        assert!(html.contains(r#"data-bucket="photos""#), "kebab data-bucket missing/wrong: {html}");
        assert!(html.contains(r#"data-key="a.png""#), "kebab data-key missing/wrong: {html}");
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
        assert!(html.contains(">report Q2.pdf<"), "filename text wrong: {html}");
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
}

#[cfg(test)]
mod integration_tests {
    use std::collections::HashMap;

    use serde_json::json;
    use wafer_core::clients::database as db;

    use super::*;
    use crate::{
        blocks::files::{BUCKETS_COLLECTION, OBJECTS_COLLECTION},
        test_support::{admin_msg, output_html, TestContext},
    };

    /// Seed two buckets + two objects in `photos`, none in `docs`.
    async fn seed_two_buckets(ctx: &TestContext, owner: &str) {
        for (name, public) in [("photos", true), ("docs", false)] {
            let mut row: HashMap<String, serde_json::Value> = HashMap::new();
            row.insert("name".into(), json!(name));
            row.insert("public".into(), json!(public));
            row.insert("created_by".into(), json!(owner));
            db::create(ctx, BUCKETS_COLLECTION, row)
                .await
                .expect("seed bucket");
        }
        for key in ["a.png", "nested/b.png"] {
            let mut row: HashMap<String, serde_json::Value> = HashMap::new();
            row.insert("bucket".into(), json!("photos"));
            row.insert("key".into(), json!(key));
            row.insert("size".into(), json!(1024));
            row.insert("uploaded_by".into(), json!(owner));
            db::create(ctx, OBJECTS_COLLECTION, row)
                .await
                .expect("seed object");
        }
    }

    #[tokio::test]
    async fn bucket_list_page_renders_user_buckets() {
        let ctx = TestContext::with_auth().await;
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
        let ctx = TestContext::with_auth().await;

        let msg = admin_msg("retrieve", "/b/storage/");
        let resp = bucket_list_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(body.contains("Files"), "missing page header");
        assert!(body.contains("No buckets yet"), "missing empty state");
    }

    #[tokio::test]
    async fn bucket_list_page_hides_other_users_buckets() {
        let ctx = TestContext::with_auth().await;
        // Seed admin_1's buckets.
        seed_two_buckets(&ctx, "admin_1").await;
        // Seed one bucket for a different user.
        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
        row.insert("name".into(), json!("secrets"));
        row.insert("created_by".into(), json!("other_user"));
        db::create(&ctx, BUCKETS_COLLECTION, row)
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
    async fn object_list_page_root_renders_files_and_folders() {
        let ctx = TestContext::with_auth().await;
        seed_two_buckets(&ctx, "admin_1").await;

        let msg = admin_msg("retrieve", "/b/storage/photos/");
        let resp = object_list_page(&ctx, &msg, "photos", "").await;
        let body = output_html(resp).await;

        assert!(body.contains(">a.png<"), "root file missing: {body}");
        assert!(body.contains("📁 nested"), "synthesized folder missing: {body}");
        // Breadcrumb has only the bucket segment, no prefix segments.
        assert!(
            body.contains(r#"href="/b/storage/""#),
            "Files crumb link missing: {body}"
        );
    }

    #[tokio::test]
    async fn object_list_page_with_prefix_strips_filename() {
        let ctx = TestContext::with_auth().await;
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
        let ctx = TestContext::with_auth().await;
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
        let ctx = TestContext::with_auth().await;
        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
        row.insert("name".into(), json!("secrets"));
        row.insert("created_by".into(), json!("other_user"));
        db::create(&ctx, BUCKETS_COLLECTION, row).await.expect("seed");

        let mut msg = admin_msg("retrieve", "/b/storage/secrets/");
        msg.set_meta("http.header.accept", "text/html");
        let resp = object_list_page(&ctx, &msg, "secrets", "").await;
        let body = output_html(resp).await;
        assert!(
            body.contains("Not found") || body.contains("404"),
            "expected 404 for cross-user bucket: {body}"
        );
    }
}
