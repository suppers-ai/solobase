mod handlers;
mod pricing;
mod purchase;
mod stripe;
mod variables;
pub(crate) mod models;

use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::DatabaseService;

pub struct ProductsBlock;

pub(crate) fn get_db(ctx: &dyn Context) -> Result<&Arc<dyn DatabaseService>, Result_> {
    ctx.services()
        .and_then(|s| s.database.as_ref())
        .ok_or_else(|| Result_::error(WaferError::new("unavailable", "Database service unavailable")))
}

impl Block for ProductsBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "products-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Products, pricing, purchases, and payment integration".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();

        // Webhook (no auth)
        if path == "/ext/products/webhooks" || path.starts_with("/ext/products/webhooks/") {
            return stripe::handle_webhook(ctx, msg);
        }

        // Admin routes
        if path.starts_with("/admin/ext/products") {
            return handlers::handle_admin(ctx, msg);
        }

        // User-facing routes
        if path.starts_with("/ext/products") {
            return handlers::handle_user(ctx, msg);
        }

        err_not_found(msg.clone(), "not found")
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
