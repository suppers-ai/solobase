use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

use crate::ui;

pub struct SystemBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SystemBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/system".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "System health, debug, navigation, and embedded static assets".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
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
                    {"id": "dashboard", "label": "Dashboard", "href": "/b/admin/", "icon": "layout-dashboard"},
                    {"id": "users", "label": "Users", "href": "/b/admin/users", "icon": "users"},
                    {"id": "variables", "label": "Variables", "href": "/b/admin/variables", "icon": "settings"},
                    {"id": "blocks", "label": "Blocks", "href": "/b/admin/blocks", "icon": "package"},
                    {"id": "logs", "label": "Logs", "href": "/b/admin/logs", "icon": "file-text"},
                    {"id": "products", "label": "Products", "href": "/b/products/", "icon": "package"},
                    {"id": "projects", "label": "Projects", "href": "/b/projects/", "icon": "server"},
                    {"id": "inspector", "label": "Inspector", "href": "/debug/inspector/ui", "icon": "globe"}
                ]);
                json_respond(msg, &nav)
            }
            // Embedded static assets (CSS, JS) with content-hash URLs for cache busting
            _ if path.starts_with("/static/app-") && path.ends_with(".css") => {
                ResponseBuilder::new(msg)
                    .set_header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(ui::assets::css().as_bytes().to_vec(), "text/css; charset=utf-8")
            }
            _ if path.starts_with("/static/htmx-") && path.ends_with(".min.js") => {
                ResponseBuilder::new(msg)
                    .set_header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(ui::assets::htmx_js().as_bytes().to_vec(), "application/javascript; charset=utf-8")
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
