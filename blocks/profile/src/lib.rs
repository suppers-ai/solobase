wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

struct ProfileBlockWasm;

impl Guest for ProfileBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/profile".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "User profile sections".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let path = msg.meta.iter().find(|e| e.key == "req.resource").map(|e| e.value.as_str()).unwrap_or("");
        match path {
            "/profile/sections" => {
                let body = serde_json::to_vec(&serde_json::json!([])).unwrap();
                BlockResult {
                    action: Action::Respond,
                    response: Some(Response {
                        data: body,
                        meta: vec![MetaEntry { key: "resp.content_type".to_string(), value: "application/json".to_string() }],
                    }),
                    error: None,
                    message: Some(msg),
                }
            }
            _ => BlockResult {
                action: Action::Error,
                error: Some(WaferError { code: ErrorCode::NotFound, message: "not found".to_string(), meta: Vec::new() }),
                response: None,
                message: Some(msg),
            },
        }
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        Ok(())
    }
}

export_block!(ProfileBlockWasm);
