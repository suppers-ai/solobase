# LLM Service Extraction — Design

**Date:** 2026-04-20
**Status:** Draft
**Phase:** D (follows Phase C native-framework extraction)
**Repos affected:** `solobase`, `gizza-ai`

## Goal

Unify browser-side LLM code into a single framework-provided service. Today, solobase-web and gizza-ai each wrap `@mlc-ai/web-llm` in parallel: duplicated model catalogs, duplicated OpenAI chunk/usage parsing, two different SW↔page bridge protocols, and two independent JS engine wrappers. This spec replaces all of that with one implementation shipped from `solobase-browser`, consumed via the existing `wafer_core::interfaces::llm::service::LlmService` trait.

End state:
- `solobase-browser::llm::BrowserLlmService` is the only browser-side `LlmService` implementation.
- Tool calls are first-class; the v0 "reject tool-calls" guard in solobase-web is removed.
- gizza-ai's `ai-bridge.js`, `sw-llm-bridge.js`, and `agent.rs` LLM glue are deleted. It becomes a pure consumer of the framework.
- No dual paths, no deprecation shims.

## Architecture

The service lives in `crates/solobase-browser/src/llm/`. It composes three concerns, each in its own module:

1. **OpenAI codec** (pure Rust, no browser dependencies) — encode chat requests to OpenAI JSON, parse streaming chunks, buffer tool-call deltas into complete tool calls, parse usage and finish reasons. Lifted from solobase-web's `llm.rs` with its existing ~100 tests preserved.
2. **Engine bridge** — Rust wasm-bindgen glue that calls into page-side JS via SW postMessage. Uses correlation IDs to multiplex concurrent streams.
3. **Service impl** — `BrowserLlmService` wires the codec and bridge together behind the `LlmService` trait.

Page-side JS (`webllm-engine.js`) and SW-side glue (`webllm-sw-bridge.js`) ship embedded in the crate via `include_str!` and are served at hashed paths by the framework's existing SW bootstrap (from Phase 1). Registration is one call:

```rust
w.llm_service("browser", solobase_browser::llm::BrowserLlmService::new());
```

If no block registered on the Wafer depends on `LlmService`, the JS assets are not loaded — non-LLM apps pay nothing.

### Crate layout

```
crates/solobase-browser/
  src/llm/
    mod.rs           # pub use re-exports
    service.rs       # BrowserLlmService + LlmService impl
    engine.rs        # wasm-bindgen bindings to the embedded JS
    openai_codec.rs  # messages_to_openai_json + chunk/usage/tool-call parsing
    catalog.rs       # default model list + capabilities
    bridge.rs        # SW-side postMessage protocol (frames, correlation IDs)
  js/
    webllm-engine.js      # page-side: wraps MLCEngine, handles postMessage
    webllm-sw-bridge.js   # SW-side: registers listener, decodes frames
```

## Rust API surface

### Public

```rust
pub struct BrowserLlmService { /* engine handle, catalog */ }

impl BrowserLlmService {
    pub fn new() -> Self;
    pub fn with_catalog(catalog: ModelCatalog) -> Self;
}

#[async_trait]
impl wafer_core::interfaces::llm::service::LlmService for BrowserLlmService {
    async fn chat_stream(&self, req: ChatRequest) -> Result<ChatStream, LlmError>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError>;
    async fn unload(&self, model: &str) -> Result<(), LlmError>;
}
```

`ChatRequest`, `ChatStream`, `ChatChunk`, `ModelInfo`, `LlmError` are defined by `wafer_core::interfaces::llm::service`. We adopt them as-is.

### Chunk shape

`ChatStream` yields:
- `Delta { content: String }` — streamed text tokens
- `ToolCall { id, name, arguments }` — emitted once per complete tool call. Tool-call-delta buffering happens inside `openai_codec`, not in consumer code.
- `Finish { reason, usage }` — exactly once, terminates the stream.

Errors surface as a `LlmError` returned from the `ChatStream` iterator, not as a `ChatChunk` variant.

### Internal modules

- **`openai_codec`** (~300 LOC, pure Rust, no `wasm-bindgen`): `messages_to_openai_json`, `parse_chunk`, `parse_usage`, `parse_finish_reason`, `parse_tool_call_deltas`. Directly lifted from `solobase-web/src/llm.rs` with its existing unit tests. The existing tool-call-rejection guard is removed.
- **`catalog`**: `ModelCatalog` struct holding the 7 default models (SmolLM2 / Qwen / Gemma / Phi / Llama, f32 + f16 tiers). Identical to both apps' current catalogs. Consumers override via `BrowserLlmService::with_catalog`.
- **`engine`**: thin wasm-bindgen layer. Functions: `create_engine(model_id)`, `chat_stream(req_json) -> stream_id`, `next_chunk(stream_id) -> Promise<frame>`, `unload(model_id)`.
- **`bridge`**: SW-side postMessage protocol (see next section). Maintains an in-memory `Map<stream_id, controller>` so multiple streams can coexist.

### Scope exclusions

- **Multimodal input** (images, audio) — not in this phase. `messages_to_openai_json` continues to reject non-text content.
- **Server-side LLM providers** (OpenAI API, Claude API, etc.) — out of scope. This service is explicitly the browser/WebGPU path. Server-side `LlmService` implementations are a separate concern.
- **Model quantization / download orchestration** — `@mlc-ai/web-llm` handles this; we don't wrap it.

## JS assets & SW↔page bridge protocol

### Page-side (`webllm-engine.js`, ~120 LOC)

- On load: listens for `message` events from the controlling SW.
- Handles three request kinds: `create_engine`, `chat_stream`, `unload`.
- For `chat_stream`: calls `engine.chat.completions.create({stream: true, ...})`, iterates the async iterator, posts one `{id, kind: "chunk", payload: openai_chunk}` frame per yield. Terminates with `{kind: "done", payload: {usage, finish_reason}}` or `{kind: "error", payload: <message>}`.
- Single source of truth for MLCEngine lifecycle (load progress, unload, GPU context reuse).

### SW-side (`webllm-sw-bridge.js`, ~60 LOC)

- Registered by the framework SW bootstrap during `install`/`activate`.
- Maintains `Map<id, {controller: ReadableStreamDefaultController}>` for in-flight streams.
- On receiving a page-side frame, routes by `id` to the right controller and enqueues the chunk.
- Exposes a small API to Rust via wasm-bindgen: `start_chat_stream(req_json) -> stream_id`, `next_chunk(stream_id) -> Promise<frame>`, `cancel(stream_id)`.

### Wire protocol

```
SW  → page:  {id: "uuid", kind: "chat_stream", payload: {messages, model, tools?, options?}}
page → SW:   {id, kind: "chunk",  payload: <openai-format delta>}   // 1..N frames
page → SW:   {id, kind: "done",   payload: {usage, finish_reason}}  // exactly 1
page → SW:   {id, kind: "error",  payload: <string>}                // alternative terminator
```

Structured-clone JSON only. No SSE, no text framing, no base64. gizza-ai's existing SSE+Uint8Array buffering is deleted.

### Serving & bootstrap integration

`webllm-engine.js` is `include_str!`'d into `solobase-browser` and written to `pkg/webllm-engine-<hash>.js` by the existing build pipeline (same treatment as `sw.js` etc.). The HTML shell script-loads it alongside other framework JS.

`solobase-browser::bootstrap::register_services(w)` (existing Phase-1 entry point) grows a new `register_llm_bridge()` step, gated by whether any registered block depends on `LlmService`. If no consumer, the JS is not loaded.

## Error handling

All errors surface as `wafer_core::interfaces::llm::service::LlmError` (variant names/shape as defined by the trait; the behavioral categories below must map onto whichever variants exist).

- **Engine load failures** (WebGPU unavailable, model fetch failed): surface from `chat_stream`'s initial future, before any stream starts.
- **Page-side runtime errors** (mid-stream): the page posts an `error` frame; the SW bridge ends the `ChatStream` with an error from the next `poll_next` call.
- **Cancellation**: dropping the `ChatStream` on the Rust side triggers a `cancel` postMessage to the page; the page aborts the `chat.completions.create` iterator. No error is surfaced — cancellation is not an error.
- **Protocol violations** (unknown frame kind, missing `id`, chunk without preceding `chat_stream` request): logged via `tracing::warn!` in the SW bridge, frame dropped. Never panic.
- **Tool-call-delta malformation** (invalid JSON arguments accumulated across deltas): surface as a stream error.

If the existing `LlmError` variants do not cleanly cover these categories, extending the trait is in scope for this phase (the trait lives in `wafer_core` but additions are backward-compatible).

## Testing

**Preserved from solobase-web:** all existing ~100 unit tests on `messages_to_openai_json`, `parse_chunk`, `parse_usage`, etc. Move intact to `solobase-browser::llm::openai_codec` tests. Tool-call-rejection assertions are deleted; new tests cover tool-call happy-path parsing.

**New unit tests:**
- `openai_codec::parse_tool_call_deltas` — accumulates partial deltas into complete `ToolCall` values.
- `bridge` — frame routing with multiple concurrent `stream_id`s, correlation correctness.
- `catalog` — default list integrity, `with_catalog` override.

**Integration (wasm-pack test):** existing solobase-web browser tests that exercise `LlmService::chat_stream` must pass against `BrowserLlmService` unchanged (aside from tool-call assertions).

**E2E:** gizza-ai's Playwright smoke runs a prompt through the agent loop and verifies at least one token streams back. This is the cross-repo end-to-end check that the framework's LLM story actually works in the consumer.

## gizza-ai migration

### Deleted

- `site/ai-bridge.js` (317 LOC)
- `site/sw-llm-bridge.js` (126 LOC)
- Any gizza-specific model catalog JS
- `localLlmChatStream`-style JS glue referenced from `agent.rs`

### Modified

- **`src/blocks/agent.rs`** — rewrite the inner loop. Resolve `LlmService` from the runtime, call `chat_stream(ChatRequest { ... })`, iterate the returned `ChatStream`, handle `Delta`/`ToolCall`/`Finish` variants directly. Tool-call buffering is now the framework's concern.
- **`Cargo.toml`** — bump the `solobase-browser` dep to the version containing the new module.
- **`site/index.html`** — drop `<script>` tags for the deleted files; framework bootstrap loads `webllm-engine.js` on demand.
- **SW entry** — remove the local `chat_stream` message handler; rely on the framework's bridge.

### Wafer registration

Wherever gizza-ai builds its `Wafer` instance (analogous to `solobase-web/src/lib.rs:54-64`):

```rust
w.llm_service("browser", solobase_browser::llm::BrowserLlmService::new());
```

## PR ordering

1. **solobase PR** — land `solobase-browser::llm`, delete solobase-web's local `llm.rs` and `webllm-engine.js`, rewire solobase-web to consume the framework. Must pass all ported `openai_codec` tests and existing solobase-web browser integration tests.
2. **gizza-ai PR** — bump `solobase-browser` dep, delete the four JS files, rewrite `agent.rs`. Lands only after (1) is merged. Playwright smoke confirms end-to-end prompt flow.

No dual-path, no deprecation shims, no temporary compat layer.

## Open questions

None at spec-writing time. All design decisions have a definite answer in the sections above.

## Out of scope (explicitly deferred)

- Multimodal input (images, audio)
- Server-side `LlmService` implementations (OpenAI API, Claude API, etc.)
- npm packaging of the JS bridge for third-party reuse
- Model quantization or download-progress UI changes
- A CLI / runtime command to pre-warm the engine on SW install
