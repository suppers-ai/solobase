//! Solobase network block wrapper.
//!
//! Wraps the wafer-core `NetworkBlock` to add:
//! - Outbound request logging to `network_request_logs`
//! - Network rule enforcement (global allow/block lists)

use std::sync::Arc;

use wafer_core::interfaces::network::service::NetworkService;
use wafer_run::block::Block;
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::BlockInfo;

use wafer_core::clients::database as db;

use super::helpers::{json_map, now_millis};

/// A network block that logs outbound requests and enforces network rules.
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

/// Check network rules. Returns `None` if allowed, or `Some(reason)` if blocked.
///
/// Rules are scoped per-block: a rule with `block_name = "suppers-ai/products"` only
/// applies to that block. A rule with `block_name = ""` or `"*"` applies to all blocks.
///
/// Evaluation: block/deny rules are checked first (any match = deny).
/// Then allow rules: if any exist for this caller, URL must match at least one.
/// No rules = allow all.
async fn check_rules(ctx: &dyn Context, caller: &str, url: &str) -> Option<String> {
    let rules = match db::list(
        ctx,
        "suppers_ai__admin__network_rules",
        &db::ListOptions {
            sort: vec![db::SortField {
                field: "priority".into(),
                desc: true,
            }],
            limit: 10_000,
            ..Default::default()
        },
    )
    .await
    {
        Ok(result) => result.records,
        Err(e) => {
            tracing::debug!("network rules query failed (table may not exist yet): {e}");
            return None;
        }
    };

    if rules.is_empty() {
        return None;
    }

    // Two-pass: block rules first, then allow rules
    let mut has_allow_rules = false;
    let mut explicitly_allowed = false;

    for rule in &rules {
        let rule_type = rule
            .data
            .get("rule_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let pattern = rule
            .data
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rule_block = rule
            .data
            .get("block_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if pattern.is_empty() {
            continue;
        }

        // Skip rules that don't apply to this caller
        if !rule_block.is_empty() && rule_block != "*" && rule_block != caller {
            continue;
        }

        let matches = pattern_matches(pattern, url);

        if rule_type == "block" && matches {
            return Some(format!("blocked by rule: {pattern}"));
        }
        if rule_type == "allow" {
            has_allow_rules = true;
            if matches {
                explicitly_allowed = true;
            }
        }
    }

    // If allow rules exist but none matched, block
    if has_allow_rules && !explicitly_allowed {
        return Some("not in allowlist".into());
    }

    None
}

/// Simple glob-style pattern matching for URL patterns.
/// Supports `*` as wildcard for any characters.
fn pattern_matches(pattern: &str, url: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Convert glob pattern to simple matching
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        // No wildcard — exact match
        return url == pattern;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = url[pos..].find(part) {
            if i == 0 && found != 0 {
                // First part must match at start
                return false;
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }

    // If pattern doesn't end with *, the url must end at pos
    if !pattern.ends_with('*') {
        return pos == url.len();
    }

    true
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

        // Check network rules
        if !url.is_empty() {
            if let Some(reason) = check_rules(ctx, &source_block, &url).await {
                let _ = db::create(
                    ctx,
                    "suppers_ai__admin__network_request_logs",
                    json_map(serde_json::json!({
                        "source_block": source_block,
                        "method": method,
                        "url": url,
                        "status_code": 0,
                        "duration_ms": 0,
                        "error_message": format!("BLOCKED: {reason}"),
                    })),
                )
                .await;

                return Result_::error(WaferError::new(
                    ErrorCode::PERMISSION_DENIED,
                    format!("network request blocked: {reason}"),
                ));
            }
        }

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
            "suppers_ai__admin__network_request_logs",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matches() {
        // Wildcard all
        assert!(pattern_matches("*", "https://example.com"));

        // Exact match
        assert!(pattern_matches(
            "https://api.stripe.com/v1",
            "https://api.stripe.com/v1"
        ));
        assert!(!pattern_matches(
            "https://api.stripe.com/v1",
            "https://api.stripe.com/v2"
        ));

        // Trailing wildcard
        assert!(pattern_matches(
            "https://api.stripe.com/*",
            "https://api.stripe.com/v1/charges"
        ));
        assert!(!pattern_matches(
            "https://api.stripe.com/*",
            "https://evil.com/api.stripe.com/"
        ));

        // Leading wildcard
        assert!(pattern_matches(
            "*.internal.corp*",
            "https://admin.internal.corp/api"
        ));
        assert!(pattern_matches(
            "*.internal.corp*",
            "http://db.internal.corp:5432/"
        ));

        // Middle wildcard
        assert!(pattern_matches(
            "https://*.example.com/*",
            "https://api.example.com/v1"
        ));
        assert!(!pattern_matches(
            "https://*.example.com/*",
            "http://api.example.com/v1"
        ));

        // No match
        assert!(!pattern_matches(
            "https://safe.com/*",
            "https://evil.com/safe.com/"
        ));
    }
}
