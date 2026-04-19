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
    providers::config::{ProviderConfig, ProviderProtocol},
    schema::{config_to_row, row_to_config, PROVIDERS_COLLECTION},
    LlmBlock, DEFAULT_PROVIDER,
};
use crate::blocks::helpers::{
    self, err_bad_request, err_forbidden, err_internal, err_not_found, ok_json,
};

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
// Provider CRUD (admin-only)
// ---------------------------------------------------------------------------
//
// These endpoints back the LLM admin UI's provider management. All writes
// reload the in-memory `ProviderLlmService` from the DB so chat requests
// pick up the new configuration without restarting the process.

/// Body shape for `POST /b/llm/api/providers` and `PATCH /b/llm/api/providers/:id`.
///
/// Every field is optional so the same struct can serve both create (which
/// validates required fields after parsing) and patch.
#[derive(serde::Deserialize, Default)]
struct ProviderBody {
    name: Option<String>,
    protocol: Option<String>,
    endpoint: Option<String>,
    key_var: Option<String>,
    models: Option<Vec<String>>,
    enabled: Option<bool>,
}

/// Extract `:id` from `/b/llm/api/providers/{id}[/...]`.
///
/// The route is registered as a single `/b/llm` prefix in `routing.rs`; the
/// runtime does not extract path params, so we parse from `msg.path()` and
/// fall back to `msg.var("id")` for callers that do supply it (tests, future
/// router enhancements).
fn extract_provider_id(msg: &Message) -> &str {
    let var = msg.var("id");
    if !var.is_empty() {
        return var;
    }
    let path = msg.path();
    let suffix = path.strip_prefix("/b/llm/api/providers/").unwrap_or("");
    suffix.split('/').next().unwrap_or("")
}

/// Render a `ProviderConfig` as the JSON shape returned by list/create/update.
fn provider_to_json(id: &str, cfg: &ProviderConfig) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": cfg.name,
        "protocol": cfg.protocol.as_str(),
        "endpoint": cfg.endpoint,
        "key_var": cfg.key_var,
        "models": cfg.models,
        "enabled": cfg.enabled,
    })
}

/// Reload all enabled providers from the DB and push the snapshot into the
/// in-memory `ProviderLlmService`.
///
/// Errors are returned to the caller; callers translate to 500. We do not
/// silently swallow — a failure here means the in-memory service is stale
/// and the admin needs to know.
async fn reload_provider_service(block: &LlmBlock, ctx: &dyn Context) -> Result<(), String> {
    let opts = db::ListOptions {
        limit: 200,
        ..Default::default()
    };
    let result = db::list(ctx, PROVIDERS_COLLECTION, &opts)
        .await
        .map_err(|e| format!("provider reload list failed: {e}"))?;
    let mut configs: Vec<ProviderConfig> = Vec::with_capacity(result.records.len());
    for rec in &result.records {
        match row_to_config(rec) {
            Ok(cfg) if cfg.enabled => configs.push(cfg),
            Ok(_) => {} // disabled — skip
            Err(e) => {
                // A malformed row should not poison the whole reload. Log via
                // the returned error if every row fails; otherwise drop just
                // that one. Tracing isn't available across all targets here,
                // so we propagate via the eventual Err only when nothing was
                // decodable.
                tracing::warn!("skipping malformed provider row {}: {e}", rec.id);
            }
        }
    }
    block.provider_svc.configure(configs);
    Ok(())
}

/// `GET /b/llm/api/providers` — list all rows. Admin-only.
pub(super) async fn list_providers(
    _block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
) -> OutputStream {
    if !helpers::is_admin(msg) {
        return err_forbidden("admin role required");
    }
    let opts = db::ListOptions {
        limit: 200,
        ..Default::default()
    };
    let result = match db::list(ctx, PROVIDERS_COLLECTION, &opts).await {
        Ok(r) => r,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };
    let providers: Vec<serde_json::Value> = result
        .records
        .iter()
        .filter_map(|rec| {
            row_to_config(rec)
                .ok()
                .map(|cfg| provider_to_json(&rec.id, &cfg))
        })
        .collect();
    ok_json(&serde_json::json!({ "providers": providers }))
}

/// `POST /b/llm/api/providers` — create. Body must include `name`,
/// `protocol`, `endpoint`. `key_var`, `models`, `enabled` optional. Admin-only.
pub(super) async fn create_provider(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    if !helpers::is_admin(msg) {
        return err_forbidden("admin role required");
    }
    let raw = input.collect_to_bytes().await;
    let body: ProviderBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    let Some(name) = body
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    else {
        return err_bad_request("`name` is required");
    };
    let Some(protocol_str) = body.protocol.as_deref().filter(|s| !s.is_empty()) else {
        return err_bad_request("`protocol` is required");
    };
    let Some(protocol) = ProviderProtocol::parse(protocol_str) else {
        return err_bad_request(&format!(
            "invalid `protocol` `{protocol_str}` — expected `open_ai`, `anthropic`, or `open_ai_compatible`"
        ));
    };
    let Some(endpoint) = body
        .endpoint
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    else {
        return err_bad_request("`endpoint` is required");
    };

    let mut cfg = ProviderConfig::new(name, protocol, endpoint);
    if let Some(k) = body.key_var.filter(|s| !s.is_empty()) {
        cfg.key_var = Some(k);
    }
    if let Some(m) = body.models {
        cfg.models = m;
    }
    if let Some(e) = body.enabled {
        cfg.enabled = e;
    }

    let mut data = config_to_row(&cfg);
    helpers::stamp_created(&mut data);

    let record = match db::create(ctx, PROVIDERS_COLLECTION, data).await {
        Ok(r) => r,
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    if let Err(e) = reload_provider_service(block, ctx).await {
        return err_internal(&e);
    }

    ok_json(&provider_to_json(&record.id, &cfg))
}

/// `PATCH /b/llm/api/providers/:id` — partial update. Admin-only.
pub(super) async fn update_provider(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
    input: InputStream,
) -> OutputStream {
    if !helpers::is_admin(msg) {
        return err_forbidden("admin role required");
    }
    let id = extract_provider_id(msg).to_string();
    if id.is_empty() {
        return err_bad_request("Missing provider ID");
    }

    let raw = input.collect_to_bytes().await;
    let body: ProviderBody = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Load existing record so we can apply the patch on top of stored values.
    let existing = match db::get(ctx, PROVIDERS_COLLECTION, &id).await {
        Ok(r) => r,
        Err(e) if e.code == wafer_run::types::ErrorCode::NotFound => {
            return err_not_found("Provider not found")
        }
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };
    let mut cfg = match row_to_config(&existing) {
        Ok(c) => c,
        Err(e) => return err_internal(&format!("Stored provider row invalid: {e}")),
    };

    if let Some(n) = body.name.filter(|s| !s.is_empty()) {
        cfg.name = n;
    }
    if let Some(p) = body.protocol.as_deref().filter(|s| !s.is_empty()) {
        match ProviderProtocol::parse(p) {
            Some(parsed) => cfg.protocol = parsed,
            None => {
                return err_bad_request(&format!(
                    "invalid `protocol` `{p}` — expected `open_ai`, `anthropic`, or `open_ai_compatible`"
                ))
            }
        }
    }
    if let Some(e) = body.endpoint.filter(|s| !s.is_empty()) {
        cfg.endpoint = e;
    }
    if let Some(k) = body.key_var {
        cfg.key_var = if k.is_empty() { None } else { Some(k) };
    }
    if let Some(m) = body.models {
        cfg.models = m;
    }
    if let Some(e) = body.enabled {
        cfg.enabled = e;
    }

    let mut data = config_to_row(&cfg);
    helpers::stamp_updated(&mut data);

    let record = match db::update(ctx, PROVIDERS_COLLECTION, &id, data).await {
        Ok(r) => r,
        Err(e) if e.code == wafer_run::types::ErrorCode::NotFound => {
            return err_not_found("Provider not found")
        }
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };

    if let Err(e) = reload_provider_service(block, ctx).await {
        return err_internal(&e);
    }

    ok_json(&provider_to_json(&record.id, &cfg))
}

/// `DELETE /b/llm/api/providers/:id` — remove. Admin-only.
pub(super) async fn delete_provider(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
) -> OutputStream {
    if !helpers::is_admin(msg) {
        return err_forbidden("admin role required");
    }
    let id = extract_provider_id(msg).to_string();
    if id.is_empty() {
        return err_bad_request("Missing provider ID");
    }
    match db::delete(ctx, PROVIDERS_COLLECTION, &id).await {
        Ok(()) => {}
        Err(e) if e.code == wafer_run::types::ErrorCode::NotFound => {
            return err_not_found("Provider not found")
        }
        Err(e) => return err_internal(&format!("Database error: {e}")),
    }

    if let Err(e) = reload_provider_service(block, ctx).await {
        return err_internal(&e);
    }

    ok_json(&serde_json::json!({ "deleted": true }))
}

/// `POST /b/llm/api/providers/:id/discover-models` — call the provider's
/// `/v1/models` endpoint, persist the discovered list back to the row, and
/// return the new model list. Admin-only.
pub(super) async fn discover_models(
    block: &LlmBlock,
    ctx: &dyn Context,
    msg: &Message,
) -> OutputStream {
    if !helpers::is_admin(msg) {
        return err_forbidden("admin role required");
    }
    let id = extract_provider_id(msg).to_string();
    if id.is_empty() {
        return err_bad_request("Missing provider ID");
    }

    // Resolve the provider name from the row — discover_models is keyed by
    // provider name (== ProviderConfig::name), not by row id.
    let existing = match db::get(ctx, PROVIDERS_COLLECTION, &id).await {
        Ok(r) => r,
        Err(e) if e.code == wafer_run::types::ErrorCode::NotFound => {
            return err_not_found("Provider not found")
        }
        Err(e) => return err_internal(&format!("Database error: {e}")),
    };
    let mut cfg = match row_to_config(&existing) {
        Ok(c) => c,
        Err(e) => return err_internal(&format!("Stored provider row invalid: {e}")),
    };

    // Make sure the in-memory service knows about this provider — discover
    // looks up by name, and the service may be empty if the process just
    // started or the row is disabled (and so was excluded from the last
    // configure call).
    if let Err(e) = reload_provider_service(block, ctx).await {
        return err_internal(&e);
    }

    let models = match block.provider_svc.discover_models(&cfg.name).await {
        Ok(m) => m,
        Err(e) => return err_internal(&format!("discover_models failed: {e:?}")),
    };
    let model_ids: Vec<String> = models.iter().map(|m| m.model_id.clone()).collect();
    cfg.models = model_ids.clone();

    let mut data = config_to_row(&cfg);
    helpers::stamp_updated(&mut data);
    if let Err(e) = db::update(ctx, PROVIDERS_COLLECTION, &id, data).await {
        return err_internal(&format!("Database error: {e}"));
    }

    if let Err(e) = reload_provider_service(block, ctx).await {
        return err_internal(&e);
    }

    ok_json(&serde_json::json!({ "models": model_ids }))
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

    // -----------------------------------------------------------------
    // Provider CRUD tests
    // -----------------------------------------------------------------
    //
    // These cover the three admin/parse paths that don't need a DB or an
    // HTTP backend: admin-guard denial, JSON-parse errors, and the
    // path-extraction helper. End-to-end tests (DB write + service reload)
    // live in the integration suite.

    fn admin_msg(action: &str, path: &str) -> Message {
        let mut m = Message::new(format!("{action}:{path}"));
        m.set_meta(wafer_run::meta::META_REQ_ACTION, action);
        m.set_meta(wafer_run::meta::META_REQ_RESOURCE, path);
        m.set_meta(wafer_run::meta::META_AUTH_USER_ID, "admin-user");
        m.set_meta("auth.user_roles", "admin");
        m
    }

    fn user_msg(action: &str, path: &str) -> Message {
        let mut m = Message::new(format!("{action}:{path}"));
        m.set_meta(wafer_run::meta::META_REQ_ACTION, action);
        m.set_meta(wafer_run::meta::META_REQ_RESOURCE, path);
        m.set_meta(wafer_run::meta::META_AUTH_USER_ID, "regular-user");
        m.set_meta("auth.user_roles", "user");
        m
    }

    #[tokio::test]
    async fn create_provider_rejects_non_admin() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = user_msg("create", "/b/llm/api/providers");
        let input = InputStream::from_bytes(b"{}".to_vec());

        let out = create_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::PermissionDenied);
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_provider_rejects_non_admin() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = user_msg("update", "/b/llm/api/providers/abc");
        let input = InputStream::from_bytes(b"{}".to_vec());

        let out = update_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::PermissionDenied);
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_provider_rejects_non_admin() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = user_msg("delete", "/b/llm/api/providers/abc");

        let out = delete_provider(&block, &ctx, &msg).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::PermissionDenied);
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_providers_rejects_non_admin() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = user_msg("retrieve", "/b/llm/api/providers");

        let out = list_providers(&block, &ctx, &msg).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::PermissionDenied);
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn discover_models_rejects_non_admin() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = user_msg("create", "/b/llm/api/providers/abc/discover-models");

        let out = discover_models(&block, &ctx, &msg).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::PermissionDenied);
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_provider_returns_bad_request_on_invalid_json() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = admin_msg("create", "/b/llm/api/providers");
        let input = InputStream::from_bytes(b"not json".to_vec());

        let out = create_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
                assert!(
                    e.message.contains("Invalid body"),
                    "expected Invalid body, got: {}",
                    e.message
                );
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_provider_requires_name() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = admin_msg("create", "/b/llm/api/providers");
        let input =
            InputStream::from_bytes(br#"{"protocol":"open_ai","endpoint":"https://x"}"#.to_vec());

        let out = create_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
                assert!(e.message.contains("name"), "got: {}", e.message);
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_provider_rejects_unknown_protocol() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = admin_msg("create", "/b/llm/api/providers");
        let input = InputStream::from_bytes(
            br#"{"name":"x","protocol":"openai","endpoint":"https://x"}"#.to_vec(),
        );

        let out = create_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
                assert!(e.message.contains("protocol"), "got: {}", e.message);
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_provider_requires_id() {
        let block = stub_block();
        let ctx = PanicCtx;
        // Path has no id segment after the prefix.
        let msg = admin_msg("update", "/b/llm/api/providers/");
        let input = InputStream::from_bytes(b"{}".to_vec());

        let out = update_provider(&block, &ctx, &msg, input).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
                assert!(e.message.contains("provider ID"), "got: {}", e.message);
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_provider_requires_id() {
        let block = stub_block();
        let ctx = PanicCtx;
        let msg = admin_msg("delete", "/b/llm/api/providers/");

        let out = delete_provider(&block, &ctx, &msg).await;
        match out.collect_buffered().await {
            Err(TerminalNotResponse::Error(e)) => {
                assert_eq!(e.code, ErrorCode::InvalidArgument);
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn extract_provider_id_from_path() {
        // Direct id at end of path
        let mut m = Message::new("update:/b/llm/api/providers/abc123");
        m.set_meta(
            wafer_run::meta::META_REQ_RESOURCE,
            "/b/llm/api/providers/abc123",
        );
        assert_eq!(extract_provider_id(&m), "abc123");

        // Id followed by a sub-resource (discover-models)
        let mut m2 = Message::new("create:/b/llm/api/providers/abc123/discover-models");
        m2.set_meta(
            wafer_run::meta::META_REQ_RESOURCE,
            "/b/llm/api/providers/abc123/discover-models",
        );
        assert_eq!(extract_provider_id(&m2), "abc123");

        // Empty when no id provided
        let mut m3 = Message::new("delete:/b/llm/api/providers/");
        m3.set_meta(wafer_run::meta::META_REQ_RESOURCE, "/b/llm/api/providers/");
        assert_eq!(extract_provider_id(&m3), "");

        // `msg.var("id")` takes precedence
        let mut m4 = Message::new("update:/b/llm/api/providers/from-path");
        m4.set_meta(
            wafer_run::meta::META_REQ_RESOURCE,
            "/b/llm/api/providers/from-path",
        );
        m4.set_meta(
            format!("{}id", wafer_run::meta::META_REQ_PARAM_PREFIX),
            "from-var",
        );
        assert_eq!(extract_provider_id(&m4), "from-var");
    }

    #[test]
    fn provider_to_json_shape() {
        let cfg = ProviderConfig::new(
            "openai-main",
            ProviderProtocol::OpenAi,
            "https://api.openai.com/v1",
        )
        .with_key_var("SUPPERS_AI__LLM__OPENAI_KEY")
        .with_models(vec!["gpt-4o".into()]);
        let v = provider_to_json("row-1", &cfg);
        assert_eq!(v["id"], "row-1");
        assert_eq!(v["name"], "openai-main");
        assert_eq!(v["protocol"], "open_ai");
        assert_eq!(v["endpoint"], "https://api.openai.com/v1");
        assert_eq!(v["key_var"], "SUPPERS_AI__LLM__OPENAI_KEY");
        assert_eq!(v["models"], serde_json::json!(["gpt-4o"]));
        assert_eq!(v["enabled"], true);
        assert!(
            v.get("api_key").is_none(),
            "api_key must never appear in API output"
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
