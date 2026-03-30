wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

// wafer-core clients (use WASM sync variants via WIT call-block import)
use wafer_core::clients::config;

struct UserPortalBlockWasm;

impl Guest for UserPortalBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/userportal".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "User portal configuration endpoint".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let config_val = serde_json::json!({
            "logo_url": config::get_default("LOGO_URL", "/logo.png"),
            "app_name": config::get_default("APP_NAME", "Solobase"),
            "primary_color": config::get_default("PRIMARY_COLOR", "#6366f1"),
            "enable_oauth": config::get_default("ENABLE_OAUTH", "false"),
            "allow_signup": config::get_default("ALLOW_SIGNUP", "true"),
            "show_powered_by": true,
            "features": {
                "files": config::get_default("FEATURE_FILES", "true"),
                "products": config::get_default("FEATURE_PRODUCTS", "true"),
                "user_products": config::get_default("FEATURE_USER_PRODUCTS", "false"),
                "legal_pages": config::get_default("FEATURE_LEGAL_PAGES", "true"),
                "monitoring": config::get_default("FEATURE_MONITORING", "true"),
                "deployments": config::get_default("FEATURE_DEPLOYMENTS", "true")
            }
        });
        json_respond(&msg, &config_val)
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        Ok(())
    }
}

export_block!(UserPortalBlockWasm);

// ---------------------------------------------------------------------------
// Minimal helpers (standalone — no wafer-block dependency needed)
// ---------------------------------------------------------------------------

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
