//! Solobase network block wrapper.
//!
//! Wraps the wafer-core `NetworkBlock` to add outbound request logging.
//! Network access control is enforced by WRAP grants in `call_block()`
//! before the message reaches this handler.

use std::sync::Arc;

use wafer_core::interfaces::network::service::NetworkService;
use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::streams::output::TerminalNotResponse;
use wafer_run::types::*;
use wafer_run::BlockInfo;
use wafer_run::{InputStream, OutputStream};

use wafer_core::clients::database as db;

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

/// Parse the request payload to extract method + url for logging.
fn parse_request_info(body: &[u8]) -> (String, String) {
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
        let method = v
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET")
            .to_uppercase();
        let url = v
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        (method, url)
    } else {
        ("UNKNOWN".into(), String::new())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for SolobaseNetworkBlock {
    fn info(&self) -> BlockInfo {
        self.inner.info()
    }

    async fn handle(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
    ) -> OutputStream {
        let source_block = ctx.caller_id().unwrap_or("unknown").to_string();
        let body = input.collect_to_bytes().await;
        let (method, url) = parse_request_info(&body);

        // Network access control is enforced by WRAP in call_block() before
        // reaching this handler. This block only handles logging + execution.

        // Execute the actual request (re-wrap body as a fresh InputStream).
        let start = now_millis();
        let inner_out = self
            .inner
            .handle(ctx, msg, InputStream::from_bytes(body))
            .await;
        let buffered = inner_out.collect_buffered().await;
        let duration_ms = (now_millis() - start) as i64;

        // Extract status code from response, then re-emit the buffered output.
        let (status_code, error_message, result) = match buffered {
            Ok(resp) => {
                let status = serde_json::from_slice::<serde_json::Value>(&resp.body)
                    .ok()
                    .and_then(|v| v.get("status_code").and_then(|s| s.as_i64()))
                    .unwrap_or(0);
                // Re-emit as a buffered response, preserving meta if any.
                let mut builder = crate::blocks::helpers::ResponseBuilder::new();
                for entry in &resp.meta {
                    builder = builder.set_header(&entry.key, &entry.value);
                }
                let out = builder.body(resp.body, "");
                (status, String::new(), out)
            }
            Err(terminal) => terminal_to_output(terminal),
        };

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

        result
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

/// Convert a non-Complete terminal into a re-emitted OutputStream plus the
/// status/error fields used for logging.
fn terminal_to_output(terminal: TerminalNotResponse) -> (i64, String, OutputStream) {
    match terminal {
        TerminalNotResponse::Error(e) => {
            let msg = e.message.clone();
            (0, msg, OutputStream::error(e))
        }
        TerminalNotResponse::Drop => (0, String::new(), OutputStream::drop_request()),
        TerminalNotResponse::Continue(m) => (0, String::new(), OutputStream::continue_with(m)),
        TerminalNotResponse::Malformed => (
            0,
            "malformed inner response".into(),
            OutputStream::error(WaferError::new(
                ErrorCode::Internal,
                "malformed inner response",
            )),
        ),
    }
}

// Pattern matching tests are in wafer-block/src/wrap.rs (grant_matches_resource)
