//! Shared UI components rendered with maud.

use maud::{html, Markup};

use super::icons;

// ---------------------------------------------------------------------------
// Data Table
// ---------------------------------------------------------------------------

/// Column definition for a data table.
pub struct Column {
    pub key: &'static str,
    pub label: &'static str,
    pub sortable: bool,
}

/// Options for rendering a data table.
pub struct TableOptions<'a> {
    pub id: &'a str,
    /// htmx target for sorting/pagination requests
    pub hx_target: Option<&'a str>,
    /// Base URL for sort/page links
    pub base_url: Option<&'a str>,
    pub current_sort: Option<&'a str>,
    pub sort_dir: Option<&'a str>,
}

impl<'a> Default for TableOptions<'a> {
    fn default() -> Self {
        Self {
            id: "data-table",
            hx_target: None,
            base_url: None,
            current_sort: None,
            sort_dir: None,
        }
    }
}

/// Render a data table from JSON rows.
pub fn data_table(
    columns: &[Column],
    rows: &[serde_json::Value],
    options: &TableOptions<'_>,
) -> Markup {
    html! {
        div .table-container {
            table .table id=(options.id) {
                thead {
                    tr {
                        @for col in columns {
                            @if col.sortable {
                                @let is_sorted = options.current_sort == Some(col.key);
                                @let next_dir = if is_sorted && options.sort_dir == Some("asc") { "desc" } else { "asc" };
                                th .sortable
                                    hx-get={
                                        (options.base_url.unwrap_or(""))
                                        "?sort=" (col.key) "&dir=" (next_dir)
                                    }
                                    hx-target=(options.hx_target.unwrap_or("#data-table"))
                                    hx-swap="outerHTML"
                                {
                                    (col.label)
                                    @if is_sorted {
                                        @if options.sort_dir == Some("asc") {
                                            " " (icons::chevron_up())
                                        } @else {
                                            " " (icons::chevron_down())
                                        }
                                    }
                                }
                            } @else {
                                th { (col.label) }
                            }
                        }
                    }
                }
                tbody {
                    @if rows.is_empty() {
                        tr {
                            td colspan=(columns.len().to_string()) .text-center .text-muted style="padding: 2rem;" {
                                "No data found"
                            }
                        }
                    }
                    @for row in rows {
                        tr {
                            @for col in columns {
                                td {
                                    @match row.get(col.key) {
                                        Some(serde_json::Value::String(s)) => (s),
                                        Some(serde_json::Value::Number(n)) => (n),
                                        Some(serde_json::Value::Bool(b)) => (b),
                                        Some(serde_json::Value::Null) | None => "",
                                        Some(other) => (other),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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

/// Render pagination controls.
pub fn pagination(current_page: u32, total_pages: u32, base_url: &str, hx_target: &str) -> Markup {
    if total_pages <= 1 {
        return html! {};
    }
    html! {
        div .pagination {
            div .pagination-info {
                "Page " (current_page) " of " (total_pages)
            }
            div .pagination-controls {
                @if current_page > 1 {
                    button .pagination-btn
                        hx-get={ (base_url) "?page=" (current_page - 1) }
                        hx-target=(hx_target)
                    {
                        (icons::chevron_left())
                    }
                } @else {
                    button .pagination-btn disabled { (icons::chevron_left()) }
                }

                @if current_page < total_pages {
                    button .pagination-btn
                        hx-get={ (base_url) "?page=" (current_page + 1) }
                        hx-target=(hx_target)
                    {
                        (icons::chevron_right())
                    }
                } @else {
                    button .pagination-btn disabled { (icons::chevron_right()) }
                }
            }
        }
    }
}

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

/// Render an empty state placeholder.
pub fn empty_state(title: &str, description: &str) -> Markup {
    html! {
        div .empty-state {
            div .empty-state-icon { (icons::package()) }
            div .empty-state-title { (title) }
            div .empty-state-description { (description) }
        }
    }
}

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
    if p.required { s.push_str(" required"); }
    if p.disabled { s.push_str(" disabled"); }
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

#[cfg(test)]
mod tests {
    use super::*;
    use maud::PreEscaped;

    #[test]
    fn button_primary_md() {
        let m = button(BtnVariant::Primary, CtrlSize::Md, "Save", PreEscaped(String::new()));
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
        assert!(s.contains(r#"hx-delete="/users/1""#), "extra attrs missing: {s}");
        assert!(s.contains("btn--danger"), "variant missing: {s}");
    }

    #[test]
    fn text_input_renders_label_and_value() {
        let p = FieldProps { label: "Email", name: "email", required: true, ..Default::default() };
        let s = text_input(p, "email", "alice@example.com").into_string();
        assert!(s.contains(r#"for="field-email""#));
        assert!(s.contains("Email"));
        assert!(s.contains(r#"value="alice@example.com""#));
        assert!(s.contains("required"));
        assert!(s.contains("field__req"));
    }

    #[test]
    fn select_marks_selected_option() {
        let p = FieldProps { label: "Role", name: "role", ..Default::default() };
        let opts = [("user", "User"), ("admin", "Admin")];
        let s = select_input(p, &opts, "admin").into_string();
        assert!(s.contains(r#"value="admin" selected"#));
        assert!(!s.contains(r#"value="user" selected"#));
    }

    #[test]
    fn textarea_escapes_content() {
        let p = FieldProps { label: "Bio", name: "bio", ..Default::default() };
        let s = textarea_input(p, "<script>x</script>", 4).into_string();
        assert!(s.contains("&lt;script&gt;"), "unescaped: {s}");
        assert!(!s.contains("<script>x</script>"));
    }
}
