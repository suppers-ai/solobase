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

/// Detail page hero — for a single resource.
pub struct DetailHero<'a> {
    pub icon: Option<Markup>,        // typically `components::avatar(...)` or an icon
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub badges: Vec<Markup>,         // typically `components::badge(...)` calls
    pub action_menu: Option<Markup>, // dropdown / button group
}

/// One key/value row in the right-rail metadata panel.
pub struct DetailMeta<'a> {
    pub key: &'a str,
    pub value: Markup,
}

/// `detail_page` template.
pub fn detail_page(
    hero: DetailHero<'_>,
    sections: Vec<Markup>,         // typically `components::card(...)` invocations
    meta: Vec<DetailMeta<'_>>,
) -> Markup {
    html! {
        div .page .page--detail {
            header .detail-hero {
                @if let Some(icon) = hero.icon { div .detail-hero__icon { (icon) } }
                div .detail-hero__text {
                    h1 .detail-hero__title { (hero.title) }
                    @if let Some(s) = hero.subtitle { p .detail-hero__subtitle { (s) } }
                    @if !hero.badges.is_empty() {
                        div .detail-hero__badges { @for b in &hero.badges { (b.clone()) } }
                    }
                }
                @if let Some(a) = hero.action_menu { div .detail-hero__action { (a) } }
            }
            div .detail-body {
                div .detail-body__main {
                    @for s in sections { (s) }
                }
                @if !meta.is_empty() {
                    aside .detail-meta {
                        dl {
                            @for row in &meta {
                                dt { (row.key) }
                                dd { (row.value.clone()) }
                            }
                        }
                    }
                }
            }
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

    #[test]
    fn detail_page_renders_hero_sections_and_meta() {
        let hero = DetailHero {
            icon: Some(html! { span .av {} }),
            title: "alice@example.com",
            subtitle: Some("Member since Jan 2026"),
            badges: vec![html! { span .badge { "Admin" } }],
            action_menu: None,
        };
        let sections = vec![
            html! { section .card { "Activity" } },
            html! { section .card { "Sessions" } },
        ];
        let meta = vec![
            DetailMeta { key: "ID", value: html! { code { "u_42" } } },
            DetailMeta { key: "Created", value: html! { "2026-01-12" } },
        ];
        let s = detail_page(hero, sections, meta).into_string();
        assert!(s.contains("detail-hero"));
        assert!(s.contains("alice@example.com"));
        assert!(s.contains("Admin"));
        assert!(s.contains("Activity"));
        assert!(s.contains("Sessions"));
        assert!(s.contains("u_42"));
        assert!(s.contains("Created"));
    }

    #[test]
    fn detail_page_omits_meta_aside_when_empty() {
        let hero = DetailHero { icon: None, title: "X", subtitle: None, badges: vec![], action_menu: None };
        let s = detail_page(hero, vec![], vec![]).into_string();
        assert!(!s.contains("detail-meta"));
    }
}
