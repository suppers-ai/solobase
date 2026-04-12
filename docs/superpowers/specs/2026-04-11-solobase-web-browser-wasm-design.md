# Solobase Web: Browser WASM Build

Full Solobase compiled to WASM, running entirely in the browser via a Service Worker. Feature parity with the native binary.

## Phases

This project is split into two independent phases:

**Phase 1: solobase-web** — Browser WASM runtime. All existing Solobase blocks running in the browser with sql.js + OPFS for database, OPFS for file storage, browser fetch for network. No AI-specific code. Any Solobase app works in the browser. Demo site replacement. Site updates.

**Phase 2: LLM blocks** — `suppers-ai/messages`, `suppers-ai/llm`, `suppers-ai/local-llm`, `suppers-ai/provider-llm`. Builds on Phase 1 but also works on the native binary. The AI chat app is one application built on top of Solobase, enabled by these blocks.

Phase 1 is valuable on its own — users can build and run any Solobase app in the browser without installing anything. Phase 2 adds AI capabilities on top.

---

# Phase 1: solobase-web (Browser WASM Runtime)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Browser Tab                       │
│                                                      │
│  ┌──────────────┐    fetch()    ┌─────────────────┐ │
│  │  Main Page   │ ──────────── │  Service Worker  │ │
│  │  (htmx UI)   │ ◄─────────── │                  │ │
│  │              │   HTML/JSON   │  solobase-web    │ │
│  └──────────────┘               │  .wasm           │ │
│                                 │                  │ │
│  ┌──────────────┐               │  ┌────────────┐ │ │
│  │  WebGPU /    │ ◄─────────── │  │ llm block  │ │ │
│  │  WebLLM      │               │  │            │ │ │
│  └──────────────┘               │  └────────────┘ │ │
│                                 │                  │ │
│                                 │  ┌────────────┐ │ │
│                                 │  │ sql.js     │ │ │
│                                 │  │ + OPFS     │ │ │
│                                 │  └────────────┘ │ │
│                                 │                  │ │
│                                 │  ┌────────────┐ │ │
│                                 │  │ OPFS file  │ │ │
│                                 │  │ storage    │ │ │
│                                 │  └────────────┘ │ │
│                                 └─────────────────┘ │
└─────────────────────────────────────────────────────┘
```

The Service Worker is the "server." It boots the WASM Solobase, intercepts all fetch requests from the main page, routes them through the WAFER runtime, and returns responses. From htmx's perspective nothing changes — it makes HTTP requests and gets HTML back.

In Phase 2, the AI model runs in the main thread (WebGPU requires main thread or dedicated worker access) and communicates with the Service Worker via `postMessage`.

## Crate Structure

New crate: `solobase/crates/solobase-web/`

```
solobase-web/
├── Cargo.toml          # wasm-bindgen, wasm-pack target
├── src/
│   ├── lib.rs          # wasm-bindgen entry point, SW fetch handler
│   ├── database.rs     # DatabaseService impl wrapping sql.js via JS bindings
│   ├── storage.rs      # StorageService impl wrapping OPFS
│   └── network.rs      # NetworkService impl wrapping browser fetch()
└── js/
    ├── loader.js       # Registers SW, loads WASM, bootstraps Solobase
    ├── sw.js           # Service Worker shell — loads solobase-web.wasm
    └── ai-bridge.js    # postMessage bridge between SW and main-thread AI (Phase 2)
```

Workspace addition:

```toml
members = ["crates/solobase", "crates/solobase-core", "crates/solobase-web"]
```

## Browser Service Implementations

Block registration is identical to the native binary — `solobase-web` calls the same `solobase_core::blocks::register()` functions. Only the service implementations differ:

| Service | Native (`solobase`) | Browser (`solobase-web`) |
|---------|-------------------|------------------------|
| Database | `wafer-block-sqlite` (rusqlite) | `BrowserDatabaseService` (sql.js via wasm-bindgen) |
| Storage | `wafer-block-local-storage` (std::fs) | `BrowserStorageService` (OPFS) |
| Network | `wafer-block-network` (reqwest) | `BrowserNetworkService` (fetch API) |
| HTTP listener | `wafer-block-http-listener` (hyper/axum) | SW `onfetch` handler |
| Config | env vars + rusqlite | OPFS-backed config + sql.js |

`solobase-core` stays untouched. All existing blocks work because they talk to service traits, not implementations.

## Service Worker Lifecycle

### Bootstrap sequence

1. User visits the hosted URL
2. `loader.js` runs, registers `sw.js` as the Service Worker
3. SW activates, loads `solobase_web_bg.wasm` via wasm-bindgen
4. WASM init function runs: creates WAFER runtime, registers all solobase-core blocks, opens sql.js database from OPFS, seeds default config if first run
5. SW calls `clients.claim()` to take control immediately
6. Page reloads (or loader navigates) — now all fetches go through the SW
7. First request hits the WAFER router, returns the dashboard HTML

### Request flow

```
htmx fetch("/b/auth/login", POST, body)
  → Service Worker onfetch event
    → deserialize into WAFER Message (path, method, headers, body)
      → WAFER runtime dispatches to auth block
        → auth block calls database service (sql.js)
        → auth block calls crypto (argon2 in WASM)
        → returns maud-rendered HTML
      → serialize Response (status, headers, HTML body)
    → SW returns new Response(...)
  → htmx swaps the HTML into the page
```

### Persistence

- sql.js database flushed to OPFS on every write (or debounced)
- OPFS files persist across browser sessions
- No network required after initial load (except for remote AI providers or external API calls)

### First visit vs. return visit

- First visit: downloads WASM + sql.js + static assets, seeds database
- Return visit: SW cached, WASM cached, database in OPFS. Instant boot.

---

# Phase 2: LLM Blocks

## New Blocks

### `suppers-ai/messages` — Generic message/thread system

Not AI-specific. Reusable for chat, task management, support tickets, notebooks, etc.

**Collections:**
- `threads` — id, title, created_at, updated_at, metadata (JSON)
- `messages` — id, thread_id, role (string — "user", "assistant", "system", etc.), content, created_at, metadata (JSON)

**Operations:**
- `messages.threads.list` / `create` / `get` / `update` / `delete`
- `messages.messages.list` (by thread) / `create` / `get` / `update` / `delete`

### `suppers-ai/llm` — LLM orchestrator

Thin routing layer. Receives chat requests, reads thread history from `messages`, picks which backend block to call based on config or per-message override, writes the response back.

**Operations:**
- `llm.chat` — takes a thread_id, optional provider override, returns streaming response. Routes to `local-llm` or `provider-llm` based on config/request. Writes the assistant message back to the thread via the messages block.
- `llm.providers` — aggregates available providers from all backend blocks
- `llm.models` — aggregates available models from all backend blocks
- `llm.config` — get/set default provider, per-thread provider overrides

**Routing logic:**
- Default provider configured in settings (e.g. "use local-llm for everything")
- Per-message override: chat request can specify `provider: "provider-llm/claude-sonnet"` to use a remote model for a single message
- Per-thread default: a thread can be pinned to a specific provider

### `suppers-ai/local-llm` — Local model inference

Browser-only block. Manages WebLLM models running via WebGPU in the main thread.

**Operations:**
- `local-llm.chat` — run inference on a loaded model, return streaming response
- `local-llm.models` — list available/downloaded models
- `local-llm.load` — download and activate a model
- `local-llm.unload` — free VRAM
- `local-llm.status` — model load progress, VRAM usage, active model

**WebLLM bridge:**

The model runs in the main thread (WebGPU access). When `local-llm` receives a chat request in the SW, it sends a `postMessage` to the main page. `ai-bridge.js` forwards to WebLLM, streams tokens back via `postMessage`. For streaming to the UI, the block returns a `ReadableStream` response consumed via SSE.

### `suppers-ai/provider-llm` — Remote API providers

Works on both native and browser. HTTP calls to remote LLM APIs via the network block.

**Operations:**
- `provider-llm.chat` — send messages to a remote API, return streaming response
- `provider-llm.models` — list available models for configured providers
- `provider-llm.config` — add/remove/update API provider configs (endpoint, API key, model list)

**Built-in provider support:**

| Provider | Config needed | Notes |
|----------|--------------|-------|
| OpenAI-compatible | API key + endpoint | Covers OpenAI, Ollama, vLLM, any OpenAI-compatible API |
| Anthropic | API key | Claude API |

API keys stored via the config system (`SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY`, etc.).

### LLM block interface

All three blocks share a common trait for the chat interface:

```rust
// Uses the same conditional Send pattern as the rest of the codebase:
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait LlmBackend {
    async fn chat(&self, messages: Vec<ChatMessage>, config: ChatConfig) -> Result<ChatStream>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
}
```

### Usage pattern

- Chat UI reads from `messages` to display threads/messages
- User sends a message → written to `messages`, then `llm.chat` called with the thread_id
- `llm` block reads thread history from `messages`, routes to `local-llm` or `provider-llm`
- Backend block runs inference, streams tokens back
- `llm` block writes the assistant message to `messages`
- User can override per-message: "use Claude for this one" sends to `provider-llm` even if default is local

## Build & Distribution

### Build

```
wasm-pack build solobase-web --target web
```

Produces `solobase_web_bg.wasm` + `solobase_web.js` (wasm-bindgen glue).

### Distribution

Two options that coexist:

1. **Hosted static site** — deploy to any CDN (Cloudflare Pages, Netlify, S3). Just static files. Zero backend cost.
2. **Self-hosted** — the native `solobase` binary serves the `solobase-web` assets at `/web/`. "Try before you install" or PWA companion.

### Asset sizes

| Asset | Phase | Size (est.) | Caching |
|-------|-------|------------|---------|
| `solobase_web_bg.wasm` | 1 | 5-8MB gzip | Immutable hash, long cache |
| `sql-wasm.wasm` (sql.js) | 1 | ~1MB gzip | Immutable hash, long cache |
| `loader.js` + `sw.js` | 1 | ~5KB | Short cache, controls SW version |
| `ai-bridge.js` | 2 | ~5KB | Short cache |
| WebLLM model (e.g. Phi-3-mini) | 2 | 2-4GB | Cached in browser Cache API / OPFS |

Phase 1 first visit: ~6-9MB for the full app. Phase 2 AI model downloaded separately when the user chooses to.

### PWA

Service Worker is already present. Adding a `manifest.json` makes it installable as a PWA — "Add to Home Screen" for a native app experience.

---

# Changes to Existing Code

## Phase 1 changes

**New crate: `solobase-web`** — `solobase/crates/solobase-web/`. Browser service implementations + SW entry point.

**solobase-core — no changes.** All existing blocks work as-is.

**wafer-core — likely no changes.** Service traits are already abstract. Verify that trait bounds don't block wasm32 compilation (existing `?Send` pattern suggests this was already handled).

**solobase-site** — add "Browser (No Install)" to Get Started section, replace demo link.

**deploy/demo/** — retire Fly.io deployment (Dockerfile, fly.toml).

**Native `solobase` binary — untouched.**

**`solobase-cloudflare` — untouched.**

## Phase 2 changes

**solobase-core — four new blocks:**
- `src/blocks/messages/` — `suppers-ai/messages` (generic threads + messages)
- `src/blocks/llm/` — `suppers-ai/llm` (orchestrator/router)
- `src/blocks/local_llm/` — `suppers-ai/local-llm` (WebLLM, browser-only)
- `src/blocks/provider_llm/` — `suppers-ai/provider-llm` (remote APIs, both runtimes)

**solobase-web** — register the new blocks, add `ai-bridge.js` for WebLLM postMessage bridge.

**Native `solobase` binary** — register `messages`, `llm`, and `provider-llm` blocks (not `local-llm`).

## Solobase Site Changes

The solobase-site (`packages/solobase-site/`) already has a "Get Started" section with platform-specific downloads (Linux AMD64, Linux ARM, macOS Apple Silicon, macOS Intel, Windows). The browser version appears alongside these as another platform option:

**New entry in the Get Started grid:**
- **Browser (No Install)** — "Try Now" button linking to the hosted static site
- Tagline: "No download. No setup. Runs entirely in your browser."
- Badge/label to indicate it's the same full Solobase, not a limited demo

**Constraints note on the site:**
A small expandable section or tooltip on the browser option noting:
- Data is local to the browser (per-origin, no sync between devices)
- Storage limited by browser quotas (typically 1-20GB)
- WebGPU required for local AI models (Chrome/Edge stable, Firefox behind flag, Safari partial)
- OAuth and Stripe webhooks require additional configuration
- No background processing when the tab is closed

This keeps the messaging honest without discouraging people from trying it. The constraints are natural trade-offs, not bugs.

**Demo site replacement:**

The current demo (`deploy/demo/`) runs a native binary on Fly.io in `READONLY_MODE` — a read-only, pre-seeded instance. Replace it with the browser WASM version:

- The "Demo" link in site navigation points to the hosted `solobase-web` static site (e.g. `demo.solobase.dev`)
- Users get a fully interactive demo, not read-only. They can create accounts, add data, configure blocks — all local to their browser.
- No Fly.io instance to maintain. No server cost. No security concern — everything runs in the user's browser, nothing is shared.
- Pre-seed the sql.js database with example data on first load (same seed data currently used in the Fly.io demo) so the demo feels populated out of the box.
- `deploy/demo/` (Dockerfile, fly.toml) can be retired.

---

# Constraints & Limitations

**Browser constraints (Phase 1):**
- **Per-origin data.** No sync between devices. Local-only by design.
- **Storage quotas.** Browsers allow 1-20GB per origin.
- **Service Worker lifecycle.** Browsers can terminate idle SWs. WASM module reloads from OPFS on cold restart (~100-500ms latency).
- **No background processing.** When the tab is closed, nothing runs.

**Feature parity gaps (Phase 1):**
- **OAuth callbacks** — need correct redirect URL configured for the hosted origin. Works, just requires setup.
- **Stripe webhooks** — no inbound connections. Products block works for reading but can't receive webhook events. Could poll instead.
- **Email sending** — Mailgun API key stored in browser storage. Acceptable for personal use, not shared deployments.

**AI-specific constraints (Phase 2):**
- **WebGPU support** — Chrome and Edge: stable. Firefox: behind flag. Safari: partial. Affects local AI availability, not Solobase itself.
- **Storage quotas with models** — Large AI models (2-4GB each) consume significant browser quota. Users may need to manage downloaded models.
- **VRAM** — Local model performance depends on user's GPU. Low-end devices may struggle with larger models.
