wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

mod helpers;
mod storage_routes;
mod cloud;
mod quota;
mod share;

use helpers::*;

struct FilesBlockWasm;

impl Guest for FilesBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/files".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "File storage, sharing, quotas, and access logging".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let path = msg_get_meta(&msg, "req.resource").to_string();

        // Direct share access (public, no auth)
        if path.starts_with("/storage/direct/") {
            return share::handle_direct_access(&msg);
        }

        // Cloud storage routes
        if path.starts_with("/b/cloudstorage") || path.starts_with("/admin/b/cloudstorage") {
            return cloud::handle(&msg);
        }

        // Admin storage routes
        if path.starts_with("/admin/storage") {
            return storage_routes::handle_admin(&msg);
        }

        // User storage routes
        if path.starts_with("/storage") {
            return storage_routes::handle(&msg);
        }

        err_not_found(&msg, "not found")
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        // Lifecycle is handled by the native runtime.
        Ok(())
    }
}

export_block!(FilesBlockWasm);
