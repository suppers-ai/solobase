mod handlers;
mod pricing;
mod purchase;
mod stripe;
mod variables;
pub(crate) mod models;

#[cfg(test)]
mod tests;

use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use super::rate_limit::{UserRateLimiter, RateLimit, check_rate_limit};

pub(crate) const PRODUCTS_COLLECTION: &str = "block_products_products";
pub(crate) const GROUPS_COLLECTION: &str = "block_products_groups";
pub(crate) const TYPES_COLLECTION: &str = "block_products_types";
pub(crate) const PRICING_COLLECTION: &str = "block_products_pricing_templates";
pub(crate) const PURCHASES_COLLECTION: &str = "block_products_purchases";
pub(crate) const LINE_ITEMS_COLLECTION: &str = "block_products_line_items";
pub(crate) const GROUP_TEMPLATES_COLLECTION: &str = "block_products_group_templates";
pub(crate) const PRODUCT_TEMPLATES_COLLECTION: &str = "block_products_product_templates";

pub struct ProductsBlock {
    limiter: UserRateLimiter,
}

impl Default for ProductsBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductsBlock {
    pub fn new() -> Self {
        Self { limiter: UserRateLimiter::new() }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProductsBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/products".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Products, pricing, purchases, and payment integration".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();

        // Webhook (no auth, no user rate limit)
        if path == "/b/products/webhooks" || path.starts_with("/b/products/webhooks/") {
            return stripe::handle_webhook(ctx, msg).await;
        }

        // Per-user rate limiting for authenticated endpoints
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
            let action = msg.action().to_string();
            let (default, category) = if action == "retrieve" {
                (RateLimit::API_READ, "api_read")
            } else {
                (RateLimit::API_WRITE, "api_write")
            };
            if let Some(r) = check_rate_limit(&self.limiter, ctx, msg, &user_id, category, default).await {
                return r;
            }
        }

        // Admin routes
        if path.starts_with("/admin/b/products") {
            return handlers::handle_admin(ctx, msg).await;
        }

        // User-facing routes
        if path.starts_with("/b/products") {
            return handlers::handle_user(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(&self, ctx: &dyn Context, event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        if event.event_type == LifecycleType::Init {
            // Seed default templates if they don't exist — these are required by FK constraints
            // on the groups and products tables.
            use wafer_core::clients::database as db;
            use db::ListOptions;

            let check_opts = ListOptions { limit: 1, ..Default::default() };

            // Default group template
            match db::list(ctx, GROUP_TEMPLATES_COLLECTION, &check_opts).await {
                Ok(list) if list.records.is_empty() => {
                    let mut data = std::collections::HashMap::new();
                    data.insert("name".to_string(), serde_json::Value::String("default".to_string()));
                    data.insert("display_name".to_string(), serde_json::Value::String("Default".to_string()));
                    match db::create(ctx, GROUP_TEMPLATES_COLLECTION, data).await {
                        Ok(_) => tracing::info!("seeded default group template"),
                        Err(e) => tracing::warn!("failed to seed group template: {e}"),
                    }
                }
                Ok(_) => {} // already has records
                Err(e) => tracing::warn!("failed to list group templates: {e}"),
            }

            // Default product template
            match db::list(ctx, PRODUCT_TEMPLATES_COLLECTION, &check_opts).await {
                Ok(list) if list.records.is_empty() => {
                    let mut data = std::collections::HashMap::new();
                    data.insert("name".to_string(), serde_json::Value::String("default".to_string()));
                    data.insert("display_name".to_string(), serde_json::Value::String("Default".to_string()));
                    match db::create(ctx, PRODUCT_TEMPLATES_COLLECTION, data).await {
                        Ok(_) => tracing::info!("seeded default product template"),
                        Err(e) => tracing::warn!("failed to seed product template: {e}"),
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("failed to list product templates: {e}"),
            }
        }
        Ok(())
    }
}
