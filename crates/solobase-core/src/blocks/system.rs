use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use crate::blocks::helpers::{err_not_found, ok_json, ResponseBuilder};
use crate::ui;

pub struct SystemBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SystemBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/system", "0.0.1", "http-handler@v1", "System health and embedded static assets")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Infrastructure)
            .description("Core system services including health checks and embedded static assets (CSS, JavaScript).")
            .endpoints(vec![
                BlockEndpoint::get("/health").summary("Health check"),
                BlockEndpoint::get("/b/static/app-{hash}.css").summary("Embedded CSS"),
                BlockEndpoint::get("/b/static/htmx-{hash}.min.js").summary("Embedded JavaScript"),
            ])
    }

    async fn handle(
        &self,
        _ctx: &dyn Context,
        msg: Message,
        _input: InputStream,
    ) -> OutputStream {
        let path = msg.path();

        match path {
            "/health" => {
                let resp = serde_json::json!({"status": "ok"});
                ok_json(&resp)
            }
            // Embedded static assets (CSS, JS) with content-hash URLs for cache busting
            _ if path.starts_with("/b/static/app-") && path.ends_with(".css") => {
                ResponseBuilder::new()
                    .set_header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(
                        ui::assets::css().as_bytes().to_vec(),
                        "text/css; charset=utf-8",
                    )
            }
            _ if path.starts_with("/b/static/htmx-") && path.ends_with(".min.js") => {
                ResponseBuilder::new()
                    .set_header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(
                        ui::assets::htmx_js().as_bytes().to_vec(),
                        "application/javascript; charset=utf-8",
                    )
            }
            _ => err_not_found("not found"),
        }
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
