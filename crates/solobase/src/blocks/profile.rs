use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub struct ProfileBlock;

impl Block for ProfileBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "profile-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Profile sections endpoint".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // GET /profile/sections -> empty array
        let empty: Vec<serde_json::Value> = Vec::new();
        json_respond(msg.clone(), 200, &empty)
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
