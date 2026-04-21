# LLM Service Refactor — Follow-ups

Known non-blocking follow-ups after the Phase A+B+C+D refactor landed on `main` (PRs #6, #7, #8). None of these are live regressions; all are documented dormant breaks or small polish items.

## 1. Replace dead `window.solobaseAI.*` calls in chat page JS

**Where:** `crates/solobase-core/src/blocks/llm/pages.rs`

The chat page still embeds JS that calls `window.solobaseAI.loadModel`, `.unloadModel`, `.getStatus`, `.chat`, and `.populateLocalModels`. Those symbols were defined in `js/ai-bridge.js`, which Phase D deleted. The calls now silently no-op — **chat itself still works** (the fetch-based path through `/b/llm/api/chat(/stream)` hits `BrowserLlmService` via `wafer-run/llm`), but the model-load / unload / status UX on the chat page is dead.

**Fix:** rewrite those JS calls to `fetch('/b/llm/api/models/{backend_id}/{model_id}/{load|unload|status}')` and `fetch('/b/llm/api/models')` for the picker population. The new endpoints already exist (Phase B task 16).

**Scope:** ~30 min of JS editing + a manual pass through the chat page. No Rust changes needed.

## 2. HTML-fragment responses on model-status endpoint

**Where:** `crates/solobase-core/src/blocks/llm/routes.rs::model_status` + the models admin page (`blocks/llm/ui.rs`)

The models admin page (Phase B task 18) renders a lazy `hx-get` per row against `/b/llm/api/models/{b}/{m}/status`. That endpoint currently returns JSON, so htmx replaces the status cell with raw JSON. The `badge-ready` / `badge-loading` / `badge-unloaded` / `badge-error` classes don't bind cleanly.

**Fix:** branch on `Accept: text/html` in `model_status` and emit a maud-rendered `<span class="badge badge-ready">Ready</span>` (or matching class per status variant). JSON path unchanged.

**Scope:** small — one handler + a pure maud helper. Add a unit test for each variant's HTML output.

## 3. SSE streaming assistant-message persistence

**Where:** `crates/solobase-core/src/blocks/llm/routes.rs::handle_chat_stream`

The buffered `handle_chat` persists the assistant reply via `messages_create` at the end of the stream. The SSE `handle_chat_stream` **does not** — there's an in-file `TODO(llm-phase-b-task-14)`. Root cause: `from_producer`'s spawn boundary is `'static`, so the closure can't capture `&dyn Context` / `&Message`.

**Fix options:**
- Restructure `dispatch_chat` / `handle_chat_stream` to collect the full text server-side (accumulate inside the producer) and fire a post-stream `messages_create` via a separate task that owns an owned `Message` + a way to invoke the messages block. Requires a way to reach the context from outside the producer — may need a small wafer-run API addition or a clone/ownership helper.
- Alternative: let the client persist via a separate POST after the stream closes. Simpler but adds a client-side responsibility.

**Scope:** medium. Option A is the right long-term fix but needs a design pass.

## 4. Recreate `asset-bridge.js` if any block declares `external_assets`

**Where:** `crates/solobase-web/src/asset_loader.rs` + `js/` directory

`ai-bridge.js` hosted the `load-asset-request` handler that `SwAssetLoader` posts to. With `ai-bridge.js` deleted, that handler is gone. **No in-tree block currently declares `external_assets`**, so this is dormant.

**When it matters:** if/when a block (likely `gizza-ai` or similar) declares external assets, the asset-loader JS surface needs recreating — likely as a dedicated `js/asset-bridge.js` loaded only on pages that use it. `asset_loader.rs` has doc comments pointing at this.

**Scope:** small once needed; ~50 LOC of JS + a page-scoped `<script>` tag.

## 5. Cross-repo: `solobase-cloud` alignment

**Where:** `solobase-cloud` repo (separate from solobase)

`solobase-cloud` hosts the CF Worker entry (`solobase-cloudflare`) mentioned in the original Phase B plan task 12. Phase B/D changes that may affect it:

- `BlockId::ProviderLlm` and `BlockId::LocalLlm` are gone. If `solobase-cloud` references them by name — e.g., in a block-settings list — that'll fail to compile.
- `SolobaseBuilder::llm_service(label, svc)` setter is new. The CF Worker can register its own `LlmService` (e.g., an adapter for Workers AI) via this hook.
- The `llm` Cargo feature on `solobase-core` defaults to on. `solobase-cloud` may or may not want it; if it wants the feature block but not the reqwest-based provider service, it needs a different feature shape (currently `llm` gates both — could split into `llm-feature-block` vs `llm-provider-native` later if needed).

**Fix:** audit `solobase-cloud` for `BlockId::ProviderLlm|LocalLlm` / `provider-llm` / `local-llm` string references. Patch as needed. If Workers AI integration is wanted, add an adapter crate + register via `.llm_service("workers-ai", svc)`.

**Scope:** small audit; medium if Workers AI adapter is in scope.

## 6. Config-var cleanup for `SUPPERS_AI__PROVIDER_LLM__*`

**Where:** `suppers_ai__admin__variables` DB table + admin UI

The `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY` / `_ANTHROPIC_KEY` `ConfigVar::new(...)` declarations were removed in Phase D (they lived in the deleted `provider_llm/mod.rs`). **Stored values stay** because migrated rows reference them via `key_var`. That works but is awkward — the admin UI no longer shows these keys in the "known vars" list, even though the migration still points at them.

**Fix options:**
- (a) Rename: in the migration, also UPDATE the admin variables row to rename `SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY` → `SUPPERS_AI__LLM__OPENAI_KEY`, then add `ConfigVar::new("SUPPERS_AI__LLM__OPENAI_KEY", ...)` declarations in the new `llm` block. Clean but destructive — admins who'd already renamed via the old UI would have a bad time.
- (b) Redeclare: add `ConfigVar::new("SUPPERS_AI__PROVIDER_LLM__OPENAI_KEY", ...)` (or a more neutral naming) to the new `llm` block's `config_keys` so the UI surfaces the key. Less clean but zero migration risk.
- (c) Leave as-is: admins rename / re-enter manually post-upgrade.

**Scope:** small for option (b) or (c); medium for (a) because the migration becomes more complex and needs UPSERT semantics on the admin variables table.

## 7. PR description on #6

PR #6 was titled "ProviderLlmService impl (preparatory)" but ended up absorbing Phase B/C/D through rebase+merges before landing on `main`. Not fixable retroactively, but worth noting in any release notes / CHANGELOG that the commits on `main` touch more than the title suggests.
