wit_bindgen::generate!({
    world: "wafer-block",
    path: "../../../wafer-run/wit/wit",
    additional_derives: [serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash],
    export_macro_name: "export_block",
});

use exports::wafer::block_world::block::Guest;
use wafer::block_world::types::*;

mod helpers;
mod users;
mod database;
mod iam;
mod logs;
mod settings;
mod custom_tables;
mod wafer_info;

use helpers::*;

struct AdminBlockWasm;

impl Guest for AdminBlockWasm {
    fn info() -> BlockInfo {
        BlockInfo {
            name: "suppers-ai/admin".to_string(),
            version: "1.0.0".to_string(),
            interface: "http-handler@v1".to_string(),
            summary: "Admin panel: users, database, IAM, logs, settings, wafer introspection, custom tables".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: Vec::new(),
            collections: Vec::new(),
            config_schema: None,
        }
    }

    fn handle(msg: Message) -> BlockResult {
        let path = msg_get_meta(&msg, "req.resource").to_string();

        if path.starts_with("/admin/users") {
            return users::handle(&msg);
        }
        if path.starts_with("/admin/database") {
            return database::handle(&msg);
        }
        if path.starts_with("/admin/iam") {
            return iam::handle(&msg);
        }
        if path.starts_with("/admin/logs") {
            return logs::handle(&msg);
        }
        if path.starts_with("/admin/settings") || path.starts_with("/settings") {
            return settings::handle(&msg);
        }
        if path.starts_with("/admin/wafer") {
            return wafer_info::handle(&msg);
        }
        if path.starts_with("/admin/custom-tables") {
            return custom_tables::handle(&msg);
        }

        err_not_found(&msg, "not found")
    }

    fn lifecycle(_event: LifecycleEvent) -> Result<(), WaferError> {
        // Lifecycle (seeding defaults) is handled by the native runtime.
        Ok(())
    }
}

export_block!(AdminBlockWasm);
