use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub struct SystemBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SystemBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/system".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "System health, debug, and navigation endpoints".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();

        match path {
            "/health" => {
                let resp = serde_json::json!({"status": "ok"});
                json_respond(msg, &resp)
            }
            "/debug/time" => {
                let now = chrono::Utc::now();
                let resp = serde_json::json!({
                    "utc": now.to_rfc3339(),
                    "unix": now.timestamp(),
                    "unix_ms": now.timestamp_millis()
                });
                json_respond(msg, &resp)
            }
            "/nav" => {
                let nav = serde_json::json!([
                    {"id": "dashboard", "title": "Dashboard", "label": "Dashboard", "href": "/blocks/admin/frontend/#dashboard", "icon": "layout-dashboard"},
                    {"id": "users", "title": "Users", "label": "Users", "href": "/blocks/admin/frontend/#users", "icon": "users"},
                    {"id": "database", "title": "Database", "label": "Database", "href": "/blocks/admin/frontend/#database", "icon": "database"},
                    {"id": "storage", "title": "Storage", "label": "Storage", "href": "/blocks/admin/frontend/#storage", "icon": "hard-drive"},
                    {"id": "settings", "title": "Settings", "label": "Settings", "href": "/blocks/admin/frontend/#settings", "icon": "settings"},
                    {"id": "blocks", "title": "Blocks", "label": "Blocks", "href": "/blocks/admin/frontend/#blocks", "icon": "layers"},
                    {"id": "products", "title": "Products", "label": "Products", "href": "/blocks/products/frontend/", "icon": "shopping-bag"}
                ]);
                json_respond(msg, &nav)
            }
            _ if path.starts_with("/debug/inspector") => {
                ctx.call_block("wafer-run/inspector", msg).await
            }
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
