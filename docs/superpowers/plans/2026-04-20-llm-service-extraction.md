# LLM Service Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the browser WebLLM `BrowserLlmService` into `solobase-browser::llm`, ship it behind a SW↔page postMessage bridge (reusing the existing `loadAsset` correlation-ID pattern in `solobase-browser/js/bridge.js`), lift the solobase-web "reject tool-calls" v0 guard, and migrate gizza-ai to consume the framework service.

**Architecture:** One `BrowserLlmService` in `solobase-browser` implementing `wafer_core::interfaces::llm::service::LlmService`. Pure-Rust OpenAI codec (`openai_codec.rs`) with lifted unit tests. JS engine runs in the page; SW↔page bridge multiplexes streams via correlation IDs (same pattern as `loadAsset`). Two PRs: (1) solobase-browser adds the module, solobase-web switches to it, old files deleted. (2) gizza-ai deletes its bridge JS, rewrites `agent.rs` against the trait.

**Tech Stack:** Rust (wasm32), wasm-bindgen, futures mpsc, `@mlc-ai/web-llm` 0.2.74, service-worker postMessage.

**Spec:** `docs/superpowers/specs/2026-04-20-llm-service-extraction-design.md`

---

## File structure

**Crates/paths affected (solobase repo):**

- Create: `crates/solobase-browser/src/llm/mod.rs` — pub re-exports
- Create: `crates/solobase-browser/src/llm/openai_codec.rs` — pure-Rust encode/decode + tool-call parsing (lifted + extended from solobase-web)
- Create: `crates/solobase-browser/src/llm/catalog.rs` — `ModelCatalog` + default list
- Create: `crates/solobase-browser/src/llm/bridge.rs` — Rust wasm-bindgen bindings to the JS bridge calls
- Create: `crates/solobase-browser/src/llm/service.rs` — `BrowserLlmService` + `LlmService` impl
- Modify: `crates/solobase-browser/src/lib.rs` — `pub mod llm;`
- Modify: `crates/solobase-browser/js/bridge.js` — add `llmCreateEngine`/`llmChatStream`/`llmNextChunk`/`llmUnload` + `_completeLlmStream` hook (parallels `loadAsset` + `_completeAssetLoad`)
- Modify: `crates/solobase-browser/src/bridge.rs` — extern "C" bindings for the new JS functions
- Create: `crates/solobase-browser/js/webllm-engine.js` — page-side engine, listens for SW postMessages, runs WebLLM
- Modify: `crates/solobase-browser/src/assets.rs` — serve the new `webllm-engine.js` at hashed path
- Modify: `crates/solobase-browser/bin/export-assets.rs` or `build.rs` — include the new JS in the asset export
- Modify: `crates/solobase-browser/src/lib.rs` (page-bootstrap JS) — include `<script type="module" src="/webllm-engine-<hash>.js">` in the HTML shell when an `LlmService` is registered
- Delete: `crates/solobase-web/src/llm.rs`
- Delete: `crates/solobase-web/js/webllm-engine.js`
- Modify: `crates/solobase-web/src/lib.rs` — drop `pub mod llm;`; register framework service

**Paths affected (gizza-ai repo, separate PR):**

- Delete: `site/ai-bridge.js`
- Delete: `site/sw-llm-bridge.js`
- Modify: `src/blocks/agent.rs` — use `LlmService` via wafer runtime instead of `localLlmChatStream`
- Modify: `site/index.html` — drop script tags for deleted files
- Modify: `Cargo.toml` — bump solobase-browser dep
- Modify: wherever gizza registers services — add `.llm_service("browser", BrowserLlmService::new())`

---

## Task 1: Skeleton llm module

**Files:**
- Create: `crates/solobase-browser/src/llm/mod.rs`
- Modify: `crates/solobase-browser/src/lib.rs`
- Test: via `cargo check -p solobase-browser --target wasm32-unknown-unknown`

- [ ] **Step 1.1: Create module skeleton**

Write `crates/solobase-browser/src/llm/mod.rs`:
```rust
//! Browser LLM — `LlmService` impl driving WebLLM's MLCEngine via a
//! SW↔page postMessage bridge. See `docs/superpowers/specs/2026-04-20-
//! llm-service-extraction-design.md`.

pub mod bridge;
pub mod catalog;
pub mod openai_codec;
pub mod service;

pub use catalog::{default_catalog, ModelCatalog};
pub use service::BrowserLlmService;
```

- [ ] **Step 1.2: Add empty child modules so the skeleton compiles**

Write `crates/solobase-browser/src/llm/bridge.rs`:
```rust
//! SW-side bridge to page-side WebLLM engine. Populated in Task 5.
```

Write `crates/solobase-browser/src/llm/catalog.rs`:
```rust
//! Default browser model catalog. Populated in Task 3.

use wafer_core::interfaces::llm::service::ModelInfo;

/// Browser-side model catalog. Wraps a `Vec<ModelInfo>` and can be
/// overridden by consumers via `BrowserLlmService::with_catalog`.
#[derive(Debug, Clone, Default)]
pub struct ModelCatalog {
    models: Vec<ModelInfo>,
}

impl ModelCatalog {
    pub fn new(models: Vec<ModelInfo>) -> Self {
        Self { models }
    }
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

pub fn default_catalog() -> ModelCatalog {
    ModelCatalog::new(Vec::new())
}
```

Write `crates/solobase-browser/src/llm/openai_codec.rs`:
```rust
//! Pure-Rust OpenAI chat-completion codec. Populated in Task 2.
```

Write `crates/solobase-browser/src/llm/service.rs`:
```rust
//! `BrowserLlmService` — populated in Task 6.

use crate::llm::catalog::{default_catalog, ModelCatalog};

pub struct BrowserLlmService {
    catalog: ModelCatalog,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self { catalog: default_catalog() }
    }

    pub fn with_catalog(catalog: ModelCatalog) -> Self {
        Self { catalog }
    }

    #[allow(dead_code)]
    pub(crate) fn catalog(&self) -> &ModelCatalog {
        &self.catalog
    }
}

impl Default for BrowserLlmService {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 1.3: Expose module from crate root**

Edit `crates/solobase-browser/src/lib.rs`, add `pub mod llm;` near the other `pub mod` lines.

- [ ] **Step 1.4: Verify it compiles**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: clean build.

- [ ] **Step 1.5: Commit**

```bash
git add crates/solobase-browser/src/llm crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): add llm module skeleton"
```

---

## Task 2: Port openai_codec with tests (tool-calls first-class)

**Files:**
- Modify: `crates/solobase-browser/src/llm/openai_codec.rs`

The solobase-web `src/llm.rs` has three pure-Rust functions with ~12 unit tests: `messages_to_openai_json`, `chat_chunk_from_openai_chunk`, `parse_finish_reason`, `parse_usage`. We lift them verbatim, remove the "reject tool-calls" / "reject multimodal-Parts" guards, add tool-call encode/decode, and preserve all existing tests.

- [ ] **Step 2.1: Write the codec with tool-call support**

Write `crates/solobase-browser/src/llm/openai_codec.rs`:
```rust
//! Pure-Rust encode/decode between `wafer_core::interfaces::llm::service`
//! types and OpenAI chat-completion wire format. Browser-agnostic — no
//! `wasm_bindgen`, runs on native for tests.

use wafer_core::interfaces::llm::service::{
    ChatChunk, ChatContent, ChatMessage, ChatRole, FinishReason, LlmError, ToolCall,
    ToolDefinition, TokenUsage,
};

/// Encode `ChatMessage`s + `ToolDefinition`s into the OpenAI chat-completion
/// wire body. Returns the full JSON body `{messages, tools?}` string.
///
/// Tool calls on assistant messages, tool-role messages with `tool_call_id`,
/// and tool definitions in `tools` are all forwarded. Multimodal content
/// (`ChatContent::Parts`) is rejected — browser backend is text-only.
pub fn encode_request_body(
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
) -> Result<String, LlmError> {
    let mut out_msgs = Vec::with_capacity(messages.len());
    for m in messages {
        let role = match m.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
            _ => return Err(LlmError::BackendError("webllm: unknown chat role".into())),
        };
        let content = match &m.content {
            ChatContent::Text(s) => s.clone(),
            ChatContent::Parts(_) => {
                return Err(LlmError::BackendError(
                    "webllm: multimodal content not supported".into(),
                ));
            }
            _ => {
                return Err(LlmError::BackendError(
                    "webllm: unknown content variant".into(),
                ));
            }
        };
        let mut obj = serde_json::Map::new();
        obj.insert("role".into(), serde_json::Value::String(role.into()));
        obj.insert("content".into(), serde_json::Value::String(content));
        if let Some(tc_id) = &m.tool_call_id {
            obj.insert(
                "tool_call_id".into(),
                serde_json::Value::String(tc_id.clone()),
            );
        }
        if !m.tool_calls.is_empty() {
            let tcs: Vec<serde_json::Value> = m
                .tool_calls
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": serde_json::to_string(&tc.arguments)
                                .unwrap_or_else(|_| "{}".into()),
                        },
                    })
                })
                .collect();
            obj.insert("tool_calls".into(), serde_json::Value::Array(tcs));
        }
        out_msgs.push(serde_json::Value::Object(obj));
    }

    let mut body = serde_json::Map::new();
    body.insert("messages".into(), serde_json::Value::Array(out_msgs));
    if !tools.is_empty() {
        let defs: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    },
                })
            })
            .collect();
        body.insert("tools".into(), serde_json::Value::Array(defs));
    }
    serde_json::to_string(&serde_json::Value::Object(body))
        .map_err(|e| LlmError::BackendError(format!("request encode: {e}")))
}

/// Accumulator for tool-call deltas streamed across multiple chunks.
///
/// OpenAI streams tool calls as `{index, id?, function: {name?, arguments?}}`
/// deltas across many chunks. We buffer them by `index` and emit a complete
/// `ToolCall` only when the stream's finish reason is `tool_calls`.
#[derive(Debug, Default)]
pub struct ToolCallAccumulator {
    partial: Vec<PartialToolCall>,
}

#[derive(Debug, Default, Clone)]
struct PartialToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments_str: String,
}

impl ToolCallAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a tool-call delta frame from an OpenAI streaming chunk.
    pub fn feed(&mut self, delta: &serde_json::Value) {
        let Some(arr) = delta.as_array() else { return };
        for entry in arr {
            let Some(index) = entry.get("index").and_then(|v| v.as_u64()) else { continue };
            let idx = index as usize;
            if self.partial.len() <= idx {
                self.partial.resize(idx + 1, PartialToolCall::default());
            }
            let p = &mut self.partial[idx];
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                p.id = Some(id.to_string());
            }
            if let Some(func) = entry.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    p.name = Some(name.to_string());
                }
                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                    p.arguments_str.push_str(args);
                }
            }
        }
    }

    /// Consume the accumulator and produce `ToolCall`s. Arguments strings
    /// are parsed as JSON; malformed JSON becomes a `LlmError`.
    pub fn finish(self) -> Result<Vec<ToolCall>, LlmError> {
        let mut out = Vec::with_capacity(self.partial.len());
        for p in self.partial {
            let id = p.id.unwrap_or_default();
            let name = p.name.unwrap_or_default();
            let arguments: serde_json::Value = if p.arguments_str.is_empty() {
                serde_json::Value::Object(Default::default())
            } else {
                serde_json::from_str(&p.arguments_str).map_err(|e| {
                    LlmError::BackendError(format!("tool-call arguments parse: {e}"))
                })?
            };
            out.push(ToolCall { id, name, arguments });
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.partial.is_empty()
    }
}

/// Parse a single OpenAI-format streaming chunk.
///
/// Returns `Ok(ParsedChunk::TextDelta)` for content deltas,
/// `Ok(ParsedChunk::ToolCallDelta)` for tool-call deltas (consumer feeds
/// these into a `ToolCallAccumulator`), `Ok(ParsedChunk::Finish)` for
/// terminal frames, or `Ok(ParsedChunk::Noop)` for empty/role-only frames.
#[derive(Debug)]
pub enum ParsedChunk {
    Noop,
    TextDelta(ChatChunk),
    ToolCallDelta(serde_json::Value),
    Finish {
        reason: FinishReason,
        usage: Option<TokenUsage>,
    },
}

pub fn parse_chunk(s: &str) -> Result<ParsedChunk, LlmError> {
    let v: serde_json::Value = serde_json::from_str(s)
        .map_err(|e| LlmError::BackendError(format!("chunk parse: {e}")))?;

    let choice = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first());

    let content = choice
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string);

    let tool_calls_delta = choice
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("tool_calls"))
        .cloned();

    let finish_reason = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|f| f.as_str())
        .and_then(parse_finish_reason);

    let usage = v.get("usage").and_then(parse_usage);

    if let Some(text) = content {
        if text.is_empty() {
            if let Some(reason) = finish_reason {
                return Ok(ParsedChunk::Finish { reason, usage });
            }
            return Ok(ParsedChunk::Noop);
        }
        return Ok(ParsedChunk::TextDelta(ChatChunk::text(text)));
    }

    if let Some(delta) = tool_calls_delta {
        return Ok(ParsedChunk::ToolCallDelta(delta));
    }

    if let Some(reason) = finish_reason {
        return Ok(ParsedChunk::Finish { reason, usage });
    }

    if let Some(usage) = usage {
        return Ok(ParsedChunk::Finish {
            reason: FinishReason::Stop,
            usage: Some(usage),
        });
    }

    Ok(ParsedChunk::Noop)
}

pub fn parse_finish_reason(s: &str) -> Option<FinishReason> {
    match s {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" => Some(FinishReason::ToolCall),
        "content_filter" => Some(FinishReason::ContentFilter),
        _ => None,
    }
}

pub fn parse_usage(v: &serde_json::Value) -> Option<TokenUsage> {
    let input = v.get("prompt_tokens").and_then(|n| n.as_u64())? as u32;
    let output = v.get("completion_tokens").and_then(|n| n.as_u64())? as u32;
    let mut usage = TokenUsage::default();
    usage.input_tokens = input;
    usage.output_tokens = output;
    Some(usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wafer_core::interfaces::llm::service::ChunkDelta;

    #[test]
    fn text_delta_parsed() {
        let s = r#"{"choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}"#;
        match parse_chunk(s).unwrap() {
            ParsedChunk::TextDelta(chunk) => {
                assert_eq!(chunk.delta, ChunkDelta::Text("hello".into()));
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn empty_content_is_noop() {
        let s = r#"{"choices":[{"index":0,"delta":{"content":""},"finish_reason":null}]}"#;
        assert!(matches!(parse_chunk(s).unwrap(), ParsedChunk::Noop));
    }

    #[test]
    fn finish_stop_terminal() {
        let s = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        match parse_chunk(s).unwrap() {
            ParsedChunk::Finish { reason, .. } => assert_eq!(reason, FinishReason::Stop),
            other => panic!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn finish_with_usage() {
        let s = r#"{
            "choices":[{"index":0,"delta":{},"finish_reason":"stop"}],
            "usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46}
        }"#;
        match parse_chunk(s).unwrap() {
            ParsedChunk::Finish { reason, usage } => {
                assert_eq!(reason, FinishReason::Stop);
                let u = usage.unwrap();
                assert_eq!(u.input_tokens, 12);
                assert_eq!(u.output_tokens, 34);
            }
            other => panic!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_errors() {
        let err = parse_chunk("{not json").unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn empty_object_is_noop() {
        assert!(matches!(parse_chunk("{}").unwrap(), ParsedChunk::Noop));
    }

    #[test]
    fn encode_plain_messages() {
        let msgs = vec![
            ChatMessage::system("sys"),
            ChatMessage::user("hi"),
            ChatMessage::assistant("hello"),
            ChatMessage::tool("call_1", "result"),
        ];
        let json = encode_request_body(&msgs, &[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.get("messages").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0]["role"], "system");
        assert_eq!(arr[3]["tool_call_id"], "call_1");
        assert!(parsed.get("tools").is_none());
    }

    #[test]
    fn encode_rejects_multimodal() {
        let msg_json = serde_json::json!({
            "role": "User",
            "content": { "Parts": [ { "Text": "x" } ] }
        });
        let msg: ChatMessage = serde_json::from_value(msg_json).unwrap();
        let err = encode_request_body(&[msg], &[]).unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn encode_forwards_tool_calls() {
        let msg_json = serde_json::json!({
            "role": "Assistant",
            "content": { "Text": "" },
            "tool_calls": [ {"id": "c1", "name": "get_weather", "arguments": {"q": "Paris"}} ]
        });
        let msg: ChatMessage = serde_json::from_value(msg_json).unwrap();
        let json = encode_request_body(&[msg], &[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let tcs = parsed["messages"][0]["tool_calls"].as_array().unwrap();
        assert_eq!(tcs[0]["id"], "c1");
        assert_eq!(tcs[0]["function"]["name"], "get_weather");
    }

    #[test]
    fn encode_forwards_tool_definitions() {
        let tools = vec![ToolDefinition {
            name: "get_weather".into(),
            description: "current weather".into(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let json = encode_request_body(&[], &tools).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let defs = parsed["tools"].as_array().unwrap();
        assert_eq!(defs[0]["function"]["name"], "get_weather");
        assert_eq!(defs[0]["type"], "function");
    }

    #[test]
    fn tool_call_accumulator_assembles_across_deltas() {
        let mut acc = ToolCallAccumulator::new();
        acc.feed(&serde_json::json!([
            {"index": 0, "id": "call_1", "function": {"name": "get_weather"}}
        ]));
        acc.feed(&serde_json::json!([
            {"index": 0, "function": {"arguments": "{\"q\":"}}
        ]));
        acc.feed(&serde_json::json!([
            {"index": 0, "function": {"arguments": "\"Paris\"}"}}
        ]));
        let tcs = acc.finish().unwrap();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].id, "call_1");
        assert_eq!(tcs[0].name, "get_weather");
        assert_eq!(tcs[0].arguments["q"], "Paris");
    }

    #[test]
    fn tool_call_accumulator_malformed_args_errors() {
        let mut acc = ToolCallAccumulator::new();
        acc.feed(&serde_json::json!([
            {"index": 0, "id": "c", "function": {"name": "f", "arguments": "{not json"}}
        ]));
        let err = acc.finish().unwrap_err();
        assert!(matches!(err, LlmError::BackendError(_)));
    }

    #[test]
    fn finish_reason_maps_standard_values() {
        assert_eq!(parse_finish_reason("stop"), Some(FinishReason::Stop));
        assert_eq!(parse_finish_reason("length"), Some(FinishReason::Length));
        assert_eq!(parse_finish_reason("tool_calls"), Some(FinishReason::ToolCall));
        assert_eq!(parse_finish_reason("content_filter"), Some(FinishReason::ContentFilter));
        assert_eq!(parse_finish_reason("nope"), None);
    }
}
```

- [ ] **Step 2.2: Run codec tests on native target**

Run: `cargo test -p solobase-browser --lib llm::openai_codec`
Expected: all 12 tests pass.

- [ ] **Step 2.3: Commit**

```bash
git add crates/solobase-browser/src/llm/openai_codec.rs
git commit -m "feat(solobase-browser): add openai_codec with tool-call support"
```

---

## Task 3: Port catalog with tests

**Files:**
- Modify: `crates/solobase-browser/src/llm/catalog.rs`

- [ ] **Step 3.1: Flesh out catalog with default model list**

Replace `crates/solobase-browser/src/llm/catalog.rs` with:
```rust
//! Default browser model catalog — WebLLM models with f32 + f16 tiers.

use wafer_core::interfaces::llm::service::{ModelCapabilities, ModelInfo};

/// Browser-side model catalog. Wraps a `Vec<ModelInfo>`.
///
/// Consumers override via `BrowserLlmService::with_catalog(ModelCatalog::new(...))`.
#[derive(Debug, Clone)]
pub struct ModelCatalog {
    models: Vec<ModelInfo>,
}

impl ModelCatalog {
    pub fn new(models: Vec<ModelInfo>) -> Self {
        Self { models }
    }

    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new(default_models())
    }
}

pub fn default_catalog() -> ModelCatalog {
    ModelCatalog::default()
}

fn caps() -> ModelCapabilities {
    // `ModelCapabilities` is `#[non_exhaustive]`; construct via Default.
    let mut c = ModelCapabilities::default();
    c.streaming = true;
    c.tools = true;
    c
}

fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new("webllm", "SmolLM2-1.7B-Instruct-q4f32_1-MLC", "SmolLM2 1.7B (1.1GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "Qwen2.5-1.5B-Instruct-q4f32_1-MLC", "Qwen 2.5 1.5B (1.2GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "gemma-2-2b-it-q4f32_1-MLC", "Gemma 2 2B (1.7GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "Phi-3.5-mini-instruct-q4f32_1-MLC", "Phi 3.5 Mini (2.6GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "Llama-3.2-3B-Instruct-q4f32_1-MLC", "Llama 3.2 3B (2GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "SmolLM2-1.7B-Instruct-q4f16_1-MLC", "SmolLM2 1.7B f16 (1GB)")
            .with_capabilities(caps()),
        ModelInfo::new("webllm", "Qwen2.5-1.5B-Instruct-q4f16_1-MLC", "Qwen 2.5 1.5B f16 (1GB)")
            .with_capabilities(caps()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_has_seven_models() {
        let c = ModelCatalog::default();
        assert_eq!(c.models().len(), 7);
    }

    #[test]
    fn default_catalog_caps_have_tools_enabled() {
        let c = ModelCatalog::default();
        for m in c.models() {
            assert!(m.capabilities.tools, "model {} missing tool capability", m.id);
            assert!(m.capabilities.streaming);
        }
    }

    #[test]
    fn custom_catalog_overrides_default() {
        let c = ModelCatalog::new(vec![
            ModelInfo::new("webllm", "custom-1", "Custom 1").with_capabilities(caps()),
        ]);
        assert_eq!(c.models().len(), 1);
        assert_eq!(c.models()[0].id, "custom-1");
    }
}
```

- [ ] **Step 3.2: Run catalog tests**

Run: `cargo test -p solobase-browser --lib llm::catalog`
Expected: 3 tests pass.

- [ ] **Step 3.3: Commit**

```bash
git add crates/solobase-browser/src/llm/catalog.rs
git commit -m "feat(solobase-browser): add default llm model catalog"
```

---

## Task 4: JS bridge protocol (SW side)

The SW↔page protocol follows the same pattern as `loadAsset` in `bridge.js`:
- Generate a `correlationId`.
- `postMessage` to window clients.
- Register a resolver in a module-level `Map`.
- Main SW script (`sw.js`) routes page replies back via a `globalThis.__solobaseCompleteLlmStream` hook.

**Files:**
- Modify: `crates/solobase-browser/js/bridge.js`

- [ ] **Step 4.1: Add LLM bridge block to bridge.js**

Append this section to `crates/solobase-browser/js/bridge.js`, after the asset-loader section (around line 303, before `// ─── Network (fetch) ───`):

```javascript
// ─── LLM (SW → page postMessage bridge) ─────────────────────────────────────
//
// Mirrors the loadAsset pattern: correlation-id keyed postMessage to a window
// client; resolvers kept in a Map; sw.js routes replies via globalThis hook.
// Streams use an async queue so Rust can `await` one chunk at a time while
// many chunks are buffered in flight.

const _pendingLlmRequests = new Map();   // id -> { resolve, reject } (one-shot: create, unload)
const _activeLlmStreams   = new Map();   // id -> { pushChunk, closeOk, closeErr }

async function _postToWindowClient(payload) {
    const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: false });
    if (clients.length === 0) {
        throw new Error('no active page — open the app in a tab');
    }
    clients[0].postMessage(payload);
}

function _mkId(prefix) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

/**
 * Create + initialise a WebLLM engine on the page. Resolves when loaded or
 * rejects on error.
 * @param {string} modelId
 * @returns {Promise<void>}
 */
export async function llmCreateEngine(modelId) {
    const id = _mkId('llm-create');
    const replyPromise = new Promise((resolve, reject) => {
        _pendingLlmRequests.set(id, { resolve, reject });
    });
    await _postToWindowClient({ type: 'llm-create-engine-request', id, modelId });
    return await replyPromise;
}

/**
 * Unload the engine on the page.
 * @param {string} modelId
 * @returns {Promise<void>}
 */
export async function llmUnloadEngine(modelId) {
    const id = _mkId('llm-unload');
    const replyPromise = new Promise((resolve, reject) => {
        _pendingLlmRequests.set(id, { resolve, reject });
    });
    await _postToWindowClient({ type: 'llm-unload-request', id, modelId });
    return await replyPromise;
}

/**
 * Start a streaming chat completion. Returns a stream id the caller pumps
 * with `llmNextChunk`.
 * @param {string} bodyJson - JSON request body as built by Rust encode_request_body
 * @returns {Promise<string>} stream id
 */
export async function llmChatStream(bodyJson) {
    const id = _mkId('llm-stream');
    const queue = []; // { kind: 'chunk'|'done'|'error', payload }
    const waiters = []; // Array<(frame) => void>
    const pushChunk = (frame) => {
        if (waiters.length > 0) {
            waiters.shift()(frame);
        } else {
            queue.push(frame);
        }
    };
    _activeLlmStreams.set(id, {
        pushChunk,
        closeOk: () => pushChunk({ kind: 'done' }),
        closeErr: (err) => pushChunk({ kind: 'error', payload: err }),
    });
    await _postToWindowClient({ type: 'llm-chat-stream-request', id, body: bodyJson });
    // Rust calls llmNextChunk(id) repeatedly. Stash the queue on the stream
    // entry so nextChunk can dequeue.
    _activeLlmStreams.get(id).queue = queue;
    _activeLlmStreams.get(id).waiters = waiters;
    return id;
}

/**
 * Pull the next frame from a stream. Blocks until a frame arrives.
 * Frame shape: `{kind:'chunk', payload:<openai chunk json string>}` |
 *              `{kind:'done'}` | `{kind:'error', payload:<string>}`.
 * After a terminal frame (done/error) the stream entry is removed.
 * @param {string} id
 * @returns {Promise<string>} JSON-encoded frame
 */
export async function llmNextChunk(id) {
    const stream = _activeLlmStreams.get(id);
    if (!stream) {
        return JSON.stringify({ kind: 'error', payload: 'unknown stream id' });
    }
    let frame;
    if (stream.queue.length > 0) {
        frame = stream.queue.shift();
    } else {
        frame = await new Promise((resolve) => stream.waiters.push(resolve));
    }
    if (frame.kind === 'done' || frame.kind === 'error') {
        _activeLlmStreams.delete(id);
    }
    return JSON.stringify(frame);
}

/**
 * Cancel an in-flight stream.
 * @param {string} id
 */
export async function llmCancelStream(id) {
    const stream = _activeLlmStreams.get(id);
    if (stream) {
        stream.closeErr('cancelled');
    }
    await _postToWindowClient({ type: 'llm-stream-cancel', id });
}

/**
 * Called by sw.js when a page reply arrives. Routes to the pending request
 * or active stream by id.
 *
 * Page → SW message shapes:
 *   { type: 'llm-create-engine-response', id, error? }
 *   { type: 'llm-unload-response', id, error? }
 *   { type: 'llm-chat-stream-chunk', id, chunk }    // chunk = OpenAI chunk JSON string
 *   { type: 'llm-chat-stream-done', id }
 *   { type: 'llm-chat-stream-error', id, error }
 */
export function _completeLlmMessage(msg) {
    if (msg.type === 'llm-create-engine-response' || msg.type === 'llm-unload-response') {
        const pending = _pendingLlmRequests.get(msg.id);
        if (!pending) return;
        _pendingLlmRequests.delete(msg.id);
        if (msg.error) pending.reject(new Error(msg.error));
        else pending.resolve();
        return;
    }
    const stream = _activeLlmStreams.get(msg.id);
    if (!stream) return;
    if (msg.type === 'llm-chat-stream-chunk') {
        stream.pushChunk({ kind: 'chunk', payload: msg.chunk });
    } else if (msg.type === 'llm-chat-stream-done') {
        stream.closeOk();
    } else if (msg.type === 'llm-chat-stream-error') {
        stream.closeErr(msg.error ?? 'unknown error');
    }
}

globalThis.__solobaseCompleteLlmMessage = _completeLlmMessage;
```

- [ ] **Step 4.2: Wire sw.js routing (if not already generic)**

Check `crates/solobase-browser/js/sw.js` (or equivalent top-level SW script). The existing hook for `load-asset-response` calls `globalThis.__solobaseCompleteAssetLoad`. Add a parallel dispatcher for the LLM message types:

```javascript
// In sw.js's main message listener, near the existing asset-loader dispatch:
if (data.type && data.type.startsWith('llm-')) {
    if (typeof globalThis.__solobaseCompleteLlmMessage === 'function') {
        globalThis.__solobaseCompleteLlmMessage(data);
    }
    return;
}
```

Exact location: wherever the existing `if (data.type === 'load-asset-response')` branch lives. Mirror its structure.

- [ ] **Step 4.3: Commit**

```bash
git add crates/solobase-browser/js/bridge.js crates/solobase-browser/js/sw.js
git commit -m "feat(solobase-browser): add SW-side llm bridge (bridge.js + sw.js routing)"
```

---

## Task 5: Page-side engine JS

**Files:**
- Create: `crates/solobase-browser/js/webllm-engine.js`

This runs in the page (window context). Listens for SW postMessages with types `llm-create-engine-request`, `llm-unload-request`, `llm-chat-stream-request`, `llm-stream-cancel`. Posts replies back to the SW registration as `sw.postMessage(...)`.

- [ ] **Step 5.1: Create webllm-engine.js**

Write `crates/solobase-browser/js/webllm-engine.js`:
```javascript
// webllm-engine.js — page-side WebLLM engine + SW postMessage bridge.
//
// Runs in the window (required by WebGPU). Receives requests from the SW,
// runs WebLLM, streams chunks back.

import { CreateMLCEngine } from 'https://cdn.jsdelivr.net/npm/@mlc-ai/web-llm@0.2.74/+esm';

let _engine = null;
let _engineModel = null;
const _activeStreams = new Map(); // id -> AbortController

async function swReply(payload) {
    const reg = await navigator.serviceWorker.ready;
    reg.active?.postMessage(payload);
}

async function handleCreateEngine(msg) {
    try {
        if (_engineModel !== msg.modelId) {
            if (_engine) {
                try { await _engine.unload(); } catch (_e) {}
                _engine = null;
                _engineModel = null;
            }
            _engine = await CreateMLCEngine(msg.modelId, { /* progress swallowed */ });
            _engineModel = msg.modelId;
        }
        await swReply({ type: 'llm-create-engine-response', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-create-engine-response', id: msg.id, error: String(e) });
    }
}

async function handleUnload(msg) {
    try {
        if (_engine) {
            await _engine.unload();
            _engine = null;
            _engineModel = null;
        }
        await swReply({ type: 'llm-unload-response', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-unload-response', id: msg.id, error: String(e) });
    }
}

async function handleChatStream(msg) {
    if (!_engine) {
        await swReply({ type: 'llm-chat-stream-error', id: msg.id, error: 'no engine loaded' });
        return;
    }
    const ac = new AbortController();
    _activeStreams.set(msg.id, ac);
    try {
        const body = JSON.parse(msg.body);
        const iterator = await _engine.chat.completions.create({
            messages: body.messages,
            tools: body.tools,
            stream: true,
        });
        for await (const chunk of iterator) {
            if (ac.signal.aborted) break;
            await swReply({
                type: 'llm-chat-stream-chunk',
                id: msg.id,
                chunk: JSON.stringify(chunk),
            });
        }
        await swReply({ type: 'llm-chat-stream-done', id: msg.id });
    } catch (e) {
        await swReply({ type: 'llm-chat-stream-error', id: msg.id, error: String(e) });
    } finally {
        _activeStreams.delete(msg.id);
    }
}

function handleCancel(msg) {
    const ac = _activeStreams.get(msg.id);
    if (ac) ac.abort();
}

navigator.serviceWorker.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || !msg.type) return;
    switch (msg.type) {
        case 'llm-create-engine-request': handleCreateEngine(msg); break;
        case 'llm-unload-request':        handleUnload(msg); break;
        case 'llm-chat-stream-request':   handleChatStream(msg); break;
        case 'llm-stream-cancel':         handleCancel(msg); break;
    }
});

// Announce readiness so the SW knows a window client is ready to serve.
// (The SW doesn't strictly need this — it just postMessages when it gets a
// call. Logging here aids debugging.)
console.log('webllm-engine.js loaded');
```

- [ ] **Step 5.2: Register the JS as an asset**

The framework already exports JS assets via `crates/solobase-browser/bin/export-assets.rs` or similar. Add the new file.

Edit `crates/solobase-browser/bin/export-assets.rs` (or `build.rs`):

Find the section that includes `bridge.js` or `sw.js` as assets. Add a parallel include for `webllm-engine.js`. For example:

```rust
include_str!("../js/webllm-engine.js")
```

emitted as `pkg/webllm-engine-<hash>.js` alongside the other JS assets. Follow the exact pattern of the existing asset (hash computation, filename format).

- [ ] **Step 5.3: Commit**

```bash
git add crates/solobase-browser/js/webllm-engine.js crates/solobase-browser/bin/export-assets.rs
git commit -m "feat(solobase-browser): add page-side webllm-engine.js"
```

---

## Task 6: Rust bindings for the bridge

**Files:**
- Modify: `crates/solobase-browser/src/bridge.rs` — add extern declarations
- Modify: `crates/solobase-browser/src/llm/bridge.rs` — high-level Rust wrapper

- [ ] **Step 6.1: Add extern "C" declarations in bridge.rs**

Edit `crates/solobase-browser/src/bridge.rs`, add before the closing brace of the `extern "C"` block:

```rust
    // ─── LLM bridge ───────────────────────────────────────────────────────────

    /// Create + initialize a WebLLM engine on the page. Resolves when loaded.
    #[wasm_bindgen(js_name = llmCreateEngine, catch)]
    pub async fn llm_create_engine(model_id: &str) -> Result<JsValue, JsValue>;

    /// Unload the engine on the page.
    #[wasm_bindgen(js_name = llmUnloadEngine, catch)]
    pub async fn llm_unload_engine(model_id: &str) -> Result<JsValue, JsValue>;

    /// Start a streaming chat completion. Returns the stream id as a JS string.
    #[wasm_bindgen(js_name = llmChatStream, catch)]
    pub async fn llm_chat_stream(body_json: &str) -> Result<JsValue, JsValue>;

    /// Pull the next frame from a stream.
    /// Frame JSON: `{kind:'chunk', payload:<openai chunk json>}` |
    ///             `{kind:'done'}` | `{kind:'error', payload:<string>}`.
    #[wasm_bindgen(js_name = llmNextChunk, catch)]
    pub async fn llm_next_chunk(stream_id: &str) -> Result<JsValue, JsValue>;

    /// Cancel an in-flight stream.
    #[wasm_bindgen(js_name = llmCancelStream, catch)]
    pub async fn llm_cancel_stream(stream_id: &str) -> Result<JsValue, JsValue>;
```

- [ ] **Step 6.2: Write the llm::bridge Rust-side wrapper**

Replace `crates/solobase-browser/src/llm/bridge.rs` with:
```rust
//! Rust-side wrapper over the SW↔page LLM postMessage bridge.
//!
//! Glue around the `llm*` functions in `bridge.js`. Turns the async
//! postMessage exchanges into typed Rust calls. Not testable in native —
//! `#[cfg(target_arch = "wasm32")]` gated and exercised via the
//! `BrowserLlmService` integration tests.

use wafer_core::interfaces::llm::service::LlmError;

use crate::bridge::{llm_cancel_stream, llm_chat_stream, llm_create_engine, llm_next_chunk, llm_unload_engine};

fn js_err(e: wasm_bindgen::JsValue) -> LlmError {
    LlmError::BackendError(format!(
        "webllm bridge: {}",
        e.as_string().unwrap_or_else(|| format!("{e:?}"))
    ))
}

pub async fn create_engine(model_id: &str) -> Result<(), LlmError> {
    llm_create_engine(model_id).await.map(|_| ()).map_err(js_err)
}

pub async fn unload_engine(model_id: &str) -> Result<(), LlmError> {
    llm_unload_engine(model_id).await.map(|_| ()).map_err(js_err)
}

pub async fn start_chat_stream(body_json: &str) -> Result<String, LlmError> {
    let v = llm_chat_stream(body_json).await.map_err(js_err)?;
    v.as_string()
        .ok_or_else(|| LlmError::BackendError("webllm bridge: stream id not a string".into()))
}

/// One frame pulled from the page-side stream.
pub enum StreamFrame {
    /// OpenAI chunk JSON string (pass to `openai_codec::parse_chunk`).
    Chunk(String),
    Done,
    Error(String),
}

pub async fn next_chunk(stream_id: &str) -> Result<StreamFrame, LlmError> {
    let v = llm_next_chunk(stream_id).await.map_err(js_err)?;
    let s = v
        .as_string()
        .ok_or_else(|| LlmError::BackendError("webllm bridge: frame not a string".into()))?;
    let frame: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| LlmError::BackendError(format!("webllm bridge: frame parse: {e}")))?;
    let kind = frame.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    match kind {
        "chunk" => {
            let payload = frame
                .get("payload")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(StreamFrame::Chunk(payload))
        }
        "done" => Ok(StreamFrame::Done),
        "error" => {
            let msg = frame
                .get("payload")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Ok(StreamFrame::Error(msg))
        }
        other => Err(LlmError::BackendError(format!(
            "webllm bridge: unknown frame kind '{other}'"
        ))),
    }
}

pub async fn cancel_stream(stream_id: &str) -> Result<(), LlmError> {
    llm_cancel_stream(stream_id)
        .await
        .map(|_| ())
        .map_err(js_err)
}
```

- [ ] **Step 6.3: Verify wasm32 compile**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: clean build.

- [ ] **Step 6.4: Commit**

```bash
git add crates/solobase-browser/src/bridge.rs crates/solobase-browser/src/llm/bridge.rs
git commit -m "feat(solobase-browser): add Rust-side llm bridge bindings"
```

---

## Task 7: BrowserLlmService LlmService impl

**Files:**
- Modify: `crates/solobase-browser/src/llm/service.rs`

The service tracks which model is loaded in a `RefCell<Option<String>>` (bridge owns the engine; Rust just records model id for `status`). `chat_stream` encodes the request, starts the stream, spawns a pump task that repeatedly calls `next_chunk`, parses via `openai_codec`, forwards to an mpsc. Cancellation calls `cancel_stream`.

- [ ] **Step 7.1: Implement the service**

Replace `crates/solobase-browser/src/llm/service.rs` with:
```rust
//! `BrowserLlmService` — implements `wafer_core::interfaces::llm::service::LlmService`
//! over the SW↔page WebLLM bridge.

use std::{cell::RefCell, rc::Rc};

use futures::{channel::mpsc, sink::SinkExt, stream::BoxStream};
use tokio_util::sync::CancellationToken;
use wafer_core::interfaces::llm::service::{
    ChatChunk, ChatRequest, FinishReason, LlmError, LlmService, LoadProgress, ModelInfo,
    ModelStatus,
};

use crate::llm::{
    bridge::{self, StreamFrame},
    catalog::ModelCatalog,
    openai_codec::{encode_request_body, parse_chunk, ParsedChunk, ToolCallAccumulator},
};

const WEBLLM_BACKEND: &str = "webllm";

/// `LlmService` impl backed by WebLLM running in the page.
pub struct BrowserLlmService {
    catalog: ModelCatalog,
    /// Which model the page-side engine is currently loaded with.
    /// `None` = not loaded; some str = loaded.
    loaded_model: Rc<RefCell<Option<String>>>,
}

impl BrowserLlmService {
    pub fn new() -> Self {
        Self {
            catalog: ModelCatalog::default(),
            loaded_model: Rc::new(RefCell::new(None)),
        }
    }

    pub fn with_catalog(catalog: ModelCatalog) -> Self {
        Self {
            catalog,
            loaded_model: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for BrowserLlmService {
    fn default() -> Self {
        Self::new()
    }
}

fn one_shot_err<T: 'static + Send>(err: LlmError) -> BoxStream<'static, Result<T, LlmError>> {
    Box::pin(futures::stream::once(async move { Err(err) }))
}

#[async_trait::async_trait(?Send)]
impl LlmService for BrowserLlmService {
    async fn chat_stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> BoxStream<'static, Result<ChatChunk, LlmError>> {
        if !self.claims_backend(&req.backend_id) {
            return one_shot_err(LlmError::BackendError(format!(
                "backend '{}' not claimed by webllm",
                req.backend_id
            )));
        }

        let body_json = match encode_request_body(&req.messages, &req.tools) {
            Ok(s) => s,
            Err(e) => return one_shot_err(e),
        };

        let (mut tx, rx) = mpsc::channel::<Result<ChatChunk, LlmError>>(16);

        wasm_bindgen_futures::spawn_local(async move {
            let stream_id = match bridge::start_chat_stream(&body_json).await {
                Ok(id) => id,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };

            let mut tool_acc = ToolCallAccumulator::new();
            loop {
                if cancel.is_cancelled() {
                    let _ = bridge::cancel_stream(&stream_id).await;
                    break;
                }
                let frame = match bridge::next_chunk(&stream_id).await {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                };
                match frame {
                    StreamFrame::Chunk(s) => match parse_chunk(&s) {
                        Ok(ParsedChunk::TextDelta(chunk)) => {
                            if tx.send(Ok(chunk)).await.is_err() {
                                break;
                            }
                        }
                        Ok(ParsedChunk::ToolCallDelta(delta)) => {
                            tool_acc.feed(&delta);
                        }
                        Ok(ParsedChunk::Finish { reason, usage }) => {
                            if reason == FinishReason::ToolCall && !tool_acc.is_empty() {
                                match std::mem::take(&mut tool_acc).finish() {
                                    Ok(calls) => {
                                        for call in calls {
                                            let chunk = ChatChunk::tool_call(call);
                                            if tx.send(Ok(chunk)).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e)).await;
                                        return;
                                    }
                                }
                            }
                            let _ = tx.send(Ok(ChatChunk::finish(reason, usage))).await;
                        }
                        Ok(ParsedChunk::Noop) => {}
                        Err(e) => {
                            let _ = tx.send(Err(e)).await;
                            break;
                        }
                    },
                    StreamFrame::Done => break,
                    StreamFrame::Error(msg) => {
                        let _ = tx
                            .send(Err(LlmError::BackendError(format!("webllm: {msg}"))))
                            .await;
                        break;
                    }
                }
            }
        });

        Box::pin(rx)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(self.catalog.models().to_vec())
    }

    async fn status(&self, backend_id: &str, model_id: &str) -> Result<ModelStatus, LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        let borrow = self.loaded_model.borrow();
        match borrow.as_deref() {
            Some(m) if m == model_id => Ok(ModelStatus::ready()),
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
        let (mut tx, rx) = mpsc::channel::<Result<LoadProgress, LlmError>>(8);
        let loaded_model = Rc::clone(&self.loaded_model);
        let model_id = model_id.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            if cancel.is_cancelled() {
                return;
            }
            match bridge::create_engine(&model_id).await {
                Ok(()) => {
                    *loaded_model.borrow_mut() = Some(model_id.clone());
                    let _ = tx.send(Ok(LoadProgress::new("ready"))).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                }
            }
        });
        Box::pin(rx)
    }

    async fn unload_model(&self, backend_id: &str, model_id: &str) -> Result<(), LlmError> {
        if backend_id != WEBLLM_BACKEND {
            return Err(LlmError::BackendError(format!(
                "backend '{backend_id}' not claimed by webllm"
            )));
        }
        bridge::unload_engine(model_id).await?;
        *self.loaded_model.borrow_mut() = None;
        Ok(())
    }

    fn claims_backend(&self, backend_id: &str) -> bool {
        backend_id == WEBLLM_BACKEND
    }
}
```

> **Note:** `ChatChunk::tool_call` may not exist in wafer-core today. If the trait defines a different constructor (e.g. `ChatChunk::new(ChunkDelta::ToolCall(call))` or direct struct init), use that. If no variant exists at all for streaming tool-calls, extend `wafer-core::interfaces::llm::service::ChunkDelta` to add a `ToolCall(ToolCall)` variant — that's in-scope per the spec.

- [ ] **Step 7.2: Verify it compiles**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: clean build. If `ChatChunk::tool_call` is missing, extend `wafer-core` to add it, then retry.

- [ ] **Step 7.3: Commit**

```bash
git add crates/solobase-browser/src/llm/service.rs
git commit -m "feat(solobase-browser): implement BrowserLlmService over page bridge"
```

---

## Task 8: Wire webllm-engine.js into the HTML shell

**Files:**
- Modify: wherever the solobase-browser framework emits the HTML shell that loads framework JS (likely `crates/solobase-browser/src/assets.rs` or a bootstrap HTML template).

- [ ] **Step 8.1: Find the HTML shell**

Run: `grep -rn "bridge.js\|sw\.js\|webllm" crates/solobase-browser/ --include="*.rs" --include="*.html" | head`
Expected: a handful of hits. Identify which file emits the page-side `<script>` tags for framework JS.

- [ ] **Step 8.2: Add the engine script tag**

In the identified file, add a `<script type="module" src="/webllm-engine-<hash>.js"></script>` line right after the existing bridge/sw script tags. Use the same hash-lookup mechanism the existing tags use (look up via the asset registry from step 5.2).

Conditionality: keep it unconditional for now. The engine JS listens passively for SW postMessages; if no SW call comes, it's a no-op. We can make it lazy-loaded later if bundle size becomes an issue.

- [ ] **Step 8.3: Commit**

```bash
git add <path>
git commit -m "feat(solobase-browser): load webllm-engine.js in page shell"
```

---

## Task 9: Remove solobase-web's local LLM code

**Files:**
- Delete: `crates/solobase-web/src/llm.rs`
- Delete: `crates/solobase-web/js/webllm-engine.js`
- Modify: `crates/solobase-web/src/lib.rs`

- [ ] **Step 9.1: Delete files**

```bash
rm crates/solobase-web/src/llm.rs
rm crates/solobase-web/js/webllm-engine.js
```

- [ ] **Step 9.2: Update solobase-web/src/lib.rs**

Edit `crates/solobase-web/src/lib.rs`:
- Remove `pub mod llm;`
- Change the `browser_llm` construction from `llm::BrowserLlmService::new()` to `solobase_browser::llm::BrowserLlmService::new()`

Specifically:
```rust
// Before:
let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
    Arc::new(llm::BrowserLlmService::new());

// After:
let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
    Arc::new(solobase_browser::llm::BrowserLlmService::new());
```

- [ ] **Step 9.3: Verify the web crate still builds**

Run: `cargo check -p solobase-web --target wasm32-unknown-unknown`
Expected: clean build.

- [ ] **Step 9.4: Run existing solobase-web tests (if any outside llm.rs)**

Run: `cargo test -p solobase-web`
Expected: pass (the llm.rs unit tests moved to solobase-browser and are re-run there).

- [ ] **Step 9.5: Commit**

```bash
git add crates/solobase-web
git commit -m "refactor(solobase-web): consume BrowserLlmService from solobase-browser"
```

---

## Task 10: Manual smoke test of solobase-web with framework LLM

**Files:** none (verification task).

- [ ] **Step 10.1: Build solobase-web**

Run: `cd crates/solobase-web && make build` (or whatever the existing build command is — check `justfile`/`Makefile`)
Expected: successful wasm-pack build; `pkg/` contains the built artifacts.

- [ ] **Step 10.2: Serve and smoke-test in browser**

Run: `cd crates/solobase-web && python3 -m http.server 8080 -d pkg`

In browser:
1. Open `http://localhost:8080`
2. Log in as `admin@solobase.local` / `admin`
3. Navigate to the LLM admin UI (wherever model selection lives)
4. Verify the model list renders — should show the 7 default WebLLM models
5. Pick a small model (e.g. SmolLM2 1.7B f16) and load it; confirm load completes without error
6. Send a chat message; confirm tokens stream back

- [ ] **Step 10.3: Fix issues found in smoke**

If the engine doesn't load or chunks don't arrive: check browser DevTools console on both the window and the SW (Application → Service Workers → inspect). Most likely issues: postMessage routing (sw.js dispatch), the `<script>` tag not included in the shell, or a hash mismatch. Fix at root cause, commit each fix separately.

- [ ] **Step 10.4: Commit any fixes**

If step 10.3 required fixes, commit them with clear messages.

---

## Task 11: Open solobase PR

**Files:** none (git operation).

- [ ] **Step 11.1: Push the branch and open a PR**

Run:
```bash
git push -u origin feat/native-framework  # or the current branch
gh pr create --title "feat: extract BrowserLlmService to solobase-browser (Phase D part 1)" --body "$(cat <<'EOF'
## Summary
- Moves browser WebLLM LlmService impl into solobase-browser::llm
- Adds SW↔page postMessage bridge (mirrors loadAsset correlation-id pattern)
- Lifts the v0 tool-call rejection; tool-calls now flow through the stream
- Deletes crates/solobase-web/src/llm.rs and js/webllm-engine.js; web crate consumes framework service

## Test plan
- [ ] cargo test -p solobase-browser --lib llm (codec + catalog)
- [ ] cargo check -p solobase-web --target wasm32-unknown-unknown
- [ ] Manual smoke in browser: model list, load model, stream a chat
- [ ] CI passes (fmt, clippy, wasm build, e2e)

Spec: docs/superpowers/specs/2026-04-20-llm-service-extraction-design.md
EOF
)"
```

Expected: PR URL returned. No code changes in this step.

---

## Task 12: (gizza-ai repo) Delete bridge JS

This task and all following tasks run in the **gizza-ai repo**, not solobase. They must happen AFTER the solobase PR above is merged and gizza-ai's solobase-browser dep is bumped.

**Files (in gizza-ai repo):**
- Delete: `site/ai-bridge.js`
- Delete: `site/sw-llm-bridge.js`
- Modify: `site/index.html`
- Modify: `Cargo.toml`

- [ ] **Step 12.1: Bump solobase-browser dep**

Edit `Cargo.toml` in gizza-ai — bump the solobase-browser version pin (or git rev) to the merged solobase PR's commit.

- [ ] **Step 12.2: Delete the JS files**

```bash
rm site/ai-bridge.js site/sw-llm-bridge.js
```

- [ ] **Step 12.3: Update HTML**

Edit `site/index.html` — remove any `<script src="/ai-bridge.js">` or `sw-llm-bridge.js` tags. The framework will load `webllm-engine.js` automatically.

- [ ] **Step 12.4: Commit**

```bash
git add -u site/ Cargo.toml Cargo.lock
git commit -m "refactor(gizza-ai): delete local llm bridge in favour of framework"
```

---

## Task 13: (gizza-ai repo) Rewrite agent.rs against LlmService

**Files (in gizza-ai repo):**
- Modify: `src/blocks/agent.rs`
- Modify: wherever the gizza-ai `Wafer` is built (register service)

`agent.rs` currently calls `localLlmChatStream(bodyJson)` via a wasm-bindgen extern and parses SSE frames back. After: resolve `LlmService` from the wafer runtime's service registry, call `chat_stream(req, cancel)`, iterate the returned `BoxStream`, dispatch `Delta`/`ToolCall`/`Finish` variants directly.

- [ ] **Step 13.1: Find the existing call site**

Run: `grep -n 'localLlmChatStream\|local-llm' src/blocks/agent.rs | head`
Expected: one or two call sites. Note the line range.

- [ ] **Step 13.2: Register BrowserLlmService on the Wafer**

In the file that builds the gizza-ai `Wafer` instance (analogous to solobase-web's `lib.rs:54-64`), add:
```rust
let browser_llm: Arc<dyn wafer_core::interfaces::llm::service::LlmService> =
    Arc::new(solobase_browser::llm::BrowserLlmService::new());
// ...
w.llm_service("browser", browser_llm)
```

Place it alongside the other service registrations.

- [ ] **Step 13.3: Rewrite agent.rs inner loop**

Replace the SSE-parsing loop in `src/blocks/agent.rs` with direct `LlmService::chat_stream` consumption. Concretely, find the block that looks roughly like:

```rust
let body_json = serde_json::to_string(&body)?;
let sse_bytes = localLlmChatStream(&body_json).await;
// parse SSE, decode events, etc.
```

Replace with:
```rust
use futures::StreamExt;
use wafer_core::interfaces::llm::service::{ChatRequest, ChunkDelta, LlmService};

let llm: Arc<dyn LlmService> = wafer.llm_service("browser")
    .ok_or_else(|| anyhow::anyhow!("browser llm not registered"))?;

let req = ChatRequest {
    backend_id: "webllm".into(),
    model: model_id.clone(),
    messages,
    tools,
    params: Default::default(),
    extra: Default::default(),
};
let cancel = CancellationToken::new();
let mut stream = llm.chat_stream(req, cancel.clone()).await;

while let Some(item) = stream.next().await {
    match item {
        Ok(chunk) => match chunk.delta {
            ChunkDelta::Text(t) => { /* forward text token to UI */ }
            ChunkDelta::ToolCall(tc) => { /* execute tool, build follow-up message */ }
            ChunkDelta::Empty => {}
            _ => {}
        },
        Err(e) => { /* report error, break */ break; }
    }
    if let Some(reason) = chunk.finish_reason {
        // handle terminal state (loop again if tool_call, else done)
        break;
    }
}
```

(The exact variable names and UI-forwarding logic depend on the current agent.rs structure — preserve the surrounding control flow, replace only the bridge call and its SSE-parse loop.)

- [ ] **Step 13.4: Delete the now-unused wasm-bindgen extern for localLlmChatStream**

In the same file or wherever the extern was declared, remove the `#[wasm_bindgen(module = "/site/sw-llm-bridge.js")]` block and the `localLlmChatStream` fn declaration.

- [ ] **Step 13.5: Verify the gizza-ai crate builds**

Run: `cargo check --target wasm32-unknown-unknown`
Expected: clean build.

- [ ] **Step 13.6: Commit**

```bash
git add src/ site/
git commit -m "refactor(gizza-ai): consume BrowserLlmService via wafer_core::LlmService trait"
```

---

## Task 14: (gizza-ai repo) E2E smoke

**Files:** gizza-ai site + deployment.

- [ ] **Step 14.1: Build and serve gizza-ai**

Run the gizza-ai equivalent of `wasm-pack build` + static serve. (Check gizza-ai's `Makefile`/`justfile`/`README`.)

- [ ] **Step 14.2: Browser smoke**

1. Open the gizza-ai page in a browser.
2. Trigger the agent with a prompt that exercises streaming (e.g. "say hi in 5 words").
3. Verify tokens stream visibly in the UI.
4. If agent uses tools: trigger a tool-call prompt and verify the tool executes + follow-up message streams back.

- [ ] **Step 14.3: Fix issues found**

Common issues: service registration order (BrowserLlmService must be registered before the agent block starts); stream_id routing (ensure sw.js in gizza-ai also has the `llm-*` dispatcher from Task 4.2 — or inherits it from the framework bootstrap).

- [ ] **Step 14.4: Commit any fixes**

---

## Task 15: (gizza-ai repo) Open PR

- [ ] **Step 15.1: Push and open PR**

```bash
git push -u origin <branch>
gh pr create --title "refactor: consume BrowserLlmService from solobase-browser (Phase D part 2)" --body "$(cat <<'EOF'
## Summary
- Deletes site/ai-bridge.js + site/sw-llm-bridge.js (443 LOC)
- Rewrites src/blocks/agent.rs to use wafer_core::LlmService trait
- Bumps solobase-browser dep to include BrowserLlmService

## Test plan
- [ ] cargo check --target wasm32-unknown-unknown
- [ ] Manual smoke: prompt streams tokens
- [ ] Tool-call smoke (if agent uses tools)

Depends on: <solobase PR url>
Spec: solobase/docs/superpowers/specs/2026-04-20-llm-service-extraction-design.md
EOF
)"
```

---

## Summary

- Tasks 1–11: solobase repo PR. Adds solobase-browser::llm, deletes solobase-web's local LLM code.
- Tasks 12–15: gizza-ai repo PR. Deletes the 443 LOC of bridge JS, rewrites agent.rs.

Net change across both repos:
- **Deleted:** `solobase-web/src/llm.rs` (~755 LOC), `solobase-web/js/webllm-engine.js` (~64 LOC), `gizza-ai/site/ai-bridge.js` (~317 LOC), `gizza-ai/site/sw-llm-bridge.js` (~126 LOC). Total ~1,260 LOC removed.
- **Added:** `solobase-browser/src/llm/` (~700 LOC), `solobase-browser/js/bridge.js` additions (~140 LOC), `solobase-browser/js/webllm-engine.js` (~90 LOC), agent.rs rewrite (minor net). Total ~930 LOC added.
- Net ~330 LOC removed plus single-source-of-truth codec + bridge.
