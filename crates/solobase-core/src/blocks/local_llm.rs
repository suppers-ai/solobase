use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::{InputStream, OutputStream};

use crate::blocks::helpers::{err_not_found, ResponseBuilder};

pub struct LocalLlmBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LocalLlmBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/local-llm", "0.0.1", "http-handler@v1", "Local LLM inference via WebLLM (browser only)")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Feature)
            .can_disable(true)
            .default_enabled(false)
            .description("Local LLM inference via WebLLM. Browser-only — requires WebGPU.")
            .endpoints(vec![
                BlockEndpoint::post("/b/local-llm/api/chat").summary("Chat with local model"),
                BlockEndpoint::get("/b/local-llm/api/models").summary("List available models"),
                BlockEndpoint::post("/b/local-llm/api/load").summary("Download and load a model"),
                BlockEndpoint::post("/b/local-llm/api/unload").summary("Unload model from VRAM"),
                BlockEndpoint::get("/b/local-llm/api/status").summary("Model load status"),
            ])
    }

    async fn handle(
        &self,
        _ctx: &dyn Context,
        msg: Message,
        _input: InputStream,
    ) -> OutputStream {
        let action = msg.action();
        let path = msg.path();

        match (action, path) {
            ("create", "/b/local-llm/api/chat")
            | ("retrieve", "/b/local-llm/api/models")
            | ("create", "/b/local-llm/api/load")
            | ("create", "/b/local-llm/api/unload")
            | ("retrieve", "/b/local-llm/api/status") => {
                // Return 501 Not Implemented — local-llm requires browser runtime with WebGPU
                let body = serde_json::json!({
                    "error": "not_available",
                    "message": "Local LLM requires the browser runtime with WebGPU support"
                });
                ResponseBuilder::new().status(501).json(&body)
            }
            _ => err_not_found("not found"),
        }
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
