# LLM Service Refactor

**Status:** Design
**Date:** 2026-04-15
**Scope:** wafer-core (new `LlmService` trait + `wafer-run/llm` service block), solobase-core (new consolidated `suppers-ai/llm` feature block + `ProviderLlmService` impl), solobase-web (new `BrowserLlmService` impl, removal of ai-bridge.js)
**Depends on:** [Streaming Protocol Design](../../../../wafer-run/docs/specs/2026-04-15-streaming-protocol-design.md) — must land first

## Summary

Consolidate the current three-block LLM setup (`suppers-ai/llm`, `suppers-ai/provider-llm`, `suppers-ai/local-llm`) into a single typed-service architecture mirroring `DatabaseService` / `StorageService`:

- New `LlmService` trait in `wafer-core/src/interfaces/llm/service.rs` — streaming-native, future-extensible, backend-agnostic.
- New `wafer-run/llm` service block in wafer-core that wraps an `Arc<dyn LlmService>`. Backend-agnostic; the wrapped service is typically a `MultiBackendLlmService` router that dispatches by `backend_id` across multiple concrete impls.
- New `suppers-ai/llm` feature block in solobase-core — owns all LLM admin UI, thread management, chat API endpoints, and provider-CRUD state. Routes inference through the service block.
- New `ProviderLlmService` impl in solobase-core covering OpenAI native API + Anthropic native API + OpenAI-compatible protocol variant. The OpenAI-compatible variant covers all local servers (Ollama, llama-server, LM Studio, vLLM, etc.) and all OpenAI-compatible remotes (Azure OpenAI, Together, Groq, OpenRouter, Mistral API) without additional impls.
- New `BrowserLlmService` impl in solobase-web — proper Rust wrapper around WebLLM via wasm-bindgen interop.
- Deleted: `suppers-ai/provider-llm` block, `suppers-ai/local-llm` block, `llm_backend.rs`, `ai-bridge.js`.

No separate `NativeLlmService`. Local inference is handled by user-run Ollama/llama-server/etc. configured as a local provider entry, reusing the OpenAI-compatible protocol variant in `ProviderLlmService`. This avoids C++ toolchain integration in Solobase and matches how every mainstream local-LLM product ships.

The refactor is enabled by Spec 1's streaming-native protocol: tokens flow end-to-end from backend to browser as real streams, with cancellation, backpressure, and error propagation working uniformly. The ai-bridge.js postMessage hack that today bridges service-worker ↔ main-page for WebLLM token delivery goes away entirely; browser streaming becomes indistinguishable from native.

## Motivation

The three current blocks have several problems:

- **Duplicated JSON interfaces.** `suppers-ai/llm` dispatches chat requests to `provider-llm` / `local-llm` via `ctx.call_block` with hand-designed JSON message shapes. Each backend block reimplements message decoding, validation, model-listing, etc.
- **Untyped contract.** `llm_backend.rs` documents the wire shape as Rust types but no one implements the `LlmBackend` trait — it's a phantom. Actual behavior lives in ad-hoc JSON munging in each block's `handle()`.
- **Browser streaming bypasses the protocol.** The `local-llm` block is a stub that returns 501 on native and relies on the service worker intercepting requests and forwarding to `ai-bridge.js` via `postMessage`. Tokens stream directly from WebLLM to the DOM; the Rust layer never sees them. This means browser LLM UX can't compose with other Solobase features that might want to observe, modify, log, or rate-limit LLM traffic.
- **No ergonomics for adding backends.** Want to add Gemini? Groq? A local llama-server? Today each requires designing a new block-to-block JSON protocol. The contract isn't reusable.
- **Provider CRUD is in the wrong place.** `provider-llm` owns the admin UI for providers; `llm` owns the chat UI. Users navigate between them for related settings.

The foundational protocol change (Spec 1) gives us streaming primitives. This spec uses them to build the consolidated LLM architecture Solobase needs for agent-style features, multi-provider configuration, and first-class local-LLM support.

## Architectural Shape

```
┌─────────────────────────────────────────────────────────────────┐
│  wafer-core::interfaces::llm                                    │
│                                                                  │
│  trait LlmService                                               │
│      async fn chat_stream(...) -> BoxStream<ChatChunk>          │
│      async fn list_models(...) -> Vec<ModelInfo>                │
│      async fn status(...) -> ModelStatus                        │
│      fn load_model(...) -> BoxStream<LoadProgress>              │
│      async fn unload_model(...) -> Result<(), LlmError>         │
└─────────────────────────────────────────────────────────────────┘
                   ▲            ▲            ▲
                   │ impl       │ impl       │ impl
         ┌─────────┴──────┐  ┌──┴───────────┐  ┌──┴───────────────┐
         │ MultiBackend   │  │ ProviderLlm  │  │ BrowserLlm       │
         │ LlmService     │  │ Service      │  │ Service          │
         │ (router,       │  │              │  │                  │
         │  wafer-core)   │  │ OpenAI +     │  │ WebLLM via       │
         │                │  │ Anthropic +  │  │ wasm-bindgen     │
         │ holds a        │  │ OpenAI-      │  │ (solobase-web)   │
         │ HashMap<str,   │  │ compatible   │  │                  │
         │ Arc<dyn Llm>>  │  │ protocol     │  │                  │
         │                │  │              │  │                  │
         └────────────────┘  │ (solobase-   │  └──────────────────┘
                   ▲         │  core)       │
                   │         └──────────────┘
                   │                  ▲
                   │ registered into  │ configured by feature block
                   │                  │
┌─────────────────────────────────────────────────────────────────┐
│  wafer-core::service_blocks::llm                                │
│                                                                  │
│  pub struct LlmBlock { service: Arc<dyn LlmService> }           │
│  impl Block for LlmBlock                                        │
│  registered as "wafer-run/llm"                                  │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ ctx.call_block("wafer-run/llm", ...)
                              │   [streaming protocol, Spec 1]
┌─────────────────────────────────────────────────────────────────┐
│  solobase-core/src/blocks/llm  (the feature block)              │
│                                                                  │
│  registered as "suppers-ai/llm"                                 │
│                                                                  │
│  Owns:                                                          │
│   • Admin chat UI                                               │
│   • Admin provider-CRUD UI                                      │
│   • Thread / message persistence (suppers-ai/messages integration) │
│   • Chat HTTP endpoints (buffered + SSE streaming)              │
│   • Provider config persistence (suppers_ai__llm__providers)    │
│   • Startup wiring: loads providers from DB, calls              │
│     ProviderLlmService::configure(); registers all impls into   │
│     MultiBackendLlmService; registers router as wafer-run/llm   │
└─────────────────────────────────────────────────────────────────┘
```

## The `LlmService` Trait

Lives in `wafer-core/src/interfaces/llm/service.rs` alongside `database::service` and `storage::service`.

### Types

All types are `#[non_exhaustive]` for forward compatibility.

```rust
// ------ Request side ------

#[non_exhaustive]
pub struct ChatRequest {
    pub backend_id: String,                  // router key: "openai", "anthropic", "webllm", or a local provider name
    pub model: String,                       // model id within the backend
    pub messages: Vec<ChatMessage>,
    pub params: ChatParams,
    pub tools: Vec<ToolDefinition>,
    pub extra: serde_json::Value,            // backend-specific parameter overflow
}

#[non_exhaustive]
#[derive(Default)]
pub struct ChatParams {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub seed: Option<u64>,
    pub response_format: Option<ResponseFormat>,
}

#[non_exhaustive]
pub enum ResponseFormat {
    Text,
    Json,
    JsonSchema(serde_json::Value),
}

#[non_exhaustive]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: ChatContent,
    pub tool_call_id: Option<String>,        // set on Role::Tool messages
    pub tool_calls: Vec<ToolCall>,           // set on Role::Assistant messages invoking tools
}

#[non_exhaustive]
pub enum ChatRole { System, User, Assistant, Tool }

#[non_exhaustive]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ContentPart>),                 // multimodal; impls that don't support return NotSupported
}

#[non_exhaustive]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String, detail: Option<String> },
    ImageBytes { bytes: Vec<u8>, mime_type: String },
}

#[non_exhaustive]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,       // JSON Schema
}

#[non_exhaustive]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

// ------ Response side ------

#[non_exhaustive]
pub struct ChatChunk {
    pub delta: ChunkDelta,
    pub finish_reason: Option<FinishReason>, // present on terminal chunk(s)
    pub usage: Option<TokenUsage>,           // typically on terminal chunk
}

#[non_exhaustive]
pub enum ChunkDelta {
    Text(String),
    ToolCallStart { id: String, name: String },
    ToolCallArguments { id: String, arguments_delta: String },   // partial JSON
    ToolCallComplete { id: String },
    Empty,                                    // meta-only chunks (heartbeats, usage updates)
}

#[non_exhaustive]
pub enum FinishReason { Stop, Length, ToolCall, ContentFilter, Error }

#[non_exhaustive]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}

// ------ Model management ------

#[non_exhaustive]
pub struct ModelInfo {
    pub backend_id: String,
    pub model_id: String,
    pub display_name: String,
    pub capabilities: ModelCapabilities,
}

#[non_exhaustive]
#[derive(Default)]
pub struct ModelCapabilities {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub json_mode: bool,
    pub max_context_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[non_exhaustive]
pub struct ModelStatus {
    pub state: ModelState,
    pub progress: Option<f32>,                // 0.0–1.0, set during Loading
}

#[non_exhaustive]
pub enum ModelState {
    Ready,                                    // local: loaded; remote: reachable
    Loading,
    Unloaded,                                 // local only — weights not in memory
    Error { message: String },
}

#[non_exhaustive]
pub struct LoadProgress {
    pub stage: String,                        // e.g. "downloading", "initializing", "compiling"
    pub bytes_downloaded: Option<u64>,
    pub bytes_total: Option<u64>,
}

// ------ Error type ------

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("not supported by this backend")]
    NotSupported,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("backend error: {0}")]
    BackendError(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("rate limited")]
    RateLimited,
    #[error("unauthorized")]
    Unauthorized,
    #[error("network error: {0}")]
    Network(String),
    #[error("cancelled")]
    Cancelled,
}
```

### Trait definition

```rust
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait LlmService: MaybeSend + MaybeSync + 'static {
    /// Stream of chat chunks for the given request.
    /// The returned stream may yield Err items mid-stream; consumers decide
    /// how to handle partial output (cf. Spec 1 error semantics).
    /// The cancel token is checked by the impl during long awaits.
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>>;

    /// All models exposed by this service across all its backends.
    /// The router aggregates from all registered impls.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError>;

    /// Current status for a (backend, model) pair.
    /// Remote backends return ModelState::Ready (or Error for unreachable).
    async fn status(
        &self,
        backend_id: &str,
        model_id: &str,
    ) -> Result<ModelStatus, LlmError>;

    /// Stream of load-progress events. Terminates with the final status or an error.
    /// Default impl: single-element stream yielding NotSupported — providers override for locals.
    fn load_model(
        &self,
        _backend_id: &str,
        _model_id: &str,
        _cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        Box::pin(futures::stream::once(async {
            Err(LlmError::NotSupported)
        }))
    }

    /// Unload a locally-loaded model. Default impl: NotSupported.
    async fn unload_model(
        &self,
        _backend_id: &str,
        _model_id: &str,
    ) -> Result<(), LlmError> {
        Err(LlmError::NotSupported)
    }

    /// Synchronously test whether this impl handles the given backend_id.
    /// Called by MultiBackendLlmService to route requests. Must be cheap —
    /// typically a HashMap lookup or prefix match.
    /// Default impl returns false; every concrete impl must override.
    fn claims_backend(&self, _backend_id: &str) -> bool { false }
}
```

## The `MultiBackendLlmService` Router

A concrete impl of `LlmService` that holds multiple backend impls keyed by `backend_id` and dispatches per request. Lives in `wafer-core/src/interfaces/llm/router.rs`.

```rust
pub struct MultiBackendLlmService {
    /// Ordered list of registered impls. Order matters — first impl whose
    /// claims_backend() returns true for a given backend_id wins.
    impls: Vec<(String, Arc<dyn LlmService>)>,   // (label, impl) — label for diagnostics
}

impl MultiBackendLlmService {
    pub fn new() -> Self { Self { impls: Vec::new() } }

    pub fn register(
        &mut self,
        label: impl Into<String>,
        service: Arc<dyn LlmService>,
    ) -> &mut Self {
        self.impls.push((label.into(), service));
        self
    }

    fn find(&self, backend_id: &str) -> Option<&Arc<dyn LlmService>> {
        self.impls.iter().find(|(_, svc)| svc.claims_backend(backend_id)).map(|(_, svc)| svc)
    }
}

#[async_trait]
impl LlmService for MultiBackendLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        match self.find(&req.backend_id) {
            Some(svc) => svc.chat_stream(req, cancel).await,
            None => Box::pin(futures::stream::once({
                let id = req.backend_id.clone();
                async move { Err(LlmError::InvalidRequest(format!("unknown backend_id: {}", id))) }
            })),
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let mut all = Vec::new();
        for (label, svc) in &self.impls {
            match svc.list_models().await {
                Ok(models) => all.extend(models),
                Err(e) => tracing::warn!(backend = %label, error = %e, "list_models failed"),
                // One backend failing doesn't fail the aggregation.
            }
        }
        Ok(all)
    }

    async fn status(&self, backend_id: &str, model_id: &str) -> Result<ModelStatus, LlmError> {
        self.find(backend_id)
            .ok_or_else(|| LlmError::InvalidRequest(format!("unknown backend: {}", backend_id)))?
            .status(backend_id, model_id)
            .await
    }

    fn load_model(
        &self,
        backend_id: &str,
        model_id: &str,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        match self.find(backend_id) {
            Some(svc) => svc.load_model(backend_id, model_id, cancel),
            None => Box::pin(futures::stream::once(async move {
                Err(LlmError::InvalidRequest("unknown backend".to_string()))
            })),
        }
    }

    async fn unload_model(&self, backend_id: &str, model_id: &str) -> Result<(), LlmError> {
        self.find(backend_id)
            .ok_or_else(|| LlmError::InvalidRequest(format!("unknown backend: {}", backend_id)))?
            .unload_model(backend_id, model_id)
            .await
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        self.find(backend_id).is_some()
    }
}
```

## The `wafer-run/llm` Service Block

Lives in `wafer-core/src/service_blocks/llm.rs`. Mirrors the `DatabaseBlock` shape exactly.

```rust
pub struct LlmBlock { service: Arc<dyn LlmService> }

impl LlmBlock {
    pub fn new(service: Arc<dyn LlmService>) -> Self { Self { service } }
}

pub fn register_with(w: &mut dyn BlockRegistry, service: Arc<dyn LlmService>) {
    w.register_block("wafer-run/llm".to_string(), Arc::new(LlmBlock::new(service)))
        .expect("register wafer-run/llm");
}

#[async_trait]
impl Block for LlmBlock {
    async fn handle(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
    ) -> OutputStream {
        // Decodes the operation from msg.kind, dispatches to the trait,
        // serializes results onto OutputStream.
        // See handler.rs for the full implementation.
        handler::handle_message(self.service.clone(), ctx, msg, input).await
    }
}
```

`handler::handle_message` decodes one of a handful of operation kinds:

- `"llm.chat"` → forwards req to `service.chat_stream()`, pipes each `ChatChunk` as a `StreamEvent::Chunk` (serialized as JSON), terminates with `Complete`.
- `"llm.list_models"` → calls `service.list_models()`, returns buffered JSON.
- `"llm.status"` → calls `service.status()`, returns buffered JSON.
- `"llm.load_model"` → forwards to `service.load_model()`, streams `LoadProgress` events.
- `"llm.unload_model"` → calls `service.unload_model()`, returns buffered JSON.

All consumers — third-party blocks, HTTP clients, in-process blocks — reach the service the same way: through `ctx.call_block("wafer-run/llm", msg, input)` using the streaming protocol from Spec 1. In-process, this resolves to a direct `Arc<dyn Block>::handle` call (no JSON envelope around chunks) but the `ChatRequest` is still serialized into the `Message`'s `meta` / first input chunk for the block to decode. The serialization overhead on the typed request is a small per-call cost; token chunks themselves flow through the stream as raw bytes with no JSON wrapping. This keeps one uniform access pattern — no side-channel service registry — consistent with Spec 1's principle.

## `ProviderLlmService` Impl

Lives in `solobase-core/src/blocks/llm/providers/service.rs`. Handles remote HTTP-based backends and local HTTP-based servers through a uniform OpenAI-compatible protocol path.

### Supported provider types

Declared as a typed enum so each has its own request/response mapping:

```rust
#[non_exhaustive]
pub enum ProviderProtocol {
    OpenAi,               // api.openai.com / api.openai.com-compatible, native OpenAI request shape
    Anthropic,            // api.anthropic.com native request shape
    OpenAiCompatible,     // third-party endpoints implementing OpenAI's /v1 interface:
                          //   Ollama, llama-server, LM Studio, vLLM, LocalAI, KoboldCpp,
                          //   Azure OpenAI, Together, Groq, OpenRouter, Mistral API, Anyscale, etc.
}

pub struct ProviderConfig {
    pub name: String,                // display + backend_id key, e.g. "openai-main", "local-llama"
    pub protocol: ProviderProtocol,
    pub endpoint: String,            // base URL, e.g. "https://api.openai.com/v1" or "http://localhost:11434/v1"
    pub api_key: Option<String>,     // None is valid for local servers
    pub models: Vec<String>,         // either explicit list or empty (triggers discovery from endpoint /v1/models)
}
```

### Impl structure

```rust
pub struct ProviderLlmService {
    inner: Arc<RwLock<ProviderLlmInner>>,
    http: reqwest::Client,
}

struct ProviderLlmInner {
    providers: HashMap<String, ProviderConfig>,   // keyed by ProviderConfig.name
    cached_models: HashMap<String, Vec<ModelInfo>>,  // per provider, refreshed periodically
}

impl ProviderLlmService {
    /// Feature-block startup-time configuration. Replaces the current provider list.
    /// This is a concrete method on the impl, NOT on the trait — provider CRUD is
    /// Solobase-specific orchestration, not part of the generic LLM contract.
    pub async fn configure(&self, providers: Vec<ProviderConfig>) -> Result<(), LlmError>;

    /// Called by feature block when admin discovers models by querying endpoint.
    /// Returns the models the endpoint exposes via /v1/models.
    pub async fn discover_models(
        &self,
        provider_name: &str,
    ) -> Result<Vec<ModelInfo>, LlmError>;
}

#[async_trait]
impl LlmService for ProviderLlmService {
    async fn chat_stream(...) -> BoxStream<...> {
        // Route to per-protocol client based on provider.protocol.
        // Each client:
        //   1. Translates ChatRequest → provider-native HTTP request
        //   2. Issues HTTP call with SSE streaming enabled
        //   3. Parses provider-native SSE frames → ChatChunk events
        //   4. Applies cancel token: aborts the HTTP stream on cancel.cancelled()
    }
    // ... other trait methods
}
```

### Protocol mapping details

Each `ProviderProtocol` variant has its own request encoder / response decoder pair in its own module:

- `providers/openai.rs` — native OpenAI API (also used as the basis for OpenAiCompatible).
- `providers/anthropic.rs` — Anthropic's `/v1/messages` API with its own SSE event format (`content_block_start`, `content_block_delta`, etc.).
- `providers/openai_compatible.rs` — reuses `openai.rs` for request/response, but:
  - `api_key` is optional
  - Model list comes from endpoint's `/v1/models` discovery (not hardcoded)
  - Handles graceful degradation when endpoint lacks features (no system prompts, no tool calls, etc.)
  - No Anthropic-specific shapes

Tool calling is supported natively by OpenAI + most OpenAI-compatible endpoints; Anthropic has its own tool-use shape; both are mapped to the common `ChatChunk`/`ToolCall` types by the protocol-specific decoders.

## `BrowserLlmService` Impl

Lives in `solobase-web/src/llm.rs` (new module). Thin Rust wrapper over WebLLM via wasm-bindgen.

```rust
pub struct BrowserLlmService {
    engine: RefCell<Option<WebLlmEngineHandle>>,  // initialized on first load_model
    available_models: Vec<ModelInfo>,              // hardcoded list of WebLLM-supported models
}

#[async_trait(?Send)]
impl LlmService for BrowserLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        let engine = self.engine.borrow().as_ref().ok_or(LlmError::NotSupported)?.clone();
        let (tx, rx) = mpsc::channel(16);
        wasm_bindgen_futures::spawn_local(async move {
            let js_iterator = engine.chat_completions_create(req.to_webllm()).await;
            while let Some(frame) = js_iterator.next().await {
                if cancel.is_cancelled() { break; }
                let chunk = ChatChunk::from_webllm_frame(frame);
                if tx.send(Ok(chunk)).await.is_err() { break; }
            }
        });
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn load_model(...) -> BoxStream<...> {
        // Calls WebLLM's CreateMLCEngine with a progress callback.
        // Progress callback posts LoadProgress events onto the returned stream.
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.available_models.clone())   // list is hardcoded per WebLLM's supported set
    }

    // status, unload_model implementations straightforward
}
```

**`WebLlmEngineHandle`** is a wasm-bindgen facade that wraps the JS `MLCEngine`. All wasm-bindgen interop is contained here; the rest of Solobase sees only Rust types.

### Removal of ai-bridge.js

`ai-bridge.js` is deleted. The service worker no longer intercepts `/b/local-llm/api/*` paths. All browser LLM traffic flows through normal Solobase routes:

- `POST /b/llm/api/chat/stream` → feature block → `ctx.call_block("wafer-run/llm", …)` → LlmBlock → router → `BrowserLlmService` → WebLLM → back through the streaming pipeline to the browser client as SSE.

The postMessage dance between service worker and main page goes away. Token flow is: WebLLM (main page) → `BrowserLlmService` (runs on main page) → `LlmBlock` service → `suppers-ai/llm` feature block HTTP handler → `ReadableStream` → `EventSource`.

Note that `BrowserLlmService` must run on the main page, not inside the service worker, because WebGPU is not available in service worker contexts. The solobase-web runtime already splits into a main-page portion and a service-worker portion; `BrowserLlmService` registers on the main-page side. The service-worker-side LlmService (if any chat calls arrive there) is a thin remote-proxy impl that posts messages to the main page — but unlike ai-bridge.js, this is a legitimate runtime routing pattern, not an ad-hoc JSON postMessage hack: it uses the same streaming protocol.

## The `suppers-ai/llm` Feature Block

Lives in `solobase-core/src/blocks/llm/` (same location as today). Replaces all three current blocks' admin/chat-related responsibilities.

### Responsibilities

- **Admin chat UI** — same chat interface as today, unchanged UX; backend routing is transparent.
- **Admin provider-CRUD UI** — one page showing all providers (OpenAI, Anthropic, local Ollama, etc.) with add/edit/delete; consolidates today's split between `suppers-ai/llm/settings` and `suppers-ai/provider-llm/admin`.
- **Thread / message persistence** — integrates with `suppers-ai/messages` same as today.
- **HTTP endpoints:**
  - `POST /b/llm/api/chat` — buffered response (collects the stream).
  - `POST /b/llm/api/chat/stream` — SSE streaming.
  - `GET  /b/llm/api/models` — aggregated from router.
  - `GET  /b/llm/api/providers` — returns the list of configured providers.
  - `POST /b/llm/api/providers` — admin-only, create provider.
  - `PATCH /b/llm/api/providers/:id` — admin-only, update.
  - `DELETE /b/llm/api/providers/:id` — admin-only, delete.
  - `POST /b/llm/api/models/:backend_id/:model_id/load` — streams LoadProgress as SSE.
  - `POST /b/llm/api/models/:backend_id/:model_id/unload`.
  - `GET  /b/llm/api/models/:backend_id/:model_id/status`.

### Startup wiring

Responsibilities are split between the application entry (which assembles and registers the service block) and the feature block's `lifecycle(Init)` (which configures the already-registered ProviderLlmService with persisted provider data).

**Application entry (solobase binary `main`, solobase-web entry, solobase-cloudflare entry):**

```rust
// Construct impls. Router holds Arcs so the app entry can still reach
// ProviderLlmService directly to call its concrete .configure() method.
let provider_svc = Arc::new(ProviderLlmService::new());
let mut router = MultiBackendLlmService::new();
router.register("providers", provider_svc.clone() as Arc<dyn LlmService>);

// Browser / native only: register BrowserLlmService. (Not on CF / server-side.)
#[cfg(target_arch = "wasm32")]
router.register("browser", Arc::new(BrowserLlmService::new()) as Arc<dyn LlmService>);

// Register as wafer-run/llm service block.
wafer_core::service_blocks::llm::register_with(&mut wafer, Arc::new(router));

// Stash provider_svc somewhere the feature block can reach it
// (typically via a typed slot in app-level state, passed into the feature
// block's constructor during block registration).
```

**Feature block `lifecycle(Init)`:** loads persisted provider configs from DB and calls `provider_svc.configure(...)` with them. Subsequent admin edits flow through the same call.

**Note on routing keys:** `ChatRequest.backend_id` refers to a specific provider (e.g., `"openai-main"`, `"local-llama"`, `"webllm-smollm2"`). The router needs to pick the right impl for each id. A single `ProviderLlmService` impl holds many providers internally — so the mapping is many:one at the provider level, with the router dispatching by impl. Resolution mechanism: each `LlmService` impl exposes a synchronous `claims_backend(&self, id: &str) -> bool`, and the router iterates registered impls to find the first claimer. `ProviderLlmService::claims_backend` checks its configured-providers map; `BrowserLlmService::claims_backend` checks its hardcoded WebLLM model ids (with a prefix convention like `webllm-*`).

### Admin UI behavior

- **Providers page.** Table of all configured providers. Columns: name, protocol (`OpenAI` / `Anthropic` / `OpenAI-compatible`), endpoint, auth status (has key? doesn't need one?), enabled. Row actions: edit, delete, test connection (calls `/v1/models` on endpoint), discover models.
- **Models page.** Aggregated model list from all enabled providers. Columns: backend_id, model_id, display name, capabilities (streaming/tools/vision badges), status (Ready/Loading/Unloaded for local; Ready for remote). Row actions: load / unload (for local), set as default.
- **Chat UI.** One page, model picker dropdown populated from `/b/llm/api/models`, thread sidebar, message history, streaming typing indicator.
- **Local provider setup docs.** Documentation page or first-run flow explaining: "To use local LLMs, run Ollama / llama-server / LM Studio and add it as an OpenAI-compatible provider pointing at `http://localhost:11434/v1`."

## Configuration Variables

Naming cleanup to match the new structure (follows the SOLOBASE config convention):

- **Deleted:**
  - `SUPPERS_AI__LLM__DEFAULT_PROVIDER` — replaced by per-thread / default setting in DB.
  - `SUPPERS_AI__LLM__DEFAULT_MODEL` — same; lives in settings row, not env.
  - `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY` — providers' API keys live in the provider record. Config-var approach stays available for single-provider-key deployments but is no longer hardcoded at the block-module level.
  - `SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY` — same as above.

- **Renamed / new:**
  - `SUPPERS_AI__LLM__DEFAULT_PROVIDER` — (optional) default provider name for new threads.
  - `SUPPERS_AI__LLM__DEFAULT_MODEL` — (optional) default model id.
  - API keys: configured per-provider in the DB, with an option to reference a config var via a `key_var: String` field on `ProviderConfig`. Admin UI offers both inline (stored encrypted in DB) and config-var reference.

## Removals

Deleted outright:

- `solobase-core/src/blocks/provider_llm/` — entire directory.
- `solobase-core/src/blocks/local_llm.rs` — file.
- `solobase-core/src/blocks/llm_backend.rs` — file.
- `solobase-web/js/ai-bridge.js` — file.
- Service-worker `/b/local-llm/api/*` interception logic in `solobase-web/js/sw.js`.
- Block registration entries for `BlockId::ProviderLlm` and `BlockId::LocalLlm` in `solobase-core/src/blocks/mod.rs`.
- CSP exemption for service-worker ↔ main-page postMessage LLM bridge (no longer needed; normal same-origin channels suffice).
- DB collection `suppers_ai__provider_llm__providers` — data is migrated to `suppers_ai__llm__providers` during first startup on the new version, then the old collection is dropped.

## Migration

Atomic single PR, aligned with Spec 1's migration strategy. Because Spec 1 introduces the streaming protocol at the same time, Spec 2's implementation rides on top of it.

Order of operations within the PR:

1. **Add new types** in `wafer-core/src/interfaces/llm/`: `service.rs` (trait + data types), `router.rs` (MultiBackendLlmService), `mod.rs`.
2. **Add the service block** in `wafer-core/src/service_blocks/llm.rs`: `LlmBlock`, `register_with`, `handler.rs` for message dispatch.
3. **Add `ProviderLlmService`** in `solobase-core/src/blocks/llm/providers/`: protocol clients (`openai.rs`, `anthropic.rs`, `openai_compatible.rs`), `service.rs` tying them together, `config.rs` for `ProviderConfig`.
4. **Add `BrowserLlmService`** in `solobase-web/src/llm.rs`: WebLLM wasm-bindgen facade, `LlmService` impl, model-list constants.
5. **Rewrite `suppers-ai/llm` feature block** in `solobase-core/src/blocks/llm/`: new admin UI pages (chat + providers + models), new HTTP endpoints, new startup wiring. Preserve chat thread schema and migrate the thread-level settings schema.
6. **DB migration.** During first startup on the new version, if `suppers_ai__provider_llm__providers` exists, read its rows, translate to the new `suppers_ai__llm__providers` schema, write them, then drop the old collection. One-shot migration in `lifecycle(Init)`.
7. **Delete** `provider_llm/`, `local_llm.rs`, `llm_backend.rs`, `ai-bridge.js`, service-worker local-llm interception, unused config var declarations.
8. **Update** `solobase-core/src/blocks/mod.rs` to remove `BlockId::ProviderLlm` and `BlockId::LocalLlm`; keep `BlockId::Llm` but point it at the new consolidated feature block.
9. **Update tests.**

## Testing Strategy

### Unit tests — the trait and the router

```rust
#[tokio::test]
async fn router_dispatches_by_backend_id() {
    let mut router = MultiBackendLlmService::new();
    router.register("a", Arc::new(FakeLlmService::returning_text("from-a")));
    router.register("b", Arc::new(FakeLlmService::returning_text("from-b")));

    let out = router.chat_stream(
        ChatRequest::new("b", "test-model", vec![user("hi")]),
        CancellationToken::new(),
    ).await;

    let chunks: Vec<_> = out.try_collect().await.unwrap();
    assert!(chunks.iter().any(|c| matches!(&c.delta, ChunkDelta::Text(s) if s == "from-b")));
}

#[tokio::test]
async fn router_list_models_aggregates_across_backends() {
    let mut router = MultiBackendLlmService::new();
    router.register("a", Arc::new(FakeLlmService::with_models(vec![model("a", "m1")])));
    router.register("b", Arc::new(FakeLlmService::with_models(vec![model("b", "m2")])));

    let models = router.list_models().await.unwrap();
    assert_eq!(models.len(), 2);
}

#[tokio::test]
async fn default_load_model_returns_not_supported() {
    let svc = FakeLlmService::default();
    let mut stream = svc.load_model("x", "y", CancellationToken::new());
    let first = stream.next().await.unwrap();
    assert!(matches!(first, Err(LlmError::NotSupported)));
}
```

### Unit tests — ProviderLlmService

Each protocol client tested independently via a stubbed `reqwest::Client`. Tests assert:
- Request encoding matches the target API's documented shape (OpenAI / Anthropic / OpenAI-compat).
- Response SSE frames are decoded to `ChatChunk` events in order.
- Cancellation aborts the stream mid-generation.
- Tool-call delta frames are correctly emitted as `ToolCallStart` / `ToolCallArguments` / `ToolCallComplete`.
- Network errors map to `LlmError::Network`, 401 to `Unauthorized`, 429 to `RateLimited`.

### Unit tests — BrowserLlmService

Browser-only tests using `wasm-bindgen-test`:
- Loading a model triggers progress events.
- Chat streaming yields chunks in order.
- Cancellation aborts generation promptly.

### Integration tests — feature block

End-to-end through the `TestRuntime` from Spec 1:
- `POST /b/llm/api/chat/stream` with a fake backend yields SSE-framed tokens on the wire.
- Provider CRUD: add a provider, verify DB + ProviderLlmService receive the update; list; edit; delete.
- Model discovery: a fake endpoint returns a model list, the feature block populates `/b/llm/api/models` with the discovered entries.

### Migration tests

- A test fixture database containing `suppers_ai__provider_llm__providers` records is migrated on startup to `suppers_ai__llm__providers`; records' `protocol` field is correctly assigned based on `provider_type`; old collection is dropped.

## Out of Scope (Explicit)

- **NativeLlmService (in-process local inference).** Not planned. Users run their own Ollama / llama-server / LM Studio and configure it as an OpenAI-compatible provider. If a future use case genuinely requires in-process FFI-based inference (e.g., a Solobase deployment that absolutely cannot run a sidecar), revisit at that point.
- **Embedding service.** Future sibling trait `EmbeddingService` likely follows the same pattern but is a separate spec.
- **Vector search / RAG.** Separate concern; would be a `VectorSearchService` trait integrated at the feature-block layer, not a concern for `LlmService`.
- **Prompt caching, reasoning models, extended thinking.** The types support them via `TokenUsage.cached_tokens`, `TokenUsage.reasoning_tokens`, and `extra` overflow, but the feature block's UI affordances for these are a follow-up.
- **Multimodal output generation (image/audio generation).** Not in the chat-completion surface. Separate trait family.
- **Rate limiting / quotas.** A `RateLimitedLlmService` wrapper (decorator pattern) could wrap any `LlmService`; design when needed.
- **Fallback chains / redundancy.** Similarly, a `FallbackLlmService` wrapper; design when needed.

## Open Implementation Notes

- Thread-level `provider` / `model` override schema: keep today's `suppers_ai__llm__settings` collection, or fold into per-thread columns on `suppers_ai__messages__threads`? Lean toward keeping separate for clean ownership.
- Admin UI for per-provider API key storage: inline (stored in DB, encrypted at rest) vs config-var reference (`key_var: String` field pointing at `SUPPERS_AI__LLM__OPENAI_KEY_PROD` etc.). Both supported; UI defaults to inline for simplicity.
- WebLLM model list: currently hardcoded in ai-bridge.js. In the new impl, lives as a constant in `solobase-web/src/llm.rs`. Consider promoting to a data file or config at some point.
- Error frames for SSE: aligned with Spec 1's convention (`event: error\ndata: {json}\n\n`), with provider-specific error details in the JSON.
