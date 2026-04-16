# LLM Service Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate three existing LLM blocks (`suppers-ai/llm`, `suppers-ai/provider-llm`, `suppers-ai/local-llm`) into a typed-service architecture: `LlmService` trait + `MultiBackendLlmService` router + `wafer-run/llm` service block in wafer-core, `ProviderLlmService` (OpenAI / Anthropic / OpenAI-compatible) in solobase-core, `BrowserLlmService` in solobase-web, single rewritten `suppers-ai/llm` feature block owning admin UI + provider CRUD + chat HTTP. Delete the other two blocks, `llm_backend.rs`, `ai-bridge.js`, and service-worker LLM interception.

**Architecture:** All types in `wafer-core/src/interfaces/llm/` with `#[non_exhaustive]` everywhere for forward extensibility. Backend routing via `claims_backend(id) -> bool` dispatch in the router. Each backend impl holds its own configuration (ProviderLlmService has many HTTP-based providers internally, BrowserLlmService claims the WebLLM id prefix). Feature block wires impls into the router at startup and `ProviderLlmService::configure(Vec<ProviderConfig>)` feeds persisted provider data from DB. Chat streams from impl → router → service block → feature block → HTTP SSE to client, uniformly.

**Tech Stack:** Rust async-trait, `futures::Stream`, `reqwest` (native/CF) + `wasm-bindgen-futures` (browser), `serde_json` for the `extra` overflow field, `thiserror` for `LlmError`. Existing streaming protocol primitives from Spec 1 (`OutputStream` / `InputStream` / etc.).

**Spec:** [2026-04-15-llm-service-refactor-design.md](../specs/2026-04-15-llm-service-refactor-design.md)

**Prerequisite:** Spec 1 (streaming protocol) MUST be fully landed before starting this plan. Every task below assumes `Block::handle(ctx, msg, input) -> OutputStream` is the current signature, `StreamEvent` exists, `BlockResult`/`Action`/`Response` are deleted.

**Work on branch:** `feat/llm-service-refactor` in both `wafer-run` and `solobase` monorepos (create from the tip of whatever branch lands Spec 1).

---

## Phase A: `LlmService` trait and types in wafer-core

### Task A1: Scaffold `interfaces/llm/` module

**Files:**
- Create: `wafer-run/crates/wafer-core/src/interfaces/llm/mod.rs`
- Create: `wafer-run/crates/wafer-core/src/interfaces/llm/types.rs`
- Modify: `wafer-run/crates/wafer-core/src/interfaces.rs` or `lib.rs` (register `llm` module)

- [ ] **Step 1: Write stub + test**

In `types.rs`:
```rust
// Scaffold — types added in later tasks.

#[cfg(test)]
#[test]
fn module_exists() {
    // Placeholder — real tests in later tasks.
}
```

In `mod.rs`:
```rust
pub mod types;
```

In whichever existing file registers the interfaces module, add:
```rust
pub mod llm;
```

(Look at how `database` and `storage` submodules are registered and follow the same pattern.)

- [ ] **Step 2: Run**

```bash
cd /home/joris/Programs/suppers-ai/workspace/wafer-run
cargo check -p wafer-core
```

Expected: compiles clean.

- [ ] **Step 3: Commit**

```bash
git add crates/wafer-core/src/interfaces/
git commit -m "feat(wafer-core): scaffold interfaces/llm module"
```

### Task A2: Add request data types — ChatRequest, ChatParams, ChatMessage, ChatRole, ChatContent, ContentPart

**Files:**
- Modify: `wafer-run/crates/wafer-core/src/interfaces/llm/types.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_constructs_with_defaults() {
        let req = ChatRequest {
            backend_id: "openai".into(),
            model: "gpt-4o".into(),
            messages: vec![],
            params: ChatParams::default(),
            tools: vec![],
            extra: serde_json::Value::Null,
        };
        assert_eq!(req.backend_id, "openai");
    }

    #[test]
    fn chat_message_variants() {
        let _sys = ChatMessage {
            role: ChatRole::System,
            content: ChatContent::Text("hi".into()),
            tool_call_id: None,
            tool_calls: vec![],
        };
        let _user = ChatMessage {
            role: ChatRole::User,
            content: ChatContent::Parts(vec![ContentPart::Text("hello".into())]),
            tool_call_id: None,
            tool_calls: vec![],
        };
    }

    #[test]
    fn chat_params_default_all_none() {
        let p = ChatParams::default();
        assert!(p.max_tokens.is_none());
        assert!(p.temperature.is_none());
        assert!(p.stop_sequences.is_empty());
    }

    #[test]
    fn content_part_variants_construct() {
        let _t = ContentPart::Text("hello".into());
        let _u = ContentPart::ImageUrl { url: "https://example.com/x.png".into(), detail: None };
        let _b = ContentPart::ImageBytes { bytes: vec![1, 2, 3], mime_type: "image/png".into() };
    }
}
```

- [ ] **Step 2: Run and see compile errors**

```bash
cargo test -p wafer-core interfaces::llm::types::tests
```

- [ ] **Step 3: Implement**

In `types.rs`:
```rust
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub backend_id: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub params: ChatParams,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[non_exhaustive]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatParams {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    pub seed: Option<u64>,
    pub response_format: Option<ResponseFormat>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    Json,
    JsonSchema(serde_json::Value),
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: ChatContent,
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String, detail: Option<String> },
    ImageBytes {
        #[serde(with = "serde_bytes")]
        bytes: Vec<u8>,
        mime_type: String,
    },
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
```

Add dep to `wafer-core/Cargo.toml`:
```toml
serde_bytes = "0.11"
```

(If `serde` and `serde_json` aren't already there, add them too.)

- [ ] **Step 4: Run**

```bash
cargo test -p wafer-core interfaces::llm::types::tests
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/wafer-core/src/interfaces/llm/types.rs crates/wafer-core/Cargo.toml
git commit -m "feat(wafer-core): LLM request types (ChatRequest, ChatMessage, ContentPart)"
```

### Task A3: Add response data types — ChatChunk, ChunkDelta, FinishReason, TokenUsage

**Files:**
- Modify: `wafer-run/crates/wafer-core/src/interfaces/llm/types.rs`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn chat_chunk_text_delta() {
    let chunk = ChatChunk {
        delta: ChunkDelta::Text("hello".into()),
        finish_reason: None,
        usage: None,
    };
    assert!(chunk.finish_reason.is_none());
}

#[test]
fn chat_chunk_with_usage_and_finish() {
    let chunk = ChatChunk {
        delta: ChunkDelta::Empty,
        finish_reason: Some(FinishReason::Stop),
        usage: Some(TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            cached_tokens: None,
            reasoning_tokens: None,
        }),
    };
    assert!(matches!(chunk.finish_reason, Some(FinishReason::Stop)));
}

#[test]
fn chunk_delta_tool_call_arguments_streaming() {
    let _ = ChunkDelta::ToolCallStart { id: "call_1".into(), name: "get_weather".into() };
    let _ = ChunkDelta::ToolCallArguments { id: "call_1".into(), arguments_delta: "{\"ci".into() };
    let _ = ChunkDelta::ToolCallComplete { id: "call_1".into() };
}
```

- [ ] **Step 2: Run**

- [ ] **Step 3: Implement**

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub delta: ChunkDelta,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ChunkDelta {
    Text(String),
    ToolCallStart { id: String, name: String },
    ToolCallArguments { id: String, arguments_delta: String },
    ToolCallComplete { id: String },
    Empty,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    ContentFilter,
    Error,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}
```

- [ ] **Step 4: Run**

`cargo test -p wafer-core interfaces::llm::types::tests`

- [ ] **Step 5: Commit**

```bash
git add crates/wafer-core/src/interfaces/llm/types.rs
git commit -m "feat(wafer-core): LLM response types (ChatChunk, ChunkDelta, TokenUsage)"
```

### Task A4: Add model management types — ModelInfo, ModelCapabilities, ModelStatus, ModelState, LoadProgress

**Files:**
- Modify: `wafer-run/crates/wafer-core/src/interfaces/llm/types.rs`

- [ ] **Step 1–5: Same TDD pattern**

Add tests for each type's construction; implement with `#[non_exhaustive]`:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub backend_id: String,
    pub model_id: String,
    pub display_name: String,
    pub capabilities: ModelCapabilities,
}

#[non_exhaustive]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub json_mode: bool,
    pub max_context_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub state: ModelState,
    pub progress: Option<f32>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ModelState {
    Ready,
    Loading,
    Unloaded,
    Error { message: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadProgress {
    pub stage: String,
    pub bytes_downloaded: Option<u64>,
    pub bytes_total: Option<u64>,
}
```

Commit: `feat(wafer-core): LLM model management types (ModelInfo, ModelStatus, LoadProgress)`.

### Task A5: Add `LlmError`

**Files:**
- Create: `wafer-run/crates/wafer-core/src/interfaces/llm/error.rs`
- Modify: `mod.rs` (register module)

- [ ] **Step 1: Failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_variants_construct() {
        let _ = LlmError::NotSupported;
        let _ = LlmError::InvalidRequest("bad".into());
        let _ = LlmError::BackendError("boom".into());
        let _ = LlmError::ModelNotFound("gpt-5".into());
        let _ = LlmError::RateLimited;
        let _ = LlmError::Unauthorized;
        let _ = LlmError::Network("timeout".into());
        let _ = LlmError::Cancelled;
    }

    #[test]
    fn error_displays_message() {
        let e = LlmError::InvalidRequest("missing field".into());
        let msg = format!("{}", e);
        assert!(msg.contains("missing field"));
    }
}
```

- [ ] **Step 2–5:** Implement:

```rust
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "code", content = "message")]
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

In `mod.rs`:
```rust
pub mod error;
pub mod types;
pub use error::LlmError;
pub use types::*;
```

Commit: `feat(wafer-core): LlmError type`.

### Task A6: Define `LlmService` trait

**Files:**
- Create: `wafer-run/crates/wafer-core/src/interfaces/llm/service.rs`
- Modify: `mod.rs`

- [ ] **Step 1: Failing test with a fake impl**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::stream::{self, BoxStream};
    use tokio_util::sync::CancellationToken;

    struct FakeLlm;

    #[async_trait]
    impl LlmService for FakeLlm {
        async fn chat_stream(
            &self,
            _req: ChatRequest,
            _cancel: CancellationToken,
        ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
            Box::pin(stream::once(async {
                Ok(ChatChunk {
                    delta: ChunkDelta::Text("hi".into()),
                    finish_reason: Some(FinishReason::Stop),
                    usage: None,
                })
            }))
        }

        async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
            Ok(vec![])
        }

        async fn status(&self, _: &str, _: &str) -> Result<ModelStatus, LlmError> {
            Ok(ModelStatus { state: ModelState::Ready, progress: None })
        }

        fn claims_backend(&self, backend_id: &str) -> bool {
            backend_id == "fake"
        }
    }

    #[tokio::test]
    async fn fake_impl_satisfies_trait() {
        let svc: &dyn LlmService = &FakeLlm;
        let stream = svc
            .chat_stream(
                ChatRequest {
                    backend_id: "fake".into(),
                    model: "m".into(),
                    messages: vec![],
                    params: ChatParams::default(),
                    tools: vec![],
                    extra: serde_json::Value::Null,
                },
                CancellationToken::new(),
            )
            .await;
        use futures::StreamExt;
        let chunks: Vec<_> = stream.collect().await;
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_ok());
    }

    #[tokio::test]
    async fn default_load_model_returns_not_supported() {
        let svc: &dyn LlmService = &FakeLlm;
        use futures::StreamExt;
        let stream = svc.load_model("fake", "m", CancellationToken::new());
        let events: Vec<_> = stream.collect().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Err(LlmError::NotSupported)));
    }

    #[tokio::test]
    async fn default_unload_model_returns_not_supported() {
        let svc: &dyn LlmService = &FakeLlm;
        let r = svc.unload_model("fake", "m").await;
        assert!(matches!(r, Err(LlmError::NotSupported)));
    }

    #[test]
    fn default_claims_backend_is_false() {
        struct Default;
        #[async_trait]
        impl LlmService for Default {
            async fn chat_stream(&self, _: ChatRequest, _: CancellationToken)
                -> BoxStream<'static, Result<ChatChunk, LlmError>> {
                Box::pin(stream::empty())
            }
            async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> { Ok(vec![]) }
            async fn status(&self, _: &str, _: &str) -> Result<ModelStatus, LlmError> {
                Ok(ModelStatus { state: ModelState::Ready, progress: None })
            }
            // claims_backend not overridden — uses default
        }
        assert!(!(Default).claims_backend("anything"));
    }
}
```

- [ ] **Step 2: Run**

- [ ] **Step 3: Implement**

In `service.rs`:
```rust
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use tokio_util::sync::CancellationToken;

use super::error::LlmError;
use super::types::*;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait LlmService: Send + Sync + 'static {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>>;

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError>;

    async fn status(
        &self,
        backend_id: &str,
        model_id: &str,
    ) -> Result<ModelStatus, LlmError>;

    fn load_model(
        &self,
        _backend_id: &str,
        _model_id: &str,
        _cancel: CancellationToken,
    ) -> BoxStream<'static, Result<LoadProgress, LlmError>> {
        Box::pin(stream::once(async { Err(LlmError::NotSupported) }))
    }

    async fn unload_model(
        &self,
        _backend_id: &str,
        _model_id: &str,
    ) -> Result<(), LlmError> {
        Err(LlmError::NotSupported)
    }

    fn claims_backend(&self, _backend_id: &str) -> bool {
        false
    }
}
```

- [ ] **Step 4: Run** — 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/wafer-core/src/interfaces/llm/service.rs crates/wafer-core/src/interfaces/llm/mod.rs
git commit -m "feat(wafer-core): LlmService trait with streaming chat + load progress"
```

---

## Phase B: `MultiBackendLlmService` router

### Task B1: Implement the router

**Files:**
- Create: `wafer-run/crates/wafer-core/src/interfaces/llm/router.rs`
- Modify: `mod.rs`

- [ ] **Step 1: Failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use async_trait::async_trait;
    use futures::stream;
    use std::sync::Arc;

    struct FakeSvc { id: String, models: Vec<ModelInfo> }

    #[async_trait]
    impl LlmService for FakeSvc {
        async fn chat_stream(&self, req: ChatRequest, _: CancellationToken)
            -> BoxStream<'static, Result<ChatChunk, LlmError>>
        {
            let id = self.id.clone();
            Box::pin(stream::once(async move {
                Ok(ChatChunk {
                    delta: ChunkDelta::Text(format!("from-{}: model={}", id, req.model)),
                    finish_reason: Some(FinishReason::Stop),
                    usage: None,
                })
            }))
        }
        async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> { Ok(self.models.clone()) }
        async fn status(&self, _: &str, _: &str) -> Result<ModelStatus, LlmError> {
            Ok(ModelStatus { state: ModelState::Ready, progress: None })
        }
        fn claims_backend(&self, id: &str) -> bool { id == self.id }
    }

    fn model(backend: &str, id: &str) -> ModelInfo {
        ModelInfo {
            backend_id: backend.into(),
            model_id: id.into(),
            display_name: id.into(),
            capabilities: ModelCapabilities::default(),
        }
    }

    #[tokio::test]
    async fn router_dispatches_by_backend_id() {
        let mut router = MultiBackendLlmService::new();
        router.register("svc-a", Arc::new(FakeSvc { id: "a".into(), models: vec![] }));
        router.register("svc-b", Arc::new(FakeSvc { id: "b".into(), models: vec![] }));

        let out = router.chat_stream(
            ChatRequest {
                backend_id: "b".into(),
                model: "test".into(),
                messages: vec![],
                params: Default::default(),
                tools: vec![],
                extra: serde_json::Value::Null,
            },
            CancellationToken::new(),
        ).await;

        use futures::StreamExt;
        let chunks: Vec<_> = out.collect().await;
        assert!(chunks.iter().any(|c| matches!(
            c,
            Ok(ChatChunk { delta: ChunkDelta::Text(s), .. }) if s.starts_with("from-b")
        )));
    }

    #[tokio::test]
    async fn router_unknown_backend_returns_error_stream() {
        let router = MultiBackendLlmService::new();
        let out = router.chat_stream(
            ChatRequest {
                backend_id: "missing".into(),
                model: "x".into(),
                messages: vec![],
                params: Default::default(),
                tools: vec![],
                extra: serde_json::Value::Null,
            },
            CancellationToken::new(),
        ).await;
        use futures::StreamExt;
        let chunks: Vec<_> = out.collect().await;
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], Err(LlmError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn router_list_models_aggregates() {
        let mut router = MultiBackendLlmService::new();
        router.register("a", Arc::new(FakeSvc { id: "a".into(), models: vec![model("a", "m1")] }));
        router.register("b", Arc::new(FakeSvc { id: "b".into(), models: vec![model("b", "m2")] }));
        let all = router.list_models().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn router_claims_backend_for_registered_impls() {
        let mut router = MultiBackendLlmService::new();
        router.register("a", Arc::new(FakeSvc { id: "a".into(), models: vec![] }));
        assert!(router.claims_backend("a"));
        assert!(!router.claims_backend("b"));
    }
}
```

- [ ] **Step 2: Run**

- [ ] **Step 3: Implement**

```rust
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::error::LlmError;
use super::service::LlmService;
use super::types::*;

pub struct MultiBackendLlmService {
    impls: Vec<(String, Arc<dyn LlmService>)>,
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
        self.impls.iter()
            .find(|(_, svc)| svc.claims_backend(backend_id))
            .map(|(_, svc)| svc)
    }
}

impl Default for MultiBackendLlmService {
    fn default() -> Self { Self::new() }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl LlmService for MultiBackendLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        match self.find(&req.backend_id) {
            Some(svc) => svc.chat_stream(req, cancel).await,
            None => {
                let id = req.backend_id.clone();
                Box::pin(stream::once(async move {
                    Err(LlmError::InvalidRequest(format!("unknown backend_id: {}", id)))
                }))
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let mut all = Vec::new();
        for (label, svc) in &self.impls {
            match svc.list_models().await {
                Ok(models) => all.extend(models),
                Err(e) => tracing::warn!(backend = %label, error = %e, "list_models failed"),
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
            None => Box::pin(stream::once(async move {
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

Add `tracing` dep if missing.

In `mod.rs`:
```rust
pub mod router;
pub use router::MultiBackendLlmService;
```

- [ ] **Step 4: Run** — 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/wafer-core/src/interfaces/llm/router.rs crates/wafer-core/src/interfaces/llm/mod.rs
git commit -m "feat(wafer-core): MultiBackendLlmService router dispatching by claims_backend"
```

---

## Phase C: `wafer-run/llm` service block

### Task C1: Message-protocol handler for LlmBlock

**Files:**
- Create: `wafer-run/crates/wafer-core/src/service_blocks/llm/mod.rs`
- Create: `wafer-run/crates/wafer-core/src/service_blocks/llm/handler.rs`
- Modify: `wafer-run/crates/wafer-core/src/service_blocks/mod.rs` (register `llm`)

- [ ] **Step 1: Failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // A fake LlmService that returns a canned chat stream.
    struct FakeLlm;

    #[async_trait::async_trait]
    impl LlmService for FakeLlm {
        async fn chat_stream(&self, _req: ChatRequest, _cancel: CancellationToken)
            -> BoxStream<'static, Result<ChatChunk, LlmError>>
        {
            Box::pin(futures::stream::iter(vec![
                Ok(ChatChunk { delta: ChunkDelta::Text("hello ".into()), finish_reason: None, usage: None }),
                Ok(ChatChunk { delta: ChunkDelta::Text("world".into()), finish_reason: Some(FinishReason::Stop), usage: None }),
            ]))
        }
        async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
            Ok(vec![ModelInfo { backend_id: "fake".into(), model_id: "m".into(), display_name: "M".into(), capabilities: Default::default() }])
        }
        async fn status(&self, _: &str, _: &str) -> Result<ModelStatus, LlmError> {
            Ok(ModelStatus { state: ModelState::Ready, progress: None })
        }
        fn claims_backend(&self, id: &str) -> bool { id == "fake" }
    }

    #[tokio::test]
    async fn handle_chat_streams_events() {
        let svc: Arc<dyn LlmService> = Arc::new(FakeLlm);
        let req_json = serde_json::to_vec(&ChatRequest {
            backend_id: "fake".into(),
            model: "m".into(),
            messages: vec![],
            params: Default::default(),
            tools: vec![],
            extra: serde_json::Value::Null,
        }).unwrap();
        let msg = Message { kind: "llm.chat".into(), meta: vec![] };
        let input = InputStream::from_bytes(req_json);
        let output = handle_message(svc, &dummy_ctx(), msg, input).await;
        let events: Vec<_> = output.collect().await;
        // Expect: Meta (text/event-stream), Chunk("hello "), Chunk("world"), Complete
        // (or similar — confirm exact protocol matches impl choice)
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Chunk(_))));
        assert!(events.last().map(|e| e.is_terminal()).unwrap_or(false));
    }

    #[tokio::test]
    async fn handle_list_models_returns_json() { ... }

    // Helper for tests:
    fn dummy_ctx() -> impl wafer_block::context::Context { /* test fixture */ }
}
```

(Flesh out tests for each operation kind: `llm.chat`, `llm.list_models`, `llm.status`, `llm.load_model`, `llm.unload_model`.)

- [ ] **Step 2: Run**

- [ ] **Step 3: Implement `handle_message`**

```rust
use std::sync::Arc;
use wafer_block::context::Context;
use wafer_block::core_types::{Message, MetaEntry};
use wafer_block::streams::{InputStream, OutputStream, StreamEvent};

use crate::interfaces::llm::{ChatRequest, LlmError, LlmService};

pub async fn handle_message(
    service: Arc<dyn LlmService>,
    _ctx: &dyn Context,
    msg: Message,
    input: InputStream,
) -> OutputStream {
    match msg.kind.as_str() {
        "llm.chat" => handle_chat(service, input).await,
        "llm.list_models" => handle_list_models(service).await,
        "llm.status" => handle_status(service, input).await,
        "llm.load_model" => handle_load_model(service, input).await,
        "llm.unload_model" => handle_unload_model(service, input).await,
        other => OutputStream::error(wafer_block::core_types::WaferError {
            code: wafer_block::core_types::ErrorCode::InvalidRequest,
            message: format!("unknown llm operation: {}", other),
            meta: vec![],
        }),
    }
}

async fn handle_chat(service: Arc<dyn LlmService>, input: InputStream) -> OutputStream {
    let body = input.collect_to_bytes().await;
    let cancel = input.cancel_token().clone(); // (adjust — InputStream is moved by collect_to_bytes; rework to capture cancel first)
    let req: ChatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return OutputStream::error(wafer_block::core_types::WaferError {
                code: wafer_block::core_types::ErrorCode::InvalidRequest,
                message: format!("invalid ChatRequest JSON: {}", e),
                meta: vec![],
            });
        }
    };

    let (stream, sink, out_cancel) = OutputStream::new_streaming();

    tokio::spawn(async move {
        // Declare SSE content-type so downstream HTTP adapter emits streaming body.
        if sink.send_meta(MetaEntry {
            key: "Content-Type".into(),
            value: "text/event-stream".into(),
        }).await.is_err() {
            return;
        }

        let mut chat = service.chat_stream(req, out_cancel).await;
        use futures::StreamExt;
        while let Some(result) = chat.next().await {
            match result {
                Ok(chunk) => {
                    let payload = match serde_json::to_vec(&chunk) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    if sink.send_chunk(payload).await.is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = sink.error(wafer_block::core_types::WaferError {
                        code: wafer_block::core_types::ErrorCode::Internal,
                        message: e.to_string(),
                        meta: vec![],
                    }).await;
                    return;
                }
            }
        }
        let _ = sink.complete(vec![]).await;
    });

    stream
}

async fn handle_list_models(service: Arc<dyn LlmService>) -> OutputStream {
    match service.list_models().await {
        Ok(models) => match serde_json::to_vec(&models) {
            Ok(body) => OutputStream::respond(body),
            Err(e) => OutputStream::error(wafer_block::core_types::WaferError {
                code: wafer_block::core_types::ErrorCode::Internal,
                message: format!("serialize failed: {}", e),
                meta: vec![],
            }),
        },
        Err(e) => OutputStream::error(wafer_block::core_types::WaferError {
            code: wafer_block::core_types::ErrorCode::Internal,
            message: e.to_string(),
            meta: vec![],
        }),
    }
}

// handle_status: expects JSON {"backend_id": "...", "model_id": "..."} in body.
// handle_load_model: same shape, returns streaming LoadProgress events.
// handle_unload_model: same shape, returns buffered JSON.
// (Implement each following the same pattern.)
```

Adjust capture of `cancel` from `InputStream` — the current `collect_to_bytes(self)` consumes the stream. Either:
- Clone the token before collect: `let cancel = input.cancel_token().clone(); let body = input.collect_to_bytes().await;`
- Or extract body + cancel via a helper that returns both.

- [ ] **Step 4: Run** — all handler tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/wafer-core/src/service_blocks/llm/
git commit -m "feat(wafer-core): LlmBlock message handler dispatches llm.* operations"
```

### Task C2: LlmBlock wrapper + register_with

**Files:**
- Modify: `wafer-run/crates/wafer-core/src/service_blocks/llm/mod.rs`

- [ ] **Step 1–5: Same TDD pattern**

```rust
use async_trait::async_trait;
use std::sync::Arc;
use wafer_block::block::Block;
use wafer_block::context::Context;
use wafer_block::core_types::Message;
use wafer_block::registry::BlockRegistry;
use wafer_block::streams::{InputStream, OutputStream};
use wafer_block::types::BlockInfo;

use crate::interfaces::llm::LlmService;

pub mod handler;
pub use handler::handle_message;

pub struct LlmBlock {
    service: Arc<dyn LlmService>,
}

impl LlmBlock {
    pub fn new(service: Arc<dyn LlmService>) -> Self { Self { service } }
}

pub fn register_with(w: &mut dyn BlockRegistry, service: Arc<dyn LlmService>) {
    w.register_block("wafer-run/llm".into(), Arc::new(LlmBlock::new(service)))
        .expect("register wafer-run/llm");
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Block for LlmBlock {
    async fn handle(
        &self,
        ctx: &dyn Context,
        msg: Message,
        input: InputStream,
    ) -> OutputStream {
        handle_message(self.service.clone(), ctx, msg, input).await
    }

    fn info(&self) -> BlockInfo {
        BlockInfo::builder("wafer-run/llm")
            .description("LLM service block — chat, list_models, load/unload")
            .build()
    }
}
```

Test: register with a fake runtime, send a `llm.list_models` message, assert buffered response JSON.

Commit: `feat(wafer-core): wafer-run/llm service block wrapper`.

---

## Phase D: `ProviderLlmService` impl (solobase-core)

### Task D1: Scaffold provider module + ProviderConfig types

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/mod.rs`
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/config.rs`

- [ ] **Step 1: Failing tests** for `ProviderConfig`, `ProviderProtocol`, `ProviderLlmInner` state type.

- [ ] **Step 3: Implement**

```rust
// config.rs
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
    OpenAiCompatible,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub protocol: ProviderProtocol,
    pub endpoint: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
}
```

Commit: `feat(solobase-core): ProviderConfig / ProviderProtocol types for LLM providers`.

### Task D2: OpenAI protocol client

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/openai.rs`

Implements `chat_stream_openai(config, req, cancel) -> BoxStream<Result<ChatChunk, LlmError>>`.

- [ ] **Step 1: Failing test** using `wiremock` or `httpmock` to stub OpenAI's `/v1/chat/completions` SSE response.

- [ ] **Step 3: Implement**

```rust
use futures::stream::{self, BoxStream, StreamExt};
use reqwest::Client;
use tokio_util::sync::CancellationToken;
use wafer_core::interfaces::llm::*;

pub async fn chat_stream_openai(
    config: &ProviderConfig,
    req: ChatRequest,
    cancel: CancellationToken,
    client: &Client,
) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
    let body = serde_json::json!({
        "model": req.model,
        "messages": convert_messages_to_openai(&req.messages),
        "stream": true,
        "max_tokens": req.params.max_tokens,
        "temperature": req.params.temperature,
        "top_p": req.params.top_p,
        "stop": req.params.stop_sequences,
        "seed": req.params.seed,
        "tools": if req.tools.is_empty() { None } else { Some(convert_tools_to_openai(&req.tools)) },
        // Merge extra params
    });

    let url = format!("{}/chat/completions", config.endpoint.trim_end_matches('/'));
    let mut request = client.post(&url).json(&body);
    if let Some(key) = &config.api_key {
        request = request.bearer_auth(key);
    }

    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => return Box::pin(stream::once(async move {
            Err(LlmError::Network(e.to_string()))
        })),
    };

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Box::pin(stream::once(async { Err(LlmError::Unauthorized) }));
    }
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Box::pin(stream::once(async { Err(LlmError::RateLimited) }));
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Box::pin(stream::once(async move {
            Err(LlmError::BackendError(format!("{}: {}", status, body)))
        }));
    }

    // Parse SSE stream: each "data: {...}" line is a JSON chunk with delta content.
    let byte_stream = response.bytes_stream();
    Box::pin(parse_openai_sse(byte_stream, cancel))
}

fn parse_openai_sse<S>(
    bytes: S,
    cancel: CancellationToken,
) -> impl futures::Stream<Item = Result<ChatChunk, LlmError>> + Send + 'static
where
    S: futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    // Buffer incoming bytes, split on \n\n boundaries, parse "data: ..." lines.
    // For each JSON chunk from OpenAI's SSE:
    //   - "data: [DONE]" → end of stream
    //   - "data: {...}" → parse ChatCompletionChunk → map to ChatChunk
    // Implementation detail: use tokio_util codec or a hand-rolled line-buffer.
    use tokio_stream::StreamExt as TsStreamExt;
    let mut buf = Vec::new();
    TsStreamExt::flat_map(bytes, move |chunk_result| {
        let mut chunks = Vec::new();
        if cancel.is_cancelled() {
            return stream::iter(vec![Err(LlmError::Cancelled)]);
        }
        match chunk_result {
            Ok(bytes) => {
                buf.extend_from_slice(&bytes);
                while let Some(boundary) = find_sse_boundary(&buf) {
                    let frame = buf.drain(..boundary + 2).collect::<Vec<_>>();
                    if let Some(evt) = parse_sse_frame(&frame) {
                        chunks.push(evt);
                    }
                }
            }
            Err(e) => chunks.push(Err(LlmError::Network(e.to_string()))),
        }
        stream::iter(chunks)
    })
}

// Helper stubs — fill in:
fn find_sse_boundary(buf: &[u8]) -> Option<usize> { /* find b"\n\n" */ unimplemented!() }
fn parse_sse_frame(frame: &[u8]) -> Option<Result<ChatChunk, LlmError>> { /* parse data: line */ unimplemented!() }
fn convert_messages_to_openai(messages: &[ChatMessage]) -> serde_json::Value { unimplemented!() }
fn convert_tools_to_openai(tools: &[ToolDefinition]) -> serde_json::Value { unimplemented!() }
```

(Write thorough tests using `wiremock` stubs — assert correct request encoding, correct chunk decoding, cancellation aborts the stream, 401 maps to `Unauthorized`, 429 to `RateLimited`.)

- [ ] **Step 4–5:** Run, commit.

Commit: `feat(solobase-core): OpenAI SSE chat_stream with cancel + error mapping`.

### Task D3: Anthropic protocol client

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/anthropic.rs`

Anthropic's API is different: POST to `/v1/messages`, SSE events are `content_block_start`, `content_block_delta`, `message_delta`, etc. Tool calls and prompt caching have their own shapes. Document the event mapping in the implementation.

- [ ] **Step 1–5: Same TDD pattern** as Task D2 but for Anthropic.

Commit: `feat(solobase-core): Anthropic messages API chat_stream`.

### Task D4: OpenAI-compatible variant

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/openai_compatible.rs`

Same wire shape as OpenAI (the variant exists to express which endpoints the operator has validated). Reuses `openai.rs` encoder/decoder. Differences:
- `api_key` is optional (local servers typically have no auth).
- Model list comes from endpoint's `/v1/models` discovery (not hardcoded).
- Graceful degradation for servers missing features (e.g., no tool-call support → silently skip `tools` field in the request).

- [ ] **Step 1–5: TDD with wiremock simulating Ollama / llama-server responses.**

Commit: `feat(solobase-core): OpenAI-compatible provider variant for local + third-party hosts`.

### Task D5: Model discovery via `/v1/models`

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/discovery.rs`

Queries `{endpoint}/models`, maps to `Vec<ModelInfo>` with capability inference (tools / vision / context size from model name heuristics + optional JSON metadata).

- [ ] **TDD** with wiremock.

Commit: `feat(solobase-core): discover provider models via /v1/models`.

### Task D6: `ProviderLlmService` struct + `LlmService` impl

**Files:**
- Create: `solobase/crates/solobase-core/src/blocks/llm/providers/service.rs`

```rust
pub struct ProviderLlmService {
    inner: Arc<RwLock<ProviderLlmInner>>,
    client: reqwest::Client,
}

struct ProviderLlmInner {
    providers: HashMap<String, ProviderConfig>,
    cached_models: HashMap<String, Vec<ModelInfo>>,
}

impl ProviderLlmService {
    pub fn new() -> Self { ... }
    pub async fn configure(&self, providers: Vec<ProviderConfig>) -> Result<(), LlmError> { ... }
    pub async fn discover_models(&self, provider_name: &str) -> Result<Vec<ModelInfo>, LlmError> { ... }
}

#[async_trait]
impl LlmService for ProviderLlmService {
    async fn chat_stream(&self, req: ChatRequest, cancel: CancellationToken)
        -> BoxStream<'static, Result<ChatChunk, LlmError>>
    {
        let inner = self.inner.read().await;
        let provider = match inner.providers.get(&req.backend_id) {
            Some(p) => p.clone(),
            None => return Box::pin(stream::once(async move {
                Err(LlmError::InvalidRequest(format!("unknown provider: {}", req.backend_id)))
            })),
        };
        drop(inner);

        match provider.protocol {
            ProviderProtocol::OpenAi => openai::chat_stream_openai(&provider, req, cancel, &self.client).await,
            ProviderProtocol::Anthropic => anthropic::chat_stream_anthropic(&provider, req, cancel, &self.client).await,
            ProviderProtocol::OpenAiCompatible => openai_compatible::chat_stream(&provider, req, cancel, &self.client).await,
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let inner = self.inner.read().await;
        let mut all = Vec::new();
        for (_, models) in &inner.cached_models {
            all.extend(models.clone());
        }
        Ok(all)
    }

    async fn status(&self, backend_id: &str, _model_id: &str) -> Result<ModelStatus, LlmError> {
        let inner = self.inner.read().await;
        if inner.providers.contains_key(backend_id) {
            Ok(ModelStatus { state: ModelState::Ready, progress: None })
        } else {
            Err(LlmError::ModelNotFound(backend_id.into()))
        }
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        // Synchronous — read lock or use arc-swap of a smaller structure if contention matters.
        futures::executor::block_on(async {
            self.inner.read().await.providers.contains_key(backend_id)
        })
    }
}
```

(Tune the `claims_backend` impl to avoid `block_on` if needed — consider `parking_lot::RwLock` or an `ArcSwap<HashMap>` if router dispatch becomes hot.)

- [ ] **TDD** with end-to-end test using two wiremock servers acting as OpenAI and Anthropic.

Commit: `feat(solobase-core): ProviderLlmService unifies OpenAI/Anthropic/compat impls`.

---

## Phase E: `BrowserLlmService` (solobase-web)

### Task E1: WebLLM JS facade via wasm-bindgen

**Files:**
- Create: `solobase/crates/solobase-web/src/llm/mod.rs`
- Create: `solobase/crates/solobase-web/src/llm/webllm.rs`

Wrap the `@mlc-ai/web-llm` JS SDK via `wasm-bindgen`. Expose `WebLlmEngine::create`, `chat_completions_create`, model loading with progress callback.

- [ ] **Step 1–5: Browser-test via `wasm-bindgen-test` with `#[cfg(target_arch = "wasm32")]`**

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/webllm-loader.js")]
extern "C" {
    type WebLlmEngine;

    #[wasm_bindgen(js_name = createEngine)]
    fn create_engine(model_id: &str, progress_cb: &js_sys::Function) -> JsValue; // returns Promise<WebLlmEngine>

    #[wasm_bindgen(method, js_name = chat)]
    fn chat(this: &WebLlmEngine, request_json: &str) -> JsValue; // returns AsyncIterator
}
```

Create a minimal JS loader file that imports from jsdelivr.

Commit: `feat(solobase-web): wasm-bindgen facade for WebLLM`.

### Task E2: BrowserLlmService impl

**Files:**
- Create: `solobase/crates/solobase-web/src/llm/service.rs`

Implements `LlmService` using the wasm-bindgen facade. Key methods:
- `chat_stream` — calls WebLLM's streaming chat API; async iterator → Rust stream via `wasm_bindgen_futures::stream_from_js`.
- `load_model` — triggers `CreateMLCEngine` with progress callback; progress calls become `LoadProgress` events on the returned `BoxStream`.
- `unload_model` — releases the engine.
- `list_models` — hardcoded list of WebLLM-supported models (matches today's `ai-bridge.js` list).
- `claims_backend` — returns `true` for ids matching WebLLM model patterns (e.g., anything containing "q4f32" or in a known model list).

- [ ] **TDD** via `wasm-bindgen-test`.

Commit: `feat(solobase-web): BrowserLlmService implementing LlmService over WebLLM`.

---

## Phase F: Rewrite `suppers-ai/llm` feature block

### Task F1: DB schemas

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/llm/mod.rs` (schema definitions)

Schemas:
- `suppers_ai__llm__providers` — `{ id, name, protocol, endpoint, api_key (encrypted), api_key_var (optional config-var reference), enabled, created_at, updated_at }`
- `suppers_ai__llm__settings` — per-thread provider/model override (kept from today)

Add `lifecycle(Init)` migration logic: if `suppers_ai__provider_llm__providers` exists, translate rows and copy into `suppers_ai__llm__providers`, then drop the old collection.

Commit: `feat(solobase-core): LLM feature block schemas + migration from provider_llm collection`.

### Task F2: HTTP endpoints for chat

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/llm/mod.rs` (or `handlers.rs`)

Endpoints:
- `POST /b/llm/api/chat` → decode body as `ChatRequest`, call `ctx.call_block("wafer-run/llm", ..., InputStream::from_bytes(req_json))`, `collect_buffered`, return one buffered response.
- `POST /b/llm/api/chat/stream` → same call, BUT return the `OutputStream` directly as the block's return value (the HTTP adapter from Spec 1 will detect the SSE content-type and stream).

- [ ] **TDD** via integration tests.

Commit: `feat(solobase-core): chat HTTP endpoints (buffered + streaming SSE)`.

### Task F3: Provider CRUD endpoints + admin UI

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/llm/mod.rs`
- Create/modify: `solobase/crates/solobase-core/src/blocks/llm/pages.rs` (admin UI)

- `GET    /b/llm/api/providers` → list from DB
- `POST   /b/llm/api/providers` (admin-only) → create row + call `provider_svc.configure(new_list)`
- `PATCH  /b/llm/api/providers/:id` → update + reconfigure
- `DELETE /b/llm/api/providers/:id` → delete + reconfigure
- `POST   /b/llm/api/providers/:id/discover` → query endpoint's `/v1/models`, store discovered models on the provider row

Admin UI: maud page with table of providers, add/edit/delete modal forms, test-connection / discover-models actions.

Commit: `feat(solobase-core): provider CRUD HTTP + admin UI`.

### Task F4: Model listing + status + load/unload endpoints

**Files:**
- Modify: same locations

- `GET    /b/llm/api/models` → aggregate from router via `ctx.call_block("wafer-run/llm", llm.list_models, ...)`
- `GET    /b/llm/api/models/:backend_id/:model_id/status` → buffered via llm.status
- `POST   /b/llm/api/models/:backend_id/:model_id/load` → SSE streaming via llm.load_model
- `POST   /b/llm/api/models/:backend_id/:model_id/unload` → buffered via llm.unload_model

Commit: `feat(solobase-core): model listing + load/unload HTTP endpoints`.

### Task F5: Rewrite chat UI

**Files:**
- Modify: `solobase/crates/solobase-core/src/blocks/llm/pages.rs`

Chat page consumes `/b/llm/api/chat/stream` via EventSource. Model picker populated from `/b/llm/api/models`. Thread sidebar integration with `suppers-ai/messages` unchanged.

Commit: `feat(solobase-core): rewrite chat UI to consume SSE streaming endpoint`.

### Task F6: Startup wiring

**Files:**
- Modify: Solobase binary `main` (native), `solobase-web` entry, `solobase-cloudflare` entry.

Each entry constructs `ProviderLlmService`, builds `MultiBackendLlmService`, registers impls (provider + optionally browser), registers `wafer-run/llm` via `wafer_core::service_blocks::llm::register_with`. The feature block's `lifecycle(Init)` loads saved providers from DB and calls `provider_svc.configure(...)`.

Commit: `feat(solobase): wire LlmService impls into router at app startup`.

---

## Phase G: Cleanup + removal of old code

### Task G1: Delete `suppers-ai/provider-llm` block

**Files:**
- Delete: `solobase/crates/solobase-core/src/blocks/provider_llm/` (entire directory)
- Modify: `solobase/crates/solobase-core/src/blocks/mod.rs` (remove `BlockId::ProviderLlm`, factory entry, route table entry)

- [ ] **Step 1: Confirm no references remain**

```bash
rg 'provider_llm|ProviderLlm' --type rust
```

Should only show references in the deletion list.

- [ ] **Step 2–5:** Delete, verify `cargo check`, commit.

Commit: `chore: delete suppers-ai/provider-llm block (consolidated into llm)`.

### Task G2: Delete `suppers-ai/local-llm` block

**Files:**
- Delete: `solobase/crates/solobase-core/src/blocks/local_llm.rs`
- Modify: `solobase/crates/solobase-core/src/blocks/mod.rs`

Commit: `chore: delete suppers-ai/local-llm block (folded into BrowserLlmService)`.

### Task G3: Delete `llm_backend.rs`

**File:**
- Delete: `solobase/crates/solobase-core/src/blocks/llm_backend.rs`

- [ ] **Step 1: Confirm no references**

```bash
rg 'llm_backend' --type rust
```

Commit: `chore: delete llm_backend.rs (types replaced by wafer-core interfaces::llm)`.

### Task G4: Delete `ai-bridge.js` and service-worker local-llm interception

**Files:**
- Delete: `solobase/crates/solobase-web/js/ai-bridge.js`
- Modify: `solobase/crates/solobase-web/js/sw.js` (remove `/b/local-llm/api/` path interception)

- [ ] **Browser test**: confirm chat still streams correctly through the new Rust path.

Commit: `chore: delete ai-bridge.js; streaming now flows through BrowserLlmService`.

### Task G5: Config var cleanup

**Files:**
- Modify: `solobase-core/src/config_vars.rs` or equivalent

Remove obsolete config var declarations:
- `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY`
- `SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY`

(Provider API keys now live in `suppers_ai__llm__providers` rows; config-var reference is optional via `api_key_var` field.)

Commit: `chore(config): remove obsolete provider_llm config vars`.

---

## Phase H: Integration testing

### Task H1: End-to-end streaming test — wiremock backend, real router, real feature block

**Files:**
- Create: `solobase/crates/solobase-core/tests/llm_e2e.rs`

Test: stand up a wiremock server emulating OpenAI; configure a ProviderLlmService with that endpoint; issue `POST /b/llm/api/chat/stream` through the feature block; assert received SSE frames match wiremock's stubbed response.

Commit: `test(solobase-core): e2e streaming chat through full LLM stack`.

### Task H2: Browser end-to-end test for BrowserLlmService

**Files:**
- Create: `solobase/crates/solobase-web/tests/llm_browser.rs`

Browser test via `wasm-bindgen-test`: load a tiny WebLLM model, stream a short completion, assert tokens arrive in order.

Commit: `test(solobase-web): e2e browser streaming chat via WebLLM`.

### Task H3: Provider CRUD persistence test

Test: create provider via POST, list, update, delete; verify DB state + ProviderLlmService reflects each step.

Commit: `test(solobase-core): provider CRUD persistence roundtrip`.

### Task H4: Migration test

Test: seed fixture DB with `suppers_ai__provider_llm__providers` rows; run feature block Init; verify records migrated to new collection, old collection dropped.

Commit: `test(solobase-core): migration from provider_llm collection`.

---

## Phase I: Final verification

### Task I1: Full workspace test run

- [ ] Run both workspaces' test suites end-to-end.

```bash
cd /home/joris/Programs/suppers-ai/workspace/wafer-run && cargo test --workspace
cd /home/joris/Programs/suppers-ai/workspace/solobase && cargo test --workspace
```

- [ ] Browser tests via `wasm-pack test --chrome --headless crates/solobase-web`.

### Task I2: Manual exercise

- Start native Solobase binary, exercise admin UI: add OpenAI provider, add a local Ollama provider, chat with both, observe streaming works and model picker shows both provider sets.
- Serve solobase-web, exercise browser chat with WebLLM, confirm tokens stream.
- Deploy to CF Workers preview via wrangler, exercise chat (ProviderLlmService only, no BrowserLlmService in CF context).

### Task I3: Docs

Document:
- The `LlmService` trait surface + extensibility guarantees
- Setting up a local LLM server + configuring it as an OpenAI-compatible provider in the admin UI
- How to add a new protocol variant (new `ProviderProtocol` enum arm, new `openai.rs`-style client module)

Commit: `docs: LlmService architecture + local LLM setup guide`.

---

## Migration reference (quick sheet)

### Removed blocks / files

- `solobase-core/src/blocks/provider_llm/` (dir) — DELETED
- `solobase-core/src/blocks/local_llm.rs` — DELETED
- `solobase-core/src/blocks/llm_backend.rs` — DELETED
- `solobase-web/js/ai-bridge.js` — DELETED

### New structure

- `wafer-core/src/interfaces/llm/` — trait + types + router
- `wafer-core/src/service_blocks/llm/` — service block wrapper
- `solobase-core/src/blocks/llm/providers/` — ProviderLlmService + protocol clients
- `solobase-core/src/blocks/llm/` — rewritten feature block (chat UI + provider UI + endpoints)
- `solobase-web/src/llm/` — BrowserLlmService + WebLLM facade

### Provider config migration

DB collection renamed: `suppers_ai__provider_llm__providers` → `suppers_ai__llm__providers`. One-shot migration on first startup of the new version.

### Config var changes

- Removed: `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY`, `SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY`
- API keys stored per-provider in DB (encrypted); optional config-var reference via `api_key_var` field
