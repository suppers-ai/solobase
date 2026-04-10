mod handlers;
mod pages;

use super::rate_limit::{check_user_rate_limit, UserRateLimiter};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub(crate) const PROJECTS_COLLECTION: &str = "suppers_ai__projects__deployments";
pub(crate) const PROJECT_USAGE: &str = "suppers_ai__projects__usage";

pub struct ProjectsBlock {
    limiter: UserRateLimiter,
}

impl Default for ProjectsBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectsBlock {
    pub fn new() -> Self {
        Self {
            limiter: UserRateLimiter::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProjectsBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/projects", "0.0.1", "http-handler@v1", "Project management for users and admins")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/config".into(), "wafer-run/network".into()])
            .collections(vec![
                CollectionSchema::new(PROJECTS_COLLECTION)
                    .field_ref("user_id", "string", &format!("{}.id", crate::blocks::auth::USERS_COLLECTION))
                    .field("name", "string")
                    .field_default("slug", "string", "")
                    .field_default("status", "string", "pending")
                    .field_default("config", "json", "{}")
                    .field_default("plan_id", "string", "")
                    .field_default("purchase_id", "string", "")
                    .field_default("tenant_id", "string", "")
                    .field_default("subdomain", "string", "")
                    .field_optional("provision_error", "string")
                    .field_optional("deprovision_error", "string")
                    .field_optional("grace_period_end", "datetime")
                    .field_optional("deleted_at", "datetime")
                    .index(&["user_id"])
                    .index(&["status"]),
            ])
            .grants(vec![
                wafer_run::ResourceGrant::read("suppers-ai/products", PROJECTS_COLLECTION),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Project and deployment management for multi-tenant hosting. Users can create projects with unique subdomains, activate/deactivate deployments, and manage lifecycle. Integrates with the control plane for provisioning on Cloudflare Workers for Platforms.")
            .endpoints(vec![
                BlockEndpoint::get("/b/projects/admin/", "Deployments overview", AuthLevel::Admin),
                BlockEndpoint::get("/b/projects/api/admin", "List all projects", AuthLevel::Admin),
                BlockEndpoint::get("/b/projects/api/admin/stats", "Deployment statistics", AuthLevel::Admin),
                BlockEndpoint::get("/b/projects", "List user's projects", AuthLevel::Authenticated),
                BlockEndpoint::post("/b/projects", "Create project", AuthLevel::Authenticated),
                BlockEndpoint::patch("/b/projects/{id}", "Update/activate/deactivate project", AuthLevel::Authenticated),
                BlockEndpoint::delete("/b/projects/{id}", "Delete project", AuthLevel::Authenticated),
            ])
            .config_keys(vec![
                ConfigVar::new("SUPPERS_AI__PROJECTS__CONTROL_PLANE_URL", "Control plane API URL for provisioning", "")
                    .name("Control Plane URL")
                    .input_type(InputType::Url),
                ConfigVar::new("SUPPERS_AI__PROJECTS__CONTROL_PLANE_SECRET", "Secret for control plane API authentication", "")
                    .name("Control Plane Secret")
                    .input_type(InputType::Password),
            ])
            .admin_url("/b/projects/admin/")
            .can_disable(true)
            .default_enabled(false)
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // Per-user rate limiting for authenticated endpoints
        if let Some(r) = check_user_rate_limit(&self.limiter, ctx, msg).await {
            return r;
        }

        // SSR pages — admin UI at /b/projects/admin/
        if path.starts_with("/b/projects/admin") {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
            let action = msg.action().to_string();
            let sub = path.strip_prefix("/b/projects/admin").unwrap_or("/");
            return match (action.as_str(), sub) {
                ("retrieve", "" | "/") => pages::admin_deployments(ctx, msg).await,
                ("retrieve", "/settings") => pages::settings(ctx, msg).await,
                ("create", "/settings") => pages::handle_save_settings(ctx, msg).await,
                _ => err_not_found(msg, "not found"),
            };
        }

        // Admin API at /b/projects/api/admin/... → normalize to /admin/b/projects/...
        if let Some(rest) = path.strip_prefix("/b/projects/api/admin") {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
            msg.set_meta("req.resource", format!("/admin/b/projects{rest}"));
            return handlers::handle_admin(ctx, msg).await;
        }

        // User API at /b/projects/api/... → normalize to /b/projects/...
        if let Some(rest) = path.strip_prefix("/b/projects/api") {
            msg.set_meta("req.resource", format!("/b/projects{rest}"));
            return handlers::handle_user(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/admin/"),
            wafer_run::UiRoute::admin("/admin/settings"),
        ]
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
