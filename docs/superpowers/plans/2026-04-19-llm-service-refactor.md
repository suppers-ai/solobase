# LLM Service Refactor (solobase side) — Implementation Plan

**Goal:** Replace the three-block LLM setup (`suppers-ai/llm` orchestrator, `suppers-ai/provider-llm`, `suppers-ai/local-llm`) with a consolidated single feature block backed by the new `wafer-run/llm` service block from wafer-core.

**Spec:** `docs/superpowers/specs/2026-04-15-llm-service-refactor-design.md`
**Prereq:** ✅ landed — `wafer_core::interfaces::llm::{service::LlmService, router::MultiBackendLlmService, service_blocks::llm::register_with}`.

## Architecture

```
App entry (main / solobase-web lib / solobase-cloudflare)
  │
  ├─ construct ProviderLlmService (native — reqwest via wafer_core::clients::network)
  ├─ #[cfg(target_arch = "wasm32")] construct BrowserLlmService (WebLLM wasm-bindgen)
  ├─ MultiBackendLlmService::new()
  │   .register("providers", provider_svc.clone())
  │   .register("browser",   browser_svc.clone())   // wasm32 only
  ├─ wafer_core::service_blocks::llm::register_with(&mut wafer, Arc::new(router))
  └─ register feature block "suppers-ai/llm" with provider_svc handle stashed for admin CRUD
```

The new `suppers-ai/llm` feature block owns:
- Chat UI + thread persistence (unchanged from today's orchestrator)
- Providers admin UI + CRUD (moved from `provider-llm`)
- Models admin UI (aggregated across backends)
- Chat endpoints (buffered + SSE streaming)
- Model management endpoints (load / unload / status)
- Startup `lifecycle(Init)`: load providers from DB → call `ProviderLlmService::configure(...)`
- One-shot DB migration from `suppers_ai__provider_llm__providers` → `suppers_ai__llm__providers`

## File structure

```
crates/solobase-core/src/blocks/llm/
├── mod.rs              # feature block — registration + BlockInfo + endpoints
├── routes.rs           # HTTP route handlers: chat, providers, models
├── schema.rs           # suppers_ai__llm__providers table + migration
├── ui.rs               # admin UI pages (chat, providers, models) — maud
├── providers/
│   ├── mod.rs          # ProviderLlmService + ProviderConfig + ProviderProtocol
│   ├── config.rs       # ProviderConfig + ProviderProtocol types
│   ├── openai.rs       # OpenAI native request encoder + SSE decoder
│   ├── anthropic.rs    # Anthropic /v1/messages encoder + SSE decoder
│   └── openai_compatible.rs  # Ollama/llama-server/Azure/Groq path
└── pages.rs            # EXISTING — preserved (SSR for chat), trimmed

crates/solobase-web/src/
├── lib.rs              # +register BrowserLlmService into router on wasm32
└── llm.rs              # NEW — BrowserLlmService + WebLlmEngineHandle (wasm-bindgen)

crates/solobase-web/js/
├── webllm-engine.js    # NEW, minimal — thin wasm-bindgen import surface for MLCEngine
└── (ai-bridge.js, sw.js local-llm block) — DELETED

crates/solobase-core/src/blocks/
├── provider_llm/       # DELETED (entire dir)
├── local_llm.rs        # DELETED
├── llm_backend.rs      # DELETED
└── mod.rs              # Update: remove ProviderLlm/LocalLlm BlockIds, keep Llm
```

## Phasing

The PR ships atomically (per spec) but work decomposes into four sequential phases. Each phase ends with something that compiles green and has tests.

### Phase A — `ProviderLlmService` (native-only, zero dep on existing blocks)

~10 tasks. Stands alone; doesn't touch the feature block yet. Implements `LlmService` trait from wafer-core.

1. **Module scaffold** — create `blocks/llm/providers/{mod.rs,config.rs,openai.rs,anthropic.rs,openai_compatible.rs}` as empty stubs. Wire `pub mod providers;` into `blocks/llm/mod.rs`. Verify `cargo check -p solobase-core`.
2. **`ProviderConfig` + `ProviderProtocol` types** in `providers/config.rs`. `#[derive(Serialize, Deserialize, Clone)]`. Constructor + builder. Serde roundtrip tests for the three protocol variants.
3. **OpenAI native — request encoder** in `providers/openai.rs`. `encode_chat_request(req: &ChatRequest, cfg: &ProviderConfig) -> (String /*url*/, Vec<Header>, Vec<u8> /*body*/)`. Tests assert the produced JSON matches OpenAI's documented shape for: simple text, multi-turn with system, tool definitions, temperature/max_tokens/stop_sequences.
4. **OpenAI native — SSE decoder** in `providers/openai.rs`. `decode_sse_frame(bytes) -> Vec<ChatChunk>`. Handles OpenAI's `data: {json}\n\n` frames and `data: [DONE]` sentinel. Tests: text delta, tool-call arguments delta, usage chunk on terminal, malformed frame skipped.
5. **Anthropic — request encoder** in `providers/anthropic.rs`. Maps `ChatRequest` → Anthropic `/v1/messages` shape (system as top-level, `messages` without system role, tools as Anthropic format). Tests.
6. **Anthropic — SSE decoder** in `providers/anthropic.rs`. Decodes `content_block_start`, `content_block_delta`, `message_delta`, `message_stop` into `ChatChunk`s. Maps tool-use blocks to `ToolCallStart` / `ToolCallArguments` deltas. Tests.
7. **OpenAI-compatible** in `providers/openai_compatible.rs`. Reuses `openai::encode_chat_request` but `api_key` is optional (omit `Authorization` header if `None`), and tolerates missing fields in `/v1/models` discovery responses. Tests.
8. **`ProviderLlmService` struct** in `providers/mod.rs`. Inner state: `RwLock<HashMap<String, ProviderConfig>>` + `reqwest::Client` (or the wafer_core network abstraction — pick whichever the existing `provider_llm/mod.rs` uses; survey says `wafer_core::clients::network::do_request`). Concrete methods: `new()`, `configure(Vec<ProviderConfig>)`, `discover_models(name)`.
9. **`LlmService` impl for `ProviderLlmService`**. `chat_stream` routes by `backend_id` → looks up `ProviderConfig` → calls the right protocol encoder → issues streaming HTTP → pipes decoded chunks into returned `BoxStream`. `list_models` aggregates from cached per-provider model lists. `status` returns `Ready` if endpoint reachable. `claims_backend` = "name exists in providers map". `load_model` / `unload_model` keep default `NotSupported`.
10. **Integration test** in `crates/solobase-core/tests/provider_llm_service.rs` — stub the HTTP transport (use `wiremock` or a hand-rolled fake), assert that a `chat_stream` against a provider emits the expected chunk sequence, including cancellation on drop.

**Checkpoint:** `ProviderLlmService` works standalone, covered by unit + integration tests.

---

### Phase B — Feature block rewrite

~9 tasks. Ships the new `suppers-ai/llm` endpoints + admin UI + startup wiring + DB migration, all talking to `wafer-run/llm` via `ctx.call_block("wafer-run/llm", ...)` in the chat-streaming path.

11. **DB schema** in `blocks/llm/schema.rs`. `suppers_ai__llm__providers` table: `id (pk)`, `name (unique)`, `protocol (text, enum-as-string)`, `endpoint (text)`, `api_key_encrypted (text, nullable)`, `key_var (text, nullable)`, `models (json)`, `enabled (bool)`, timestamps. Declared via `wafer_core::interfaces::database::service::Table`.
12. **Startup wiring** — `lifecycle(Init)` on the feature block: ensure schema, load rows, translate to `Vec<ProviderConfig>`, call `ProviderLlmService::configure(...)`. The `ProviderLlmService` `Arc` is passed in via the block's constructor. App entries (`solobase-native/src/main.rs`, `solobase-web/src/lib.rs`, `solobase-cloudflare/src/lib.rs`) construct the service, build the router, register with `register_with`, and stash the `Arc<ProviderLlmService>` on the feature block instance.
13. **Migration task** — in the same `lifecycle(Init)`, if `suppers_ai__provider_llm__providers` exists: read rows, map fields, insert into `suppers_ai__llm__providers`, drop old table. Guard with a marker row in a migrations table to run once.
14. **Chat endpoints** in `blocks/llm/routes.rs`:
    - `POST /b/llm/api/chat` — buffered — collects the output stream into one JSON response.
    - `POST /b/llm/api/chat/stream` — SSE — forwards `StreamEvent::Chunk` as `data: {json}\n\n`. Piped from `ctx.call_block("wafer-run/llm", llm.chat, body)`.
    - Preserves thread/message persistence from current `llm/mod.rs:handle_chat`.
15. **Providers CRUD endpoints** — `GET/POST /b/llm/api/providers`, `PATCH/DELETE /b/llm/api/providers/:id`, `POST /b/llm/api/providers/:id/discover-models`. Admin-only via `RouteAccess::Admin` on `ExtraRoute`. Each write calls the feature block's cached `provider_svc.configure(...)` to push updates down.
16. **Models endpoints** — `GET /b/llm/api/models` (aggregated via `ctx.call_block("wafer-run/llm", llm.list_models)`), `GET /b/llm/api/models/:backend_id/:model_id/status`, `POST /b/llm/api/models/:backend_id/:model_id/load` (SSE LoadProgress), `POST .../unload`.
17. **Admin UI — providers page** in `blocks/llm/ui.rs`. maud template. Table of providers with add/edit/delete + discover-models actions. Consolidates what was split between `llm/settings` + `provider-llm/admin`.
18. **Admin UI — models page**. Table of models aggregated across providers. Columns: name, backend, capabilities badges, status (Ready/Loading/Unloaded). For local-only backends: load/unload actions.
19. **Admin UI — chat page**. Mostly preserved from current `llm/pages.rs`. New model-picker populated from `/b/llm/api/models`.

**Checkpoint:** Native (and CF) builds produce a working LLM feature block. Browser still uses the old `ai-bridge.js` path until Phase C.

---

### Phase C — `BrowserLlmService` (wasm32 only)

~5 tasks. Replaces `ai-bridge.js` + service-worker bridge with a proper Rust impl.

20. **`WebLlmEngineHandle` in `solobase-web/src/llm.rs`** — wasm-bindgen import surface for MLCEngine methods we need: `create`, `chat.completions.create` (streaming), `unload`. A small accompanying `js/webllm-engine.js` hosts the MLC import (one file, minimal — just exports the typed surface wasm-bindgen binds against).
21. **`BrowserLlmService` struct** — `engine: RefCell<Option<WebLlmEngineHandle>>`, `available_models: Vec<ModelInfo>` (constant — the current WebLLM model list). `LlmService` impl starts with `claims_backend` (prefix `webllm-*`) + `list_models` (returns the constant).
22. **`BrowserLlmService::chat_stream`** — if engine is None, `NotSupported`. Otherwise `mpsc::channel(16)`, `spawn_local` to drive WebLLM async iterator, forward each frame as `ChatChunk`, check cancel token each iteration. Return `ReceiverStream`.
23. **`BrowserLlmService::{load_model, status, unload_model}`** — `load_model` streams `LoadProgress` via WebLLM's progress callback (wasm-bindgen closure). `status` reflects the current `engine` state.
24. **Wire into solobase-web entry** — construct `BrowserLlmService`, register into the router under `"browser"`. Remove block registrations for `local-llm`. Delete `ai-bridge.js` + service-worker interception of `/b/local-llm/api/*` (keep the SW bridge infrastructure used for other things).

**Checkpoint:** Browser path flows through `BrowserLlmService` → `LlmBlock` → the new chat routes, not ai-bridge.

---

### Phase D — Cleanup + PR

~3 tasks.

25. **Delete old files.** `crates/solobase-core/src/blocks/{provider_llm/, local_llm.rs, llm_backend.rs}`. `crates/solobase-web/js/ai-bridge.js`. Service-worker `handleLocalLlm` in `sw.js` and its routing. Config-var declarations for `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY` / `_ANTHROPIC_KEY`.
26. **Update `blocks/mod.rs`.** Remove `BlockId::ProviderLlm` + `BlockId::LocalLlm`, their `pub mod` lines, entries in `solobase_blocks()` + `make_block()`. Update `routing.rs` to drop `/b/provider-llm` + `/b/local-llm` entries.
27. **Format + clippy + push + PR.** `cargo +nightly fmt --all`, `cargo clippy -p solobase-core -p solobase-web -p solobase-native --all-targets -- -D warnings`, `cargo test -p solobase-core -p solobase-web`, push, open PR.

---

## Out of scope

- `wafer_core::clients::llm` typed client wrapper — follow-up.
- `EmbeddingService` unification — separate spec.
- Rate-limiting / fallback-chain wrappers — noted in spec as future decorator pattern.

## Self-review

**Spec coverage:** §`ProviderLlmService` → Phase A (tasks 2–9). §`BrowserLlmService` → Phase C. §Feature block responsibilities → Phase B (11–19). §Migration → task 13. §Removals → Phase D (25–26). §Testing Strategy unit + integration → tasks 3–7, 10.

**Phase ordering ratchet:** Phase A is independent of everything else. Phase B depends on A. Phase C only touches solobase-web; can run in parallel with B if we want, but serializing keeps the commit log readable. Phase D is cleanup.

**Placeholders:** none — each task names exact files + what goes where. Detailed wire-format code is deferred to the task when written (spec has the shapes).
