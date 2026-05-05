//! User-facing UI pages for the suppers-ai/vector block.
//!
//! Pure render helpers live alongside async handlers; helpers are
//! unit-tested directly without `Context`.

use maud::{html, Markup};

const TABLE_PREFIX: &str = "suppers_ai__vector__";

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
            div.empty-state {
                p { "No vector indexes yet." }
            }
        };
    }

    html! {
        table.data-table {
            thead { tr {
                th { "Name" }
                th { "Model" }
                th { "Dimensions" }
                th { "Vectors" }
                th { "Keyword search" }
            } }
            tbody {
                @for r in rows {
                    tr data-index-name=(strip_prefix(&r.name)) {
                        td { (strip_prefix(&r.name)) }
                        td { (r.model) }
                        td { (r.dimensions) }
                        td { (r.vector_count) }
                        td {
                            @if r.keyword_search {
                                span.badge.badge-success { "Yes" }
                            } @else {
                                span.badge { "No" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn strip_prefix(name: &str) -> &str {
    name.strip_prefix(TABLE_PREFIX).unwrap_or(name)
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
