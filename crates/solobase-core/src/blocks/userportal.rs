use maud::html;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::{config, database as db};
use wafer_core::clients::database::{Filter, FilterOp, ListOptions, SortField};
use super::helpers::RecordExt;
use crate::ui::{self, components, icons, NavItem, SiteConfig, UserInfo};

pub struct UserPortalBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for UserPortalBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/userportal", "0.0.1", "http-handler@v1", "User portal — dashboard, projects, and API keys")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/config".into()])
            .category(wafer_run::BlockCategory::Feature)
            .description("User-facing dashboard for managing projects and API keys. Shows plan overview, project management with subdomain provisioning, and API key generation. Also serves the portal configuration endpoint used by the frontend.")
            .endpoints(vec![
                BlockEndpoint::get("/b/userportal/", "Dashboard", AuthLevel::Authenticated),
                BlockEndpoint::get("/b/userportal/projects", "Manage projects", AuthLevel::Authenticated),
                BlockEndpoint::get("/b/userportal/api-keys", "Manage API keys", AuthLevel::Authenticated),
                BlockEndpoint::get("/b/userportal/config", "Portal configuration", AuthLevel::Public),
            ])
            .config_keys(vec![
                BlockConfigKey::new("APP_NAME", "Application display name", "Solobase"),
                BlockConfigKey::new("LOGO_URL", "Logo image URL", ""),
                BlockConfigKey::new("PRIMARY_COLOR", "Primary brand color", "#6366f1"),
                BlockConfigKey::new("ALLOW_SIGNUP", "Allow new user registration", "true"),
                BlockConfigKey::new("ENABLE_OAUTH", "Enable OAuth login", "false"),
            ])
            .can_disable(true)
            .default_enabled(false)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::authenticated("/"),
            wafer_run::UiRoute::authenticated("/projects"),
            wafer_run::UiRoute::authenticated("/api-keys"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // SSR pages
        if path.starts_with("/b/userportal") {
            let sub = path.strip_prefix("/b/userportal").unwrap_or("/");
            return match sub {
                "" | "/" => portal_dashboard(ctx, msg).await,
                "/projects" => portal_projects(ctx, msg).await,
                "/api-keys" => portal_api_keys(ctx, msg).await,
                // Keep config endpoint accessible
                "/config" => self.handle_config(ctx, msg).await,
                _ => err_not_found(msg, "not found"),
            };
        }

        // Legacy config endpoint
        self.handle_config(ctx, msg).await
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

impl UserPortalBlock {
    async fn handle_config(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // Read block enabled state from block_settings table
        let block_rows = db::query_raw(ctx,
            "SELECT block_name, enabled FROM block_settings",
            &[],
        ).await.unwrap_or_default();

        let is_enabled = |name: &str| -> bool {
            block_rows.iter()
                .find(|r| r.data.get("block_name").and_then(|v| v.as_str()) == Some(name))
                .and_then(|r| r.data.get("enabled").and_then(|v| v.as_i64()))
                .map(|v| v != 0)
                .unwrap_or(true)
        };

        let config_val = serde_json::json!({
            "logo_url": config::get_default(ctx, "LOGO_URL", "").await,
            "app_name": config::get_default(ctx, "APP_NAME", "Solobase").await,
            "primary_color": config::get_default(ctx, "PRIMARY_COLOR", "#6366f1").await,
            "enable_oauth": config::get_default(ctx, "ENABLE_OAUTH", "false").await,
            "allow_signup": config::get_default(ctx, "ALLOW_SIGNUP", "true").await,
            "show_powered_by": true,
            "features": {
                "files": is_enabled("suppers-ai/files"),
                "products": is_enabled("suppers-ai/products"),
                "user_products": config::get_default(ctx, "FEATURE_USER_PRODUCTS", "false").await,
                "legal_pages": is_enabled("suppers-ai/legalpages"),
                "projects": is_enabled("suppers-ai/projects"),
                "userportal": is_enabled("suppers-ai/userportal"),
            }
        });
        json_respond(msg, &config_val)
    }
}

// ---------------------------------------------------------------------------
// SSR Pages
// ---------------------------------------------------------------------------

fn portal_nav() -> Vec<NavItem> {
    vec![
        NavItem { label: "Overview".into(), href: "/b/userportal/".into(), icon: "layout-dashboard" },
        NavItem { label: "Projects".into(), href: "/b/userportal/projects".into(), icon: "server" },
        NavItem { label: "API Keys".into(), href: "/b/userportal/api-keys".into(), icon: "key" },
    ]
}

fn portal_page(title: &str, config: &SiteConfig, path: &str, user: Option<&UserInfo>, content: maud::Markup, msg: &mut Message) -> Result_ {
    let is_fragment = ui::is_htmx(msg);
    let markup = ui::layout::block_shell(title, config, &portal_nav(), user, path, content, is_fragment);
    ui::html_response(msg, markup)
}

async fn portal_dashboard(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();
    let app_name = config::get_default(ctx, "APP_NAME", "Solobase").await;

    // Count user's projects
    let project_opts = ListOptions {
        filters: vec![
            Filter { field: "user_id".into(), operator: FilterOp::Equal, value: serde_json::json!(user_id) },
            Filter { field: "deleted_at".into(), operator: FilterOp::IsNull, value: serde_json::Value::Null },
        ],
        limit: 1, ..Default::default()
    };
    let project_count = db::list(ctx, "block_deployments", &project_opts).await.map(|r| r.total_count).unwrap_or(0);

    // Count user's API keys
    let key_opts = ListOptions {
        filters: vec![
            Filter { field: "user_id".into(), operator: FilterOp::Equal, value: serde_json::json!(user_id) },
        ],
        limit: 1, ..Default::default()
    };
    let key_count = db::list(ctx, "api_keys", &key_opts).await.map(|r| r.total_count).unwrap_or(0);

    let content = html! {
        (components::page_header("Dashboard", Some(&format!("Welcome to {app_name}")), None))

        div .stats-grid {
            (components::stat_card("Projects", &project_count.to_string(), icons::server()))
            (components::stat_card("API Keys", &key_count.to_string(), icons::key()))
        }

        div .flex .gap-4 .mt-4 {
            a .btn .btn-primary href="/b/userportal/projects" { "Manage Projects" }
            a .btn .btn-secondary href="/b/userportal/api-keys" { "API Keys" }
        }
    };

    portal_page("Dashboard", &config, "/b/userportal/", user.as_ref(), content, msg)
}

async fn portal_projects(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();
    let (page, page_size, _) = msg.pagination_params(20);

    let filters = vec![
        Filter { field: "user_id".into(), operator: FilterOp::Equal, value: serde_json::json!(user_id) },
        Filter { field: "deleted_at".into(), operator: FilterOp::IsNull, value: serde_json::Value::Null },
    ];
    let sort = vec![SortField { field: "created_at".into(), desc: true }];
    let result = db::paginated_list(ctx, "block_deployments", page as i64, page_size as i64, filters, sort).await;

    let content = html! {
        (components::page_header("My Projects", Some("Manage your deployments"), None))

        div #projects-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Name" } th { "Status" } th { "Subdomain" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No projects yet" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td .font-medium { (r.str_field("name")) }
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
                    (components::pagination(list.page as u32, total_pages, "/b/userportal/projects", "#projects-content"))
                }
                Err(e) => { div .login-error { "Error: " (e.message) } }
            }
        }
    };

    portal_page("My Projects", &config, "/b/userportal/projects", user.as_ref(), content, msg)
}

async fn portal_api_keys(ctx: &dyn Context, msg: &mut Message) -> Result_ {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let user_id = msg.user_id().to_string();

    let opts = ListOptions {
        filters: vec![
            Filter { field: "user_id".into(), operator: FilterOp::Equal, value: serde_json::json!(user_id) },
        ],
        sort: vec![SortField { field: "created_at".into(), desc: true }],
        limit: 100, ..Default::default()
    };
    let result = db::list(ctx, "api_keys", &opts).await;

    let content = html! {
        (components::page_header("API Keys", Some("Manage your API access keys"), None))

        div #api-keys-content {
            @match &result {
                Ok(list) => {
                    div .table-container {
                        table .table {
                            thead { tr { th { "Prefix" } th { "Name" } th { "Status" } th { "Created" } } }
                            tbody {
                                @if list.records.is_empty() {
                                    tr { td colspan="4" .text-center .text-muted style="padding:2rem;" { "No API keys" } }
                                }
                                @for r in &list.records {
                                    tr {
                                        td { code { (r.str_field("key_prefix")) "..." } }
                                        td { (r.str_field("name")) }
                                        td {
                                            @if r.str_field("revoked_at").is_empty() {
                                                (components::status_badge("active"))
                                            } @else {
                                                (components::status_badge("disabled"))
                                            }
                                        }
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

    portal_page("API Keys", &config, "/b/userportal/api-keys", user.as_ref(), content, msg)
}
