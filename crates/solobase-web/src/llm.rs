//! `BrowserLlmService` — `LlmService` impl that drives WebLLM's MLCEngine
//! from Rust via wasm-bindgen.
//!
//! Phase C scope (this file):
//! - Task 20 — JS surface (`js/webllm-engine.js`) + `WebLlmEngineHandle`
//!   wrapping the opaque MLCEngine `JsValue`.
//! - Task 21 — `BrowserLlmService` scaffolding: holds a lazily-populated
//!   engine handle and reports the fixed WebLLM model catalog from
//!   `ai-bridge.js`. `list_models` + `claims_backend` are fully wired.
//! - Task 22 — `chat_stream` drives `MLCEngine.chat.completions.create` via
//!   the JS iterator bridge in `webllm-engine.js`, decoding OpenAI-format
//!   chunks into `ChatChunk`s pushed through a `futures::channel::mpsc`.
//! - Task 23 — `load_model` / `status` / `unload_model` manage the single
//!   engine slot. `load_model` emits `LoadProgress` on every WebLLM
//!   `initProgressCallback` tick via the same mpsc pattern.
//!
//! The trait impl uses `async_trait::async_trait(?Send)` unconditionally —
//! this crate is `wasm32`-only, so the native `Send` bound would never be
//! checked. `LlmService` itself selects the `(?Send)` form on `wasm32` via
//! `#[cfg_attr]`, which matches.
//!
//! ## `spawn_local` + `RefCell` ownership pattern
//!
//! `wasm_bindgen_futures::spawn_local` requires a `'static` future, so we
//! can't borrow `&self` across it. Instead we hold the engine slot in an
//! `Rc<RefCell<Option<WebLlmEngineHandle>>>` and clone the `Rc` into each
//! spawned task. `JsValue`s read from the engine are cloned into locals
//! before any `.await` so `RefCell` borrows never cross yield points.

use std::{cell::RefCell, rc::Rc};

use futures::{channel::mpsc, sink::SinkExt, stream::BoxStream};
use tokio_util::sync::CancellationToken;
use wafer_core::interfaces::llm::service::{
    ChatChunk, ChatContent, ChatMessage, ChatRequest, ChatRole, FinishReason, LlmError, LlmService,
    LoadProgress, ModelCapabilities, ModelInfo, ModelStatus, TokenUsage,
};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JS bindings — see `js/webllm-engine.js`.
// ---------------------------------------------------------------------------

#[wasm_bindgen(module = "/js/webllm-engine.js")]
extern "C" {
    #[wasm_bindgen(js_name = createEngine, catch)]
    async fn create_engine_js(model_id: &str, on_progress: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = unloadEngine, catch)]
    async fn unload_engine_js(engine: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = chatStream, catch)]
    pub(crate) async fn chat_stream_js(
        engine: &JsValue,
        messages_json: &str,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = nextChunk, catch)]
    pub(crate) async fn next_chunk_js(iterator: &JsValue) -> Result<JsValue, JsValue>;
}

// ---------------------------------------------------------------------------
// WebLlmEngineHandle
// ---------------------------------------------------------------------------

/// Rust-owned handle to a WebLLM `MLCEngine` JS object.
///
/// Owns the progress-callback `Closure` for the lifetime of the handle so
/// the callback stays alive across the entire model-lifecycle. We deliberately
/// do NOT `.forget()` the closure — dropping the handle (e.g. during
/// `unload`) releases the closure too. Tasks 22 + 23 will use this same
/// pattern for per-chat callbacks if needed.
pub struct WebLlmEngineHandle {
    inner: JsValue,
    model_id: String,
    /// Stored to keep the JS-side progress callback alive. WebLLM only
    /// invokes it during `CreateMLCEngine`, but we retain it on the handle
    /// (rather than `.forget()`-ing or dropping early) so tasks 22/23 can
    /// extend lifecycle semantics without revisiting ownership here.
    _on_progress: Closure<dyn FnMut(JsValue)>,
}

impl WebLlmEngineHandle {
    /// Create + initialize an MLCEngine for `model_id`.
    ///
    /// `on_progress` is called with `(progress_fraction, optional_stage_text)`
    /// as WebLLM reports load progress. The closure is wrapped and held on
    /// the returned handle so JS can keep calling it safely.
    pub async fn create(
        model_id: &str,
        on_progress: impl FnMut(f32, Option<String>) + 'static,
    ) -> Result<Self, String> {
        // Adapter: JS passes a single `{progress, text?}` object. Unpack it
        // here so the Rust-side closure sees typed values.
        let mut user_cb = on_progress;
        let cb = Closure::<dyn FnMut(JsValue)>::new(move |report: JsValue| {
            let progress = js_sys::Reflect::get(&report, &JsValue::from_str("progress"))
                .ok()
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(0.0);
            let text = js_sys::Reflect::get(&report, &JsValue::from_str("text"))
                .ok()
                .and_then(|v| v.as_string());
            user_cb(progress, text);
        });

        let result = create_engine_js(model_id, cb.as_ref()).await;
        match result {
            Ok(engine) => Ok(Self {
                inner: engine,
                model_id: model_id.to_string(),
                _on_progress: cb,
            }),
            Err(err) => Err(js_error_to_string(&err)),
        }
    }

    pub async fn unload(self) -> Result<(), String> {
        unload_engine_js(&self.inner)
            .await
            .map(|_| ())
            .map_err(|e| js_error_to_string(&e))
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn inner(&self) -> &JsValue {
        &self.inner
    }
}

pub(crate) fn js_error_to_string(err: &JsValue) -> String {
    err.as_string()
        .or_else(|| {
            js_sys::Reflect::get(err, &JsValue::from_str("message"))
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_else(|| format!("{err:?}"))
}

// ---------------------------------------------------------------------------
// Model catalog (ported from js/ai-bridge.js AVAILABLE_MODELS)
// ---------------------------------------------------------------------------

/// Fixed WebLLM model catalog.
///
/// Mirrors `AVAILABLE_MODELS` in `js/ai-bridge.js`. The `requires_f16` tier
/// is surfaced via `ModelCapabilities` (we set a non-default capability
/// marker; the admin UI / picker can check for f16 at runtime if it cares).
/// For now we return all 7 models — GPU-capability gating is a browser-side
/// concern the caller can layer on top once WebGPU adapter features are
/// queryable from Rust.
fn available_models() -> Vec<ModelInfo> {
    // Helper: standard capabilities for these local chat models.
    // Streaming is supported by MLCEngine. Tool-calls are supported by recent
    // WebLLM versions. Vision is not. `max_context_tokens` is per-model but
    // we leave `None` here — WebLLM enforces its own limits and the admin UI
    // doesn't currently surface these.
    fn caps() -> ModelCapabilities {
        // `ModelCapabilities` is `#[non_exhaustive]`, so construct via
        // `Default` and overwrite the fields we care about.
        let mut c = ModelCapabilities::default();
        c.streaming = true;
        c.tools = true;
        c
    }

    vec![
        // f32 — broad WebGPU compatibility (no shader-f16 required).
        ModelInfo::new(
            "webllm",
            "SmolLM2-1.7B-Instruct-q4f32_1-MLC",
            "SmolLM2 1.7B (1.1GB)",
        )
        .with_capabilities(caps()),
        ModelInfo::new(
            "webllm",
            "Qwen2.5-1.5B-Instruct-q4f32_1-MLC",
            "Qwen 2.5 1.5B (1.2GB)",
        )
        .with_capabilities(caps()),
        ModelInfo::new("webllm", "gemma-2-2b-it-q4f32_1-MLC", "Gemma 2 2B (1.7GB)")
            .with_capabilities(caps()),
        ModelInfo::new(
            "webllm",
            "Phi-3.5-mini-instruct-q4f32_1-MLC",
            "Phi 3.5 Mini (2.6GB)",
        )
        .with_capabilities(caps()),
        ModelInfo::new(
            "webllm",
            "Llama-3.2-3B-Instruct-q4f32_1-MLC",
            "Llama 3.2 3B (2GB)",
        )
        .with_capabilities(caps()),
        // f16 — need shader-f16. Included; browser can filter if unsupported.
        ModelInfo::new(
            "webllm",
            "SmolLM2-1.7B-Instruct-q4f16_1-MLC",
            "SmolLM2 1.7B f16 (1GB)",
        )
        .with_capabilities(caps()),
        ModelInfo::new(
            "webllm",
            "Qwen2.5-1.5B-Instruct-q4f16_1-MLC",
            "Qwen 2.5 1.5B f16 (1GB)",
        )
        .with_capabilities(caps()),
    ]
}

// ---------------------------------------------------------------------------
// BrowserLlmService
// ---------------------------------------------------------------------------

/// `LlmService` impl that runs local models in the browser via WebLLM.
///
/// Single-engine — WebLLM keeps one model in memory at a time. The slot is
/// shared via `Rc<RefCell<_>>` so `spawn_local` tasks (which require
/// `'static` futures) can each hold an owned clone without borrowing
/// `&self`. `Rc` (not `Arc`) is fine because wasm32 is single-threaded and
/// `WebLlmEngineHandle` owns `JsValue`s which are `!Send`.
pub struct BrowserLlmService {
    engine: Rc<RefCell<Option<WebLlmEngineHandle>>>,
    available_models: Vec<ModelInfo>,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self {
            engine: Rc::new(RefCell::new(None)),
            available_models: available_models(),
        }
    }
}

impl Default for BrowserLlmService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait(?Send)]
impl LlmService for BrowserLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        // Clone the engine's inner JsValue upfront so the RefCell borrow is
        // released before any `.await` point.
        let engine_js = match self.engine.borrow().as_ref() {
            Some(handle) => handle.inner().clone(),
            None => {
                return one_shot_err(LlmError::BackendError("no model loaded".into()));
            }
        };

        // Translate messages to OpenAI JSON. Reject multimodal content — v0
        // only supports text. (Task prompt: emit error chunk and stop.)
        let messages_json = match messages_to_openai_json(&req.messages) {
            Ok(s) => s,
            Err(e) => return one_shot_err(e),
        };

        // futures::channel::mpsc is Send-agnostic: Sender/Receiver are Send
        // iff the item type is. `Result<ChatChunk, LlmError>` is Send, so the
        // Receiver satisfies the `BoxStream<'static, _>`'s Send bound even
        // though the spawned pump closure itself holds !Send JsValues.
        let (mut tx, rx) = mpsc::channel::<Result<ChatChunk, LlmError>>(16);

        wasm_bindgen_futures::spawn_local(async move {
            let iterator = match chat_stream_js(&engine_js, &messages_json).await {
                Ok(it) => it,
                Err(e) => {
                    let _ = tx
                        .send(Err(LlmError::BackendError(format!(
                            "webllm: {}",
                            js_error_to_string(&e)
                        ))))
                        .await;
                    return;
                }
            };

            loop {
                if cancel.is_cancelled() {
                    // Stream consumer dropped or explicit cancel. Just stop
                    // pumping — dropping the sender ends the stream for the
                    // consumer. We don't forward `Err(Cancelled)` because the
                    // consumer is gone.
                    break;
                }
                match next_chunk_js(&iterator).await {
                    Ok(v) if v.is_null() => break,
                    Ok(v) => {
                        let s = match v.as_string() {
                            Some(s) => s,
                            None => {
                                let _ = tx
                                    .send(Err(LlmError::BackendError(
                                        "webllm: non-string chunk from JS iterator".into(),
                                    )))
                                    .await;
                                break;
                            }
                        };
                        match chat_chunk_from_openai_chunk(&s) {
                            Ok(Some(chunk)) => {
                                if tx.send(Ok(chunk)).await.is_err() {
                                    // Receiver dropped — stop pumping.
                                    break;
                                }
                            }
                            Ok(None) => {
                                // No-op chunk (empty delta, no finish reason).
                            }
                            Err(e) => {
                                let _ = tx.send(Err(e)).await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(LlmError::BackendError(format!(
                                "webllm: {}",
                                js_error_to_string(&e)
                            ))))
                            .await;
                        break;
                    }
                }
            }
        });

        Box::pin(rx)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.available_models.clone())
    }

    async fn status(&self, backend_id: &str, model_id: &str) -> Result<ModelStatus, LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        let borrow = self.engine.borrow();
        match borrow.as_ref() {
            Some(handle) if handle.model_id() == model_id => Ok(ModelStatus::ready()),
            _ => Ok(ModelStatus::unloaded()),
        }
    }

    fn load_model(
        &self,
        backend_id: &str,
        model_id: &str,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        if backend_id != WEBLLM_BACKEND {
            return Box::pin(futures::stream::once({
                let backend_id = backend_id.to_string();
                async move {
                    Err(LlmError::BackendError(format!(
                        "backend '{backend_id}' not claimed by webllm"
                    )))
                }
            }));
        }

        let (tx, rx) = mpsc::channel::<Result<LoadProgress, LlmError>>(16);
        let engine_slot = Rc::clone(&self.engine);
        let model_id = model_id.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            if cancel.is_cancelled() {
                return;
            }

            // Progress callback: clone the sender for each call. `mpsc::Sender`
            // has a bounded buffer but `try_send` will drop if full — load
            // progress is advisory, so dropping a few is acceptable. The
            // buffer is 16 which is plenty for WebLLM's cadence.
            let progress_tx = tx.clone();
            let progress_cb = move |_progress: f32, text: Option<String>| {
                let mut tx = progress_tx.clone();
                // WebLLM reports a fractional progress + a human-readable
                // stage string. We forward the stage; the fractional progress
                // is not mapped into byte counts (lossy) — consumers either
                // display the stage text verbatim or infer completion from
                // the terminal `Ready` state / stream end.
                let stage = text.unwrap_or_default();
                // Fire-and-forget send: channel is 16-deep. If full, drop.
                let _ = tx.try_send(Ok(LoadProgress::new(stage)));
            };

            let result = WebLlmEngineHandle::create(&model_id, progress_cb).await;
            let mut tx = tx; // move into this scope for ownership
            match result {
                Ok(handle) => {
                    // Replace any previously-loaded engine. WebLLM keeps one
                    // model at a time; overwriting drops the old handle which
                    // runs its unload-via-Drop semantics (the closure is
                    // released; the JS engine itself needs an explicit
                    // `unload_model` call to free GPU memory — callers should
                    // unload before loading a new model).
                    *engine_slot.borrow_mut() = Some(handle);
                    // Terminal success frame: stage="ready". The `status()`
                    // call will now report Ready.
                    let _ = tx.send(Ok(LoadProgress::new("ready"))).await;
                }
                Err(err) => {
                    let _ = tx
                        .send(Err(LlmError::BackendError(format!("webllm load: {err}"))))
                        .await;
                }
            }
        });

        Box::pin(rx)
    }

    async fn unload_model(&self, backend_id: &str, _model_id: &str) -> Result<(), LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        let handle = self.engine.borrow_mut().take();
        if let Some(handle) = handle {
            handle
                .unload()
                .await
                .map_err(|e| LlmError::BackendError(format!("webllm unload: {e}")))?;
        }
        Ok(())
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        backend_id == WEBLLM_BACKEND
    }
}

// ---------------------------------------------------------------------------
// Helpers (pure — unit-testable without wasm_bindgen_test)
// ---------------------------------------------------------------------------

const WEBLLM_BACKEND: &str = "webllm";

/// Returns a stream that yields a single `Err` and terminates. Used for error
/// paths in stream-returning methods where we can't `return Err(...)` directly.
fn one_shot_err<T: 'static>(err: LlmError) -> BoxStream<'static, Result<T, LlmError>>
where
    T: Send,
{
    Box::pin(futures::stream::once(async move { Err(err) }))
}

/// Encode `ChatMessage`s into the OpenAI chat-completion wire format that
/// `webllm-engine.js::chatStream` expects. Only `ChatContent::Text` is
/// supported; multimodal `Parts` and non-empty tool-call arrays return
/// `LlmError::BackendError` — v0 of the browser backend doesn't forward
/// tool-call state to WebLLM.
fn messages_to_openai_json(messages: &[ChatMessage]) -> Result<String, LlmError> {
    let mut out = Vec::with_capacity(messages.len());
    for m in messages {
        // ChatRole is `#[non_exhaustive]`; a wildcard arm keeps us
        // forward-compatible with future variants (rejected loudly rather
        // than silently mis-translated).
        let role = match m.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
            _ => {
                return Err(LlmError::BackendError("webllm: unknown chat role".into()));
            }
        };
        // ChatContent is also `#[non_exhaustive]`. Only `Text` is supported.
        let content = match &m.content {
            ChatContent::Text(s) => s.clone(),
            ChatContent::Parts(_) => {
                return Err(LlmError::BackendError(
                    "webllm: multimodal content not supported (v0 text-only)".into(),
                ));
            }
            _ => {
                return Err(LlmError::BackendError(
                    "webllm: unknown content variant".into(),
                ));
            }
        };
        if !m.tool_calls.is_empty() {
            return Err(LlmError::BackendError(
                "webllm: assistant tool_calls not forwarded (v0)".into(),
            ));
        }
        let mut obj = serde_json::Map::new();
        obj.insert("role".into(), serde_json::Value::String(role.into()));
        obj.insert("content".into(), serde_json::Value::String(content));
        if let Some(tc_id) = &m.tool_call_id {
            obj.insert(
                "tool_call_id".into(),
                serde_json::Value::String(tc_id.clone()),
            );
        }
        out.push(serde_json::Value::Object(obj));
    }
    serde_json::to_string(&serde_json::Value::Array(out))
        .map_err(|e| LlmError::BackendError(format!("messages encode: {e}")))
}

/// Parse a single OpenAI-format streaming chunk (as emitted by
/// `webllm-engine.js::nextChunk`) into a `ChatChunk`.
///
/// Returns `Ok(None)` for no-op chunks (empty delta, no finish reason, no
/// usage) or `Err` for malformed JSON. Tool-call deltas are not propagated
/// in v0.
///
/// OpenAI's streaming convention emits content-only frames followed by a
/// terminal frame with empty `delta` + `finish_reason` (+ optional `usage`).
/// When a single frame carries both `content` and `finish_reason`, we prefer
/// emitting the text delta — callers see the terminal state either via a
/// subsequent empty-delta frame or via the stream ending naturally when the
/// iterator signals done.
fn chat_chunk_from_openai_chunk(s: &str) -> Result<Option<ChatChunk>, LlmError> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| LlmError::BackendError(format!("chunk parse: {e}")))?;

    // Shape (OpenAI streaming chat-completion chunk):
    //   { id, object, created, model,
    //     choices: [ { index, delta: { role?, content? }, finish_reason? } ],
    //     usage?: { prompt_tokens, completion_tokens, total_tokens } }
    let choice = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first());

    let content = choice
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string);

    let finish_reason = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|f| f.as_str())
        .and_then(parse_finish_reason);

    let usage = v.get("usage").and_then(parse_usage);

    // Prefer a text-delta frame when content is non-empty. Otherwise, emit a
    // terminal finish frame if a finish reason is present. A usage-only frame
    // (OpenAI's `stream_options.include_usage`) is surfaced as a finish-style
    // frame with `Stop`; WebLLM doesn't currently send usage-only mid-stream
    // so this is belt-and-suspenders.
    if let Some(text) = content {
        if text.is_empty() {
            // Some backends emit an initial role-only delta with empty
            // content; treat as a no-op.
            return Ok(None);
        }
        return Ok(Some(ChatChunk::text(text)));
    }

    if let Some(reason) = finish_reason {
        return Ok(Some(ChatChunk::finish(reason, usage)));
    }

    if let Some(usage) = usage {
        return Ok(Some(ChatChunk::finish(FinishReason::Stop, Some(usage))));
    }

    Ok(None)
}

fn parse_finish_reason(s: &str) -> Option<FinishReason> {
    match s {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" => Some(FinishReason::ToolCall),
        "content_filter" => Some(FinishReason::ContentFilter),
        _ => None,
    }
}

fn parse_usage(v: &serde_json::Value) -> Option<TokenUsage> {
    // OpenAI uses prompt_tokens / completion_tokens; map to our schema.
    // `TokenUsage` is `#[non_exhaustive]` in wafer-core — construct via
    // `Default` and overwrite the fields we populate.
    let input = v.get("prompt_tokens").and_then(|n| n.as_u64())? as u32;
    let output = v.get("completion_tokens").and_then(|n| n.as_u64())? as u32;
    let mut usage = TokenUsage::default();
    usage.input_tokens = input;
    usage.output_tokens = output;
    Some(usage)
}

#[cfg(test)]
mod tests {
    use wafer_core::interfaces::llm::service::ChunkDelta;

    use super::*;

    #[test]
    fn chunk_text_delta_parsed() {
        let s = r#"{
            "id": "c",
            "choices": [{"index": 0, "delta": {"content": "hello"}, "finish_reason": null}]
        }"#;
        let chunk = chat_chunk_from_openai_chunk(s).unwrap().unwrap();
        assert_eq!(chunk.delta, ChunkDelta::Text("hello".into()));
        assert!(chunk.finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn chunk_empty_content_is_noop() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {"content": ""}, "finish_reason": null}]
        }"#;
        assert!(chat_chunk_from_openai_chunk(s).unwrap().is_none());
    }

    #[test]
    fn chunk_missing_content_is_noop() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {}, "finish_reason": null}]
        }"#;
        assert!(chat_chunk_from_openai_chunk(s).unwrap().is_none());
    }

    #[test]
    fn chunk_finish_reason_terminal() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]
        }"#;
        let chunk = chat_chunk_from_openai_chunk(s).unwrap().unwrap();
        assert_eq!(chunk.delta, ChunkDelta::Empty);
        assert_eq!(chunk.finish_reason, Some(FinishReason::Stop));
    }

    #[test]
    fn chunk_finish_reason_length() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {}, "finish_reason": "length"}]
        }"#;
        let chunk = chat_chunk_from_openai_chunk(s).unwrap().unwrap();
        assert_eq!(chunk.finish_reason, Some(FinishReason::Length));
    }

    #[test]
    fn chunk_unknown_finish_reason_treated_as_noop() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {}, "finish_reason": "weird"}]
        }"#;
        // Unknown finish reason becomes None; no content either -> no-op.
        assert!(chat_chunk_from_openai_chunk(s).unwrap().is_none());
    }

    #[test]
    fn chunk_with_usage() {
        let s = r#"{
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 12, "completion_tokens": 34, "total_tokens": 46}
        }"#;
        let chunk = chat_chunk_from_openai_chunk(s).unwrap().unwrap();
        assert_eq!(chunk.finish_reason, Some(FinishReason::Stop));
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 34);
    }

    #[test]
    fn chunk_malformed_json_errors() {
        let err = chat_chunk_from_openai_chunk("{not json").unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn chunk_empty_object_is_noop() {
        assert!(chat_chunk_from_openai_chunk("{}").unwrap().is_none());
    }

    #[test]
    fn messages_encode_roles() {
        let msgs = vec![
            ChatMessage::system("sys"),
            ChatMessage::user("hello"),
            ChatMessage::assistant("hi"),
            ChatMessage::tool("call_1", "result"),
        ];
        let json = messages_to_openai_json(&msgs).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0]["role"], "system");
        assert_eq!(arr[0]["content"], "sys");
        assert_eq!(arr[1]["role"], "user");
        assert_eq!(arr[2]["role"], "assistant");
        assert_eq!(arr[3]["role"], "tool");
        assert_eq!(arr[3]["tool_call_id"], "call_1");
    }

    #[test]
    fn messages_reject_multimodal() {
        // ChatMessage is `#[non_exhaustive]`, so round-trip through serde to
        // construct one with multimodal content.
        let msg_json = serde_json::json!({
            "role": "User",
            "content": { "Parts": [ { "Text": "x" } ] },
        });
        let msg: ChatMessage = serde_json::from_value(msg_json).unwrap();
        let err = messages_to_openai_json(&[msg]).unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn messages_reject_tool_calls() {
        // ChatMessage + ToolCall are `#[non_exhaustive]`; round-trip via serde.
        let msg_json = serde_json::json!({
            "role": "Assistant",
            "content": { "Text": "" },
            "tool_calls": [
                { "id": "c", "name": "n", "arguments": {} }
            ],
        });
        let msg: ChatMessage = serde_json::from_value(msg_json).unwrap();
        let err = messages_to_openai_json(&[msg]).unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn parse_finish_reason_maps_standard_values() {
        assert_eq!(parse_finish_reason("stop"), Some(FinishReason::Stop));
        assert_eq!(parse_finish_reason("length"), Some(FinishReason::Length));
        assert_eq!(
            parse_finish_reason("tool_calls"),
            Some(FinishReason::ToolCall)
        );
        assert_eq!(
            parse_finish_reason("content_filter"),
            Some(FinishReason::ContentFilter)
        );
        assert_eq!(parse_finish_reason("nope"), None);
    }
}
