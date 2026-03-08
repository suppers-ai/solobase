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
            name: "@solobase/system".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "System health, debug, and navigation endpoints".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
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
                    {"id": "dashboard", "title": "Dashboard", "label": "Dashboard", "href": "/admin", "path": "/admin", "icon": "LayoutDashboard"},
                    {"id": "users", "title": "Users", "label": "Users", "href": "/admin/users", "path": "/admin/users", "icon": "Users"},
                    {"id": "database", "title": "Database", "label": "Database", "href": "/admin/database", "path": "/admin/database", "icon": "Database"},
                    {"id": "iam", "title": "IAM", "label": "IAM", "href": "/admin/iam", "path": "/admin/iam", "icon": "Shield"},
                    {"id": "logs", "title": "Logs", "label": "Logs", "href": "/admin/logs", "path": "/admin/logs", "icon": "FileText"},
                    {"id": "settings", "title": "Settings", "label": "Settings", "href": "/admin/settings", "path": "/admin/settings", "icon": "Settings"},
                    {"id": "legalpages", "title": "Legal Pages", "label": "Legal Pages", "href": "/admin/legalpages", "path": "/admin/legalpages", "icon": "Scale"},
                    {"id": "products", "title": "Products", "label": "Products", "href": "/admin/ext/products", "path": "/admin/ext/products", "icon": "ShoppingBag"},
                    {"id": "files", "title": "Files", "label": "Files", "href": "/admin/storage", "path": "/admin/storage", "icon": "FolderOpen"},
                    {"id": "custom-tables", "title": "Custom Tables", "label": "Custom Tables", "href": "/admin/custom-tables", "path": "/admin/custom-tables", "icon": "Table"}
                ]);
                json_respond(msg, &nav)
            }
            _ => err_not_found(msg, "not found"),
        }
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
