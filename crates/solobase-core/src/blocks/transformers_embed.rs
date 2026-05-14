//! `suppers-ai/transformers-embed` — browser-side (Transformers.js) embedding block.
//!
//! Mirror of `blocks/fastembed.rs`, but accepts an injected
//! `Arc<dyn EmbeddingService>` so the WASM-specific service in
//! `solobase-browser` can be constructed by `solobase-web` and passed in
//! via the `SolobaseBuilder`. Compiled only on `wasm32`.

use std::sync::Arc;

use wafer_core::interfaces::vector::{
    handler::handle_embedding_message, service::EmbeddingService,
};
use wafer_run::{
    block::{Block, BlockInfo},
    common::ServiceOp,
    context::Context,
    types::*,
    InputStream, OutputStream,
};

use crate::blocks::helpers::err_internal;

/// Browser-side embedding block backed by an injected `EmbeddingService`.
///
/// The service (typically `BrowserEmbeddingService` from `solobase-browser`) is
/// constructed and injected by `solobase-web` via `SolobaseBuilder::embedding_service`.
pub struct TransformersEmbedBlock {
    service: Arc<dyn EmbeddingService>,
}

impl TransformersEmbedBlock {
    pub fn new(service: Arc<dyn EmbeddingService>) -> Self {
        Self { service }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for TransformersEmbedBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new(
            "suppers-ai/transformers-embed",
            "0.0.1",
            "embedding@v1",
            "Browser text embedding via Transformers.js",
        )
        .category(wafer_run::BlockCategory::Service)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let body = input.collect_to_bytes().await;
        match msg.kind.as_str() {
            ServiceOp::EMBEDDING_EMBED => {
                handle_embedding_message(self.service.as_ref(), &msg, &body).await
            }
            other => err_internal("unsupported op", other),
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
