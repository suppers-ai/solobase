//! SSR pages for the projects block.

use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};
use maud::{html, Markup, PreEscaped};
use wafer_core::clients::database::{Filter, FilterOp, SortField};
use wafer_core::clients::{config, database as db};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

use super::PROJECTS_COLLECTION;

fn projects_nav() -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Deployments".into(),
            href: "/b/projects/admin/".into(),
            icon: "server",
        },
        NavItem {
            label: "Settings".into(),
            href: "/b/projects/admin/settings".into(),
            icon: "settings",
        },
    ]
}

fn projects_page(
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
        &projects_nav(),
        user,
        path,
        content,
        is_fragment,
    );
    ui::html_response(msg, markup)
}

// ---------------------------------------------------------------------------
// Admin: Deployments list + stats
// ---------------------------------------------------------------------------

pub async fn admin_deployments(ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
        PROJECTS_COLLECTION,
        page as i64,
        page_size as i64,
        filters,
        sort,
    )
    .await;

    // Stats counts
    let one = db::ListOptions {
        limit: 1,
        ..Default::default()
    };
    let total = db::list(ctx, PROJECTS_COLLECTION, &one)
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);

    let count_by_status = |status: &str| -> db::ListOptions {
        db::ListOptions {
            filters: vec![Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!(status),
            }],
            limit: 1,
            ..Default::default()
        }
    };
    let active = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("active"))
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let pending = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("pending"))
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);
    let stopped = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("stopped"))
        .await
        .map(|r| r.total_count)
        .unwrap_or(0);

    let content = html! {
        (components::page_header("Deployments", Some("Manage project deployments"), None))

        div .stats-grid {
            (components::stat_card("Total", &total.to_string(), icons::server()))
            (components::stat_card("Active", &active.to_string(), icons::globe()))
            (components::stat_card("Pending", &pending.to_string(), icons::refresh_cw()))
            (components::stat_card("Stopped", &stopped.to_string(), icons::x()))
        }

        // Status filter
        div .filter-bar {
            @for s in &["all", "pending", "active", "inactive", "stopped", "deleted"] {
                a .btn .(if (status_filter.is_empty() && *s == "all") || status_filter == *s { "btn-primary" } else { "btn-secondary" })
                    .btn-sm
                    href={"/b/projects/?status=" (*s)}
                    hx-get={"/b/projects/?status=" (*s)}
                    hx-target="#content"
                    hx-push-url="true"
                { (*s) }
            }
        }

        div #deployments-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "User" } th { "Status" } th { "Subdomain" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="5" .text-center .text-muted style="padding:2rem;" { "No deployments" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
                                        td .text-muted .text-sm { (r.str_field("user_id").get(..8).unwrap_or("—")) }
                                        td { (components::status_badge(r.str_field("status"))) }
                                        td .text-sm {
                                            @let sub = r.str_field("subdomain");
                                            @if sub.is_empty() { "—" } @else { (sub) }
                                        }
                                        td .text-muted .text-sm { (r.str_field("created_at").get(..10).unwrap_or("")) }
                                    }
                                }
                            }
                        }
                    }
                    @let total_pages = ((list.total_count as f64) / (list.page_size.max(1) as f64)).ceil() as u32;
                    (components::pagination(list.page as u32, total_pages, "/b/projects/admin/", "#deployments-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    projects_page(
        "Deployments",
        &config,
        "/b/projects/admin/",
        user.as_ref(),
        content,
        msg,
    )
}

// ---------------------------------------------------------------------------
// Admin: Settings
// ---------------------------------------------------------------------------

const SETTINGS_KEYS: &[(&str, &str, &str, &str, bool)] = &[
    (
        "SUPPERS_AI__PROJECTS__CONTROL_PLANE_URL",
        "Control Plane URL",
        "API URL for the deployment control plane (e.g. https://control.solobase.dev).",
        "",
        false,
    ),
    (
        "SUPPERS_AI__PROJECTS__CONTROL_PLANE_SECRET",
        "Control Plane Secret",
        "Authentication secret for control plane API calls.",
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
        (components::page_header("Settings", Some("Configure deployment infrastructure"), None))

        form #settings-form onsubmit="return submitProjectSettings(event)" {
            h3 style="font-size:1rem;font-weight:600;margin:0 0 1rem;padding-bottom:0.5rem;border-bottom:1px solid var(--border-color)" {
                (icons::server()) " Control Plane"
            }

            @for (key, label, help, default, ref value, sensitive) in &values {
                div .form-group style="margin-bottom:1.25rem" {
                    label .form-label for=(key) { (label) }
                    @if *sensitive {
                        div style="display:flex;align-items:center;gap:0.5rem" {
                            input .form-input #(key) name=(key) type="password" value=(value)
                                placeholder=(if value.is_empty() { "Not configured" } else { "******** (set)" })
                                style="flex:1";
                            button type="button" .btn .btn-ghost .btn-sm
                                onclick={"var i=document.getElementById('" (key) "');i.type=i.type==='password'?'text':'password'"}
                            { (icons::eye()) }
                        }
                    } @else {
                        input .form-input #(key) name=(key) type="text" value=(value) placeholder=(default);
                    }
                    p .text-muted style="font-size:0.8rem;margin-top:0.25rem" { (help) }
                }
            }

            button .btn .btn-primary type="submit" style="margin-top:1rem" { "Save Settings" }
        }

        script { (PreEscaped(r#"
function submitProjectSettings(e) {
    e.preventDefault();
    var form = document.getElementById('settings-form');
    var data = {};
    form.querySelectorAll('input[name]').forEach(function(el) { data[el.name] = el.value; });
    var btn = form.querySelector('button[type="submit"]');
    btn.disabled = true; btn.textContent = 'Saving...';
    fetch('/b/projects/admin/settings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) })
    .then(function(r) { return r.json(); })
    .then(function(d) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: d.message || 'Saved', type: d.error ? 'error' : 'success' } })); })
    .catch(function(err) { document.body.dispatchEvent(new CustomEvent('showToast', { detail: { message: 'Error: ' + err.message, type: 'error' } })); })
    .finally(function() { btn.disabled = false; btn.textContent = 'Save Settings'; });
    return false;
}
"#)) }
    };

    projects_page(
        "Settings",
        &site_config,
        "/b/projects/admin/settings",
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
