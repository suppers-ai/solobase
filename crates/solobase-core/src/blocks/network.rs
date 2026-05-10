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
use wafer_run::{block::Block, context::Context, types::*, BlockInfo, InputStream, OutputStream};

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

        // Drain the inner stream into a buffer of events so we can forward
        // them verbatim, preserving all frame boundaries.
        let mut events: Vec<StreamEvent> = Vec::new();
        let mut inner = inner_out;
        while let Some(evt) = inner.next().await {
            events.push(evt);
        }

        // Re-emit the captured events to the caller, preserving frame
        // boundaries so the typed network client can decode header + body.
        OutputStream::from_producer(move |sink, _cancel| async move {
            for evt in events {
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
