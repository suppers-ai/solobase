wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

mod helpers;
mod handlers;
mod pricing;
mod purchase;
mod stripe;
mod variables;
mod models;

use helpers::*;

pub(crate) const PRODUCTS_COLLECTION: &str = "block_products_products";
pub(crate) const GROUPS_COLLECTION: &str = "block_products_groups";
pub(crate) const TYPES_COLLECTION: &str = "block_products_types";
pub(crate) const PRICING_COLLECTION: &str = "block_products_pricing_templates";
pub(crate) const PURCHASES_COLLECTION: &str = "block_products_purchases";
pub(crate) const LINE_ITEMS_COLLECTION: &str = "block_products_line_items";
pub(crate) const GROUP_TEMPLATES_COLLECTION: &str = "block_products_group_templates";
pub(crate) const PRODUCT_TEMPLATES_COLLECTION: &str = "block_products_product_templates";

struct ProductsBlockWasm;

impl Guest for ProductsBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/products".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Products, pricing, purchases, and payment integration".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let path = msg_path(&msg).to_string();

        // Webhook (no auth required)
        if path == "/b/products/webhooks" || path.starts_with("/b/products/webhooks/") {
            return stripe::handle_webhook(&msg);
        }

        // Admin routes
        if path.starts_with("/admin/b/products") {
            return handlers::handle_admin(&msg);
        }

        // User-facing routes
        if path.starts_with("/b/products") {
            return handlers::handle_user(&msg);
        }

        err_not_found(&msg, "not found")
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        // Lifecycle (template seeding) is handled by the native runtime.
        Ok(())
    }
}

export_block!(ProductsBlockWasm);
