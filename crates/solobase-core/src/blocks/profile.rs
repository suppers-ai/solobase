use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub struct ProfileBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProfileBlock {
    fn info(&self) -> BlockInfo {
        use wafer_run::AuthLevel;

        BlockInfo::new("suppers-ai/profile", "0.0.1", "http-handler@v1", "Profile sections endpoint")
            .instance_mode(InstanceMode::Singleton)
            .requires(vec!["wafer-run/database".into()])
            .category(wafer_run::BlockCategory::Feature)
            .description("User profile sections endpoint. Provides a placeholder API for user profile customization.")
            .endpoints(vec![
                BlockEndpoint::get("/profile/sections", "Profile sections", AuthLevel::Authenticated),
            ])
            .can_disable(true)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // GET /profile/sections -> empty array
        let empty: Vec<serde_json::Value> = Vec::new();
        json_respond(msg, &empty)
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
