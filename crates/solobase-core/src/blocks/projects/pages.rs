//! SSR pages for the projects block.

use maud::{html, Markup};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_core::clients::database as db;
use wafer_core::clients::database::{Filter, FilterOp, SortField};
use crate::blocks::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};

use super::PROJECTS_COLLECTION;

fn projects_nav() -> Vec<NavItem> {
    vec![
        NavItem { label: "Deployments".into(), href: "/b/projects/".into(), icon: "server" },
    ]
}

fn projects_page(title: &str, config: &SiteConfig, path: &str, user: Option<&UserInfo>, content: Markup, msg: &mut Message) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(title, config, &projects_nav(), user, path, content, is_fragment);
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
            field: "status".into(), operator: FilterOp::Equal,
            value: serde_json::Value::String(status_filter.clone()),
        });
    }

    let sort = vec![SortField { field: "created_at".into(), desc: true }];
    let result = db::paginated_list(ctx, PROJECTS_COLLECTION, page as i64, page_size as i64, filters, sort).await;

    // Stats counts
    let one = db::ListOptions { limit: 1, ..Default::default() };
    let total = db::list(ctx, PROJECTS_COLLECTION, &one).await.map(|r| r.total_count).unwrap_or(0);

    let count_by_status = |status: &str| -> db::ListOptions {
        db::ListOptions {
            filters: vec![Filter { field: "status".into(), operator: FilterOp::Equal, value: serde_json::json!(status) }],
            limit: 1, ..Default::default()
        }
    };
    let active = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("active")).await.map(|r| r.total_count).unwrap_or(0);
    let pending = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("pending")).await.map(|r| r.total_count).unwrap_or(0);
    let stopped = db::list(ctx, PROJECTS_COLLECTION, &count_by_status("stopped")).await.map(|r| r.total_count).unwrap_or(0);

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
                    (components::pagination(list.page as u32, total_pages, "/b/projects/", "#deployments-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    projects_page("Deployments", &config, "/b/projects/", user.as_ref(), content, msg)
}
