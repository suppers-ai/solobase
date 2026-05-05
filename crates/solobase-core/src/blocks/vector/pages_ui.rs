//! User-facing UI pages for the suppers-ai/vector block.
//!
//! Pure render helpers live alongside async handlers; helpers are
//! unit-tested directly without `Context`.

use maud::{html, Markup};
use wafer_run::{context::Context, types::Message, OutputStream};

use super::service::display_index_name;
use crate::ui::{
    nav_groups,
    shell::{Crumb, Topbar},
    shelled_response,
    templates::{list_page, PageHeader},
    SiteConfig, UserInfo,
};

#[derive(Clone, Debug)]
pub struct IndexRow {
    pub name: String,
    pub model: String,
    pub dimensions: u32,
    pub vector_count: u64,
    pub keyword_search: bool,
}

pub fn render_index_list_table(rows: &[IndexRow]) -> Markup {
    if rows.is_empty() {
        return html! {
            div .empty-state {
                p { "No vector indexes yet." }
            }
        };
    }

    html! {
        table .data-table {
            thead { tr {
                th { "Name" }
                th { "Model" }
                th { "Dimensions" }
                th { "Vectors" }
                th { "Keyword search" }
            } }
            tbody {
                @for r in rows {
                    @let display = display_index_name(&r.name);
                    tr data-index-name=(display) {
                        td data-label="Name" { (display) }
                        td data-label="Model" { (r.model) }
                        td data-label="Dimensions" { (r.dimensions) }
                        td data-label="Vectors" { (r.vector_count) }
                        td data-label="Keyword search" {
                            @if r.keyword_search {
                                span .badge.badge-success { "Yes" }
                            } @else {
                                span .badge { "No" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// GET `/b/vector/` — admin-facing index listing.
///
/// Reads the per-index metadata registry (`suppers_ai__vector__registry`)
/// and decorates each row with a live vector count from the underlying
/// `_meta` table. A failure to load the registry (e.g. fresh DB where the
/// table doesn't exist yet) falls through to the empty state — the
/// `wafer-block-sqlite` service returns an empty list rather than erroring
/// for unknown collections, so this only logs when something more serious
/// happens.
pub async fn index_list_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let rows = match super::service::list_index_rows(ctx).await {
        Ok(rs) => rs,
        Err(e) => {
            tracing::warn!(error = %e, "vector index list failed");
            Vec::new()
        }
    };

    let body = list_page(
        PageHeader {
            title: "Vector indexes",
            subtitle: Some("Per-index counts, model, dimensions"),
            primary_action: None,
        },
        None,
        render_index_list_table(&rows),
        None,
    );

    let groups = nav_groups::admin();
    let topbar = Topbar {
        crumbs: vec![Crumb {
            label: "Vector indexes",
            href: None,
        }],
        primary_action: None,
        show_palette: true,
    };
    shelled_response(
        msg,
        "Vector indexes",
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

    fn sample_index(name: &str, model: &str, dims: u32, count: u64, kw: bool) -> IndexRow {
        IndexRow {
            name: name.into(),
            model: model.into(),
            dimensions: dims,
            vector_count: count,
            keyword_search: kw,
        }
    }

    #[test]
    fn render_index_list_table_renders_rows_and_empty_state() {
        let empty = render_index_list_table(&[]).into_string();
        assert!(empty.contains("No vector indexes yet"), "missing empty hint: {empty}");

        let rows = vec![sample_index("docs", "fastembed", 384, 1234, true)];
        let html = render_index_list_table(&rows).into_string();
        assert!(html.contains("docs"), "missing index name");
        assert!(html.contains("fastembed"), "missing model");
        assert!(html.contains("384"), "missing dimensions");
        assert!(html.contains("1234"), "missing count");
    }

    #[test]
    fn render_index_list_table_strips_storage_prefix() {
        let row = sample_index("suppers_ai__vector__docs", "fastembed", 384, 0, false);
        let html = render_index_list_table(&[row]).into_string();
        assert!(html.contains(">docs<"), "prefix not stripped: {html}");
        assert!(!html.contains("suppers_ai__vector__"), "raw prefix leaked");
    }
}

#[cfg(test)]
mod integration_tests {
    use std::collections::HashMap;

    use serde_json::json;
    use wafer_core::clients::database as db;

    use super::*;
    use crate::test_support::{admin_msg, output_html, TestContext};

    /// Seed one row in the vector registry plus the matching `_meta` table
    /// so the listing has both a registry entry and a vector count to show.
    async fn seed_docs_index(ctx: &TestContext) {
        // Registry row. The SQLite service auto-creates the table on first
        // insert (`ensure_table`) so we don't need DDL here.
        let mut registry_row: HashMap<String, serde_json::Value> = HashMap::new();
        registry_row.insert(
            "prefixed_name".into(),
            json!("suppers_ai__vector__docs"),
        );
        registry_row.insert("model".into(), json!("fastembed"));
        registry_row.insert("dimensions".into(), json!(384));
        registry_row.insert("keyword_search".into(), json!(1));
        db::create(ctx, "suppers_ai__vector__registry", registry_row)
            .await
            .expect("seed registry row");

        // One row in the meta table so the count is non-zero. The handler
        // calls `db::count` against the prefixed storage name (which is the
        // registry-recorded `prefixed_name`).
        let mut meta_row: HashMap<String, serde_json::Value> = HashMap::new();
        meta_row.insert("vector_id".into(), json!("v1"));
        db::create(ctx, "suppers_ai__vector__docs", meta_row)
            .await
            .expect("seed meta row");
    }

    #[tokio::test]
    async fn index_list_page_renders_admin_view() {
        let ctx = TestContext::with_auth().await;
        seed_docs_index(&ctx).await;

        let msg = admin_msg("retrieve", "/b/vector/");
        let resp = index_list_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(
            body.contains("Vector indexes"),
            "missing page header: {body}"
        );
        assert!(body.contains(">docs<"), "seeded row missing: {body}");
        assert!(body.contains("fastembed"), "missing model column: {body}");
    }

    #[tokio::test]
    async fn index_list_page_renders_empty_state_on_fresh_db() {
        // No registry table at all: handler must fall through cleanly to
        // the "No vector indexes yet" empty state, not error out.
        let ctx = TestContext::with_auth().await;

        let msg = admin_msg("retrieve", "/b/vector/");
        let resp = index_list_page(&ctx, &msg).await;
        let body = output_html(resp).await;

        assert!(
            body.contains("Vector indexes"),
            "missing page header: {body}"
        );
        assert!(
            body.contains("No vector indexes yet"),
            "missing empty state copy: {body}"
        );
    }
}
