wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

struct SystemBlockWasm;

impl Guest for SystemBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/system".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "System health, debug, and navigation endpoints".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let path = msg_path(&msg);

        match path {
            "/health" => {
                let resp = serde_json::json!({"status": "ok"});
                json_respond(&msg, &resp)
            }
            "/debug/time" => {
                let resp = serde_json::json!({
                    "error": "time not available in WASM component"
                });
                json_respond(&msg, &resp)
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
                    {"id": "products", "title": "Products", "label": "Products", "href": "/admin/b/products", "path": "/admin/b/products", "icon": "ShoppingBag"},
                    {"id": "files", "title": "Files", "label": "Files", "href": "/admin/storage", "path": "/admin/storage", "icon": "FolderOpen"},
                    {"id": "custom-tables", "title": "Custom Tables", "label": "Custom Tables", "href": "/admin/custom-tables", "path": "/admin/custom-tables", "icon": "Table"}
                ]);
                json_respond(&msg, &nav)
            }
            _ => err_not_found(&msg, "not found"),
        }
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        Ok(())
    }
}

export_block!(SystemBlockWasm);

// ---------------------------------------------------------------------------
// Minimal helpers (standalone — no wafer-block dependency needed)
// ---------------------------------------------------------------------------

fn msg_get_meta<'a>(msg: &'a Message, key: &str) -> &'a str {
    msg.meta
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
        .unwrap_or("")
}

fn msg_path<'a>(msg: &'a Message) -> &'a str {
    msg_get_meta(msg, "req.resource")
}

fn json_respond(msg: &Message, data: &serde_json::Value) -> BlockResult {
    match serde_json::to_vec(data) {
        Ok(body) => BlockResult {
            action: Action::Respond,
            response: Some(Response {
                data: body,
                meta: vec![MetaEntry {
                    key: "resp.content_type".to_string(),
                    value: "application/json".to_string(),
                }],
            }),
            error: None,
            message: Some(msg.clone()),
        },
        Err(e) => BlockResult {
            action: Action::Error,
            error: Some(WaferError {
                code: ErrorCode::Internal,
                message: e.to_string(),
                meta: Vec::new(),
            }),
            response: None,
            message: Some(msg.clone()),
        },
    }
}

fn err_not_found(msg: &Message, message: &str) -> BlockResult {
    BlockResult {
        action: Action::Error,
        error: Some(WaferError {
            code: ErrorCode::NotFound,
            message: message.to_string(),
            meta: Vec::new(),
        }),
        response: None,
        message: Some(msg.clone()),
    }
}
