//! `BrowserLlmService` — `LlmService` impl that drives WebLLM's MLCEngine
//! from Rust via wasm-bindgen.
//!
//! Phase C scope (this file):
//! - Task 20 — JS surface (`js/webllm-engine.js`) + `WebLlmEngineHandle`
//!   wrapping the opaque MLCEngine `JsValue`.
//! - Task 21 — `BrowserLlmService` scaffolding: holds a lazily-populated
//!   engine handle and reports the fixed WebLLM model catalog from
//!   `ai-bridge.js`. `list_models` + `claims_backend` are fully wired;
//!   `chat_stream`, `load_model`, `status`, `unload_model` return
//!   placeholders that tasks 22 / 23 will replace.
//!
//! The trait impl uses `async_trait::async_trait(?Send)` unconditionally —
//! this crate is `wasm32`-only, so the native `Send` bound would never be
//! checked. `LlmService` itself selects the `(?Send)` form on `wasm32` via
//! `#[cfg_attr]`, which matches.

use std::cell::RefCell;

use futures::stream::BoxStream;
use tokio_util::sync::CancellationToken;
use wafer_core::interfaces::llm::service::{
    ChatChunk, ChatRequest, LlmError, LlmService, LoadProgress, ModelCapabilities, ModelInfo,
    ModelStatus,
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
/// Single-engine — WebLLM keeps one model in memory at a time. Tasks 22/23
/// will use the `engine` slot to back `chat_stream` / `status` / `load_model`
/// / `unload_model`. `RefCell` is safe because `wasm32` is single-threaded.
pub struct BrowserLlmService {
    #[allow(dead_code)] // task 22/23 will read/write this
    engine: RefCell<Option<WebLlmEngineHandle>>,
    available_models: Vec<ModelInfo>,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self {
            engine: RefCell::new(None),
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
        _req: ChatRequest,
        _cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        // Task 22 — placeholder. Returns a single Err so the router surfaces
        // a meaningful message if the wiring is exercised early.
        Box::pin(futures::stream::once(async move {
            Err(LlmError::BackendError(
                "BrowserLlmService::chat_stream not yet implemented (task 22)".into(),
            ))
        }))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.available_models.clone())
    }

    async fn status(&self, _backend_id: &str, _model_id: &str) -> Result<ModelStatus, LlmError> {
        // Task 23 — placeholder.
        Err(LlmError::BackendError(
            "BrowserLlmService::status not yet implemented (task 23)".into(),
        ))
    }

    fn load_model(
        &self,
        _backend_id: &str,
        _model_id: &str,
        _cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        // Task 23 — placeholder.
        Box::pin(futures::stream::once(async move {
            Err(LlmError::BackendError(
                "BrowserLlmService::load_model not yet implemented (task 23)".into(),
            ))
        }))
    }

    async fn unload_model(&self, _backend_id: &str, _model_id: &str) -> Result<(), LlmError> {
        // Task 23 — placeholder.
        Err(LlmError::BackendError(
            "BrowserLlmService::unload_model not yet implemented (task 23)".into(),
        ))
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        backend_id == "webllm"
    }
}
