//! Solobase network block wrapper.
//!
//! Wraps the wafer-core `NetworkBlock` to add outbound request logging.
//! Network access control is enforced by WRAP grants in `call_block()`
//! before the message reaches this handler.

use std::sync::Arc;

use futures::StreamExt;
use wafer_block::{
    codec,
    stream::StreamEvent,
    wire::network::{Request as NetRequest, ResponseHeader as NetResponseHeader},
};
use wafer_core::{clients::database as db, interfaces::network::service::NetworkService};
use wafer_run::{
    block::Block, context::Context, types::*, BlockInfo, InputStream, OutputStream,
};

use super::helpers::{json_map, now_millis};

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

/// Parse the request payload (a MessagePack-encoded `wire::network::Request`)
/// to extract method + url for logging. Decode failures fall back to
/// `UNKNOWN`/empty so logging never blocks the actual request.
fn parse_request_info(body: &[u8]) -> (String, String) {
    match codec::decode::<NetRequest>(body) {
        Ok(req) => (req.method.to_uppercase(), req.url),
        Err(_) => ("UNKNOWN".into(), String::new()),
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseNetworkBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(&self, ctx: &dyn Context, msg: Message, input: InputStream) -> OutputStream {
        let source_block = ctx.caller_id().unwrap_or("unknown").to_string();
        let body = input.collect_to_bytes().await;
        let (method, url) = parse_request_info(&body);

        // Network access control is enforced by WRAP in call_block() before
        // reaching this handler. This block only handles logging + execution.

        // Dispatch to the wrapped wafer-core network block. The response is a
        // two-frame stream: a `wire::network::ResponseHeader` chunk followed
        // by zero-or-more body chunks. We must preserve frame boundaries when
        // forwarding — `collect_buffered` would concatenate header + body and
        // break the downstream `buffered_header_and_body` decoder.
        let start = now_millis();
        let inner_out = self
            .inner
            .handle(ctx, msg, InputStream::from_bytes(body))
            .await;

        // Drain the inner stream into a buffer of events so we can both log
        // (via the decoded header's `status_code`) and forward verbatim.
        let mut events: Vec<StreamEvent> = Vec::new();
        let mut inner = inner_out;
        while let Some(evt) = inner.next().await {
            events.push(evt);
        }
        let duration_ms = (now_millis() - start) as i64;

        // Pick out the status_code from the first Chunk (the header frame).
        let mut status_code: i64 = 0;
        let mut error_message = String::new();
        for evt in &events {
            match evt {
                StreamEvent::Chunk(bytes) => {
                    if let Ok(header) = codec::decode::<NetResponseHeader>(bytes) {
                        status_code = header.status_code as i64;
                    }
                    break;
                }
                StreamEvent::Error(e) => {
                    error_message = e.message.clone();
                    break;
                }
                _ => continue,
            }
        }

        // Log the request (best-effort, don't fail the actual request)
        let _ = db::create(
            ctx,
            crate::blocks::admin::NETWORK_REQUEST_LOGS_COLLECTION,
            json_map(serde_json::json!({
                "source_block": source_block,
                "method": method,
                "url": url,
                "status_code": status_code,
                "duration_ms": duration_ms,
                "error_message": error_message,
            })),
        )
        .await;

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
