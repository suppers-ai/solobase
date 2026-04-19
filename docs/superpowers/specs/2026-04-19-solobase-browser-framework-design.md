# solobase-browser Framework Design

**Date:** 2026-04-19
**Status:** Design approved; pending implementation plan
**Sub-project:** 1 of 4 in the solobase framework refactor

## Problem

`crates/solobase-web/` today is structured as an application, not a framework. It bundles:

- Rust-side platform services (bridge, database, storage, network, crypto, logger, asset_loader) that any WAFER-in-browser project needs.
- JS-side static assets (sw.js, loader.js, bridge.js, sql.js integration, HTML shell) that any browser project needs.
- WAFER runtime wiring specific to Solobase.
- The Solobase admin app itself (block registrations, feature-specific routing).

Third-party projects (gizza-ai being the concrete example) want to build WAFER-in-browser applications without duplicating all of this. Today they do:

- Copy-paste `src/bridge.rs`, `src/database.rs`, `src/storage.rs`, `src/network.rs` verbatim (`gizza-ai/src/bridge.rs:5` literally says "Copied from solobase-web").
- Vendor `site/sw.js`, `site/loader.js`, `site/bridge.js` and hand-edit for their project.
- Reach across the filesystem at build time: `cp ../solobase/crates/solobase-web/pkg/sql-wasm-esm.js dist/`.

Consequences:

- Bug fixes propagate by hand, or not at all.
- Cross-repo filesystem coupling breaks any build that isn't a local sibling checkout (CI, npm consumers).
- Changes to the browser platform layer can't be made confidently because we don't know who has copied what.

## Non-Goals

- CLI tool `solobase build --target {wasm,native}` (sub-project 3).
- Native framework extraction (sub-project 2).
- Migration of gizza-ai onto the framework (sub-project 4).
- Unifying `App` into a target-agnostic `solobase::App` facade. Deferred until both browser and native frameworks exist.
- Changes to the npm package `packages/solobase-web/`.
- Feature-block extraction. `solobase-core` feature blocks stay in `solobase-core`; the framework does not depend on them.

## Chosen Approach

**Extract browser platform services into a new crate `solobase-browser` that owns the reusable Rust code, vendors the JS/HTML/sql.js assets via `include_bytes!`, and exposes a minimal `App` API plus two framework entrypoints. Migrate `solobase-web` onto the new framework as the first validation consumer.**

User code is plain Rust with no macros or inventory tricks. The user crate is a `cdylib` that defines its own `#[wasm_bindgen]` entrypoints and calls into the framework's `framework_init(register_fn)` and `framework_handle_request(req)` async functions. This mirrors how axum consumers write `#[tokio::main]`+`axum::serve(...)` — idiomatic, explicit, zero hidden control flow. When the CLI ships in sub-project 3, it can generate this boilerplate for users who want zero-setup, but the boilerplate is always optional.

Alternative approaches considered and rejected:

- **Inventory/ctor-style auto-registration.** User writes `#[solobase::register] fn register(app: &mut App)` and a linker-inserted registry calls it at startup. Hidden control flow; relies on linker-level hacks; confusing when debugging.
- **Macro-generated entrypoints (`export_entrypoints!(register);`).** Less hidden than inventory but still macro-generated code in every user crate.
- **CLI-generated wrapper crate.** Clean for users but depends on the CLI existing (not until sub-project 3) and puts generated code in a temp dir, which is confusing during debugging.

## Architecture

### New crate: `crates/solobase-browser/`

```
crates/solobase-browser/
├── Cargo.toml
├── src/
│   ├── lib.rs             — framework_init(), framework_handle_request(), re-exports
│   ├── app.rs             — App type wrapping Wafer with register_block()
│   ├── assets.rs          — static_assets(), write_to()
│   ├── bridge.rs          — moved from solobase-web
│   ├── database.rs        — moved from solobase-web
│   ├── storage.rs         — moved from solobase-web
│   ├── network.rs         — moved from solobase-web
│   ├── crypto.rs          — moved from solobase-web
│   ├── logger.rs          — moved from solobase-web
│   ├── asset_loader.rs    — moved from solobase-web
│   └── convert.rs         — moved from solobase-web (if generic; see open questions)
├── assets/
│   ├── sw.js.tmpl
│   ├── loader.js
│   ├── bridge.js
│   ├── index.html.tmpl
│   └── vendor/
│       ├── sql-wasm-esm.js       — vendored from sql.js 1.11.0
│       └── sql-wasm.wasm         — vendored from sql.js 1.11.0
├── bin/
│   └── export-assets.rs   — small CLI binary: writes assets to a dir, runs the bundler
└── tools/bundle/          — former crates/solobase-web-bundle moved here (private module)
    └── (content from PR #4)
```

Dependencies: `wafer-run`, `wafer-core`, `wafer-block`, `wafer-block-*` (browser-compatible backend blocks), `wasm-bindgen`, `web-sys`, `js-sys`, `serde`, `serde_json`. Does not depend on `solobase-core` or any feature blocks.

### Refactored crate: `crates/solobase-web/`

After migration:

```
crates/solobase-web/
├── Cargo.toml             — depends on solobase-browser + solobase-core
├── src/
│   ├── lib.rs             — thin wasm-bindgen wrappers + app's register() fn
│   └── config.rs          — app-specific config (kept)
├── js/
│   └── ai-bridge.js       — Solobase's local-LLM integration (kept)
└── Makefile               — invokes solobase-browser's export-assets binary
```

The app's `src/lib.rs` becomes roughly:

```rust
use solobase_browser::{framework_init, framework_handle_request, App};
use solobase_core::blocks;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    framework_init(register).await
}

#[wasm_bindgen]
pub async fn handle_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    framework_handle_request(req).await
}

fn register(app: &mut App) -> Result<(), String> {
    app.register_block("suppers-ai/auth", blocks::auth::AuthBlock::new())?;
    app.register_block("suppers-ai/admin", blocks::admin::AdminBlock::new())?;
    app.register_block("suppers-ai/files", blocks::files::FilesBlock::new())?;
    // ... remaining feature blocks
    Ok(())
}
```

Everything else that currently lives in `solobase-web/src/*.rs` moves to `solobase-browser`. The `solobase-web-bundle` workspace member is relocated to `solobase-browser/tools/bundle/` and becomes an internal module of the framework; its public `run` function stays, so the bundler remains unit-testable.

### Dependency diagram

```
solobase-browser  (NEW framework: Rust services + vendored JS assets + bundler + App)
        ▲
        │
solobase-web  (REFACTORED: ~30 lines of wasm-bindgen glue + solobase's register fn)
        │
        ▼  (SW host delivers solobase_web_bg.wasm + framework assets to the browser)
```

## Public API

### `solobase_browser::App`

```rust
pub struct App {
    wafer: wafer_run::Wafer,
}

impl App {
    /// Register a WAFER block with the runtime.
    ///
    /// Block names must follow `{org}/{block}` convention; duplicate names are rejected.
    pub fn register_block<B>(&mut self, name: impl Into<String>, block: B) -> Result<(), String>
    where
        B: wafer_block::WaferBlock + 'static;
}
```

That is the entire user-facing surface for MVP. No `intercept_routes`, no `seed_config`, no service swapping, no middleware. Defaults:

- SW URL interception prefixes: `/b/`, `/health`, `/openapi.json`, `/.well-known/agent.json`. Hardcoded in `sw.js.tmpl`. Identical to current Solobase behavior.
- Platform services (sql.js database, OPFS storage, fetch network, browser crypto, console logger, asset loader) are auto-registered with `Wafer` before the user's `register` runs. The user cannot remove them; they're the point of the framework.
- Config is env-driven (`SOLOBASE_*` convention), no in-code seeding API. If a consumer needs to seed config, they do it outside the framework entry.

Additional extensibility (`intercept_routes`, service swapping, middleware) is backwards-compatibly addable later when a concrete consumer need arrives.

### Framework entrypoints

```rust
pub async fn framework_init<F>(register_fn: F) -> Result<(), JsValue>
where
    F: FnOnce(&mut App) -> Result<(), String>;

pub async fn framework_handle_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue>;
```

`framework_init` is idempotent (guard against double invocation; return Ok if already initialized). It:

1. Installs browser platform services on a `Wafer` instance.
2. Calls the user's `register_fn` with a mutable `App` that holds the `Wafer`.
3. Starts the runtime (`wafer.start_without_bind()`).
4. Stores the `Wafer` in a `thread_local` RefCell for `framework_handle_request` to dispatch through.

`framework_handle_request` converts `web_sys::Request` → WAFER `Message`, dispatches through the `site-main` flow, converts the result back to `web_sys::Response`. Returns 500 on internal errors.

User code uses them like this (identical for every browser-target consumer):

```rust
use solobase_browser::{framework_init, framework_handle_request, App};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    framework_init(register).await
}

#[wasm_bindgen]
pub async fn handle_request(req: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    framework_handle_request(req).await
}

fn register(app: &mut App) -> Result<(), String> {
    // user's blocks
}
```

~10 lines of boilerplate, all explicit, all auditable.

### `solobase_browser::assets`

```rust
pub struct Asset {
    pub path: &'static str,   // e.g. "sw.js.tmpl"
    pub bytes: &'static [u8], // via include_bytes!
}

pub fn static_assets() -> &'static [Asset];

pub fn write_to(dir: &std::path::Path) -> std::io::Result<()>;
```

`static_assets()` returns the full list. `write_to(dir)` creates `dir` (if missing) and writes every asset; used by the framework's `export-assets` bin and by consumer build scripts.

Contents of `static_assets()` (MVP):

- `sw.js.tmpl`
- `loader.js`
- `bridge.js`
- `index.html.tmpl`
- `vendor/sql-wasm-esm.js` (1.11.0, vendored)
- `vendor/sql-wasm.wasm` (1.11.0, vendored)

### `export-assets` bin

Small binary inside the framework crate invoked from consumer Makefiles:

```bash
cargo run -p solobase-browser --bin export-assets -- <pkg-dir> [--dev]
```

It calls `assets::write_to(pkg_dir)` then runs the internal bundler (formerly `solobase-web-bundle`) on the resulting directory. Same content-hashing + template rendering flow as PR #4, just exposed via a framework-provided binary rather than a workspace-member binary.

## Asset Packaging

**All static assets are vendored inside the crate via `include_bytes!`** and exposed through the public `assets` module. Rationale:

- Single source of truth. No separate asset artifact to keep in sync; `cargo publish` ships everything needed.
- No network fetches during consumer builds. Removes the `npm pack sql.js@1.11.0` step from the current Makefile, improving reliability in sandboxed CI and offline builds.
- Cargo caches the crate's assets alongside its Rust code. No filesystem path coupling between repos.

**sql.js is not content-hashed.** It keeps its canonical filenames (`sql-wasm-esm.js`, `sql-wasm.wasm`). Rationale:

- sql.js is version-pinned in the framework; its bytes only change when we explicitly bump the vendored version. That's rare (roughly annual).
- Content-hashing it broke cross-repo filesystem consumers (gizza-ai) for no benefit — see PR #4 integration testing for context.
- When the version does bump, a deploy cycle triggers new `solobase_web_bg.wasm` bytes, which in turn changes `sw.js` hashed imports, triggering SW update. Users get the new sql.js on their next navigation.

The content-hashing set therefore shrinks to: `solobase_web.js`, `solobase_web_bg.wasm`. The bundler's `REWRITES` table drops its sql.js entry; the `rewrite_all` helper remains in the crate in case a future consumer needs it.

**~1.5 MB of sql.js assets are committed to the crate.** This is accepted as the cost of removing the cross-repo fetch dependency. The crate is internal-use (not yet on crates.io); publish size is not a blocker.

## Migration Plan for `crates/solobase-web/`

Four steps, each independently revertable:

1. **Create `crates/solobase-browser/`** with moved Rust modules + vendored assets + `App` + `framework_init`/`framework_handle_request` + `export-assets` bin + bundler. Add to workspace members. Old `solobase-web` remains untouched and still builds. Ship.
2. **Refactor `crates/solobase-web/src/lib.rs`** to the ~30-line wrapper described above. Delete `crates/solobase-web/src/{bridge,database,storage,network,crypto,logger,asset_loader}.rs` and `crates/solobase-web/js/*.{js,tmpl,html}` except `ai-bridge.js`. Decide on `convert.rs` case-by-case (see Risks: function-level decision based on caller analysis). Update `Cargo.toml` to depend on `solobase-browser`. Update `Makefile` to invoke `export-assets`. Verify `make build` still produces a working site. Ship.
3. **Add `examples/minimal-browser/` smoke test.** ~20 lines of Rust registering one no-op block; builds in CI. Catches accidental solobase-core coupling leaking into the framework.
4. **Remove the now-unused `crates/solobase-web-bundle/` workspace member.** Its content lives at `solobase-browser/tools/bundle/`. Ship.

If step 2 surfaces a framework-shape issue (e.g., `framework_init` needs more knobs), iterate on `solobase-browser` before running step 3.

## Testing

- **`solobase-browser` unit tests.** Existing `solobase-web-bundle` tests carry over unchanged (just relocated). New tests:
  - `App::register_block` rejects duplicate names.
  - `App::register_block` rejects names violating the `{org}/{block}` convention (leverage existing wafer-run validation where possible).
  - `assets::write_to` writes every asset with correct bytes to a temp dir.
  - `assets::static_assets` returns non-empty and matches the on-disk asset set.
- **`solobase-web` integration.** `make build` produces the same `pkg/` shape as today (hashed `solobase_web_*`, `asset-manifest.json`, rendered `sw.js`/`index.html`, no unresolved `__` placeholders). The Playwright E2E scaffold from PR #4 continues to apply.
- **Framework consumer smoke test.** `examples/minimal-browser/` is a real consumer: a trivial cdylib crate registering one no-op block, with a tiny Makefile that invokes `export-assets`. Compiles in CI. Its existence forces us to catch any accidental dependency on `solobase-core` in the framework itself.

## Risks

- **`framework_init` storing state in a `thread_local`**: matches current behavior (`crates/solobase-web/src/lib.rs` uses `thread_local! static RUNTIME`). WASM is single-threaded, so this is safe. The framework exposes no API to reset the runtime; double-init returns Ok silently. If future work needs teardown/replace, add it then.
- **Hidden coupling via `solobase-core`**: the framework must not pull in feature blocks. Enforced by the `examples/minimal-browser/` smoke test — if it fails to build, we've leaked a feature-block dependency.
- **Config handling**: the framework assumes env-based config via `SOLOBASE_*` conventions. Consumers outside the Solobase family (e.g. gizza-ai) may not follow this convention. They can either adopt the convention or seed their own config before calling `framework_init`. If this becomes a pain point, add an `App::seed_config(kv)` helper later — backwards-compatibly.
- **`convert.rs` boundary ambiguity**: some code in `solobase-web/src/convert.rs` today may be app-specific and some framework-general. Decide at implementation time, file by file. Default: if a function is called only from the platform-service code, move it; otherwise keep it with the app.

## Summary

Extract browser platform services + JS assets from `solobase-web` into a new `solobase-browser` framework crate. Expose a minimal `App` API (`register_block` only) and two framework async functions (`framework_init`, `framework_handle_request`) that users call from their own `#[wasm_bindgen]` entrypoints. Vendor sql.js in the crate via `include_bytes!` and drop it from content-hashing. Migrate the existing `solobase-web` app onto the framework as the first validation consumer. Defer CLI, native extraction, and gizza-ai migration to later sub-projects.
