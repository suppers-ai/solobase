//! Page templates — six standard layouts used by every block, plus a tiny
//! status template. Each template returns the body markup that goes inside
//! the shell (or the standalone `auth_split` / `status_page`). Pages
//! declare their template inputs and call one function — no bespoke
//! page HTML outside this module.

use maud::{html, Markup};

use super::components;

/// Header line for list / detail / form pages.
pub struct PageHeader<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub primary_action: Option<Markup>, // typically a `components::button(...)` invocation
}

fn render_header(h: &PageHeader<'_>) -> Markup {
    html! {
        header .page-header {
            div .page-header__text {
                h1 .page-header__title { (h.title) }
                @if let Some(s) = h.subtitle { p .page-header__subtitle { (s) } }
            }
            @if let Some(a) = &h.primary_action {
                div .page-header__action { (a.clone()) }
            }
        }
    }
}

/// `list_page` template.
///
/// Sections (each rendered when present):
///   - Page header: title + optional subtitle + optional primary action
///   - Filter row: free-form markup the page provides (search input, facets)
///   - Table: `components::data_table` already handled by caller
///   - Pagination: `components::pagination` already handled by caller
pub fn list_page(
    header: PageHeader<'_>,
    filters: Option<Markup>,
    table: Markup,
    pagination: Option<Markup>,
) -> Markup {
    html! {
        div .page .page--list {
            (render_header(&header))
            @if let Some(f) = filters { div .page-filters { (f) } }
            div .page-body { (table) }
            @if let Some(p) = pagination { div .page-pagination { (p) } }
        }
    }
}

// Suppress unused warning until later phases consume `components`.
#[allow(dead_code)]
fn _components_keep_alive(_: components::BtnVariant) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::components::{button, BtnVariant, CtrlSize};
    use maud::PreEscaped;

    #[test]
    fn list_page_renders_header_table_pagination() {
        let header = PageHeader {
            title: "Users",
            subtitle: Some("142 total"),
            primary_action: Some(button(BtnVariant::Primary, CtrlSize::Md, "+ Invite", PreEscaped(String::new()))),
        };
        let table = html! { div .data-table { table {} } };
        let pagination = Some(html! { nav .pagination { "1/4" } });
        let s = list_page(header, None, table, pagination).into_string();
        assert!(s.contains("page--list"));
        assert!(s.contains(">Users<"));
        assert!(s.contains("142 total"));
        assert!(s.contains("+ Invite"));
        assert!(s.contains("data-table"));
        assert!(s.contains("page-pagination"));
    }

    #[test]
    fn list_page_omits_optional_sections_when_absent() {
        let header = PageHeader { title: "Empty", subtitle: None, primary_action: None };
        let table = html! { div .empty { "none" } };
        let s = list_page(header, None, table, None).into_string();
        assert!(!s.contains("page-filters"));
        assert!(!s.contains("page-pagination"));
        assert!(!s.contains("page-header__action"));
        assert!(!s.contains("page-header__subtitle"));
    }
}
