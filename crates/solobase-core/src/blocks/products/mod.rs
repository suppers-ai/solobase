mod handlers;
pub(crate) mod migrations;
mod pages;
mod pricing;
mod purchase;
mod repo;
mod stripe;
mod variables;

#[cfg(test)]
mod tests;

pub(crate) use handlers::{
    GROUPS_TABLE, GROUP_TEMPLATES_TABLE, PRODUCTS_TABLE, PRODUCT_TEMPLATES_TABLE, TYPES_TABLE,
};
pub(crate) use pricing::TABLE as PRICING_TABLE;
pub(crate) use repo::purchases::{LINE_ITEMS_TABLE, PURCHASES_TABLE};
pub(crate) use variables::TABLE as VARIABLES_TABLE;
use wafer_run::{
    context::Context, Block, BlockEndpoint, BlockInfo, ConfigVar, InputStream, InputType,
    InstanceMode, LifecycleEvent, LifecycleType, Message, OutputStream, WaferError,
};

use super::rate_limit::{check_user_rate_limit, RateLimitOutcome, UserRateLimiter};
use crate::blocks::helpers::{self, err_not_found};

/// The products block's own declared config vars. Single source of truth for
/// both `BlockInfo::config_keys` and the admin settings page (which renders
/// these via `ui::settings_form` rather than a parallel tuple table).
pub(crate) fn config_vars() -> Vec<ConfigVar> {
    vec![
        ConfigVar::new(
            "SUPPERS_AI__PRODUCTS__STRIPE_SECRET_KEY",
            "Stripe API secret key",
            "",
        )
        .name("Stripe Secret Key")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__PRODUCTS__STRIPE_WEBHOOK_SECRET",
            "Stripe webhook signing secret",
            "",
        )
        .name("Stripe Webhook Secret")
        .input_type(InputType::Password)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__PRODUCTS__STRIPE_API_URL",
            "Stripe API base URL",
            "https://api.stripe.com",
        )
        .name("Stripe API URL")
        .input_type(InputType::Url),
        ConfigVar::new(
            "SUPPERS_AI__PRODUCTS__WEBHOOK_URL",
            "Webhook URL for billing events",
            "",
        )
        .name("Billing Webhook URL")
        .input_type(InputType::Url)
        .optional(),
        ConfigVar::new(
            "SUPPERS_AI__PRODUCTS__WEBHOOK_SECRET",
            "Webhook signing secret",
            "",
        )
        .name("Billing Webhook Secret")
        .input_type(InputType::Password)
        .auto_generate(),
    ]
}

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
        use wafer_run::{AuthLevel, CollectionSchema};

        BlockInfo::new("suppers-ai/products", "0.0.1", "http-handler@v1", "Products, pricing, purchases, and payment integration")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into(), "wafer-run/config".into(), "wafer-run/network".into()])
            // Advisory table list — admin "Database tables" discovery + the
            // WRAP grant-UI read only `CollectionSchema::name`. The schema
            // itself (columns, indexes, FKs) lives solely in the block's
            // hand-authored `migrations/*.sqlite.sql` files (the single
            // source for both runtime `migrations::apply()` and the
            // Cloudflare D1 build).
            .collections(vec![
                CollectionSchema::new(PRODUCTS_TABLE),
                CollectionSchema::new(GROUPS_TABLE),
                CollectionSchema::new(TYPES_TABLE),
                CollectionSchema::new(PRICING_TABLE),
                CollectionSchema::new(PURCHASES_TABLE),
                CollectionSchema::new(LINE_ITEMS_TABLE),
                CollectionSchema::new(GROUP_TEMPLATES_TABLE),
                CollectionSchema::new(PRODUCT_TEMPLATES_TABLE),
                CollectionSchema::new(VARIABLES_TABLE),
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
            .config_keys(config_vars())
            .admin_url("/b/products/admin/")
            .can_disable(true)
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        mut msg: Message,
        input: InputStream,
    ) -> OutputStream {
        let path = msg.path().to_string();
        let action = msg.action().to_string();

        // Settings save (POST to admin settings page)
        if action == "create" && path == "/b/products/admin/settings" {
            let is_admin = helpers::is_admin(&msg);
            if !is_admin {
                return crate::ui::forbidden_response(&msg);
            }
            return pages::handle_save_settings(ctx, input).await;
        }

        // SSR pages (GET requests to specific page paths)
        if action == "retrieve" && path.starts_with("/b/products/") {
            let sub = path.strip_prefix("/b/products").unwrap_or("/");
            // Admin pages under /b/products/admin/...
            if sub.starts_with("/admin") {
                let is_admin = helpers::is_admin(&msg);
                if !is_admin {
                    return crate::ui::forbidden_response(&msg);
                }
                let admin_sub = sub.strip_prefix("/admin").unwrap_or("/");
                return match admin_sub {
                    "" | "/" => pages::overview(ctx, &msg).await,
                    "/manage" => pages::manage_products(ctx, &msg).await,
                    "/groups" => pages::groups(ctx, &msg).await,
                    "/pricing" => pages::pricing(ctx, &msg).await,
                    "/purchases" => pages::purchases(ctx, &msg).await,
                    "/settings" => pages::settings(ctx, &msg).await,
                    _ => err_not_found("not found"),
                };
            }
            // User-facing pages (require auth but not admin)
            match sub {
                "/my-products" => return pages::my_products(ctx, &msg).await,
                "/my-purchases" => return pages::my_purchases(ctx, &msg).await,
                _ => {} // fall through to API handlers
            }
        }

        // Webhook (no auth, no user rate limit)
        if path == "/b/products/webhooks" || path.starts_with("/b/products/webhooks/") {
            return stripe::handle_webhook(ctx, &msg, input).await;
        }

        // Per-user rate limiting for authenticated endpoints. Allowed(headers)
        // is discarded: attaching X-RateLimit-* to a streaming OutputStream
        // would need platform-side middleware to inject headers after the
        // handler returns. Limits are still enforced, just not surfaced.
        if let RateLimitOutcome::Limited(out) =
            check_user_rate_limit(&self.limiter, ctx, &msg).await
        {
            return out;
        }

        // Admin API at /b/products/api/admin/... → normalize to /admin/b/products/...
        if let Some(rest) = path.strip_prefix("/b/products/api/admin") {
            let is_admin = helpers::is_admin(&msg);
            if !is_admin {
                return crate::ui::forbidden_response(&msg);
            }
            msg.set_meta("req.resource", format!("/admin/b/products{rest}"));
            return handlers::handle_admin(ctx, &msg, input).await;
        }

        // User API at /b/products/api/... → normalize to /b/products/...
        if let Some(rest) = path.strip_prefix("/b/products/api") {
            msg.set_meta("req.resource", format!("/b/products{rest}"));
            return handlers::handle_user(ctx, &msg, input).await;
        }

        // User endpoints at /b/products/... (catalog, checkout, subscription, etc.)
        if path.starts_with("/b/products/") || path == "/b/products" {
            return handlers::handle_user(ctx, &msg, input).await;
        }

        err_not_found("not found")
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        if event.event_type == LifecycleType::Init {
            // Apply block-owned schema migrations. Migration 002 seeds the
            // default group/product templates (the static FK-parent rows the
            // groups/products tables require) via idempotent INSERTs, so there
            // is no longer a per-request runtime existence-check + seed here —
            // the hash-gate short-circuits in memory once applied.
            migrations::apply(ctx).await.map_err(|e| {
                WaferError::new(
                    wafer_run::ErrorCode::Internal,
                    format!("products migrations: {e}"),
                )
            })?;
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_block::register_static_block!("suppers-ai/products", ProductsBlock);
