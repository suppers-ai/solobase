//! `suppers-ai/fastembed` — native ONNX embedding block.
//!
//! Wraps a [`FastembedService`] from `wafer-block-fastembed` and exposes it
//! as a WAFER block speaking the `embedding@v1` service protocol. App blocks
//! (notably `suppers-ai/vector`) dispatch to it via
//! `ctx.call_block("suppers-ai/fastembed", ...)` whenever they need to embed
//! text with a locally-hosted model.
//!
//! This block is feature-gated behind `native-embedding` because the
//! underlying fastembed-rs crate pulls in ONNX Runtime (~100 MB of native
//! deps on most platforms). Consumers that only need remote embedding
//! providers — or the browser runtime where ONNX isn't applicable — should
//! not pay the build cost, so registration is conditional in `blocks::mod`.
//!
//! ## Lazy service construction
//!
//! `FastembedService::default_model()` triggers ONNX model download + load
//! (tens to hundreds of MB) — not cheap. The `info()` path in
//! `blocks::all_block_infos()` constructs every block just to read its
//! metadata, so we must *not* eagerly load the model in the constructor.
//! The service is built lazily on the first `handle()` call and cached in
//! a `OnceLock` for the lifetime of the singleton.

use std::sync::{Arc, OnceLock};

use wafer_block_fastembed::FastembedService;
use wafer_core::interfaces::vector::handler::handle_embedding_message;
use wafer_run::{
    block::{Block, BlockInfo},
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::blocks::helpers::err_internal;

/// Native ONNX embedding block.
///
/// Singleton. The wrapped `FastembedService` is initialized on the first
/// `handle()` call — construction is free, no model weights are loaded
/// until someone asks to embed something. An init error surfaces to the
/// caller as an `Internal` error on that first request.
pub struct FastembedBlock {
    service: OnceLock<Arc<FastembedService>>,
}

impl FastembedBlock {
    /// Build a `FastembedBlock` with a lazy service.
    ///
    /// The model is loaded on first embed, not here — this is cheap enough
    /// to call from `blocks::all_block_infos()` without triggering an ONNX
    /// download.
    pub fn new() -> Self {
        Self {
            service: OnceLock::new(),
        }
    }

    fn get_service(&self) -> Result<&FastembedService, String> {
        // OnceLock::get_or_init can't surface errors, so we do a
        // get-then-try-set dance. The race case — two callers both trying
        // to init — is resolved by `set`: whichever arrives second returns
        // Err and we fall back to `get().unwrap()` to grab the winner.
        if let Some(svc) = self.service.get() {
            return Ok(svc.as_ref());
        }
        let built = FastembedService::default_model()
            .map_err(|e| format!("fastembed init failed: {e}"))?;
        let arc = Arc::new(built);
        match self.service.set(arc) {
            Ok(()) => Ok(self.service.get().expect("just set").as_ref()),
            Err(_) => Ok(self.service.get().expect("other thread set it").as_ref()),
        }
    }
}

impl Default for FastembedBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for FastembedBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/fastembed",
            "0.0.1",
            "embedding@v1",
            "Native ONNX text embedding via fastembed-rs",
        )
        .instance_mode(InstanceMode::Singleton)
        .category(wafer_run::BlockCategory::Service)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let body = input.collect_to_bytes().await;
        let svc = match self.get_service() {
            Ok(s) => s,
            Err(e) => return err_internal(&e),
        };
        handle_embedding_message(svc, &msg, &body).await
    }

    async fn lifecycle(
        &self,
        _ctx: &dyn Context,
        _event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
