//! Image-generation feature block. Exposes:
//!   POST /b/image/api/generate — returns PNG bytes
//!   GET  /b/image/api/models   — aggregated list of registered image models
//!
//! Delegates to the runtime's `wafer-run/image` service block via the typed
//! native client at `wafer_core::clients::image`. The actual generation
//! backend is `BrowserImageService` in solobase-browser (transformers.js +
//! SD-Turbo on WebGPU); native deployments register zero image services and
//! the block 4xx's accordingly.

pub mod routes;

use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    AuthLevel, InputStream, OutputStream,
};

use crate::{blocks::helpers::err_not_found, ui};

pub struct ImageBlock;

impl ImageBlock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ImageBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/image",
            "0.0.1",
            "http-handler@v1",
            "Text-to-image surface — routes to the wafer-run/image service block.",
        )
        .instance_mode(InstanceMode::Singleton)
        .requires(vec!["wafer-run/image".into()])
        .category(wafer_run::BlockCategory::Feature)
        .description(
            "Text-to-image surface. Forwards prompts to the runtime image \
             service router; the configured backend (e.g. BrowserImageService \
             with transformers.js + SD-Turbo) renders and returns PNG bytes.",
        )
        .endpoints(vec![
            BlockEndpoint::post("/b/image/api/generate")
                .summary("Generate an image from a prompt")
                .auth(AuthLevel::Authenticated),
            BlockEndpoint::get("/b/image/api/models")
                .summary("List available image models (aggregated across backends)")
                .auth(AuthLevel::Authenticated),
        ])
        .can_disable(true)
        .default_enabled(true)
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let user_id = msg.user_id().to_string();
        if user_id.is_empty() {
            return ui::forbidden_response(&msg);
        }

        match (msg.action(), msg.path()) {
            ("create", "/b/image/api/generate") => routes::handle_generate(ctx, input).await,
            ("retrieve", "/b/image/api/models") => routes::handle_list_models(ctx).await,
            _ => err_not_found("not found"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
::wafer_run::register_static_block!("suppers-ai/image", ImageBlock);
