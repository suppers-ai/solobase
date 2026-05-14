//! Shared UI components rendered with maud.

use maud::{html, Markup};

use super::icons;

// ---------------------------------------------------------------------------
// Data Table
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tab Navigation
// ---------------------------------------------------------------------------

/// A tab definition.
pub struct Tab {
    pub id: &'static str,
    pub label: &'static str,
    pub href: String,
    pub icon: Option<&'static str>,
}

/// Render a tab navigation bar.
pub fn tab_navigation(tabs: &[Tab], active_id: &str) -> Markup {
    html! {
        div .tabs {
            @for tab in tabs {
                a .tab
                    .(if tab.id == active_id { "active" } else { "" })
                    href=(tab.href)
                    hx-get=(tab.href)
                    hx-target="#content"
                    hx-push-url="true"
                {
                    @if let Some(icon_name) = tab.icon {
                        span .nav-icon { (super::sidebar::nav_icon(icon_name)) }
                    }
                    (tab.label)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stat Card
// ---------------------------------------------------------------------------

/// Render a stat card.
pub fn stat_card(label: &str, value: &str, icon: Markup) -> Markup {
    html! {
        div .stat-card {
            div .stat-header {
                div .stat-content {
                    div .stat-label { (label) }
                    div .stat-value { (value) }
                }
                div .stat-icon { (icon) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Search Input
// ---------------------------------------------------------------------------

/// Render a search input with htmx-powered search.
/// If `current_value` is non-empty, shows a "Results for X" banner with a clear button.
pub fn search_input(name: &str, placeholder: &str, hx_get: &str, hx_target: &str) -> Markup {
    search_input_with_value(name, placeholder, hx_get, hx_target, "")
}

/// Search input with a pre-filled value and results banner.
pub fn search_input_with_value(
    name: &str,
    placeholder: &str,
    hx_get: &str,
    hx_target: &str,
    current_value: &str,
) -> Markup {
    html! {
        @if !current_value.is_empty() {
            div .flex .items-center .gap-2 .mb-2 style="font-size:0.875rem" {
                span .text-muted { "Results for " }
                span .font-semibold { "\"" (current_value) "\"" }
                a .btn .btn-ghost .btn-sm
                    href=(hx_get)
                    hx-get=(hx_get)
                    hx-target=(hx_target)
                { (icons::x()) " Clear" }
            }
        }
        div .search-input {
            span .search-input-icon { (icons::search()) }
            input .form-input
                type="search"
                name=(name)
                placeholder=(placeholder)
                value=(current_value)
                hx-get=(hx_get)
                hx-trigger="input changed delay:300ms, search"
                hx-target=(hx_target)
                autocomplete="off";
        }
    }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Status Badge
// ---------------------------------------------------------------------------

/// Render a colored status badge.
pub fn status_badge(status: &str) -> Markup {
    let class = match status.to_lowercase().as_str() {
        "active" | "enabled" | "completed" | "running" => "badge-success",
        "inactive" | "disabled" | "stopped" => "badge-danger",
        "pending" | "draft" => "badge-warning",
        _ => "badge-info",
    };
    html! {
        span .badge .(class) { (status) }
    }
}

// ---------------------------------------------------------------------------
// Modal
// ---------------------------------------------------------------------------

/// Render a modal container (hidden by default).
pub fn modal(id: &str, title: &str, body: Markup) -> Markup {
    html! {
        div .modal-overlay id=(id) hidden
            onclick="if(event.target===this)closeModal(this.id)"
        {
            div .modal {
                div .modal-header {
                    h3 .modal-title { (title) }
                    button .modal-close onclick={"closeModal('" (id) "')"} {
                        (icons::x())
                    }
                }
                div .modal-body {
                    (body)
                }
            }
        }
    }
}

/// Render a modal with a footer (for action buttons).
pub fn modal_with_footer(id: &str, title: &str, body: Markup, footer: Markup) -> Markup {
    html! {
        div .modal-overlay id=(id) hidden
            onclick="if(event.target===this)closeModal(this.id)"
        {
            div .modal {
                div .modal-header {
                    h3 .modal-title { (title) }
                    button .modal-close onclick={"closeModal('" (id) "')"} {
                        (icons::x())
                    }
                }
                div .modal-body {
                    (body)
                }
                div .modal-footer {
                    (footer)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Empty State
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Loading Spinner
// ---------------------------------------------------------------------------

/// Render a loading spinner.
pub fn loading_spinner(message: Option<&str>) -> Markup {
    html! {
        div .loading-spinner {
            div .spinner {}
            @if let Some(msg) = message {
                div .text-muted { (msg) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Page Header
// ---------------------------------------------------------------------------

/// Render a page header with title, optional subtitle, and optional action slot.
pub fn page_header(title: &str, subtitle: Option<&str>, action: Option<Markup>) -> Markup {
    html! {
        div .flex .items-center .justify-between .mb-4 {
            div {
                h1 .page-title { (title) }
                @if let Some(sub) = subtitle {
                    p .page-subtitle { (sub) }
                }
            }
            @if let Some(action) = action {
                div { (action) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Confirm Dialog (htmx pattern)
// ---------------------------------------------------------------------------

/// Render a confirm dialog for destructive actions.
pub fn confirm_dialog(
    id: &str,
    title: &str,
    message: &str,
    confirm_label: &str,
    hx_action: &str,
) -> Markup {
    modal_with_footer(
        id,
        title,
        html! { p { (message) } },
        html! {
            button .btn .btn-secondary onclick={"closeModal('" (id) "')"} { "Cancel" }
            button .btn .btn-danger
                hx-post=(hx_action)
                hx-target="body"
            { (confirm_label) }
        },
    )
}

// ---------------------------------------------------------------------------
// Toast container (rendered once per page)
// ---------------------------------------------------------------------------

/// The toast container div — included automatically by `layout::page()`.
pub fn toast_container() -> Markup {
    html! {
        div #toast-container .toast-container {}
    }
}

// ---------------------------------------------------------------------------
// Canonical button (Phase 1)
// ---------------------------------------------------------------------------

/// Visual variant for buttons.
#[derive(Debug, Clone, Copy)]
pub enum BtnVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Size for buttons (and other form controls when adopted).
#[derive(Debug, Clone, Copy)]
pub enum CtrlSize {
    Sm,
    Md,
    Lg,
}

impl BtnVariant {
    fn class(self) -> &'static str {
        match self {
            BtnVariant::Primary => "btn btn--primary",
            BtnVariant::Secondary => "btn btn--secondary",
            BtnVariant::Ghost => "btn btn--ghost",
            BtnVariant::Danger => "btn btn--danger",
        }
    }
}

impl CtrlSize {
    fn class(self) -> &'static str {
        match self {
            CtrlSize::Sm => "btn--sm",
            CtrlSize::Md => "btn--md",
            CtrlSize::Lg => "btn--lg",
        }
    }
}

/// Canonical button. Use for every button on every new page.
///
/// `extra_attrs` is a maud `PreEscaped` block of additional attributes
/// (e.g. `hx-post=...`, `type="submit"`, `disabled`). Pass
/// `maud::PreEscaped(String::new())` if none.
pub fn button(
    variant: BtnVariant,
    size: CtrlSize,
    label: &str,
    extra_attrs: maud::PreEscaped<String>,
) -> maud::Markup {
    use maud::PreEscaped;
    let class = format!("{} {}", variant.class(), size.class());
    let extra = extra_attrs.0;
    let label_escaped = maud::html! { (label) }.into_string();
    PreEscaped(format!(
        r#"<button class="{class}" {extra}>{label_escaped}</button>"#,
    ))
}

// ---------------------------------------------------------------------------
// Form-control components (Phase 1)
// ---------------------------------------------------------------------------

/// Common props for any labeled form control.
#[derive(Default)]
pub struct FieldProps<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub helper: Option<&'a str>,
    pub error: Option<&'a str>,
    pub required: bool,
    pub disabled: bool,
}

fn field_attrs(p: &FieldProps) -> String {
    let mut s = format!(r#"name="{}""#, html_escape(p.name));
    if p.required {
        s.push_str(" required");
    }
    if p.disabled {
        s.push_str(" disabled");
    }
    s
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn field_wrap(p: &FieldProps, control: maud::Markup) -> maud::Markup {
    use maud::html;
    let id = format!("field-{}", p.name);
    let has_error = p.error.is_some();
    html! {
        div .field .(if has_error { "field--error" } else { "" }) {
            @if !p.label.is_empty() {
                label for=(id) .field__label { (p.label) @if p.required { span .field__req { " *" } } }
            }
            (control)
            @if let Some(h) = p.helper { div .field__helper { (h) } }
            @if let Some(e) = p.error { div .field__error { (e) } }
        }
    }
}

pub fn text_input<'a>(p: FieldProps<'a>, input_type: &'a str, value: &'a str) -> maud::Markup {
    use maud::{html, PreEscaped};
    let id = format!("field-{}", p.name);
    let attrs = field_attrs(&p);
    let v = html_escape(value);
    let t = html_escape(input_type);
    let inner = html! {
        (PreEscaped(format!(
            r#"<input id="{id}" type="{t}" value="{v}" class="field__input" {attrs} />"#,
        )))
    };
    field_wrap(&p, inner)
}

pub fn textarea_input<'a>(p: FieldProps<'a>, value: &'a str, rows: u32) -> maud::Markup {
    use maud::{html, PreEscaped};
    let id = format!("field-{}", p.name);
    let attrs = field_attrs(&p);
    let v = html_escape(value);
    let inner = html! {
        (PreEscaped(format!(
            r#"<textarea id="{id}" class="field__input field__input--textarea" rows="{rows}" {attrs}>{v}</textarea>"#,
        )))
    };
    field_wrap(&p, inner)
}

pub fn select_input<'a>(
    p: FieldProps<'a>,
    options: &[(&'a str, &'a str)], // (value, label)
    selected: &'a str,
) -> maud::Markup {
    use maud::html;
    let id = format!("field-{}", p.name);
    let inner = html! {
        select id=(id) .field__input name=(p.name) required[p.required] disabled[p.disabled] {
            @for (val, label) in options {
                option value=(val) selected[*val == selected] { (label) }
            }
        }
    };
    field_wrap(&p, inner)
}

// ---------------------------------------------------------------------------
// data_table, empty_state, pagination (Phase 1)
// ---------------------------------------------------------------------------

/// One column declaration for `data_table`.
pub struct TableCol<'a> {
    pub label: &'a str,
    pub width: Option<&'a str>, // CSS width, e.g. "160px" or "30%"
}

/// `data_table` — caller passes pre-rendered cell markup per row.
/// Sticky header. Optional row-link via `row_href` closure.
///
/// `rows` is a Vec because we need to know if it's empty. If empty, an
/// `empty_state` is rendered in place of the table body.
pub fn data_table<'a, F>(
    columns: &[TableCol<'a>],
    rows: Vec<Vec<maud::Markup>>,
    row_href: Option<F>,
    empty: maud::Markup,
) -> maud::Markup
where
    F: Fn(usize) -> Option<String>,
{
    use maud::html;
    if rows.is_empty() {
        return html! { div .data-table__empty { (empty) } };
    }
    html! {
        div .data-table {
            table {
                thead { tr {
                    @for col in columns {
                        @match col.width {
                            Some(w) => th style=(format!("width:{w}")) { (col.label) },
                            None => th { (col.label) },
                        }
                    }
                } }
                tbody {
                    @for (i, cells) in rows.into_iter().enumerate() {
                        @let href = row_href.as_ref().and_then(|f| f(i));
                        tr .(if href.is_some() { "data-table__row data-table__row--linked" } else { "data-table__row" }) {
                            @for cell in cells { td { (cell) } }
                            @if let Some(h) = href {
                                td .data-table__row-href { a href=(h) aria-label="Open" { "›" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn empty_state(
    icon: maud::Markup,
    title: &str,
    body: &str,
    action: Option<maud::Markup>,
) -> maud::Markup {
    use maud::html;
    html! {
        div .empty {
            div .empty__icon { (icon) }
            h3 .empty__title { (title) }
            p .empty__body { (body) }
            @if let Some(a) = action { div .empty__action { (a) } }
        }
    }
}

pub fn pagination(page: u32, per_page: u32, total: u32, base_href: &str) -> maud::Markup {
    use maud::{html, PreEscaped};
    let total_pages = if total == 0 {
        1
    } else {
        (total + per_page - 1) / per_page
    };
    let prev_disabled = page <= 1;
    let next_disabled = page >= total_pages;
    let join = if base_href.contains('?') { '&' } else { '?' };
    let prev_href = format!(
        "{}{}page={}",
        base_href,
        join,
        page.saturating_sub(1).max(1)
    );
    let next_href = format!("{}{}page={}", base_href, join, (page + 1).min(total_pages));
    html! {
        nav .pagination aria-label="Pagination" {
            span .pagination__count { (format!("{} total", total)) }
            a .pagination__prev .(if prev_disabled { "is-disabled" } else { "" })
                href=(PreEscaped(&prev_href)) aria-disabled=(prev_disabled.to_string()) { "‹ Prev" }
            span .pagination__page { (format!("{} / {}", page, total_pages)) }
            a .pagination__next .(if next_disabled { "is-disabled" } else { "" })
                href=(PreEscaped(&next_href)) aria-disabled=(next_disabled.to_string()) { "Next ›" }
        }
    }
}

// ---------------------------------------------------------------------------
// Card, Badge, Avatar (Phase 1)
// ---------------------------------------------------------------------------

/// Card — wrapped panel with optional title and action slot.
pub fn card(
    title: Option<&str>,
    body: maud::Markup,
    actions: Option<maud::Markup>,
) -> maud::Markup {
    use maud::html;
    html! {
        section .card {
            @if title.is_some() || actions.is_some() {
                header .card__head {
                    @if let Some(t) = title { h3 .card__title { (t) } }
                    @if let Some(a) = actions { div .card__actions { (a) } }
                }
            }
            div .card__body { (body) }
        }
    }
}

/// Badge — small status pill.
#[derive(Debug, Clone, Copy)]
pub enum BadgeVariant {
    Neutral,
    Admin,
    User,
    Success,
    Warning,
    Danger,
}

impl BadgeVariant {
    fn class(self) -> &'static str {
        match self {
            BadgeVariant::Neutral => "badge badge--neutral",
            BadgeVariant::Admin => "badge badge--admin",
            BadgeVariant::User => "badge badge--user",
            BadgeVariant::Success => "badge badge--success",
            BadgeVariant::Warning => "badge badge--warning",
            BadgeVariant::Danger => "badge badge--danger",
        }
    }
}

pub fn badge(variant: BadgeVariant, label: &str) -> maud::Markup {
    use maud::html;
    html! { span class=(variant.class()) { (label) } }
}

/// Avatar — flat brand-orange circle with the seed's first character. The
/// initial varies per user, but the background does not: a single brand
/// color across every avatar is calmer to scan than a wall of per-hash
/// hues, and the colored variants previously made the admin lists feel
/// like distinct personas instead of rows of the same shape.
pub fn avatar(seed: &str, size: CtrlSize) -> maud::Markup {
    use maud::html;
    let initial = seed.chars().next().unwrap_or('?').to_ascii_uppercase();
    let size_class = match size {
        CtrlSize::Sm => "avatar--sm",
        CtrlSize::Md => "avatar--md",
        CtrlSize::Lg => "avatar--lg",
    };
    html! {
        span class={ "avatar " (size_class) } { (initial) }
    }
}

#[cfg(test)]
mod tests {
    use maud::PreEscaped;

    use super::*;

    #[test]
    fn button_primary_md() {
        let m = button(
            BtnVariant::Primary,
            CtrlSize::Md,
            "Save",
            PreEscaped(String::new()),
        );
        let s = m.into_string();
        assert!(s.contains("btn--primary"), "missing variant class: {s}");
        assert!(s.contains("btn--md"), "missing size class: {s}");
        assert!(s.contains(">Save</button>"), "missing label: {s}");
    }

    #[test]
    fn button_extra_attrs_pass_through() {
        let m = button(
            BtnVariant::Danger,
            CtrlSize::Sm,
            "Delete",
            PreEscaped(r#"hx-delete="/users/1" type="button""#.to_string()),
        );
        let s = m.into_string();
        assert!(
            s.contains(r#"hx-delete="/users/1""#),
            "extra attrs missing: {s}"
        );
        assert!(s.contains("btn--danger"), "variant missing: {s}");
    }

    #[test]
    fn text_input_renders_label_and_value() {
        let p = FieldProps {
            label: "Email",
            name: "email",
            required: true,
            ..Default::default()
        };
        let s = text_input(p, "email", "alice@example.com").into_string();
        assert!(s.contains(r#"for="field-email""#));
        assert!(s.contains("Email"));
        assert!(s.contains(r#"value="alice@example.com""#));
        assert!(s.contains("required"));
        assert!(s.contains("field__req"));
    }

    #[test]
    fn select_marks_selected_option() {
        let p = FieldProps {
            label: "Role",
            name: "role",
            ..Default::default()
        };
        let opts = [("user", "User"), ("admin", "Admin")];
        let s = select_input(p, &opts, "admin").into_string();
        assert!(s.contains(r#"value="admin" selected"#));
        assert!(!s.contains(r#"value="user" selected"#));
    }

    #[test]
    fn textarea_escapes_content() {
        let p = FieldProps {
            label: "Bio",
            name: "bio",
            ..Default::default()
        };
        let s = textarea_input(p, "<script>x</script>", 4).into_string();
        assert!(s.contains("&lt;script&gt;"), "unescaped: {s}");
        assert!(!s.contains("<script>x</script>"));
    }

    #[test]
    fn card_renders_title_and_body() {
        let body = maud::html! { p { "hello" } };
        let s = card(Some("Recent activity"), body, None).into_string();
        assert!(s.contains("card__title"));
        assert!(s.contains("Recent activity"));
        assert!(s.contains("hello"));
    }

    #[test]
    fn card_omits_head_when_no_title_no_actions() {
        let body = maud::html! { p { "x" } };
        let s = card(None, body, None).into_string();
        assert!(!s.contains("card__head"));
    }

    #[test]
    fn badge_admin_has_class_and_label() {
        let s = badge(BadgeVariant::Admin, "Admin").into_string();
        assert!(s.contains("badge--admin"));
        assert!(s.contains(">Admin</span>"));
    }

    #[test]
    fn avatar_uses_brand_color_for_every_seed() {
        // Background lives in CSS (`.avatar { background: var(--primary-color) }`),
        // so the rendered markup carries no inline gradient/hue style — every
        // avatar is the same flat brand-orange circle, only the initial varies.
        let a = avatar("alice@example.com", CtrlSize::Md).into_string();
        let b = avatar("bob@example.com", CtrlSize::Md).into_string();
        for s in [&a, &b] {
            assert!(!s.contains("linear-gradient"), "no gradient in markup: {s}");
            assert!(!s.contains("hsl("), "no inline hue in markup: {s}");
            assert!(s.contains("avatar--md"), "size class present: {s}");
        }
        assert!(a.contains(">A</span>"), "alice initial A: {a}");
        assert!(b.contains(">B</span>"), "bob initial B: {b}");
    }

    #[test]
    fn avatar_is_deterministic_per_seed() {
        // Two calls with the same seed render identical markup.
        let a = avatar("alice@example.com", CtrlSize::Md).into_string();
        let b = avatar("alice@example.com", CtrlSize::Md).into_string();
        assert_eq!(a, b);
    }

    #[test]
    fn data_table_empty_renders_empty_slot() {
        let cols = [TableCol {
            label: "Name",
            width: None,
        }];
        let empty = empty_state(
            maud::html! { "📭" },
            "No users",
            "Invite someone to get started.",
            None,
        );
        let s =
            data_table::<fn(usize) -> Option<String>>(&cols, Vec::new(), None, empty).into_string();
        assert!(s.contains("data-table__empty"));
        assert!(s.contains("No users"));
        assert!(!s.contains("<tbody>"));
    }

    #[test]
    fn data_table_with_rows_renders_thead_and_tbody() {
        let cols = [
            TableCol {
                label: "Name",
                width: Some("200px"),
            },
            TableCol {
                label: "Role",
                width: None,
            },
        ];
        let rows = vec![
            vec![maud::html! { "alice" }, maud::html! { "admin" }],
            vec![maud::html! { "bob" }, maud::html! { "user" }],
        ];
        let s = data_table::<fn(usize) -> Option<String>>(
            &cols,
            rows,
            None,
            empty_state(maud::html! {}, "", "", None),
        )
        .into_string();
        assert!(s.contains("<thead>"));
        assert!(s.contains("<tbody>"));
        assert!(s.contains("alice"));
        assert!(s.contains(r#"style="width:200px""#));
    }

    #[test]
    fn data_table_row_href_renders_link_cell() {
        let cols = [TableCol {
            label: "Name",
            width: None,
        }];
        let rows = vec![vec![maud::html! { "alice" }]];
        let s = data_table(
            &cols,
            rows,
            Some(|i: usize| Some(format!("/users/{i}"))),
            empty_state(maud::html! {}, "", "", None),
        )
        .into_string();
        assert!(s.contains(r#"href="/users/0""#));
        assert!(s.contains("data-table__row--linked"));
    }

    #[test]
    fn pagination_clamps_prev_at_page_1() {
        let s = pagination(1, 25, 100, "/users").into_string();
        assert!(s.contains(r#"aria-disabled="true""#)); // prev disabled at page 1
        assert!(s.contains("100 total"));
        assert!(s.contains("1 / 4"));
    }

    #[test]
    fn pagination_appends_query_correctly() {
        let s = pagination(2, 10, 30, "/users?role=admin").into_string();
        assert!(s.contains("/users?role=admin&page=1"));
        assert!(s.contains("/users?role=admin&page=3"));
    }
}
