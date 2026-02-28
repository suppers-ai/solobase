use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;

pub struct UserPortalBlock;

impl Block for UserPortalBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "userportal-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "User portal configuration endpoint".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        // GET /ext/userportal/config -> static config
        let config = serde_json::json!({
            "logo_url": get_config_or(ctx, "LOGO_URL", "/logo.png"),
            "app_name": get_config_or(ctx, "APP_NAME", "Solobase"),
            "primary_color": get_config_or(ctx, "PRIMARY_COLOR", "#6366f1"),
            "enable_oauth": get_config_or(ctx, "ENABLE_OAUTH", "false"),
            "allow_signup": get_config_or(ctx, "ALLOW_SIGNUP", "true"),
            "show_powered_by": true,
            "features": {
                "files": true,
                "products": true,
                "legal_pages": true,
                "monitoring": true
            }
        });
        json_respond(msg.clone(), 200, &config)
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}

fn get_config_or(ctx: &dyn Context, key: &str, default: &str) -> String {
    ctx.services()
        .and_then(|s| s.config.as_ref())
        .and_then(|c| c.get(key))
        .unwrap_or_else(|| default.to_string())
}
