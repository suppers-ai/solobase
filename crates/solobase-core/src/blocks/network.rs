//! Solobase network block wrapper.
//!
//! Wraps the wafer-core `NetworkBlock` to add outbound request logging.
//! Network access control is enforced by WRAP grants in `call_block()`
//! before the message reaches this handler.

use std::sync::Arc;

use wafer_core::interfaces::network::service::NetworkService;
use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::BlockInfo;

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
fn parse_request_info(msg: &Message) -> (String, String) {
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&msg.data) {
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

    async fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let source_block = ctx.caller_id().unwrap_or("unknown").to_string();
        let (method, url) = parse_request_info(msg);

        // Network access control is enforced by WRAP in call_block() before
        // reaching this handler. This block only handles logging + execution.

        // Execute the actual request
        let start = now_millis();
        let result = self.inner.handle(ctx, msg).await;
        let duration_ms = (now_millis() - start) as i64;

        // Extract status code from response
        let (status_code, error_message) = match &result.action {
            Action::Respond => {
                if let Some(ref resp) = result.response {
                    let status = serde_json::from_slice::<serde_json::Value>(&resp.data)
                        .ok()
                        .and_then(|v| v.get("status_code").and_then(|s| s.as_i64()))
                        .unwrap_or(0);
                    (status, String::new())
                } else {
                    (0, String::new())
                }
            }
            Action::Error => {
                let err_msg = result
                    .error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_default();
                (0, err_msg)
            }
            _ => (0, String::new()),
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

// Pattern matching tests are in wafer-block/src/wrap.rs (grant_matches_resource)
