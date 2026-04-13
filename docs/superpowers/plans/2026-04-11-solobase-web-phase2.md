# Phase 2: LLM Blocks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four new blocks to solobase-core — `messages` (generic threads/messages), `llm` (orchestrator), `local-llm` (WebLLM browser inference), and `provider-llm` (remote API providers) — enabling AI chat and other message-based features on both native and browser Solobase.

**Architecture:** Each block follows the existing solobase-core pattern: struct implementing the `Block` trait, registered via `blocks/mod.rs`. The `messages` block is fully generic (no AI dependency). The `llm` block orchestrates between `local-llm` and `provider-llm` backends. All blocks use the WAFER service interfaces (database, config, network) and render UI via maud + htmx.

**Tech Stack:** Rust, maud (HTML), htmx, wafer-core clients (database, config, network, crypto), serde for JSON API

**Spec:** `docs/superpowers/specs/2026-04-11-solobase-web-browser-wasm-design.md` (Phase 2 section)

---

## File Structure

```
crates/solobase-core/src/blocks/
├── mod.rs                      # Modified: add Messages, Llm, ProviderLlm, LocalLlm to BlockId + registration
├── messages/
│   ├── mod.rs                  # MessagesBlock: Block trait impl, collections, handle()
│   └── pages.rs                # Maud UI: thread list, message view, compose
├── llm/
│   ├── mod.rs                  # LlmBlock: orchestrator, routes to backends
│   └── pages.rs                # Maud UI: chat interface, model picker, settings
├── provider_llm/
│   ├── mod.rs                  # ProviderLlmBlock: remote API calls (OpenAI-compatible, Anthropic)
│   └── pages.rs                # Maud UI: provider config, API key management
├── local_llm.rs                # LocalLlmBlock: stub for WebLLM (browser-only, Phase 2b)
```

```
crates/solobase-core/src/routing.rs    # Modified: add BlockId variants
```

---

### Task 1: Register new block IDs and module structure

**Files:**
- Modify: `crates/solobase-core/src/routing.rs`
- Modify: `crates/solobase-core/src/blocks/mod.rs`
- Create: `crates/solobase-core/src/blocks/messages/mod.rs` (stub)
- Create: `crates/solobase-core/src/blocks/llm/mod.rs` (stub)
- Create: `crates/solobase-core/src/blocks/provider_llm/mod.rs` (stub)
- Create: `crates/solobase-core/src/blocks/local_llm.rs` (stub)

- [ ] **Step 1: Add BlockId variants**

In `crates/solobase-core/src/routing.rs`, add to the `BlockId` enum:

```rust
Messages,
Llm,
ProviderLlm,
LocalLlm,
```

- [ ] **Step 2: Create stub modules**

Create four stub files that each contain a minimal block struct:

`crates/solobase-core/src/blocks/messages/mod.rs`:
```rust
use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct MessagesBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for MessagesBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/messages", "0.0.1", "http-handler@v1", "Generic message threads and messages")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Feature)
            .can_disable(true)
            .default_enabled(true)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        err_not_found(msg, "not implemented yet")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
```

`crates/solobase-core/src/blocks/llm/mod.rs`:
```rust
use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct LlmBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LlmBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/llm", "0.0.1", "http-handler@v1", "LLM orchestrator — routes to provider or local backends")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Feature)
            .requires(vec!["suppers-ai/messages".into(), "wafer-run/database".into(), "wafer-run/config".into()])
            .can_disable(true)
            .default_enabled(true)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        err_not_found(msg, "not implemented yet")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
```

`crates/solobase-core/src/blocks/provider_llm/mod.rs`:
```rust
use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct ProviderLlmBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for ProviderLlmBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/provider-llm", "0.0.1", "http-handler@v1", "Remote LLM API providers (OpenAI, Anthropic)")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Feature)
            .requires(vec!["wafer-run/network".into(), "wafer-run/config".into(), "wafer-run/database".into()])
            .can_disable(true)
            .default_enabled(true)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        err_not_found(msg, "not implemented yet")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
```

`crates/solobase-core/src/blocks/local_llm.rs`:
```rust
use std::sync::Arc;
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::helpers::*;
use wafer_run::types::*;

pub struct LocalLlmBlock;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Block for LocalLlmBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo::new("suppers-ai/local-llm", "0.0.1", "http-handler@v1", "Local LLM inference via WebLLM (browser only)")
            .instance_mode(InstanceMode::Singleton)
            .category(wafer_run::BlockCategory::Feature)
            .can_disable(true)
            .default_enabled(false)
    }

    async fn handle(&self, _ctx: &dyn Context, msg: &mut Message) -> Result_ {
        err_not_found(msg, "local-llm not available in this runtime")
    }

    async fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
```

- [ ] **Step 3: Register blocks in mod.rs**

In `crates/solobase-core/src/blocks/mod.rs`:

Add module declarations:
```rust
pub mod messages;
pub mod llm;
pub mod provider_llm;
pub mod local_llm;
```

Add to `SOLOBASE_BLOCKS`:
```rust
("messages", BlockId::Messages),
("llm", BlockId::Llm),
("provider-llm", BlockId::ProviderLlm),
("local-llm", BlockId::LocalLlm),
```

Add to `make_block()`:
```rust
BlockId::Messages => Arc::new(messages::MessagesBlock),
BlockId::Llm => Arc::new(llm::LlmBlock),
BlockId::ProviderLlm => Arc::new(provider_llm::ProviderLlmBlock),
BlockId::LocalLlm => Arc::new(local_llm::LocalLlmBlock),
```

Add to `block_id_to_name()`:
```rust
BlockId::Messages => "messages",
BlockId::Llm => "llm",
BlockId::ProviderLlm => "provider-llm",
BlockId::LocalLlm => "local-llm",
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p solobase-core`
Run: `cargo check -p solobase`
Run: `cd crates/solobase-web && wasm-pack build --target web --dev`

All three should succeed.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-core/src/
git commit -m "feat: register messages, llm, provider-llm, local-llm block stubs"
```

---

### Task 2: Messages block — collections and CRUD API

**Files:**
- Modify: `crates/solobase-core/src/blocks/messages/mod.rs`

Implement the full `messages` block with two collections (threads, messages) and JSON API endpoints.

- [ ] **Step 1: Define collections in BlockInfo**

Update `info()` in `messages/mod.rs`:

```rust
fn info(&self) -> BlockInfo {
    use wafer_run::types::CollectionSchema;
    use wafer_run::AuthLevel;

    BlockInfo::new("suppers-ai/messages", "0.0.1", "http-handler@v1", "Generic message threads and messages")
        .instance_mode(InstanceMode::Singleton)
        .category(wafer_run::BlockCategory::Feature)
        .requires(vec!["wafer-run/database".into(), "wafer-run/config".into()])
        .collections(vec![
            CollectionSchema::new(THREADS_COLLECTION)
                .field("title", "string")
                .field_default("metadata", "text", "{}")
                .index(&["updated_at"]),
            CollectionSchema::new(MESSAGES_COLLECTION)
                .field("thread_id", "string")
                .field("role", "string")
                .field_default("content", "text", "")
                .field_default("metadata", "text", "{}")
                .index(&["thread_id", "created_at"]),
        ])
        .can_disable(true)
        .default_enabled(true)
        .description("Generic message/thread system. Reusable for chat, task management, support tickets, notebooks.")
        .admin_url("/b/messages/")
        .endpoints(vec![
            // Threads API
            BlockEndpoint::get("/b/messages/api/threads").summary("List threads"),
            BlockEndpoint::post("/b/messages/api/threads").summary("Create thread"),
            BlockEndpoint::get("/b/messages/api/threads/{id}").summary("Get thread"),
            BlockEndpoint::patch("/b/messages/api/threads/{id}").summary("Update thread"),
            BlockEndpoint::delete("/b/messages/api/threads/{id}").summary("Delete thread"),
            // Messages API
            BlockEndpoint::get("/b/messages/api/threads/{id}/messages").summary("List messages in thread"),
            BlockEndpoint::post("/b/messages/api/threads/{id}/messages").summary("Create message"),
            BlockEndpoint::get("/b/messages/api/messages/{id}").summary("Get message"),
            BlockEndpoint::patch("/b/messages/api/messages/{id}").summary("Update message"),
            BlockEndpoint::delete("/b/messages/api/messages/{id}").summary("Delete message"),
        ])
}
```

- [ ] **Step 2: Implement handle() with CRUD operations**

The handle method should pattern-match on (action, path) and delegate to handler functions. Use `wafer_core::clients::database` for all DB operations. Follow the patterns from `legalpages` or `products` blocks.

Key operations:
- Thread CRUD: list (with pagination), create (auto-generate ID), get by ID, update title/metadata, delete (cascade delete messages)
- Message CRUD: list by thread_id (sorted by created_at), create (with thread_id, role, content), get, update, delete

Path params: extract thread ID and message ID from paths like `/b/messages/api/threads/{id}` and `/b/messages/api/threads/{id}/messages`.

Read an existing block (e.g., `legalpages/mod.rs`) to see how path params are extracted and how CRUD is done with `wafer_core::clients::database`.

- [ ] **Step 3: Verify compilation and test**

Run: `cargo check -p solobase-core`
Run: `cargo check -p solobase`

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-core/src/blocks/messages/
git commit -m "feat(messages): threads and messages CRUD API"
```

---

### Task 3: Messages block — maud UI

**Files:**
- Create: `crates/solobase-core/src/blocks/messages/pages.rs`
- Modify: `crates/solobase-core/src/blocks/messages/mod.rs` (add UI routes)

- [ ] **Step 1: Create pages.rs with thread list and message view**

The UI needs:
- **Thread list** (`GET /b/messages/`) — shows all threads with title, last message preview, timestamp. "New Thread" button.
- **Thread view** (`GET /b/messages/threads/{id}`) — shows messages in a thread. Input field to add new message. Each message shows role, content, timestamp.
- **New thread form** — simple form with title input.

Follow the maud patterns from `userportal.rs` or `legalpages/pages.rs`. Use `crate::ui` helpers for page layout, cards, forms.

Use htmx for:
- Creating threads (`hx-post`)
- Sending messages (`hx-post`, `hx-swap="beforeend"` to append new messages)
- Deleting threads (`hx-delete`, `hx-swap="outerHTML"`)

- [ ] **Step 2: Add UI routes to handle() and ui_routes()**

In `mod.rs`, add:
```rust
fn ui_routes(&self) -> Vec<UiRoute> {
    vec![
        UiRoute::new("/b/messages/", "Messages"),
    ]
}
```

Add to `handle()`:
```rust
("retrieve", "/b/messages/") => pages::thread_list_page(ctx, msg).await,
("retrieve", path) if path.starts_with("/b/messages/threads/") => pages::thread_view_page(ctx, msg).await,
```

- [ ] **Step 3: Verify and commit**

Run: `cargo check -p solobase-core`

```bash
git add crates/solobase-core/src/blocks/messages/
git commit -m "feat(messages): maud UI for thread list and message view"
```

---

### Task 4: Provider LLM block — OpenAI-compatible API integration

**Files:**
- Modify: `crates/solobase-core/src/blocks/provider_llm/mod.rs`
- Create: `crates/solobase-core/src/blocks/provider_llm/pages.rs`

- [ ] **Step 1: Define collections and config**

The provider-llm block needs:
- A `providers` collection to store configured providers (name, type, endpoint, model list)
- Config vars for API keys: `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY`, `SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY`

Update `info()`:
```rust
fn info(&self) -> BlockInfo {
    use wafer_run::types::CollectionSchema;

    BlockInfo::new("suppers-ai/provider-llm", "0.0.1", "http-handler@v1", "Remote LLM API providers")
        .instance_mode(InstanceMode::Singleton)
        .category(wafer_run::BlockCategory::Feature)
        .requires(vec!["wafer-run/network".into(), "wafer-run/config".into(), "wafer-run/database".into()])
        .collections(vec![
            CollectionSchema::new(PROVIDERS_COLLECTION)
                .field("name", "string")
                .field("provider_type", "string")   // "openai" | "anthropic"
                .field_default("endpoint", "string", "https://api.openai.com/v1")
                .field_default("models", "text", "[]") // JSON array of model IDs
                .field_default("enabled", "int", "1"),
        ])
        .config_keys(vec![
            ConfigVar::new("SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY", "OpenAI API key", "")
                .name("OpenAI API Key")
                .sensitive(true),
            ConfigVar::new("SUPPERS_AI__PROVIDER_LLM__ANTHROPIC_KEY", "Anthropic API key", "")
                .name("Anthropic API Key")
                .sensitive(true),
        ])
        .can_disable(true)
        .default_enabled(true)
        .admin_url("/b/provider-llm/")
        .description("Remote LLM API providers. Supports OpenAI-compatible APIs and Anthropic Claude.")
        .endpoints(vec![
            BlockEndpoint::post("/b/provider-llm/api/chat").summary("Chat completion"),
            BlockEndpoint::get("/b/provider-llm/api/models").summary("List available models"),
            BlockEndpoint::get("/b/provider-llm/api/providers").summary("List configured providers"),
            BlockEndpoint::post("/b/provider-llm/api/providers").summary("Add provider"),
            BlockEndpoint::patch("/b/provider-llm/api/providers/{id}").summary("Update provider"),
            BlockEndpoint::delete("/b/provider-llm/api/providers/{id}").summary("Delete provider"),
        ])
}
```

- [ ] **Step 2: Implement chat endpoint**

The core `POST /b/provider-llm/api/chat` endpoint:
1. Accepts `{ messages: [{role, content}], model: "gpt-4o", provider_id?: "..." }`
2. Looks up the provider config (or uses default)
3. Gets the API key from config
4. Calls the provider's API via `wafer_core::clients::network::do_request()`
5. Returns the response

For OpenAI-compatible APIs:
```
POST {endpoint}/chat/completions
Authorization: Bearer {api_key}
Content-Type: application/json

{"model": "gpt-4o", "messages": [...]}
```

For Anthropic:
```
POST https://api.anthropic.com/v1/messages
x-api-key: {api_key}
anthropic-version: 2023-06-01
Content-Type: application/json

{"model": "claude-sonnet-4-20250514", "messages": [...], "max_tokens": 4096}
```

Parse the response and return a normalized format:
```json
{"content": "...", "model": "...", "usage": {"input_tokens": N, "output_tokens": N}}
```

- [ ] **Step 3: Implement provider CRUD and models listing**

- `GET /b/provider-llm/api/providers` — list from DB
- `POST /b/provider-llm/api/providers` — create provider config
- `PATCH /b/provider-llm/api/providers/{id}` — update
- `DELETE /b/provider-llm/api/providers/{id}` — delete
- `GET /b/provider-llm/api/models` — aggregate models from all enabled providers

- [ ] **Step 4: Seed default providers on lifecycle Init**

In `lifecycle()`, seed two default providers if none exist:
- OpenAI (endpoint: `https://api.openai.com/v1`, models: `["gpt-4o", "gpt-4o-mini"]`)
- Anthropic (endpoint: `https://api.anthropic.com/v1`, models: `["claude-sonnet-4-20250514", "claude-haiku-4-5-20251001"]`)

- [ ] **Step 5: Create pages.rs with provider management UI**

Simple admin page at `/b/provider-llm/`:
- List configured providers with edit/delete
- Form to add new provider (name, type, endpoint, models)
- API key input fields (from config vars)

- [ ] **Step 6: Verify and commit**

```bash
cargo check -p solobase-core && cargo check -p solobase
git add crates/solobase-core/src/blocks/provider_llm/
git commit -m "feat(provider-llm): OpenAI and Anthropic API integration"
```

---

### Task 5: LLM orchestrator block

**Files:**
- Modify: `crates/solobase-core/src/blocks/llm/mod.rs`
- Create: `crates/solobase-core/src/blocks/llm/pages.rs`

- [ ] **Step 1: Define collections and config**

The llm block needs:
- A `settings` collection for per-thread provider overrides
- Config var for default provider

Update `info()` with:
```rust
.collections(vec![
    CollectionSchema::new(SETTINGS_COLLECTION)
        .field("thread_id", "string")
        .field("provider_block", "string")  // "suppers-ai/provider-llm" or "suppers-ai/local-llm"
        .field_default("model", "string", "")
        .index(&["thread_id"]),
])
.config_keys(vec![
    ConfigVar::new("SUPPERS_AI__LLM__DEFAULT_PROVIDER", "Default LLM provider block", "suppers-ai/provider-llm")
        .name("Default Provider"),
    ConfigVar::new("SUPPERS_AI__LLM__DEFAULT_MODEL", "Default model ID", "")
        .name("Default Model"),
])
```

- [ ] **Step 2: Implement chat orchestration**

The core `POST /b/llm/api/chat` endpoint:
1. Accepts `{ thread_id: "...", message: "user message", provider?: "...", model?: "..." }`
2. Writes the user message to the messages block: `ctx.call_block("suppers-ai/messages", ...)` with a create-message request
3. Reads thread history from messages block
4. Determines which backend to use (check per-thread setting → default provider config)
5. Calls the backend: `ctx.call_block("suppers-ai/provider-llm", ...)` or `ctx.call_block("suppers-ai/local-llm", ...)`
6. Writes the assistant response to the messages block
7. Returns the response

- [ ] **Step 3: Implement provider/model listing aggregation**

- `GET /b/llm/api/providers` — calls both `provider-llm` and `local-llm` to aggregate available providers
- `GET /b/llm/api/models` — same, aggregate models
- `GET /b/llm/api/config` — get default provider + per-thread overrides
- `POST /b/llm/api/config` — set default provider or per-thread override

- [ ] **Step 4: Create pages.rs with chat UI**

The main chat interface at `/b/llm/`:
- Left sidebar: thread list (from messages block)
- Main area: message history + input
- Top bar: model picker dropdown, provider indicator
- "New Chat" button creates a new thread

The chat input uses htmx:
```html
<form hx-post="/b/llm/api/chat" hx-target="#messages" hx-swap="beforeend">
    <input name="thread_id" type="hidden" value="{thread_id}">
    <textarea name="message" placeholder="Type a message..."></textarea>
    <button type="submit">Send</button>
</form>
```

The response returns an HTML fragment with both the user message and assistant response appended.

Settings page at `/b/llm/settings`:
- Default provider selection
- Default model selection

- [ ] **Step 5: Verify and commit**

```bash
cargo check -p solobase-core && cargo check -p solobase
git add crates/solobase-core/src/blocks/llm/
git commit -m "feat(llm): chat orchestrator with provider routing"
```

---

### Task 6: Local LLM block — WebLLM bridge (browser only)

**Files:**
- Modify: `crates/solobase-core/src/blocks/local_llm.rs`
- Create: `crates/solobase-web/js/ai-bridge.js`
- Modify: `crates/solobase-web/src/bridge.rs` (add AI bridge functions)

This block is a stub on native (returns "not available") and functional on browser via WebLLM.

- [ ] **Step 1: Expand local_llm.rs with collections and endpoints**

```rust
fn info(&self) -> BlockInfo {
    BlockInfo::new("suppers-ai/local-llm", "0.0.1", "http-handler@v1", "Local LLM inference via WebLLM")
        .instance_mode(InstanceMode::Singleton)
        .category(wafer_run::BlockCategory::Feature)
        .collections(vec![
            CollectionSchema::new(MODELS_COLLECTION)
                .field("model_id", "string")
                .field_default("status", "string", "available")  // available, downloading, loaded
                .field_default("size_bytes", "int", "0")
                .index(&["status"]),
        ])
        .can_disable(true)
        .default_enabled(false)
        .description("Local LLM inference via WebLLM. Browser-only — requires WebGPU.")
        .endpoints(vec![
            BlockEndpoint::post("/b/local-llm/api/chat").summary("Chat with local model"),
            BlockEndpoint::get("/b/local-llm/api/models").summary("List available models"),
            BlockEndpoint::post("/b/local-llm/api/load").summary("Download and load a model"),
            BlockEndpoint::post("/b/local-llm/api/unload").summary("Unload model from VRAM"),
            BlockEndpoint::get("/b/local-llm/api/status").summary("Model load status"),
        ])
}
```

- [ ] **Step 2: Implement handle() — native stub returns 501**

On native, all endpoints return:
```json
{"error": "not_available", "message": "Local LLM requires the browser runtime with WebGPU support"}
```

The actual WebLLM integration will be wired up as a browser-specific override in solobase-web in a future task (requires JS bridge to main thread via postMessage).

- [ ] **Step 3: Create ai-bridge.js stub**

Create `crates/solobase-web/js/ai-bridge.js` with placeholder functions:

```javascript
// ai-bridge.js — postMessage bridge between Service Worker and main-thread WebLLM
// This runs in the main page (not the SW). The SW communicates via postMessage.
//
// Phase 2b will implement the full WebLLM integration.
// For now, this is a stub that the SW can check for availability.

let webllmReady = false;

export function isWebLlmAvailable() {
    return webllmReady;
}

export async function chatWithLocalModel(messagesJson) {
    throw new Error('WebLLM not yet initialized — load a model first');
}

export async function loadModel(modelId) {
    throw new Error('WebLLM integration not yet implemented');
}

export function getStatus() {
    return JSON.stringify({ available: false, loaded_model: null, vram_used: 0 });
}
```

- [ ] **Step 4: Verify and commit**

```bash
cargo check -p solobase-core && cargo check -p solobase
git add crates/solobase-core/src/blocks/local_llm.rs crates/solobase-web/js/ai-bridge.js
git commit -m "feat(local-llm): stub block + ai-bridge.js placeholder"
```

---

### Task 7: Integration test — end-to-end chat flow

**Files:** No new files — testing existing code

- [ ] **Step 1: Build and serve solobase-web**

```bash
cd crates/solobase-web && make dev
```

Serve on a port and register the SW.

- [ ] **Step 2: Test messages API**

Via fetch in the browser:
- `POST /b/messages/api/threads` — create a thread
- `GET /b/messages/api/threads` — list threads (should show 1)
- `POST /b/messages/api/threads/{id}/messages` — add a message
- `GET /b/messages/api/threads/{id}/messages` — list messages

- [ ] **Step 3: Test provider-llm API**

- `GET /b/provider-llm/api/providers` — should show seeded providers
- `GET /b/provider-llm/api/models` — should show model list

- [ ] **Step 4: Test chat UI**

Navigate to `/b/llm/` and verify the chat interface renders.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A && git commit -m "fix: integration test fixes for LLM blocks"
```

---

## Notes

### Block interaction pattern

The `llm` block calls `messages` and `provider-llm`/`local-llm` via `ctx.call_block()`. This means:
- The blocks must declare `requires` in their `BlockInfo`
- The calling block constructs a `Message` with the right `kind` and data, then calls `ctx.call_block(target_block, &mut msg)`
- The target block's `handle()` processes it and returns the result

### Streaming (future)

The current implementation returns the full response. Server-Sent Events (SSE) for streaming will be a follow-up — it requires returning a `ReadableStream` response from the Service Worker, which needs additional plumbing in `convert.rs`.

### Local LLM (Phase 2b)

The `local-llm` block is a stub in this plan. The full WebLLM integration requires:
1. `ai-bridge.js` in the main page (not SW) that initializes WebLLM
2. A `postMessage` protocol between the SW and main page
3. Model download/caching via the Cache API
4. Streaming token responses back to the SW

This is a separate, substantial task that should be its own plan.
