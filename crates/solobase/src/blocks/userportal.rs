use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_core::clients::config;

pub struct UserPortalBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for UserPortalBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "@solobase/userportal".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "User portal configuration endpoint".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
            runtime: wafer_run::types::BlockRuntime::Native,
            requires: Vec::new(),
        }
    }

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // GET /ext/userportal/config -> static config
        let config_val = serde_json::json!({
            "logo_url": config::get_default(ctx, "LOGO_URL", "/logo.png").await,
            "app_name": config::get_default(ctx, "APP_NAME", "Solobase").await,
            "primary_color": config::get_default(ctx, "PRIMARY_COLOR", "#6366f1").await,
            "enable_oauth": config::get_default(ctx, "ENABLE_OAUTH", "false").await,
            "allow_signup": config::get_default(ctx, "ALLOW_SIGNUP", "true").await,
            "show_powered_by": true,
            "features": {
                "files": true,
                "products": true,
                "legal_pages": true,
                "monitoring": true
            }
        });
        json_respond(msg, &config_val)
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
