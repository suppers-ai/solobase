mod handlers;
pub(crate) mod models;
mod pages;
mod pricing;
mod purchase;
mod stripe;
mod variables;

#[cfg(test)]
mod tests;

use super::rate_limit::{check_user_rate_limit, UserRateLimiter};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub(crate) const PRODUCTS_COLLECTION: &str = "suppers_ai__products__products";
pub(crate) const GROUPS_COLLECTION: &str = "suppers_ai__products__groups";
pub(crate) const TYPES_COLLECTION: &str = "suppers_ai__products__types";
pub(crate) const PRICING_COLLECTION: &str = "suppers_ai__products__pricing_templates";
pub(crate) const PURCHASES_COLLECTION: &str = "suppers_ai__products__purchases";
pub(crate) const LINE_ITEMS_COLLECTION: &str = "suppers_ai__products__line_items";
pub(crate) const GROUP_TEMPLATES_COLLECTION: &str = "suppers_ai__products__group_templates";
pub(crate) const PRODUCT_TEMPLATES_COLLECTION: &str = "suppers_ai__products__product_templates";
pub(crate) const SUBSCRIPTIONS: &str = "suppers_ai__products__subscriptions";
pub(crate) const VARIABLES_COLLECTION: &str = "suppers_ai__products__variables";

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
        Self {
            limiter: UserRateLimiter::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProductsBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::types::CollectionSchema;
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/products", "0.0.1", "http-handler@v1", "Products, pricing, purchases, and payment integration")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/config".into(), "wafer-run/network".into()])
            .collections(vec![
                CollectionSchema::new(PRODUCTS_COLLECTION)
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
                    .field_default("requires", "string", "")
                    .field_default("created_by", "string", "")
                    .field_optional("deleted_at", "datetime")
                    .index(&["status"])
                    .index(&["group_id"])
                    .index(&["created_by"]),
                CollectionSchema::new(GROUPS_COLLECTION)
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("template_id", "string", "")
                    .field_default("group_template_id", "string", "")
                    .field_default("user_id", "string", "")
                    .field_default("status", "string", "active")
                    .field_default("created_by", "string", ""),
                CollectionSchema::new(TYPES_COLLECTION)
                    .field("name", "string")
                    .field_default("description", "string", "")
                    .field_default("is_system", "bool", "false"),
                CollectionSchema::new(PRICING_COLLECTION)
                    .field("name", "string")
                    .field_default("price_formula", "string", "")
                    .field_default("template_data", "json", "{}"),
                CollectionSchema::new(PURCHASES_COLLECTION)
                    .field_ref("user_id", "string", &format!("{}.id", crate::blocks::auth::USERS_COLLECTION))
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
                CollectionSchema::new(LINE_ITEMS_COLLECTION)
                    .field("purchase_id", "string")
                    .field("product_id", "string")
                    .field_default("product_name", "string", "")
                    .field_default("quantity", "int", "1")
                    .field_default("unit_price", "float", "0")
                    .field_default("total_price", "float", "0")
                    .field_default("variables", "json", "{}")
                    .index(&["purchase_id"]),
                CollectionSchema::new(GROUP_TEMPLATES_COLLECTION)
                    .field("name", "string")
                    .field_default("display_name", "string", ""),
                CollectionSchema::new(PRODUCT_TEMPLATES_COLLECTION)
                    .field("name", "string")
                    .field_default("display_name", "string", ""),
                CollectionSchema::new(VARIABLES_COLLECTION)
                    .field("name", "string")
                    .field_default("var_type", "string", "number")
                    .field_optional("default_value", "string")
                    .field_default("scope", "string", "system")
                    .field_default("product_id", "string", ""),
            ])
            .category(wafer_run::BlockCategory::Feature)
            .description("Product catalog, pricing engine, and payment processing. Manages products, groups, pricing templates with formula evaluation, purchases, and Stripe integration for checkout and recurring subscriptions.")
            .endpoints(vec![
                BlockEndpoint::get("/b/products/admin/").summary("Overview").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/products/admin/manage").summary("Manage products").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/products/api/admin/products").summary("List products API").auth(AuthLevel::Admin),
                BlockEndpoint::post("/b/products/api/admin/products").summary("Create product").auth(AuthLevel::Admin),
                BlockEndpoint::get("/b/products/catalog").summary("Browse catalog"),
                BlockEndpoint::post("/b/products/checkout").summary("Stripe checkout").auth(AuthLevel::Authenticated),
                BlockEndpoint::get("/b/products/subscription").summary("Subscription status").auth(AuthLevel::Authenticated),
            ])
            .config_keys(vec![
                ConfigVar::new("SUPPERS_AI__PRODUCTS__STRIPE_SECRET_KEY", "Stripe API secret key", "")
                    .name("Stripe Secret Key")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET", "Stripe webhook signing secret", "")
                    .name("Stripe Webhook Secret")
                    .input_type(InputType::Password),
                ConfigVar::new("SUPPERS_AI__PRODUCTS__STRIPE_API_URL", "Stripe API base URL", "https://api.stripe.com")
                    .name("Stripe API URL")
                    .input_type(InputType::Url),
                ConfigVar::new("SUPPERS_AI__PRODUCTS__WEBHOOK_URL", "Webhook URL for billing events", "")
                    .name("Billing Webhook URL")
                    .input_type(InputType::Url),
                ConfigVar::new("SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET", "Webhook signing secret", "")
                    .name("Billing Webhook Secret")
                    .input_type(InputType::Password)
                    .auto_generate(),
            ])
            .admin_url("/b/products/admin/")
            .can_disable(true)
    }

    fn ui_routes(&self) -> Vec<wafer_run::UiRoute> {
        vec![
            wafer_run::UiRoute::admin("/admin/"),
            wafer_run::UiRoute::admin("/admin/manage"),
            wafer_run::UiRoute::admin("/admin/groups"),
            wafer_run::UiRoute::admin("/admin/pricing"),
            wafer_run::UiRoute::admin("/admin/purchases"),
            wafer_run::UiRoute::admin("/admin/settings"),
            wafer_run::UiRoute::authenticated("/my-products"),
            wafer_run::UiRoute::authenticated("/my-purchases"),
        ]
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path().to_string();
        let action = msg.action().to_string();

        // Settings save (POST to admin settings page)
        if action == "create" && path == "/b/products/admin/settings" {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
            return pages::handle_save_settings(ctx, msg).await;
        }

        // SSR pages (GET requests to specific page paths)
        if action == "retrieve" && path.starts_with("/b/products/") {
            let sub = path.strip_prefix("/b/products").unwrap_or("/");
            // Admin pages under /b/products/admin/...
            if sub.starts_with("/admin") {
                let is_admin = msg
                    .get_meta("auth.user_roles")
                    .split(',')
                    .any(|r| r.trim() == "admin");
                if !is_admin {
                    return crate::ui::forbidden_response(msg);
                }
                let admin_sub = sub.strip_prefix("/admin").unwrap_or("/");
                return match admin_sub {
                    "" | "/" => pages::overview(ctx, msg).await,
                    "/manage" => pages::manage_products(ctx, msg).await,
                    "/groups" => pages::groups(ctx, msg).await,
                    "/pricing" => pages::pricing(ctx, msg).await,
                    "/purchases" => pages::purchases(ctx, msg).await,
                    "/settings" => pages::settings(ctx, msg).await,
                    _ => err_not_found(msg, "not found"),
                };
            }
            // User-facing pages (require auth but not admin)
            match sub {
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
        if let Some(r) = check_user_rate_limit(&self.limiter, ctx, msg).await {
            return r;
        }

        // Admin API at /b/products/api/admin/... → normalize to /admin/b/products/...
        if let Some(rest) = path.strip_prefix("/b/products/api/admin") {
            let is_admin = msg
                .get_meta("auth.user_roles")
                .split(',')
                .any(|r| r.trim() == "admin");
            if !is_admin {
                return crate::ui::forbidden_response(msg);
            }
            msg.set_meta("req.resource", format!("/admin/b/products{rest}"));
            return handlers::handle_admin(ctx, msg).await;
        }

        // User API at /b/products/api/... → normalize to /b/products/...
        if let Some(rest) = path.strip_prefix("/b/products/api") {
            msg.set_meta("req.resource", format!("/b/products{rest}"));
            return handlers::handle_user(ctx, msg).await;
        }

        err_not_found(msg, "not found")
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if event.event_type == LifecycleType::Init {
            // Seed default templates if they don't exist — these are required by FK constraints
            // on the groups and products tables.
            use db::ListOptions;
            use wafer_core::clients::database as db;

            let check_opts = ListOptions {
                limit: 1,
                ..Default::default()
            };

            // Default group template
            match db::list(ctx, GROUP_TEMPLATES_COLLECTION, &check_opts).await {
                Ok(list) if list.records.is_empty() => {
                    let mut data = std::collections::HashMap::new();
                    data.insert(
                        "name".to_string(),
                        serde_json::Value::String("default".to_string()),
                    );
                    data.insert(
                        "display_name".to_string(),
                        serde_json::Value::String("Default".to_string()),
                    );
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
                    data.insert(
                        "name".to_string(),
                        serde_json::Value::String("default".to_string()),
                    );
                    data.insert(
                        "display_name".to_string(),
                        serde_json::Value::String("Default".to_string()),
                    );
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
