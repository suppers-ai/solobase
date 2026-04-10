//! SSR pages for the products block (admin + user views).

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup, PreEscaped};
use wafer_core::clients::config;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use super::{GROUPS_COLLECTION, PRICING_COLLECTION, PRODUCTS_COLLECTION, PURCHASES_COLLECTION};

/// Admin nav items.
fn products_admin_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Overview".into(),
            href: "/b/products/admin/".into(),
            icon: "bar-chart",
        },
        NavItem {
            label: "Products".into(),
            href: "/b/products/admin/manage".into(),
            icon: "package",
        },
        NavItem {
            label: "Groups".into(),
            href: "/b/products/admin/groups".into(),
            icon: "folder",
        },
        NavItem {
            label: "Pricing".into(),
            href: "/b/products/admin/pricing".into(),
            icon: "dollar-sign",
        },
        NavItem {
            label: "Purchases".into(),
            href: "/b/products/admin/purchases".into(),
            icon: "shopping-cart",
        },
        NavItem {
            label: "Settings".into(),
            href: "/b/products/admin/settings".into(),
            icon: "settings",
        },
    ]
}

fn products_page(
    title: &str,
    config: &SiteConfig,
    path: &str,
    user: Option<&UserInfo>,
    content: Markup,
    msg: &mut Message,
) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(
        title,
        config,
        &products_admin_nav(),
        user,
        path,
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Admin: Overview (stats)
// ---------------------------------------------------------------------------

pub async fn overview(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let one = ListOptions {
        limit: 1,
        ..Default::default()
    };

    let products_count = db::list(ctx, PRODUCTS_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let groups_count = db::list(ctx, GROUPS_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let purchases_count = db::list(ctx, PURCHASES_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let pricing_count = db::list(ctx, PRICING_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);

    let content = html! {
        (components::page_header("Products Overview", Some("Product catalog statistics"), None))
        div .stats-grid {
            (components::stat_card("Products", &products_count.to_string(), icons::package()))
            (components::stat_card("Groups", &groups_count.to_string(), icons::folder()))
            (components::stat_card("Pricing Templates", &pricing_count.to_string(), icons::dollar_sign()))
            (components::stat_card("Purchases", &purchases_count.to_string(), icons::shopping_cart()))
        }
    };

    products_page(
        "Products",
        &config,
        "/b/products/admin/",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Admin: Manage Products
// ---------------------------------------------------------------------------

pub async fn manage_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let (page, page_size, _) = msg.pagination_params(20);
    let search = msg.query("search").to_string();

    let mut filters = vec![Filter {
        field: "deleted_at".into(),
        operator: FilterOp::IsNull,
        value: serde_json::Value::Null,
    }];
    if !search.is_empty() {
        filters.push(Filter {
            field: "name".into(),
            operator: FilterOp::Like,
            value: serde_json::Value::String(format!("%{search}%")),
        });
    }

    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        PRODUCTS_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    let content = html! {
        (components::page_header("Products", Some("Manage your product catalog"), None))

        div .filter-bar {
            (components::search_input("search", "Search products...", "/b/products/admin/manage", "#products-content"))
        }

        div #products-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead {
                                tr {
                                    th { "Name" }
                                    th { "Status" }
                                    th { "Price" }
                                    th { "Group" }
                                    th { "Created" }
                                }
                            }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="5" .text-center .text-muted style="padding:2rem;" { "No products found" } }
                                }
                                @for record in &list.records {
                                    @let name = record.str_field("name");
                                    @let status = record.str_field("status");
                                    @let price = record.str_field("price");
                                    @let currency = record.str_field("currency");
                                    @let group_id = record.str_field("group_id");
                                    @let created = record.str_field("created_at");
                                    tr {
                                        td .font-medium { (name) }
                                        td { (components::status_badge(status)) }
                                        td { (price) " " span .text-muted { (currency) } }
                                        td .text-muted .text-sm { @if group_id.is_empty() { "—" } @else { (group_id.get(..8).unwrap_or(group_id)) } }
                                        td .text-muted .text-sm { (created.get(..10).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                    @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                    (components::pagination(list.page as u32, total_pages, "/b/products/admin/manage", "#products-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    products_page(
        "Products",
        &config,
        "/b/products/admin/manage",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Admin: Groups
// ---------------------------------------------------------------------------

pub async fn groups(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let opts = ListOptions {
        sort: vec![SortField {
            field: "name".into(),
            desc: false,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, GROUPS_COLLECTION, &opts).await;

    let content = html! {
        (components::page_header("Groups", Some("Organize products into groups"), None))

        div #groups-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "Description" } th { "Status" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No groups" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
                                        td .text-muted .text-sm { (r.str_field("description")) }
                                        td { (components::status_badge(r.str_field("status"))) }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    products_page(
        "Groups",
        &config,
        "/b/products/admin/groups",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Admin: Pricing Templates
// ---------------------------------------------------------------------------

pub async fn pricing(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let opts = ListOptions {
        sort: vec![SortField {
            field: "name".into(),
            desc: false,
        }],
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, PRICING_COLLECTION, &opts).await;

    let content = html! {
        (components::page_header("Pricing Templates", Some("Define pricing formulas for products"), None))

        div #pricing-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "Formula" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="3" .text-center .text-muted style="padding:2rem;" { "No pricing templates" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
                                        td .text-sm { code { (r.str_field("price_formula")) } }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    products_page(
        "Pricing",
        &config,
        "/b/products/admin/pricing",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Admin: Purchases
// ---------------------------------------------------------------------------

pub async fn purchases(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let (page, page_size, _) = msg.pagination_params(20);
    let status_filter = msg.query("status").to_string();

    let mut filters = Vec::new();
    if !status_filter.is_empty() && status_filter != "all" {
        filters.push(Filter {
            field: "status".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(status_filter.clone()),
        });
    }

    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        PURCHASES_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    let content = html! {
        (components::page_header("Purchases", Some("Track customer orders and payments"), None))

        // Status filter
        div .filter-bar {
            @for s in &["all", "pending", "completed", "refunded", "cancelled"] {
                a .btn .(if (status_filter.is_empty() && *s == "all") || status_filter == *s { "btn-primary" } else { "btn-secondary" })
                    .btn-sm
                    href={"/b/products/purchases?status=" (*s)}
                    hx-get={"/b/products/purchases?status=" (*s)}
                    hx-target="#content"
                    hx-push-url="true"
                { (*s) }
            }
        }

        div #purchases-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "User" } th { "Status" } th { "Total" } th { "Provider" } th { "Date" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="5" .text-center .text-muted style="padding:2rem;" { "No purchases" } }
                                }
                                @for r in &list.records {
                                    @let total_cents = r.i64_field("total_cents");
                                    @let amount = format!("{:.2}", total_cents as f64 / 100.0);
                                    tr {
                                        td .text-sm { (r.str_field("user_id").get(..8).unwrap_or("—")) }
                                        td { (components::status_badge(r.str_field("status"))) }
                                        td .font-medium { (amount) " " span .text-muted { (r.str_field("currency")) } }
                                        td .text-muted .text-sm { (r.str_field("provider")) }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                    @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                    (components::pagination(list.page as u32, total_pages, "/b/products/admin/purchases", "#purchases-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    products_page(
        "Purchases",
        &config,
        "/b/products/admin/purchases",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// User: My Products
// ---------------------------------------------------------------------------

pub async fn my_products(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();
    let (page, page_size, _) = msg.pagination_params(20);

    let filters = vec![
        Filter {
            field: "created_by".into(),
            operator: FilterOp::Equal,
            value: serde_json::Value::String(user_id),
        },
        Filter {
            field: "deleted_at".into(),
            operator: FilterOp::IsNull,
            value: serde_json::Value::Null,
        },
    ];
    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        PRODUCTS_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    let nav = vec![
        NavItem {
            label: "My Products".into(),
            href: "/b/products/my-products".into(),
            icon: "package",
        },
        NavItem {
            label: "My Purchases".into(),
            href: "/b/products/my-purchases".into(),
            icon: "shopping-cart",
        },
    ];

    let content = html! {
        (components::page_header("My Products", None, None))

        div #my-products-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "Status" } th { "Price" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No products yet" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
                                        td { (components::status_badge(r.str_field("status"))) }
                                        td { (r.str_field("price")) " " span .text-muted { (r.str_field("currency")) } }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                    @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                    (components::pagination(list.page as u32, total_pages, "/b/products/my-products", "#my-products-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(
        "My Products",
        &config,
        &nav,
        user.as_ref(),
        "/b/products/my-products",
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// User: My Purchases
// ---------------------------------------------------------------------------

pub async fn my_purchases(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();
    let (page, page_size, _) = msg.pagination_params(20);

    let filters = vec![Filter {
        field: "user_id".into(),
        operator: FilterOp::Equal,
        value: serde_json::Value::String(user_id),
    }];
    let sort = vec![SortField {
        field: "created_at".into(),
        desc: true,
    }];
    let result = db::paginated_list(
        ctx,
        PURCHASES_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    let nav = vec![
        NavItem {
            label: "My Products".into(),
            href: "/b/products/my-products".into(),
            icon: "package",
        },
        NavItem {
            label: "My Purchases".into(),
            href: "/b/products/my-purchases".into(),
            icon: "shopping-cart",
        },
    ];

    let content = html! {
        (components::page_header("My Purchases", None, None))

        div #my-purchases-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Status" } th { "Total" } th { "Provider" } th { "Date" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No purchases yet" } }
                                }
                                @for r in &list.records {
                                    @let total_cents = r.i64_field("total_cents");
                                    @let amount = format!("{:.2}", total_cents as f64 / 100.0);
                                    tr {
                                        td { (components::status_badge(r.str_field("status"))) }
                                        td .font-medium { (amount) " " span .text-muted { (r.str_field("currency")) } }
                                        td .text-muted .text-sm { (r.str_field("provider")) }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                    @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                    (components::pagination(list.page as u32, total_pages, "/b/products/my-purchases", "#my-purchases-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(
        "My Purchases",
        &config,
        &nav,
        user.as_ref(),
        "/b/products/my-purchases",
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Admin: Settings
// ---------------------------------------------------------------------------

const SETTINGS_KEYS: &[(&str, &str, &str, &str, bool)] = &[
    (
        "SOLOBASE_SHARED__ALLOW_USER_PRODUCTS",
        "Allow User Products",
        "Allow users to create their own products.",
        "false",
        false,
    ),
    (
        "SUPPERS_AI__PRODUCTS__STRIPE_SECRET_KEY",
        "Stripe Secret Key",
        "Stripe API secret key for payment processing.",
        "",
        true,
    ),
    (
        "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
        "Stripe Webhook Secret",
        "Stripe webhook signing secret for verifying events.",
        "",
        true,
    ),
    (
        "SUPPERS_AI__PRODUCTS__STRIPE_API_URL",
        "Stripe API URL",
        "Stripe API base URL (default is production).",
        "https://api.stripe.com",
        false,
    ),
    (
        "SOLOBASE_SHARED__FRONTEND_URL",
        "Frontend URL",
        "Frontend URL for checkout success/cancel redirects.",
        "http://localhost:5173",
        false,
    ),
    (
        "SUPPERS_AI__PRODUCTS__WEBHOOK_URL",
        "Billing Webhook URL",
        "Webhook URL for outbound billing events.",
        "",
        false,
    ),
    (
        "SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET",
        "Billing Webhook Secret",
        "Signing secret for outbound billing webhooks.",
        "",
        true,
    ),
];

pub async fn settings(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let site_config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let mut values = Vec::new();
    for &(key, label, help, default, sensitive) in SETTINGS_KEYS {
        let value = config::get_default(ctx, key, default).await;
        values.push((key, label, help, default, value, sensitive));
    }

    let content = html! {
        (components::page_header("Settings", Some("Configure payments and integrations"), None))

        form #settings-form onsubmit="return submitSettings(event)" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::settings()) " Features"
            }
            @for (key, label, help, _default, ref value, _sensitive) in values.iter().take(1) {
                div .form-group style="margin-bottom:1.25rem" {
                    label style="display:flex;align-items:center;gap:0.75rem;cursor:pointer" {
                        input type="checkbox" name=(key)
                            checked[value.as_str() == "true"]
                            style="width:1.25rem;height:1.25rem;accent-color:var(--primary)";
                        span .form-label style="margin:0" { (label) }
                    }
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                }
            }

            h3 style="font-size:1rem;font-weight:600;margin:1.5rem 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::dollar_sign()) " Stripe"
            }
            @for (key, label, help, default, ref value, sensitive) in values.iter().skip(1).take(3) {
                @if *sensitive {
                    (render_sensitive_field(key, label, help, value))
                } @else {
                    div .form-group style="margin-bottom:1.25rem" {
                        label .form-label for=(key) { (label) }
                        input .form-input #(key) name=(key) type="text" value=(value) placeholder=(default);
                        p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                    }
                }
            }

            h3 style="font-size:1rem;font-weight:600;margin:1.5rem 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::globe()) " Webhooks"
            }
            @for (key, label, help, default, ref value, sensitive) in values.iter().skip(4) {
                @if *sensitive {
                    (render_sensitive_field(key, label, help, value))
                } @else {
                    div .form-group style="margin-bottom:1.25rem" {
                        label .form-label for=(key) { (label) }
                        input .form-input #(key) name=(key) type="text" value=(value) placeholder=(default);
                        p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                    }
                }
            }

            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }

        script { (PreEscaped(r#"
function submitSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name]').forEach(function(el) {
        if (el.type === 'checkbox') { data[el.name] = el.checked ? 'true' : 'false'; }
        else { data[el.name] = el.value; }
    });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/products/admin/settings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) })
    .then(function(r) { return r.json(); })
    .then(function(d) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' } })); })
    .catch(function(err) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: 'Error: ' + err.message, type: 'error' } })); })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#)) }
    };

    products_page(
        "Settings",
        &site_config,
        "/b/products/admin/settings",
        user.as_ref(),
        content,
        msg,
    )
}

pub async fn handle_save_settings(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let body: std::collections::HashMap<String, String> = match msg.decode() {
        Ok(b) => b,
        Err(e) => {
            return json_respond(
                msg,
                &serde_json::json!({"error": format!("Invalid request: {e}")}),
            )
        }
    };
    for &(key, _, _, _, _) in SETTINGS_KEYS {
        if let Some(value) = body.get(key) {
            let _ = config::set(ctx, key, value).await;
        }
    }
    json_respond(msg, &serde_json::json!({"message": "Settings saved"}))
}

fn render_sensitive_field(key: &str, label: &str, help: &str, value: &str) -> Markup {
    let has_value = !value.is_empty();
    html! {
        div .form-group style="margin-bottom:1.25rem" {
            label .form-label for=(key) { (label) }
            div style="display:flex;align-items:center;gap:0.5rem" {
                input .form-input #(key) name=(key) type="password" value=(value)
                    placeholder=(if has_value { "******** (set)" } else { "Not configured" })
                    style="flex:1";
                button type="button" .btn .btn-ghost .btn-sm
                    onclick={"var i=document.getElementById('" (key) "');i.type=i.type==='password'?'text':'password'"}
                { (icons::eye()) }
            }
            p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
        }
    }
}
