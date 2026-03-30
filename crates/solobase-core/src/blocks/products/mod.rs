mod addons;
mod handlers;
mod pages;
mod pricing;
mod purchase;
mod stripe;
mod variables;
pub(crate) mod models;

#[cfg(test)]
mod tests;

use wafer_run::block::{Block, BlockInfo, AdminUIInfo};
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
        use wafer_run::types::CollectionSchema;

        BlockInfo {
            name: "suppers-ai/products".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Products, pricing, purchases, and payment integration".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: Some(AdminUIInfo {
                label: "Products".to_string(),
                description: "Manage products, pricing, and purchases".to_string(),
                url: "/b/products/".to_string(),
            }),
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: vec![
                CollectionSchema::new("block_products_products")
                    .field("name", "string")
                    .field_default("description", "text", "")
                    .field_default("slug", "string", "")
                    .field_default("price", "float", "0")
                    .field_default("base_price", "float", "0")
                    .field_default("currency", "string", "USD")
                    .field_default("status", "string", "draft")
                    .field_default("category", "string", "")
                    .field_default("tags", "json", "[]")
                    .field_default("metadata", "json", "{}")
                    .field_default("image_url", "string", "")
                    .field_default("stock", "int", "0")
                    .field_default("group_id", "string", "")
                    .field_default("type_id", "string", "")
                    .field_default("group_template_id", "string", "")
                    .field_default("product_template_id", "string", "")
                    .field_default("pricing_template_id", "string", "")
                    .field_default("created_by", "string", "")
                    .field_optional("deleted_at", "datetime")
                    .index(&["status"])
                    .index(&["group_id"])
                    .index(&["created_by"]),
                CollectionSchema::new("block_products_groups")
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("template_id", "string", "")
                    .field_default("group_template_id", "string", "")
                    .field_default("user_id", "string", "")
                    .field_default("status", "string", "active")
                    .field_default("created_by", "string", ""),
                CollectionSchema::new("block_products_types")
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("is_system", "bool", "false"),
                CollectionSchema::new("block_products_pricing_templates")
                    .field("name", "string")
                    .field_default("price_formula", "string", "")
                    .field_default("template_data", "json", "{}"),
                CollectionSchema::new("block_products_purchases")
                    .field_ref("user_id", "string", "auth_users.id")
                    .field_default("status", "string", "pending")
                    .field_default("total_cents", "int", "0")
                    .field_default("amount_cents", "int", "0")
                    .field_default("currency", "string", "USD")
                    .field_default("provider", "string", "manual")
                    .field_default("metadata", "json", "{}")
                    .field_default("stripe_payment_intent_id", "string", "")
                    .field_optional("refunded_at", "datetime")
                    .field_default("refunded_by", "string", "")
                    .field_default("refund_reason", "string", "")
                    .field_optional("payment_at", "datetime")
                    .index(&["user_id"])
                    .index(&["status"]),
                CollectionSchema::new("block_products_line_items")
                    .field("purchase_id", "string")
                    .field("product_id", "string")
                    .field_default("product_name", "string", "")
                    .field_default("quantity", "int", "1")
                    .field_default("unit_price", "float", "0")
                    .field_default("total_price", "float", "0")
                    .field_default("variables", "json", "{}")
                    .index(&["purchase_id"]),
                CollectionSchema::new("block_products_group_templates")
                    .field("name", "string")
                    .field_default("display_name", "string", ""),
                CollectionSchema::new("block_products_product_templates")
                    .field("name", "string")
                    .field_default("display_name", "string", ""),
                CollectionSchema::new("block_products_variables")
                    .field("name", "string")
                    .field_default("var_type", "string", "number")
                    .field_optional("default_value", "string")
                    .field_default("scope", "string", "system")
                    .field_default("product_id", "string", ""),
            ],
            config_schema: None,
        }
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/"),
            wafer_run::UiRoute::admin("/manage"),
            wafer_run::UiRoute::admin("/groups"),
            wafer_run::UiRoute::admin("/pricing"),
            wafer_run::UiRoute::admin("/purchases"),
            wafer_run::UiRoute::authenticated("/my-products"),
            wafer_run::UiRoute::authenticated("/my-purchases"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();
        let action = msg.action().to_string();

        // SSR pages (GET requests to specific page paths)
        if action == "retrieve" && path.starts_with("/b/products/") {
            let sub = path.strip_prefix("/b/products").unwrap_or("/");
            match sub {
                "/" => return pages::overview(ctx, msg).await,
                "/manage" => return pages::manage_products(ctx, msg).await,
                "/groups" => return pages::groups(ctx, msg).await,
                "/pricing" => return pages::pricing(ctx, msg).await,
                "/purchases" => return pages::purchases(ctx, msg).await,
                "/my-products" => return pages::my_products(ctx, msg).await,
                "/my-purchases" => return pages::my_purchases(ctx, msg).await,
                _ => {} // fall through to API handlers
            }
        }

        // Webhook (no auth, no user rate limit)
        if path == "/b/products/webhooks" || path.starts_with("/b/products/webhooks/") {
            return stripe::handle_webhook(ctx, msg).await;
        }

        // Per-user rate limiting for authenticated endpoints
        let user_id = msg.user_id().to_string();
        if !user_id.is_empty() {
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
