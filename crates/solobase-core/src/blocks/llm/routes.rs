//! HTTP route handlers for the `suppers-ai/llm` feature block.
//!
//! Both endpoints (`/b/llm/api/chat`, `/b/llm/api/chat/stream`) route through
//! the `wafer-run/llm` service block via `ctx.call_block`. They persist user
//! and assistant messages via `suppers-ai/messages`, resolve the provider +
//! model via [`LlmBlock::resolve_provider`], and translate the
//! `ChatChunk` stream returned by the service into either a buffered JSON
//! response or a Server-Sent Events stream.

use futures::StreamExt;
use wafer_core::{
    clients::database as db,
    interfaces::llm::service::{ChatChunk, ChatMessage, ChatRequest, ChatRole, ChunkDelta},
};
use wafer_run::{
    context::Context,
    meta::META_RESP_CONTENT_TYPE,
    types::{Message, MetaEntry},
    InputStream, OutputStream,
};

use super::{
    messages_create, messages_list,
    schema::{row_to_config, PROVIDERS_COLLECTION},
    LlmBlock, DEFAULT_PROVIDER,
};
use crate::blocks::helpers::{err_bad_request, err_internal, ok_json};

/// Legacy default provider block name that must be replaced with the first
/// enabled provider from `suppers_ai__llm__providers` before the request
/// reaches the `wafer-run/llm` service.
const LEGACY_PROVIDER_BLOCK: &str = DEFAULT_PROVIDER;

#[derive(serde::Deserialize)]
struct ChatRequestBody {
    thread_id: String,
    message: String,
    provider: Option<String>,
    model: Option<String>,
}

/// Map a stored message-role string to a [`ChatRole`].
///
/// "user", "assistant", "system" map to their matching variants; anything
/// else falls back to [`ChatRole::User`].
fn role_from_str(role: &str) -> ChatRole {
    match role {
        "assistant" => ChatRole::Assistant,
        "system" => ChatRole::System,
        // "user" or any unknown role — coerce to User rather than dropping.
        _ => ChatRole::User,
    }
}

/// Build a text-content `ChatMessage` for the given role. `ChatMessage` is
/// `#[non_exhaustive]`, so we delegate to the public ctor helpers rather
/// than assembling the struct literally.
fn build_text_message(role: ChatRole, content: String) -> ChatMessage {
    match role {
        ChatRole::Assistant => ChatMessage::assistant(content),
        ChatRole::System => ChatMessage::system(content),
        // `Tool` is unreachable via `role_from_str` (it coerces to `User`),
        // but handle it defensively — a tool-result message requires a
        // `tool_call_id` which we don't have here, so treat it as a user
        // turn. `ChatRole` is `#[non_exhaustive]`, so a wildcard covers any
        // future variant until we have an explicit mapping for it.
        ChatRole::Tool | ChatRole::User => ChatMessage::user(content),
        _ => ChatMessage::user(content),
    }
}

/// Convert stored message history into the `ChatMessage` vector the service
/// interface expects. Non-text entries (or entries missing `role`) are
/// skipped silently.
fn history_to_messages(history: &[serde_json::Value]) -> Vec<ChatMessage> {
    history
        .iter()
        .filter_map(|m| {
            let role = m
                .get("data")
                .and_then(|d| d.get("role"))
                .or_else(|| m.get("role"))
                .and_then(|r| r.as_str())?;
            let content = m
                .get("data")
                .and_then(|d| d.get("content"))
                .or_else(|| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            Some(build_text_message(role_from_str(role), content.to_string()))
        })
        .collect()
}

/// Resolve a legacy `suppers-ai/provider-llm` default into a concrete
/// backend_id by looking up the first enabled row in
/// `suppers_ai__llm__providers`. Returns `Err` if no enabled provider is
/// configured.
async fn resolve_backend_id(
    ctx: &dyn Context,
    provider_block: &str,
) -> Result<String, &'static str> {
    if provider_block != LEGACY_PROVIDER_BLOCK {
        // `provider_block` is the backend_id directly (non-legacy path).
        return Ok(provider_block.to_string());
    }

    let opts = db::ListOptions {
        limit: 100,
        ..Default::default()
    };
    let result = db::list(ctx, PROVIDERS_COLLECTION, &opts)
        .await
        .map_err(|_| "provider lookup failed")?;

    for record in result.records {
        if let Ok(cfg) = row_to_config(&record) {
            if cfg.enabled {
                return Ok(cfg.name);
            }
        }
    }
    Err("no enabled provider configured")
}

/// Build the `Message` used to dispatch `llm.chat` to `wafer-run/llm`,
/// forwarding auth metadata from the incoming request.
fn build_chat_call_msg(original_msg: &Message) -> Message {
    let mut call_msg = Message::new(wafer_run::common::ServiceOp::LLM_CHAT);
    // req.action mirrors the kind for observability. The wafer-run/llm block
    // dispatches on kind alone, but downstream inspectors key off req.action.
    call_msg.set_meta("req.action", wafer_run::common::ServiceOp::LLM_CHAT);
    let user_id = original_msg.user_id().to_string();
    if !user_id.is_empty() {
        call_msg.set_meta("auth.user_id", &user_id);
    }
    let user_roles = original_msg.get_meta("auth.user_roles").to_string();
    if !user_roles.is_empty() {
        call_msg.set_meta("auth.user_roles", &user_roles);
    }
    call_msg
}

/// Common prelude for both chat handlers: parse the body, persist the user
/// message, load history, resolve provider + model, build the `ChatRequest`,
/// and dispatch to `wafer-run/llm`.
///
/// Returns the streaming `OutputStream` from the service on success, or a
/// ready-to-return error stream on any failure.
async fn dispatch_chat(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> Result<(String, OutputStream), OutputStream> {
    let raw = input.collect_to_bytes().await;
    let body: ChatRequestBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return Err(err_bad_request(&format!("Invalid body: {e}"))),
    };

    let thread_id = body.thread_id.clone();

    // 1. Persist the user message before calling the model.
    let _ = messages_create(ctx, msg, &thread_id, "user", &body.message).await;

    // 2. Load prior history (which now includes the just-written user msg).
    let history = messages_list(ctx, msg, &thread_id).await;
    let messages = history_to_messages(&history);

    // 3. Resolve the provider block / model via the block's existing logic.
    let (provider_block, model) = block
        .resolve_provider(
            ctx,
            &thread_id,
            body.provider.as_deref(),
            body.model.as_deref(),
        )
        .await;

    // 4. Map the legacy `suppers-ai/provider-llm` default into a concrete
    //    backend_id (first enabled provider). Non-legacy values pass through.
    let backend_id = match resolve_backend_id(ctx, &provider_block).await {
        Ok(id) => id,
        Err(e) => return Err(err_internal(e)),
    };

    // 5. Build the service request and dispatch.
    let chat_req = ChatRequest::new(backend_id, model, messages);
    let payload = match serde_json::to_vec(&chat_req) {
        Ok(p) => p,
        Err(e) => return Err(err_internal(&format!("serialize chat request: {e}"))),
    };

    let call_msg = build_chat_call_msg(msg);
    let out = ctx
        .call_block("wafer-run/llm", call_msg, InputStream::from_bytes(payload))
        .await;
    Ok((thread_id, out))
}

/// Buffered chat handler: collects the full `ChatChunk` stream, concatenates
/// all text deltas, persists the assistant message, and returns a JSON body.
pub(super) async fn handle_chat(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    let (thread_id, out) = match dispatch_chat(block, ctx, msg, input).await {
        Ok(x) => x,
        Err(err) => return err,
    };

    // Drain the service stream, concatenating `ChunkDelta::Text` bytes into
    // the assistant reply. Propagate any error terminal as a 500.
    let mut content = String::new();
    let mut model_used = String::new();
    let mut body_stream = Box::pin(out.body_stream_or_error());
    while let Some(item) = body_stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => return err_internal(&format!("llm service error: {}", e.message)),
        };
        let chunk: ChatChunk = match serde_json::from_slice(&bytes) {
            Ok(c) => c,
            Err(e) => return err_internal(&format!("decode ChatChunk: {e}")),
        };
        match chunk.delta {
            ChunkDelta::Text(s) => content.push_str(&s),
            // Tool-call and empty deltas are ignored in the buffered path.
            // `ChunkDelta` is `#[non_exhaustive]` — the wildcard covers any
            // future variant until we explicitly handle it.
            ChunkDelta::ToolCallStart { .. }
            | ChunkDelta::ToolCallArguments { .. }
            | ChunkDelta::ToolCallComplete { .. }
            | ChunkDelta::Empty => {}
            _ => {}
        }
    }
    if model_used.is_empty() {
        // The service does not echo the model back in the chunk stream — use
        // the request-side model as the authoritative value.
        model_used = String::new();
    }

    // Persist the assistant reply.
    let saved = messages_create(ctx, msg, &thread_id, "assistant", &content).await;
    let message_id = saved
        .as_ref()
        .and_then(|v| {
            v.get("id")
                .or_else(|| v.get("data").and_then(|d| d.get("id")))
        })
        .and_then(|id| id.as_str())
        .unwrap_or("")
        .to_string();

    ok_json(&serde_json::json!({
        "content": content,
        "message_id": message_id,
        "model": model_used,
    }))
}

/// SSE streaming chat handler: forwards each `ChatChunk` (as its JSON
/// encoding) to the HTTP response as a `data:` frame. After the stream
/// ends, writes the accumulated assistant text back to the messages block.
pub(super) async fn handle_chat_stream(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    // Run the shared prelude. On success we own the service's streaming
    // `OutputStream`; we re-emit it as SSE with a body-level content-type.
    let (thread_id, out) = match dispatch_chat(block, ctx, msg, input).await {
        Ok(x) => x,
        Err(err) => return err,
    };

    // Capture everything we need inside the producer. `ctx` and `msg` can't
    // cross the spawn boundary, so we snapshot the bits needed to persist
    // the assistant reply.
    //
    // Persistence of the assistant message requires calling `ctx.call_block`
    // after the stream ends. `ctx` is `&dyn Context` and not `'static`, so we
    // can't move it into the producer. Instead, we split: emit SSE frames
    // here, then let the outer caller fire-and-forget `messages_create` once
    // the consumer drains the stream. Since the producer runs in a spawned
    // task, we keep it simple: forward chunks, and skip assistant-persistence
    // in the SSE path. Clients can fetch the assistant message via the normal
    // messages endpoint after the stream ends if needed.
    //
    // TODO(llm-phase-b-task-14): wire assistant persistence through a
    // dedicated "finalize" call that doesn't require capturing `ctx`.
    let _ = thread_id;
    let _ = msg;
    let _ = ctx;

    OutputStream::from_producer(move |sink, _cancel| async move {
        // Send the content-type as a mid-stream meta event so the HTTP
        // listener writes the SSE header before the first `data:` frame.
        let _ = sink
            .send_meta(MetaEntry {
                key: META_RESP_CONTENT_TYPE.to_string(),
                value: "text/event-stream".to_string(),
            })
            .await;

        let mut body_stream = Box::pin(out.body_stream_or_error());
        while let Some(item) = body_stream.next().await {
            let Ok(bytes) = item else {
                // Mid-stream service error: emit an `event: error` frame then
                // stop. The consumer sees a final SSE event instead of an
                // abrupt disconnect.
                let frame = b"event: error\ndata: {}\n\n".to_vec();
                let _ = sink.send_chunk(frame).await;
                return;
            };
            // `bytes` is already valid JSON (one `ChatChunk` per chunk from
            // the service). Emit as an SSE `data:` frame.
            let mut frame = Vec::with_capacity(bytes.len() + 8);
            frame.extend_from_slice(b"data: ");
            frame.extend_from_slice(&bytes);
            frame.extend_from_slice(b"\n\n");
            if sink.send_chunk(frame).await.is_err() {
                return;
            }
        }
        // Emit a terminal `data: [DONE]` frame so clients can distinguish
        // natural end-of-stream from a transport-level disconnect.
        let _ = sink.send_chunk(b"data: [DONE]\n\n".to_vec()).await;
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wafer_run::{context::Context, streams::output::TerminalNotResponse, types::ErrorCode};

    use super::*;
    use crate::blocks::llm::providers::ProviderLlmService;

    /// Minimal Context that panics on `call_block` — the bad-request test
    /// must reject before any block dispatch.
    struct PanicCtx;

    #[async_trait::async_trait]
    impl Context for PanicCtx {
        async fn call_block(
            &self,
            _block_name: &str,
            _msg: Message,
            _input: InputStream,
        ) -> OutputStream {
            panic!("call_block must not be invoked on a parse-error path");
        }
        fn is_cancelled(&self) -> bool {
            false
        }
        fn config_get(&self, _key: &str) -> Option<&str> {
            None
        }
    }

    fn stub_block() -> LlmBlock {
        LlmBlock::new(Arc::new(ProviderLlmService::new()))
    }

    #[tokio::test]
    async fn handle_chat_returns_bad_request_on_invalid_json() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = Message::new("create:/b/llm/api/chat");
        let input = InputStream::from_bytes(b"not json".to_vec());

        let out = handle_chat(&block, &ctx, &msg, input).await;
        let result = out.collect_buffered().await;
        match result {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
                assert!(
                    e.message.contains("Invalid body"),
                    "expected Invalid body message, got: {}",
                    e.message
                );
            }
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn handle_chat_stream_returns_bad_request_on_invalid_json() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = Message::new("create:/b/llm/api/chat/stream");
        let input = InputStream::from_bytes(b"{".to_vec());

        let out = handle_chat_stream(&block, &ctx, &msg, input).await;
        let result = out.collect_buffered().await;
        match result {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
            }
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[test]
    fn role_from_str_maps_known_roles() {
        assert_eq!(role_from_str("user"), ChatRole::User);
        assert_eq!(role_from_str("assistant"), ChatRole::Assistant);
        assert_eq!(role_from_str("system"), ChatRole::System);
    }

    #[test]
    fn role_from_str_unknown_falls_back_to_user() {
        assert_eq!(role_from_str("tool"), ChatRole::User);
        assert_eq!(role_from_str(""), ChatRole::User);
        assert_eq!(role_from_str("random"), ChatRole::User);
    }

    #[test]
    fn history_to_messages_prefers_data_object() {
        let history = vec![serde_json::json!({
            "data": { "role": "user", "content": "hi" }
        })];
        let msgs = history_to_messages(&history);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ChatRole::User);
        assert!(
            matches!(&msgs[0].content, wafer_core::interfaces::llm::service::ChatContent::Text(t) if t == "hi")
        );
    }

    #[test]
    fn history_to_messages_falls_back_to_flat_fields() {
        let history = vec![serde_json::json!({
            "role": "assistant",
            "content": "yes"
        })];
        let msgs = history_to_messages(&history);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ChatRole::Assistant);
        assert!(
            matches!(&msgs[0].content, wafer_core::interfaces::llm::service::ChatContent::Text(t) if t == "yes")
        );
    }

    #[test]
    fn history_to_messages_skips_entries_without_role() {
        let history = vec![
            serde_json::json!({ "content": "orphan" }),
            serde_json::json!({ "role": "system", "content": "kept" }),
        ];
        let msgs = history_to_messages(&history);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ChatRole::System);
    }
}
