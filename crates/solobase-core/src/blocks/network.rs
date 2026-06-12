//! Solobase network block wrapper.
//!
//! Wraps the wafer-core `NetworkBlock` to buffer the response stream so
//! callers receive a complete, re-emittable event sequence. Network access
//! control is enforced by WRAP grants in `call_block()` before the message
//! reaches this handler.

use std::sync::Arc;

use futures::StreamExt;
use wafer_block::stream::StreamEvent;
use wafer_core::interfaces::network::service::NetworkService;
use wafer_run::{Block, context::Context, BlockInfo, InputStream, OutputStream, LifecycleEvent, Message, WaferError};

/// A network block that logs outbound requests.
pub struct SolobaseNetworkBlock {
    inner: wafer_core::service_blocks::network::NetworkBlock,
}

impl SolobaseNetworkBlock {
    pub fn new(service: Arc<dyn NetworkService>) -> Self {
        Self {
            inner: wafer_core::service_blocks::network::NetworkBlock::new(service),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseNetworkBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let body = input.collect_to_bytes().await;

        // Network access control is enforced by WRAP in call_block() before
        // reaching this handler.

        // Dispatch to the wrapped wafer-core network block. The response is a
        // two-frame stream: a `wire::network::ResponseHeader` chunk followed
        // by zero-or-more body chunks. We must preserve frame boundaries when
        // forwarding — `collect_buffered` would concatenate header + body and
        // break the downstream `buffered_header_and_body` decoder.
        let inner_out = self
            .inner
            .handle(ctx, msg, InputStream::from_bytes(body))
            .await;

        // Forward events to the caller as they arrive. Previously we drained
        // the inner stream into a `Vec<StreamEvent>` before re-emitting,
        // which buffered the entire HTTP response body in memory. Forwarding
        // event-by-event keeps frame boundaries intact (each `StreamEvent`
        // maps 1:1 to a `sink.*` call) while staying genuinely streaming.
        OutputStream::from_producer(move |sink, _cancel| async move {
            let mut inner = inner_out;
            while let Some(evt) = inner.next().await {
                match evt {
                    StreamEvent::Chunk(bytes) => {
                        if sink.send_chunk(bytes).await.is_err() {
                            return;
                        }
                    }
                    StreamEvent::Meta(entry) => {
                        let _ = sink.send_meta(entry).await;
                    }
                    StreamEvent::Complete { meta } => {
                        let _ = sink.complete(meta).await;
                        return;
                    }
                    StreamEvent::Error(e) => {
                        let _ = sink.error(*e).await;
                        return;
                    }
                    StreamEvent::Drop => {
                        let _ = sink.drop_request().await;
                        return;
                    }
                    StreamEvent::Continue(m) => {
                        let _ = sink.continue_with(m).await;
                        return;
                    }
                    StreamEvent::Halt { body, meta } => {
                        let _ = sink.halt(body, meta).await;
                        return;
                    }
                }
            }
        })
    }

    async fn lifecycle(
        &self,
        ctx: &dyn Context,
        event: LifecycleEvent,
    ) -> std::result::Result<(), WaferError> {
        self.inner.lifecycle(ctx, event).await
    }
}

/// Create a new SolobaseNetworkBlock (caller must register it with the runtime).
pub fn create(service: Arc<dyn NetworkService>) -> Arc<SolobaseNetworkBlock> {
    Arc::new(SolobaseNetworkBlock::new(service))
}

// Pattern matching tests are in wafer-block/src/wrap.rs (grant_matches_resource)
