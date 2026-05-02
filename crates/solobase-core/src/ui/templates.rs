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
    pub icon: Option<Markup>, // typically `components::avatar(...)` or an icon
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub badges: Vec<Markup>, // typically `components::badge(...)` calls
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
    sections: Vec<Markup>, // typically `components::card(...)` invocations
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

/// One section of a form — a labeled group of fields.
pub struct FormSection<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub body: Markup,
}

/// `form_page` template.
///
/// `tabs` is an optional left-rail of section anchors (used by the admin
/// Settings consolidation in Phase 3). Pass `None` for a single-column form.
pub fn form_page(
    header: PageHeader<'_>,
    tabs: Option<Vec<(String, String, bool)>>, // (label, href, is_active)
    sections: Vec<FormSection<'_>>,
    submit_url: &str,
    method: &str,
    save_label: &str,
) -> Markup {
    let has_tabs = tabs.is_some();
    html! {
        div .page .page--form {
            (render_header(&header))
            form .form-page action=(submit_url) method=(method) {
                div .(if has_tabs { "form-grid form-grid--with-tabs" } else { "form-grid" }) {
                    @if let Some(t) = tabs {
                        nav .form-tabs aria-label="Form sections" {
                            ul {
                                @for (label, href, active) in t {
                                    li .(if active { "is-active" } else { "" }) {
                                        a href=(href) aria-current=[active.then_some("page")] { (label) }
                                    }
                                }
                            }
                        }
                    }
                    div .form-sections {
                        @for sec in sections {
                            section .form-section {
                                header .form-section__head {
                                    h2 .form-section__title { (sec.title) }
                                    @if let Some(d) = sec.description {
                                        p .form-section__desc { (d) }
                                    }
                                }
                                div .form-section__body { (sec.body) }
                            }
                        }
                    }
                }
                footer .form-bar {
                    button type="submit" .btn .btn--primary .btn--md { (save_label) }
                }
            }
        }
    }
}

pub struct StatTile<'a> {
    pub label: &'a str,
    pub value: &'a str,         // pre-formatted (caller decides rounding/units)
    pub trend: Option<&'a str>, // e.g. "+12% 7d"
}

pub fn dashboard_page(
    header: PageHeader<'_>,
    stats: Vec<StatTile<'_>>,
    primary_card: Markup,
    secondary_card: Markup,
    full_width_card: Option<Markup>,
    top_card: Option<Markup>,
) -> Markup {
    html! {
        div .page .page--dashboard {
            (render_header(&header))
            @if let Some(tc) = top_card { div .dashboard-top { (tc) } }
            @if !stats.is_empty() {
                div .stats-grid {
                    @for s in &stats {
                        div .stat-tile {
                            div .stat-tile__label { (s.label) }
                            div .stat-tile__value { (s.value) }
                            @if let Some(t) = s.trend { div .stat-tile__trend { (t) } }
                        }
                    }
                }
            }
            div .dashboard-grid {
                div .dashboard-grid__primary { (primary_card) }
                div .dashboard-grid__secondary { (secondary_card) }
            }
            @if let Some(fw) = full_width_card { div .dashboard-wide { (fw) } }
        }
    }
}

pub fn chat_page(
    thread_list: Markup,
    messages: Markup,
    composer: Markup,
    right_rail: Option<Markup>,
) -> Markup {
    html! {
        div .page--chat {
            aside .chat-threads { (thread_list) }
            section .chat-main {
                div .chat-messages { (messages) }
                div .chat-composer { (composer) }
            }
            @if let Some(r) = right_rail {
                aside .chat-rail { (r) }
            }
        }
    }
}

pub struct BrandPanel<'a> {
    pub logo_html: Option<Markup>,
    pub headline: &'a str,
    pub tagline: Option<&'a str>,
}

pub fn auth_split(brand: BrandPanel<'_>, form_card: Markup) -> Markup {
    html! {
        div .auth-split {
            aside .auth-split__brand {
                @if let Some(l) = brand.logo_html { div .auth-split__logo { (l) } }
                h1 .auth-split__headline { (brand.headline) }
                @if let Some(t) = brand.tagline { p .auth-split__tagline { (t) } }
            }
            main .auth-split__form { (form_card) }
        }
    }
}

/// Tiny template for `/`, 404, 403, 500 — auth-split-shaped, just an
/// illustrated message + primary action. Replaces the inline 404/403
/// markup currently in `ui/mod.rs`.
pub fn status_page(
    code: &str, // "404", "403", "500", or "" for "/"
    title: &str,
    body: &str,
    primary_action: Option<(String, String)>, // (label, href)
) -> Markup {
    html! {
        div .status-page {
            div .status-page__inner {
                @if !code.is_empty() { div .status-page__code { (code) } }
                h1 .status-page__title { (title) }
                p .status-page__body { (body) }
                @if let Some((label, href)) = primary_action {
                    a .btn .btn--primary .btn--md href=(href) { (label) }
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
    use maud::PreEscaped;

    use super::*;
    use crate::ui::components::{button, BtnVariant, CtrlSize};

    #[test]
    fn list_page_renders_header_table_pagination() {
        let header = PageHeader {
            title: "Users",
            subtitle: Some("142 total"),
            primary_action: Some(button(
                BtnVariant::Primary,
                CtrlSize::Md,
                "+ Invite",
                PreEscaped(String::new()),
            )),
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
        let header = PageHeader {
            title: "Empty",
            subtitle: None,
            primary_action: None,
        };
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
            DetailMeta {
                key: "ID",
                value: html! { code { "u_42" } },
            },
            DetailMeta {
                key: "Created",
                value: html! { "2026-01-12" },
            },
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
        let hero = DetailHero {
            icon: None,
            title: "X",
            subtitle: None,
            badges: vec![],
            action_menu: None,
        };
        let s = detail_page(hero, vec![], vec![]).into_string();
        assert!(!s.contains("detail-meta"));
    }

    #[test]
    fn form_page_with_tabs_marks_active() {
        let header = PageHeader {
            title: "Settings",
            subtitle: None,
            primary_action: None,
        };
        let tabs = Some(vec![
            (
                "Email".to_string(),
                "/b/admin/settings/email".to_string(),
                false,
            ),
            (
                "Network".to_string(),
                "/b/admin/settings/network".to_string(),
                true,
            ),
        ]);
        let sections = vec![FormSection {
            title: "Network",
            description: None,
            body: html! { "..." },
        }];
        let s = form_page(
            header,
            tabs,
            sections,
            "/b/admin/settings/network",
            "post",
            "Save",
        )
        .into_string();
        assert!(s.contains("form-grid--with-tabs"));
        assert!(s.contains(r#"aria-current="page""#));
        assert!(s.contains("is-active"));
        assert!(s.contains(r#"action="/b/admin/settings/network""#));
        assert!(s.contains(">Save</button>"));
    }

    #[test]
    fn form_page_without_tabs_uses_single_column() {
        let header = PageHeader {
            title: "Profile",
            subtitle: None,
            primary_action: None,
        };
        let sections = vec![FormSection {
            title: "Account",
            description: Some("Public info"),
            body: html! { "..." },
        }];
        let s = form_page(header, None, sections, "/me", "post", "Update").into_string();
        assert!(!s.contains("form-grid--with-tabs"));
        assert!(s.contains("Account"));
        assert!(s.contains("Public info"));
    }

    #[test]
    fn dashboard_renders_stats_and_cards() {
        let header = PageHeader {
            title: "Dashboard",
            subtitle: None,
            primary_action: None,
        };
        let stats = vec![
            StatTile {
                label: "Users",
                value: "142",
                trend: Some("+5 7d"),
            },
            StatTile {
                label: "Storage",
                value: "1.2 GB",
                trend: None,
            },
        ];
        let primary = html! { section .card { "Quick actions" } };
        let secondary = html! { section .card { "Recent activity" } };
        let s = dashboard_page(header, stats, primary, secondary, None, None).into_string();
        assert!(s.contains("stats-grid"));
        assert!(s.contains(">Users<"));
        assert!(s.contains("142"));
        assert!(s.contains("+5 7d"));
        assert!(s.contains("Quick actions"));
        assert!(s.contains("Recent activity"));
        assert!(!s.contains("dashboard-wide"));
    }

    #[test]
    fn dashboard_page_renders_optional_top_card_above_stats() {
        let header = PageHeader {
            title: "Dash",
            subtitle: None,
            primary_action: None,
        };
        let m = dashboard_page(
            header,
            vec![StatTile {
                label: "Users",
                value: "1",
                trend: None,
            }],
            html! { div #primary {} },
            html! { div #secondary {} },
            None,
            Some(html! { div #top-card { "QA" } }),
        );
        let s = m.into_string();
        let top = s.find("dashboard-top").expect("dashboard-top div present");
        let stats = s.find("stats-grid").expect("stats-grid div present");
        assert!(top < stats, "top card must render above stats");
        assert!(s.contains(r#"id="top-card""#));
    }

    #[test]
    fn chat_page_with_rail() {
        let s = chat_page(
            html! { div { "threads" } },
            html! { div { "messages" } },
            html! { textarea {} },
            Some(html! { div { "rail" } }),
        )
        .into_string();
        assert!(s.contains("chat-threads"));
        assert!(s.contains("chat-main"));
        assert!(s.contains("chat-messages"));
        assert!(s.contains("chat-composer"));
        assert!(s.contains("chat-rail"));
        assert!(s.contains(">rail<"));
    }

    #[test]
    fn chat_page_no_rail_omits_aside() {
        let s = chat_page(
            html! { div { "threads" } },
            html! { div {} },
            html! { textarea {} },
            None,
        )
        .into_string();
        assert!(!s.contains("chat-rail"));
    }

    #[test]
    fn auth_split_renders_brand_and_form() {
        let brand = BrandPanel {
            logo_html: Some(html! { div .logo {} }),
            headline: "Welcome back",
            tagline: Some("Sign in to continue."),
        };
        let form = html! { section .card { "form" } };
        let s = auth_split(brand, form).into_string();
        assert!(s.contains("auth-split__brand"));
        assert!(s.contains("auth-split__form"));
        assert!(s.contains("Welcome back"));
        assert!(s.contains("Sign in to continue."));
    }

    #[test]
    fn status_page_404_renders_code_and_action() {
        let s = status_page(
            "404",
            "Page not found",
            "We couldn't find that page.",
            Some(("Go home".to_string(), "/".to_string())),
        )
        .into_string();
        assert!(s.contains(">404<"));
        assert!(s.contains("Page not found"));
        assert!(s.contains("Go home"));
        assert!(s.contains(r#"href="/""#));
    }

    #[test]
    fn status_page_no_code_no_action() {
        let s = status_page("", "Hello", "Welcome.", None).into_string();
        assert!(!s.contains("status-page__code"));
        assert!(!s.contains(r#"class="btn"#));
    }
}
