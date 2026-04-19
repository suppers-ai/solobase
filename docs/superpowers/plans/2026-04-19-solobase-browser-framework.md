# solobase-browser Framework Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the reusable browser platform layer from `crates/solobase-web/` into a new `crates/solobase-browser/` framework crate (Rust services + vendored JS assets + bundler + SW runtime plumbing), then migrate `solobase-web` to consume it as a validation.

**Architecture:** Framework is a toolbox — explicit factory functions (`make_database_service`, `make_crypto_service`, ...), an async `db_init` helper, SW runtime plumbing (`store_wafer`, `dispatch_request`, `is_initialized`), and a static-asset API. Consumers compose these in their own `#[wasm_bindgen]` entrypoints with any app-level builder they prefer. No closure-based `App`, no macros, no inventory registries. Duplication during the move: the existing `solobase-web-bundle` crate stays until its consumer (Makefile) is switched over, to keep every intermediate commit buildable.

**Tech Stack:** Rust cdylib for wasm32-unknown-unknown; `wafer-run`, `wafer-core`, `wafer-block-*` crates; `wasm-bindgen`, `web-sys`, `js-sys`; `sha2`, `hex`, `serde_json`. JS assets are plain static files. Testing via `cargo test` (native + wasm-bindgen-test where services need a browser env).

**Spec:** `docs/superpowers/specs/2026-04-19-solobase-browser-framework-design.md`

---

## File Structure

### New files

- `crates/solobase-browser/Cargo.toml`
- `crates/solobase-browser/src/lib.rs` — public API surface + re-exports
- `crates/solobase-browser/src/runtime.rs` — `RUNTIME` thread_local, `is_initialized`, `store_wafer`, `dispatch_request`
- `crates/solobase-browser/src/assets.rs` — `Asset`, `static_assets()`, `write_to()`
- `crates/solobase-browser/src/bridge.rs` — copied from `solobase-web`
- `crates/solobase-browser/src/database.rs` — copied + `make_database_service()`
- `crates/solobase-browser/src/storage.rs` — copied + `make_storage_service()`
- `crates/solobase-browser/src/network.rs` — copied + `make_network_service()`
- `crates/solobase-browser/src/crypto.rs` — copied + `make_crypto_service(secret)`
- `crates/solobase-browser/src/logger.rs` — copied + `make_console_logger()`
- `crates/solobase-browser/src/asset_loader.rs` — copied + `make_sw_asset_loader()`
- `crates/solobase-browser/src/convert.rs` — copied (`request_to_message`, `output_to_response`)
- `crates/solobase-browser/src/tools/bundle/mod.rs` — re-export module for bundle library
- `crates/solobase-browser/src/tools/bundle/lib.rs` — moved from `solobase-web-bundle/src/lib.rs`
- `crates/solobase-browser/src/tools/bundle/{hash,build_id,manifest,rename,template}.rs` — moved from bundler
- `crates/solobase-browser/src/tools/bundle/tests/integration.rs` — moved
- `crates/solobase-browser/src/tools/bundle/tests/fixtures/pkg-in/*` — moved
- `crates/solobase-browser/bin/export-assets.rs` — CLI binary
- `crates/solobase-browser/assets/sw.js.tmpl` — copied from `solobase-web/js/`
- `crates/solobase-browser/assets/loader.js` — copied
- `crates/solobase-browser/assets/bridge.js` — copied
- `crates/solobase-browser/assets/index.html.tmpl` — copied
- `crates/solobase-browser/assets/vendor/sql-wasm-esm.js` — vendored from sql.js 1.11.0
- `crates/solobase-browser/assets/vendor/sql-wasm.wasm` — vendored from sql.js 1.11.0
- `examples/minimal-browser/Cargo.toml`
- `examples/minimal-browser/src/lib.rs` — tiny cdylib using framework factories
- `examples/minimal-browser/Makefile`

### Modified files

- `Cargo.toml` (workspace root) — add `crates/solobase-browser` and `examples/minimal-browser` to members
- `crates/solobase-web/Cargo.toml` — depend on `solobase-browser`; drop now-indirect deps
- `crates/solobase-web/src/lib.rs` — rewritten to explicit-composition shape
- `crates/solobase-web/Makefile` — invoke `cargo run -p solobase-browser --bin export-assets`

### Removed files

- `crates/solobase-web/src/bridge.rs`
- `crates/solobase-web/src/database.rs`
- `crates/solobase-web/src/storage.rs`
- `crates/solobase-web/src/network.rs`
- `crates/solobase-web/src/crypto.rs`
- `crates/solobase-web/src/logger.rs`
- `crates/solobase-web/src/asset_loader.rs`
- `crates/solobase-web/src/convert.rs`
- `crates/solobase-web/js/sw.js.tmpl`
- `crates/solobase-web/js/loader.js`
- `crates/solobase-web/js/bridge.js`
- `crates/solobase-web/js/index.html.tmpl`
- `crates/solobase-web-bundle/` (entire crate — duplicated in `solobase-browser/src/tools/bundle/` during Task 2)

### Preserved files

- `crates/solobase-web/src/config.rs` (app-specific: reads variables, feature flags)
- `crates/solobase-web/js/ai-bridge.js` (Solobase-specific local-LLM integration)
- `crates/solobase-web/js/manifest.json` (PWA manifest, app-specific)

---

## Task 1: Scaffold `solobase-browser` crate

**Files:**
- Create: `crates/solobase-browser/Cargo.toml`
- Create: `crates/solobase-browser/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — append to `members`)

- [ ] **Step 1: Create `crates/solobase-browser/Cargo.toml`**

Use the existing `crates/solobase-web/Cargo.toml` as a template. Keep all deps EXCEPT the `solobase` and `solobase-core` path deps, which must not be pulled into the framework:

```toml
[package]
name = "solobase-browser"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Browser platform services + Service-Worker plumbing for WAFER-in-browser applications"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Request", "Response", "ResponseInit",
    "Headers", "ReadableStream",
    "Url",
    "console",
] }
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { workspace = true }
wafer-run = { workspace = true }
wafer-core = { workspace = true }
wafer-block = { workspace = true }
wafer-block-config = { workspace = true }
wafer-block-crypto = { workspace = true }
async-trait = "0.1"
serde-wasm-bindgen = "0.6"
hex = "0.4"
pbkdf2 = "0.12"
hkdf = "0.12"
sha2 = "0.10"
hmac = "0.12"
base64ct = { version = "1", features = ["alloc"] }

# For the bundler module (tools/bundle):
anyhow = "1"
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
tempfile = "3"

[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
uuid = { version = "1", features = ["v4", "js"] }

[lints]
workspace = true

[package.metadata.wasm-pack.profile.release]
wasm-opt = false

[package.metadata.wasm-pack.profile.dev]
wasm-opt = false
```

- [ ] **Step 2: Create `src/lib.rs` as a stub**

```rust
//! Browser platform services + Service-Worker plumbing for WAFER-in-browser
//! applications.
//!
//! This crate is the browser half of Solobase's framework layer. It provides
//! factory functions for platform services (sql.js database, OPFS storage,
//! fetch network, browser crypto, console logger, SW asset loader), an async
//! `db_init` helper, and thread_local SW runtime plumbing
//! (`store_wafer`/`dispatch_request`/`is_initialized`). Consumers compose these
//! in their own `#[wasm_bindgen]` entrypoints using any app-level builder.
```

- [ ] **Step 3: Add to workspace members**

In the repo's root `Cargo.toml`, append `"crates/solobase-browser"` to the `members` array (alphabetical ordering by existing convention).

- [ ] **Step 4: Verify the empty crate builds**

Run: `cargo check -p solobase-browser`
Expected: completes with zero errors. A warning about empty `src/lib.rs` (no items) is acceptable.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/solobase-browser/
git commit -m "feat(solobase-browser): scaffold framework crate skeleton"
```

---

## Task 2: Migrate bundler into `solobase-browser/src/tools/bundle/`

**Files:**
- Create: `crates/solobase-browser/src/tools/mod.rs`
- Create: `crates/solobase-browser/src/tools/bundle/mod.rs`
- Create: `crates/solobase-browser/src/tools/bundle/{lib,hash,build_id,manifest,rename,template}.rs`
- Create: `crates/solobase-browser/tests/bundle_integration.rs` (moved from `solobase-web-bundle/tests/integration.rs`)
- Create: `crates/solobase-browser/tests/bundle_fixtures/pkg-in/*` (moved from `solobase-web-bundle/tests/fixtures/pkg-in/*`)
- Modify: `crates/solobase-browser/src/lib.rs` (add `pub mod tools;`)

- [ ] **Step 1: Copy bundler source**

Create `crates/solobase-browser/src/tools/mod.rs`:

```rust
pub mod bundle;
```

Create `crates/solobase-browser/src/tools/bundle/mod.rs`. Use the content of `crates/solobase-web-bundle/src/lib.rs`, **unchanged** except:

- Change the top-of-file module declarations from `pub mod build_id; pub mod hash; ...` to local module imports if needed (the content already uses `pub mod X;` which works fine under a submodule).

Copy each of:
- `crates/solobase-web-bundle/src/hash.rs` → `crates/solobase-browser/src/tools/bundle/hash.rs`
- `crates/solobase-web-bundle/src/build_id.rs` → `crates/solobase-browser/src/tools/bundle/build_id.rs`
- `crates/solobase-web-bundle/src/manifest.rs` → `crates/solobase-browser/src/tools/bundle/manifest.rs`
- `crates/solobase-web-bundle/src/rename.rs` → `crates/solobase-browser/src/tools/bundle/rename.rs`
- `crates/solobase-web-bundle/src/template.rs` → `crates/solobase-browser/src/tools/bundle/template.rs`

verbatim. Every file is a pure copy; no code edits.

Update `crates/solobase-browser/src/tools/bundle/build_id.rs` to reference `crate::tools::bundle::hash::short_hash` instead of `crate::hash::short_hash`. This is the only import that needs adjusting because of the module-path change.

- [ ] **Step 2: Add `pub mod tools;` to `crates/solobase-browser/src/lib.rs`**

Append:

```rust
pub mod tools;
```

- [ ] **Step 3: Move integration test fixtures**

Copy `crates/solobase-web-bundle/tests/integration.rs` to `crates/solobase-browser/tests/bundle_integration.rs`. Change the import at the top from:

```rust
use solobase_web_bundle::run;
```

to:

```rust
use solobase_browser::tools::bundle::run;
```

Change the `fixture_path` constant from `"tests/fixtures/pkg-in"` to `"tests/bundle_fixtures/pkg-in"`.

Copy every file under `crates/solobase-web-bundle/tests/fixtures/pkg-in/` to `crates/solobase-browser/tests/bundle_fixtures/pkg-in/`, preserving contents byte-for-byte.

- [ ] **Step 4: Verify bundler tests pass in the new location**

Run: `cargo test -p solobase-browser --lib tools::bundle::`
Expected: all unit tests from the bundler (hash, build_id, manifest, rename, template) pass.

Run: `cargo test -p solobase-browser --test bundle_integration`
Expected: both integration tests (`end_to_end_renames_rewrites_and_templates`, `deterministic_across_runs`) pass.

- [ ] **Step 5: Drop sql.js from the hash set in the migrated bundler**

Per the spec, sql.js is no longer content-hashed — it keeps canonical filenames since it's version-pinned in the vendored copy. In `crates/solobase-browser/src/tools/bundle/mod.rs` (the migrated `lib.rs`), change:

```rust
const HASHED_ASSETS: &[(&str, &str)] = &[
    ("solobase_web.js", "solobase_web.js"),
    ("solobase_web_bg.wasm", "solobase_web_bg.wasm"),
    ("sql-wasm-esm.js", "sql-wasm-esm.js"),
    ("sql-wasm.wasm", "sql-wasm.wasm"),
];

const REWRITES: &[(&str, char, &str, bool)] = &[
    ("solobase_web.js", '\'', "solobase_web_bg.wasm", false),
    ("sql-wasm-esm.js", '"', "sql-wasm.wasm", true),
];
```

to:

```rust
const HASHED_ASSETS: &[(&str, &str)] = &[
    ("solobase_web.js", "solobase_web.js"),
    ("solobase_web_bg.wasm", "solobase_web_bg.wasm"),
];

const REWRITES: &[(&str, char, &str, bool)] = &[
    ("solobase_web.js", '\'', "solobase_web_bg.wasm", false),
];
```

The `rewrite_all` helper in `rename.rs` stays — it's still unit-tested for its own behavior; we just no longer invoke it from the pipeline. The sql.js fixture files at `tests/bundle_fixtures/pkg-in/sql-wasm-esm.js` and `sql-wasm.wasm` can stay in the fixture (they're no longer referenced by `HASHED_ASSETS`, so the pipeline ignores them — harmless).

Run: `cargo test -p solobase-browser --lib tools::bundle::`
Expected: all bundler unit tests still pass (removing entries doesn't touch any `.rs` test).

Run: `cargo test -p solobase-browser --test bundle_integration`
Expected: both integration tests still pass. The `end_to_end_renames_rewrites_and_templates` test asserts only that `solobase_web-<hash>.js` and `solobase_web_bg-<hash>.wasm` appear; it does not assert anything about sql.js filenames, so dropping sql.js from `HASHED_ASSETS` is invisible to the test.

- [ ] **Step 6: Leave old bundler crate in place**

`crates/solobase-web-bundle/` is untouched. Its tests still exist and still pass. `solobase-web/Makefile` still calls `cargo run -p solobase-web-bundle`. Both copies of the bundler coexist; we'll delete the old one in Task 23.

- [ ] **Step 7: Commit**

```bash
git add crates/solobase-browser/
git commit -m "feat(solobase-browser): migrate bundler; drop sql.js from hash set"
```

---

## Task 3: Vendor JS assets into `solobase-browser/assets/`

**Files:**
- Create: `crates/solobase-browser/assets/sw.js.tmpl` (copied)
- Create: `crates/solobase-browser/assets/loader.js` (copied)
- Create: `crates/solobase-browser/assets/index.html.tmpl` (copied)
- Create: `crates/solobase-browser/assets/vendor/sql-wasm-esm.js` (vendored from sql.js 1.11.0)
- Create: `crates/solobase-browser/assets/vendor/sql-wasm.wasm` (vendored from sql.js 1.11.0)

**Note**: `bridge.js` is NOT a runtime asset. It's referenced at compile time by `bridge.rs`'s `#[wasm_bindgen(module = "...")]` attribute; wasm-pack reads that attribute and copies `bridge.js` into its `snippets/<hash>/` output automatically. The framework's runtime asset set does not include it. `bridge.js` is handled as part of Task 5 (copying `bridge.rs`).

- [ ] **Step 1: Copy the three hand-written JS/HTML files**

```bash
mkdir -p crates/solobase-browser/assets/vendor
cp crates/solobase-web/js/sw.js.tmpl crates/solobase-browser/assets/sw.js.tmpl
cp crates/solobase-web/js/loader.js crates/solobase-browser/assets/loader.js
cp crates/solobase-web/js/index.html.tmpl crates/solobase-browser/assets/index.html.tmpl
```

- [ ] **Step 2: Vendor sql.js 1.11.0**

Run these commands once from the worktree root:

```bash
cd /tmp && npm pack sql.js@1.11.0 --silent 2>/dev/null && \
  tar xzf sql.js-1.11.0.tgz && \
  cp package/dist/sql-wasm.js $OLDPWD/crates/solobase-browser/assets/vendor/sql-wasm.js && \
  cp package/dist/sql-wasm.wasm $OLDPWD/crates/solobase-browser/assets/vendor/sql-wasm.wasm && \
  rm -rf package sql.js-1.11.0.tgz
```

Then build the ESM wrapper at `crates/solobase-browser/assets/vendor/sql-wasm-esm.js`:

```bash
cd crates/solobase-browser/assets/vendor
printf '// ESM wrapper for sql.js 1.11.0 UMD build (IIFE to isolate scope)\nconst _sqlJs = (function() {\n  var module = { exports: {} };\n  var exports = module.exports;\n' > sql-wasm-esm.js
cat sql-wasm.js >> sql-wasm-esm.js
printf '\n  return module.exports.default || module.exports;\n})();\nexport default _sqlJs;\n' >> sql-wasm-esm.js
rm sql-wasm.js
```

The wrapper's format matches the existing `crates/solobase-web/Makefile:15-19` recipe that produced today's `pkg/sql-wasm-esm.js`. Verify by checking the first and last lines match what the Makefile would emit.

- [ ] **Step 3: Add a `.gitattributes` entry for the `.wasm` (optional, safety)**

If not already present in the repo's `.gitattributes`, append:

```
*.wasm binary
```

This prevents Git from normalizing line endings on the binary. Check with `git check-attr -a crates/solobase-browser/assets/vendor/sql-wasm.wasm` — if `binary` is set, skip.

- [ ] **Step 4: Verify assets exist with expected content shape**

```bash
ls -la crates/solobase-browser/assets/
ls -la crates/solobase-browser/assets/vendor/
test -s crates/solobase-browser/assets/sw.js.tmpl          # non-empty
test -s crates/solobase-browser/assets/loader.js
test -s crates/solobase-browser/assets/index.html.tmpl
test -s crates/solobase-browser/assets/vendor/sql-wasm-esm.js
test -s crates/solobase-browser/assets/vendor/sql-wasm.wasm
echo "OK"
```

Expected: `OK` printed.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/assets/
git commit -m "feat(solobase-browser): vendor JS assets and sql.js 1.11.0"
```

---

## Task 4: `assets` module — `Asset` type, `static_assets()`, `write_to()`

**Files:**
- Create: `crates/solobase-browser/src/assets.rs`
- Modify: `crates/solobase-browser/src/lib.rs` (add `pub mod assets;`)

- [ ] **Step 1: Write the failing test**

Create `crates/solobase-browser/src/assets.rs`:

```rust
//! Static assets shipped with the framework crate, exposed as a typed
//! `Asset` slice plus a `write_to(dir)` convenience.

use std::path::Path;

pub struct Asset {
    /// Path relative to the target directory, using forward slashes. E.g.
    /// `"sw.js.tmpl"` or `"vendor/sql-wasm.wasm"`.
    pub path: &'static str,
    pub bytes: &'static [u8],
}

pub fn static_assets() -> &'static [Asset] {
    &ASSETS
}

pub fn write_to(dir: &Path) -> std::io::Result<()> {
    for asset in static_assets() {
        let out = dir.join(asset.path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, asset.bytes)?;
    }
    Ok(())
}

const ASSETS: &[Asset] = &[
    Asset {
        path: "sw.js.tmpl",
        bytes: include_bytes!("../assets/sw.js.tmpl"),
    },
    Asset {
        path: "loader.js",
        bytes: include_bytes!("../assets/loader.js"),
    },
    Asset {
        path: "index.html.tmpl",
        bytes: include_bytes!("../assets/index.html.tmpl"),
    },
    Asset {
        path: "vendor/sql-wasm-esm.js",
        bytes: include_bytes!("../assets/vendor/sql-wasm-esm.js"),
    },
    Asset {
        path: "vendor/sql-wasm.wasm",
        bytes: include_bytes!("../assets/vendor/sql-wasm.wasm"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_assets_is_non_empty_and_has_expected_paths() {
        let paths: Vec<&str> = static_assets().iter().map(|a| a.path).collect();
        assert!(paths.contains(&"sw.js.tmpl"));
        assert!(paths.contains(&"loader.js"));
        assert!(paths.contains(&"index.html.tmpl"));
        assert!(paths.contains(&"vendor/sql-wasm-esm.js"));
        assert!(paths.contains(&"vendor/sql-wasm.wasm"));
    }

    #[test]
    fn every_asset_has_non_empty_bytes() {
        for asset in static_assets() {
            assert!(!asset.bytes.is_empty(), "asset {:?} has empty bytes", asset.path);
        }
    }

    #[test]
    fn write_to_writes_all_files_with_correct_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        write_to(tmp.path()).unwrap();
        for asset in static_assets() {
            let got = std::fs::read(tmp.path().join(asset.path)).unwrap();
            assert_eq!(got, asset.bytes, "mismatched bytes for {:?}", asset.path);
        }
    }
}
```

- [ ] **Step 2: Add `pub mod assets;` to `src/lib.rs`**

Append:

```rust
pub mod assets;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p solobase-browser --lib assets::`
Expected: all three tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-browser/src/assets.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): assets module with include_bytes + write_to"
```

---

## Task 5: Copy `bridge.rs` + co-locate `bridge.js`

**Files:**
- Create: `crates/solobase-browser/src/bridge.rs`
- Create: `crates/solobase-browser/<path>/bridge.js` — path determined by bridge.rs's `module` attribute
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Inspect `bridge.rs`'s `module` attribute**

```bash
grep -n "wasm_bindgen(module" crates/solobase-web/src/bridge.rs
```

Note the exact string (e.g. `module = "/js/bridge.js"` or `module = "/site/bridge.js"`). wasm-pack resolves this path relative to the Cargo.toml of the crate containing `bridge.rs` at compile time, and copies the JS file into its `snippets/<hash>/` output. The leading slash means "crate-root-relative", not a runtime URL.

Call the resolved path `<bridge-js-path>` — e.g. `crates/solobase-browser/js/bridge.js` if the attribute is `/js/bridge.js`.

- [ ] **Step 2: Copy `bridge.rs` verbatim**

```bash
cp crates/solobase-web/src/bridge.rs crates/solobase-browser/src/bridge.rs
```

Do NOT modify the `module` attribute — keep the exact path that worked in `solobase-web`. Both crates have the same relative-path shape (Cargo.toml at `crates/<name>/Cargo.toml`), so the attribute resolves identically.

- [ ] **Step 3: Copy `bridge.js` to the matching relative path**

```bash
# Adjust destination to match <bridge-js-path>. Example for module = "/js/bridge.js":
mkdir -p crates/solobase-browser/js
cp crates/solobase-web/js/bridge.js crates/solobase-browser/js/bridge.js
```

The exact destination depends on Step 1's output. Whatever the directory layout, the end result is that `<crate-root>/<module-path-minus-leading-slash>` exists and contains the `bridge.js` source.

- [ ] **Step 4: Add `pub mod bridge;` to `src/lib.rs`**

Append:

```rust
pub mod bridge;
```

- [ ] **Step 5: Verify the crate builds under wasm32**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles successfully. If wasm-pack complains it can't find `bridge.js`, Step 3 landed the file in the wrong directory — re-read Step 1's grep output and move it.

Note: native `cargo check -p solobase-browser` will fail because `bridge.rs` uses wasm-bindgen extern imports that only resolve under wasm32. This is expected and matches the current `solobase-web` behavior. All subsequent wasm-only modules have the same property.

- [ ] **Step 6: Commit**

```bash
git add crates/solobase-browser/src/bridge.rs crates/solobase-browser/src/lib.rs crates/solobase-browser/<bridge-js-parent-dir>/
git commit -m "feat(solobase-browser): copy bridge.rs + co-locate bridge.js for wasm-pack snippets"
```

---

## Task 6: Copy `database.rs` + add `make_database_service()` factory

**Files:**
- Create: `crates/solobase-browser/src/database.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy `database.rs` verbatim**

```bash
cp crates/solobase-web/src/database.rs crates/solobase-browser/src/database.rs
```

- [ ] **Step 2: Add the factory function**

Append to `crates/solobase-browser/src/database.rs`:

```rust
/// Factory: returns an `Arc<dyn DatabaseService>` backed by the
/// browser's sql.js + OPFS integration. Call after `crate::db_init()`
/// has completed.
pub fn make_database_service() -> std::sync::Arc<dyn wafer_core::interfaces::database::service::DatabaseService> {
    std::sync::Arc::new(BrowserDatabaseService)
}
```

Note: the exact trait path `wafer_core::interfaces::database::service::DatabaseService` must match what the current `solobase-web/src/lib.rs` uses at line 79 (`Arc::new(database::BrowserDatabaseService)` passed into `.database(...)` on `SolobaseBuilder`). Grep `wafer_core::interfaces` in `solobase-web/` to confirm the exact module path before writing the factory signature; adjust if the trait lives elsewhere.

- [ ] **Step 3: Add `pub mod database;` to `src/lib.rs`**

Append:

```rust
pub mod database;
pub use database::make_database_service;
```

- [ ] **Step 4: Verify it builds**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/database.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy database.rs + make_database_service factory"
```

---

## Task 7: Copy `storage.rs` + add `make_storage_service()` factory

**Files:**
- Create: `crates/solobase-browser/src/storage.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/storage.rs crates/solobase-browser/src/storage.rs
```

- [ ] **Step 2: Append factory**

```rust
pub fn make_storage_service() -> std::sync::Arc<dyn wafer_core::interfaces::storage::service::StorageService> {
    std::sync::Arc::new(BrowserStorageService)
}
```

(Confirm the exact trait path by grepping `wafer_core::interfaces` in the current `solobase-web` code. Use whatever path `solobase-web/src/lib.rs:80` expects when calling `.storage(...)`.)

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod storage;
pub use storage::make_storage_service;
```

- [ ] **Step 4: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/storage.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy storage.rs + make_storage_service factory"
```

---

## Task 8: Copy `network.rs` + add `make_network_service()` factory

**Files:**
- Create: `crates/solobase-browser/src/network.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/network.rs crates/solobase-browser/src/network.rs
```

- [ ] **Step 2: Append factory**

```rust
pub fn make_network_service() -> std::sync::Arc<dyn wafer_core::interfaces::network::service::NetworkService> {
    std::sync::Arc::new(BrowserNetworkService)
}
```

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod network;
pub use network::make_network_service;
```

- [ ] **Step 4: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/network.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy network.rs + make_network_service factory"
```

---

## Task 9: Copy `crypto.rs` + add `make_crypto_service(secret)` factory

**Files:**
- Create: `crates/solobase-browser/src/crypto.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/crypto.rs crates/solobase-browser/src/crypto.rs
```

- [ ] **Step 2: Append factory**

```rust
/// Factory: returns an `Arc<dyn CryptoService>` seeded with `jwt_secret`.
/// The secret is used for HMAC-based JWT signing inside browser contexts.
/// It is the caller's responsibility to source this secret (typically from
/// an `SUPPERS_AI__AUTH__JWT_SECRET` config var).
pub fn make_crypto_service(jwt_secret: String) -> std::sync::Arc<dyn wafer_core::interfaces::crypto::service::CryptoService> {
    std::sync::Arc::new(BrowserCryptoService::new(jwt_secret))
}
```

The `BrowserCryptoService::new(jwt_secret)` constructor is the one used today at `solobase-web/src/lib.rs:82`.

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod crypto;
pub use crypto::make_crypto_service;
```

- [ ] **Step 4: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/crypto.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy crypto.rs + make_crypto_service factory"
```

---

## Task 10: Copy `logger.rs` + add `make_console_logger()` factory

**Files:**
- Create: `crates/solobase-browser/src/logger.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/logger.rs crates/solobase-browser/src/logger.rs
```

- [ ] **Step 2: Append factory**

```rust
pub fn make_console_logger() -> std::sync::Arc<dyn wafer_core::interfaces::logger::Logger> {
    std::sync::Arc::new(ConsoleLogger)
}
```

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod logger;
pub use logger::make_console_logger;
```

- [ ] **Step 4: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/logger.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy logger.rs + make_console_logger factory"
```

---

## Task 11: Copy `asset_loader.rs` + add `make_sw_asset_loader()` factory

**Files:**
- Create: `crates/solobase-browser/src/asset_loader.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/asset_loader.rs crates/solobase-browser/src/asset_loader.rs
```

- [ ] **Step 2: Append factory**

```rust
/// Factory: returns an `Arc<dyn AssetLoader>` that bridges
/// `load_asset` calls to the Service Worker via postMessage.
/// Install via `wafer.set_asset_loader(solobase_browser::make_sw_asset_loader())`
/// before calling `wafer.start_without_bind()`.
pub fn make_sw_asset_loader() -> std::sync::Arc<dyn wafer_run::AssetLoader> {
    std::sync::Arc::new(SwAssetLoader::new())
}
```

(Confirm the exact trait import path by grepping for `AssetLoader` in the current `solobase-web` code; the existing call `wafer.set_asset_loader(Arc::new(asset_loader::SwAssetLoader::new()))` at `solobase-web/src/lib.rs:106` points to the right type.)

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod asset_loader;
pub use asset_loader::make_sw_asset_loader;
```

- [ ] **Step 4: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/asset_loader.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy asset_loader.rs + make_sw_asset_loader factory"
```

---

## Task 12: Copy `convert.rs`

**Files:**
- Create: `crates/solobase-browser/src/convert.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Copy verbatim**

```bash
cp crates/solobase-web/src/convert.rs crates/solobase-browser/src/convert.rs
```

This module provides `request_to_message` and `output_to_response` — the Request↔Message and Output↔Response conversions used by `handle_request`. It is framework-general (no app-specific logic); every browser consumer needs it.

- [ ] **Step 2: Update `lib.rs`**

Append:

```rust
pub mod convert;
```

(No re-export needed — consumers don't call these directly; they go through `dispatch_request` added in Task 14.)

- [ ] **Step 3: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-browser/src/convert.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): copy convert.rs (Request↔Message conversions)"
```

---

## Task 13: `db_init()` wrapper

**Files:**
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Add `db_init` at the top of `src/lib.rs`**

Append after the module declarations:

```rust
/// Load sql.js WASM and open (or create) the OPFS-backed database.
/// Idempotent-safe to call once at startup, before constructing platform
/// services. Wraps `bridge::dbInit()`.
pub async fn db_init() {
    bridge::dbInit().await;
}
```

- [ ] **Step 2: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): db_init() wrapper over bridge::dbInit()"
```

---

## Task 14: `runtime` module — `is_initialized`, `store_wafer`, `dispatch_request`

**Files:**
- Create: `crates/solobase-browser/src/runtime.rs`
- Modify: `crates/solobase-browser/src/lib.rs`

- [ ] **Step 1: Create `src/runtime.rs`**

```rust
//! Service-Worker-side Wafer runtime storage and dispatch.
//!
//! `store_wafer` stashes a fully-started `Wafer` in a `thread_local` cell;
//! `dispatch_request` converts an incoming `web_sys::Request` into a WAFER
//! `Message`, dispatches it through the stored `Wafer`'s `site-main` flow,
//! and converts the output back into a `web_sys::Response`. WASM is
//! single-threaded, so the thread_local is safe without Send/Sync bounds.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use crate::convert;

thread_local! {
    static RUNTIME: RefCell<Option<wafer_run::Wafer>> = const { RefCell::new(None) };
}

/// True if `store_wafer` has been called in this SW context.
pub fn is_initialized() -> bool {
    RUNTIME.with(|r| r.borrow().is_some())
}

/// Store a fully-started `Wafer` in the SW's thread_local. Subsequent
/// `dispatch_request` calls route through this Wafer.
///
/// Panics in debug if called twice. In release, silently overwrites the
/// previous value — consumers should guard with `is_initialized()` at the
/// top of their `initialize()` to make the double-call case explicit.
pub fn store_wafer(wafer: wafer_run::Wafer) {
    RUNTIME.with(|r| {
        let mut borrow = r.borrow_mut();
        debug_assert!(borrow.is_none(), "store_wafer called twice");
        *borrow = Some(wafer);
    });
}

/// Convert a browser `Request` into a WAFER `Message`, dispatch through
/// the stored `Wafer`'s `site-main` flow, and return a browser `Response`.
/// Returns a 503-shaped `Response` if called before `store_wafer`.
/// Internal errors return a 500-shaped `Response`.
pub async fn dispatch_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    // SAFETY: wasm32 is single-threaded, and the RefCell value is never
    // replaced after `store_wafer()` stores it. Using a raw pointer avoids
    // holding a RefCell borrow across `.await`, which would break when
    // concurrent fetch events interleave at await points.
    let wafer_ptr = RUNTIME.with(|r| {
        let borrow = r.borrow();
        match borrow.as_ref() {
            Some(w) => Ok(w as *const wafer_run::Wafer),
            None => Err(()),
        }
    });

    let wafer_ptr = match wafer_ptr {
        Ok(p) => p,
        Err(()) => {
            return Ok(build_error_response(
                503,
                "solobase-browser: runtime not initialized — call store_wafer() first",
            )?);
        }
    };

    let (msg, input) = convert::request_to_message(&request).await?;
    let wafer = unsafe { &*wafer_ptr };
    let output = wafer.run("site-main", msg, input).await;
    convert::output_to_response(output).await
}

fn build_error_response(status: u16, body: &str) -> Result<web_sys::Response, JsValue> {
    let mut init = web_sys::ResponseInit::new();
    init.status(status);
    web_sys::Response::new_with_opt_str_and_init(Some(body), &init)
}
```

- [ ] **Step 2: Update `src/lib.rs`**

Append:

```rust
pub mod runtime;
pub use runtime::{dispatch_request, is_initialized, store_wafer};
```

- [ ] **Step 3: Build**

Run: `cargo check -p solobase-browser --target wasm32-unknown-unknown`
Expected: compiles.

- [ ] **Step 4: Testing note**

`dispatch_request` involves `web_sys::Request` + `Response` which require a browser-ish environment (`wasm-bindgen-test`). A full round-trip unit test would require `wasm-bindgen-test` infrastructure which the crate doesn't currently have. Instead, coverage is provided by:

- The existing Playwright E2E scaffold in `crates/solobase-web/tests/e2e/sw-update.spec.ts` (from PR #4), which exercises `dispatch_request` indirectly via the live SW after Task 21's migration lands.
- A smoke test that verifies `is_initialized()` returns `false` before `store_wafer`; native `cargo test -p solobase-browser --lib runtime::` on a feature-gated native-testable subset.

For this plan, skip the native unit test — the integration coverage above is sufficient. Document this in a comment on `runtime.rs` if not already present.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/src/runtime.rs crates/solobase-browser/src/lib.rs
git commit -m "feat(solobase-browser): runtime plumbing (store_wafer, dispatch_request, is_initialized)"
```

---

## Task 15: `export-assets` bin

**Files:**
- Create: `crates/solobase-browser/bin/export-assets.rs`
- Modify: `crates/solobase-browser/Cargo.toml`

- [ ] **Step 1: Declare the binary in Cargo.toml**

In `crates/solobase-browser/Cargo.toml`, append:

```toml
[[bin]]
name = "export-assets"
path = "bin/export-assets.rs"
required-features = []
```

- [ ] **Step 2: Implement `bin/export-assets.rs`**

```rust
//! Writes framework static assets to a target directory, then runs the
//! bundler to content-hash assets and render templates. Invoked from
//! consumer Makefiles after `wasm-pack build`.
//!
//! Usage: `export-assets <pkg-dir> [--repo-dir <path>] [--dev]`

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "export-assets")]
#[command(about = "Write solobase-browser assets + run the bundler post-processor")]
struct Cli {
    /// Path to the `pkg/` directory produced by wasm-pack.
    pkg_dir: PathBuf,

    /// Repo root (used to read `git rev-parse` for the build id).
    /// Defaults to `pkg_dir`'s parent.
    #[arg(long)]
    repo_dir: Option<PathBuf>,

    /// Skip asset hashing; render templates with canonical filenames.
    #[arg(long)]
    dev: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Write static assets into pkg_dir.
    solobase_browser::assets::write_to(&cli.pkg_dir)?;

    // 2. Run the bundler to content-hash assets + render templates.
    let repo = cli
        .repo_dir
        .clone()
        .or_else(|| cli.pkg_dir.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| cli.pkg_dir.clone());
    solobase_browser::tools::bundle::run(&cli.pkg_dir, &repo, cli.dev)?;

    Ok(())
}
```

- [ ] **Step 3: Smoke-test the binary**

Run:

```bash
cargo build -p solobase-browser --bin export-assets --release
./target/release/export-assets --help
```

Expected: usage text printed; exit 0.

- [ ] **Step 4: End-to-end smoke test with a temp dir**

```bash
rm -rf /tmp/export-assets-smoke
mkdir -p /tmp/export-assets-smoke
# Also write the wasm-pack-like outputs (dummy content is fine for the assert).
echo "// fake glue
const url = new URL('solobase_web_bg.wasm', import.meta.url);" > /tmp/export-assets-smoke/solobase_web.js
printf "dummy" > /tmp/export-assets-smoke/solobase_web_bg.wasm
./target/release/export-assets /tmp/export-assets-smoke --dev
ls /tmp/export-assets-smoke/
```

Expected: directory contains the vendored `sw.js`, `loader.js`, `bridge.js`, `index.html`, `vendor/sql-wasm-esm.js`, `vendor/sql-wasm.wasm`, plus an `asset-manifest.json` and rendered templates. No `__PLACEHOLDER__` tokens remain in any `.js`/`.html` file.

```bash
grep -E '__[A-Z_]+__' /tmp/export-assets-smoke/sw.js /tmp/export-assets-smoke/index.html && echo "FAIL" || echo "OK"
```

Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-browser/Cargo.toml crates/solobase-browser/bin/
git commit -m "feat(solobase-browser): export-assets bin for consumer Makefiles"
```

---

## Task 16: Update `solobase-web/Cargo.toml` to depend on `solobase-browser`

**Files:**
- Modify: `crates/solobase-web/Cargo.toml`

- [ ] **Step 1: Add the dep**

Add to the `[dependencies]` table:

```toml
solobase-browser = { path = "../solobase-browser" }
```

- [ ] **Step 2: Remove direct deps that now come transitively from `solobase-browser`**

Delete these entries from `[dependencies]` (all are pulled in via `solobase-browser`):

- `wasm-bindgen`
- `wasm-bindgen-futures`
- `web-sys`
- `js-sys`
- `async-trait`
- `serde-wasm-bindgen`
- `hex`
- `pbkdf2`
- `hkdf`
- `sha2`
- `hmac`
- `base64ct`
- `chrono` (workspace — keep only if solobase-web's remaining code uses it directly; check after Task 17)
- `wafer-block-config`
- `wafer-block-crypto`

Keep:

- `serde`, `serde_json` (used by app-specific code)
- `wafer-run`, `wafer-core`, `wafer-block` (used by register fn block types)
- `solobase`, `solobase-core` (app-specific)

Also delete the `[target.'cfg(target_arch = "wasm32")'.dependencies]` block (moved to `solobase-browser`).

Keep the `[package.metadata.wasm-pack.profile.*]` sections (consumer-level wasm-pack config).

- [ ] **Step 3: Verify the tree is self-consistent (do not build yet)**

Run: `cargo metadata --format-version 1 -p solobase-web --no-deps > /dev/null`
Expected: no errors reported. (This doesn't compile — it just validates the manifest.)

The crate will not compile yet because `solobase-web/src/lib.rs` still references modules that are about to be replaced. That's Task 17.

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/Cargo.toml
git commit -m "refactor(solobase-web): depend on solobase-browser; drop now-indirect deps"
```

---

## Task 17: Rewrite `solobase-web/src/lib.rs` to explicit composition

**Files:**
- Modify: `crates/solobase-web/src/lib.rs`

- [ ] **Step 1: Replace the file contents**

Replace `crates/solobase-web/src/lib.rs` with:

```rust
//! Solobase app compiled to WASM for running in the browser via Service Worker.
//!
//! Thin wasm-bindgen wrapper around the `solobase-browser` framework. Uses
//! `SolobaseBuilder` (from the `solobase` crate) to wire up the full Solobase
//! block suite.

use std::sync::Arc;

use solobase::builder::{self, SolobaseBuilder};
use wafer_core::interfaces::config::service::ConfigService;
use wasm_bindgen::prelude::*;

pub mod config;

const SOLOBASE_CSP: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' https://cdn.jsdelivr.net; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: blob: https:; ",
    "font-src 'self' https:; ",
    "connect-src 'self' https://cdn.jsdelivr.net https://esm.run https://huggingface.co ",
        "https://raw.githubusercontent.com https://*.huggingface.co https://*.hf.co https://*.xethub.hf.co; ",
    "frame-ancestors 'none'; ",
    "base-uri 'self'; ",
    "form-action 'self'",
);

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    if solobase_browser::is_initialized() {
        return Ok(());
    }

    // 1. Load sql.js WASM + open/create the OPFS database.
    solobase_browser::db_init().await;

    // 2. Seed variables and load config.
    let vars = config::seed_and_load_variables();
    web_sys::console::log_1(
        &format!("solobase: {} variables loaded from database", vars.len()).into(),
    );

    // 3. Load feature flag settings.
    let features = config::load_block_settings();

    // 4. Extract JWT secret.
    let jwt_secret = vars
        .get("SUPPERS_AI__AUTH__JWT_SECRET")
        .cloned()
        .unwrap_or_default();

    // 5. Build config service.
    let config_svc = wafer_block_config::service::EnvConfigService::new();
    for (key, value) in &vars {
        config_svc.set(key, value);
    }

    // 6. Build WAFER runtime via SolobaseBuilder, using framework service factories.
    let (mut wafer, storage_block) = SolobaseBuilder::new()
        .database(solobase_browser::make_database_service())
        .storage(solobase_browser::make_storage_service())
        .config(Arc::new(config_svc))
        .crypto(solobase_browser::make_crypto_service(jwt_secret))
        .network(solobase_browser::make_network_service())
        .logger(solobase_browser::make_console_logger())
        .block_settings(features)
        .block_config(
            "wafer-run/security-headers",
            serde_json::json!({ "csp": SOLOBASE_CSP }),
        )
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 6b. Register the SW-side external-asset loader.
    wafer.set_asset_loader(solobase_browser::make_sw_asset_loader());

    // 7. Start runtime.
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 8. Inject WRAP grants.
    builder::post_start(&wafer, &storage_block);

    web_sys::console::log_1(&"solobase: WAFER runtime started".into());

    // 9. Store in framework's thread_local.
    solobase_browser::store_wafer(wafer);

    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(request).await
}
```

This preserves every step of the current flow and their comments. The only changes are: (a) factory calls replace inline `Arc::new(database::BrowserDatabaseService)` etc., (b) `store_wafer` replaces the thread_local manipulation, (c) `dispatch_request` replaces the hand-written dispatch path.

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p solobase-web --target wasm32-unknown-unknown`
Expected: compiles cleanly. A warning that `wafer-core`, `wafer-block` are unused is acceptable at this stage and will be addressed in Task 18 cleanup if the warnings arise.

If any `wafer-core::interfaces::config::service::ConfigService` import fails because of a pruned dep, temporarily restore `wafer-block-config` (or the correct crate) to `Cargo.toml`. The goal is the new `lib.rs` works; Task 16 was best-effort and can be revisited.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web/src/lib.rs
git commit -m "refactor(solobase-web): rewrite lib.rs as explicit composition over solobase-browser"
```

---

## Task 18: Delete old `solobase-web/src/` modules

**Files:**
- Delete: `crates/solobase-web/src/{bridge,database,storage,network,crypto,logger,asset_loader,convert}.rs`

- [ ] **Step 1: Remove the files**

```bash
cd /home/joris/Programs/suppers-ai/workspace/solobase
git rm crates/solobase-web/src/bridge.rs
git rm crates/solobase-web/src/database.rs
git rm crates/solobase-web/src/storage.rs
git rm crates/solobase-web/src/network.rs
git rm crates/solobase-web/src/crypto.rs
git rm crates/solobase-web/src/logger.rs
git rm crates/solobase-web/src/asset_loader.rs
git rm crates/solobase-web/src/convert.rs
```

- [ ] **Step 2: Verify the crate still builds**

Run: `cargo build -p solobase-web --target wasm32-unknown-unknown`
Expected: compiles. Nothing references the deleted modules after Task 17's rewrite.

- [ ] **Step 3: Commit**

```bash
git commit -m "refactor(solobase-web): delete modules now provided by solobase-browser"
```

---

## Task 19: Delete old `solobase-web/js/` files that moved

**Files:**
- Delete: `crates/solobase-web/js/sw.js.tmpl`
- Delete: `crates/solobase-web/js/loader.js`
- Delete: `crates/solobase-web/js/bridge.js` — only if its twin now lives under `solobase-browser/` (Task 5 Step 3). Verify before deleting.
- Delete: `crates/solobase-web/js/index.html.tmpl`

Preserved:
- `crates/solobase-web/js/ai-bridge.js` — Solobase-specific local-LLM bridge
- `crates/solobase-web/js/manifest.json` — PWA manifest

- [ ] **Step 1: Remove the files**

```bash
git rm crates/solobase-web/js/sw.js.tmpl
git rm crates/solobase-web/js/loader.js
git rm crates/solobase-web/js/bridge.js
git rm crates/solobase-web/js/index.html.tmpl
```

- [ ] **Step 2: Commit**

```bash
git commit -m "refactor(solobase-web): drop JS assets now vendored in solobase-browser"
```

---

## Task 20: Update `solobase-web/Makefile` to invoke `export-assets`

**Files:**
- Modify: `crates/solobase-web/Makefile`

- [ ] **Step 1: Replace `build` and `dev` targets**

The current Makefile (after PR #4) has a `build` target that runs:

```makefile
build: pkg/sql-wasm-esm.js
	wasm-pack build --target web --release --out-dir pkg
	cp js/sw.js.tmpl pkg/
	cp js/index.html.tmpl pkg/
	cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-web-bundle --release -- pkg/ --repo-dir $(CURDIR)/../..
```

Replace both `build` and `dev` with:

```makefile
# Build for production (framework provides assets + hashing)
build:
	wasm-pack build --target web --release --out-dir pkg
	cp js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-browser --release --bin export-assets -- pkg/ --repo-dir $(CURDIR)/../..

# Build for development (no hashing; canonical filenames)
dev:
	wasm-pack build --target web --dev --out-dir pkg
	cp js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-browser --release --bin export-assets -- pkg/ --repo-dir $(CURDIR)/../.. --dev
```

Also remove the now-unused `SQL_JS_VERSION := 1.11.0` line and the `pkg/sql-wasm.wasm pkg/sql-wasm.js` and `pkg/sql-wasm-esm.js: pkg/sql-wasm.js` rules — sql.js is now vendored in `solobase-browser/assets/vendor/` and written by `export-assets`.

The `serve` and `clean` targets are unchanged.

- [ ] **Step 2: Run a clean prod build**

```bash
cd crates/solobase-web
make clean
make build
```

Expected: build succeeds. `pkg/` contains:

- `solobase_web-<hash>.js`
- `solobase_web_bg-<hash>.wasm`
- `asset-manifest.json`
- `sw.js` (rendered, no `__PLACEHOLDER__`)
- `index.html` (rendered)
- `loader.js`
- `bridge.js`
- `ai-bridge.js`
- `manifest.json`
- `vendor/sql-wasm-esm.js`
- `vendor/sql-wasm.wasm`
- `snippets/<wasm-pack-hash>/` (wasm-pack output)

- [ ] **Step 3: Verify no placeholders leaked**

```bash
grep -E '__[A-Z_]+__' crates/solobase-web/pkg/sw.js crates/solobase-web/pkg/index.html && echo "FAIL" || echo "OK"
```

Expected: `OK`.

- [ ] **Step 4: Verify sw.js imports a hashed URL**

```bash
head -2 crates/solobase-web/pkg/sw.js
```

Expected: build-id comment and import from `/solobase_web-<hash>.js`.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/Makefile
git commit -m "build(solobase-web): invoke solobase-browser's export-assets"
```

---

## Task 21: Manual browser smoke test

- [ ] **Step 1: Serve**

```bash
cd crates/solobase-web
make serve &
```

Wait for the server to print its port.

- [ ] **Step 2: Open in a browser**

Navigate to `http://localhost:8080`. Open DevTools → Application → Service Workers. Verify:

- `/sw.js` is activated.
- Network panel shows `/sw.js` fetched (not from disk cache).
- Network panel shows `/solobase_web-<hash>.js` and `/solobase_web_bg-<hash>.wasm` fetched.
- Network panel shows `/vendor/sql-wasm-esm.js` and `/vendor/sql-wasm.wasm` fetched when the DB initializes.
- Console shows `solobase: WAFER runtime started` or equivalent.
- Navigating to `/b/auth/` (or any other block UI) reaches the WASM runtime and renders a page.

- [ ] **Step 3: Document the result**

Manual verification step. No commit. If the smoke test fails, diagnose before proceeding. Common failure modes:

- Asset 404s → check `export-assets` wrote the file to the right location.
- SW install fails → check `sw.js` imports resolve to the hashed paths.
- DB init fails → check `vendor/sql-wasm.wasm` is actually served from `/vendor/sql-wasm.wasm` (SW fetch bypass list must include `/vendor/` or the SW fetch handler must pass through).

---

## Task 22: Create `examples/minimal-browser/` smoke example

**Files:**
- Create: `examples/minimal-browser/Cargo.toml`
- Create: `examples/minimal-browser/src/lib.rs`
- Create: `examples/minimal-browser/README.md`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Add to workspace**

Append `"examples/minimal-browser"` to the `members` array in the root `Cargo.toml`.

- [ ] **Step 2: Create `examples/minimal-browser/Cargo.toml`**

```toml
[package]
name = "minimal-browser"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Smoke test: smallest possible solobase-browser consumer"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
solobase-browser = { path = "../../crates/solobase-browser" }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Request", "Response"] }
wafer-run = { workspace = true }
wafer-block = { workspace = true }
async-trait = "0.1"
```

- [ ] **Step 3: Create `examples/minimal-browser/src/lib.rs`**

```rust
//! Smallest-possible consumer of `solobase-browser`. No SolobaseBuilder;
//! no solobase-core. Builds a bare Wafer with framework platform services
//! and registers one no-op block. Its purpose is to fail-loud if
//! solobase-browser accidentally grows a dependency on solobase-core.

use std::sync::Arc;

use async_trait::async_trait;
use wasm_bindgen::prelude::*;

// Minimal no-op block. Real consumers would use real WAFER blocks.
struct NoopBlock;

#[async_trait]
impl wafer_block::WaferBlock for NoopBlock {
    fn name(&self) -> &str { "minimal/noop" }
    // Any other required trait methods with empty defaults.
    // If `WaferBlock` has required methods beyond `name()`, add stubbed
    // implementations that return `Default::default()` or the minimal
    // legal value for each return type. Keep the block boring.
}

#[wasm_bindgen]
pub async fn initialize() -> Result<(), JsValue> {
    if solobase_browser::is_initialized() {
        return Ok(());
    }

    solobase_browser::db_init().await;

    // Construct a bare Wafer using wafer-run's own builder (not SolobaseBuilder).
    // Register framework platform services + the no-op block.
    let mut wafer = wafer_run::Wafer::builder()
        .database(solobase_browser::make_database_service())
        .storage(solobase_browser::make_storage_service())
        .network(solobase_browser::make_network_service())
        .crypto(solobase_browser::make_crypto_service(String::new()))
        .logger(solobase_browser::make_console_logger())
        .register_block("minimal/noop", NoopBlock)
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    wafer.set_asset_loader(solobase_browser::make_sw_asset_loader());
    wafer
        .start_without_bind()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    solobase_browser::store_wafer(wafer);
    Ok(())
}

#[wasm_bindgen]
pub async fn handle_request(request: web_sys::Request) -> Result<web_sys::Response, JsValue> {
    solobase_browser::dispatch_request(request).await
}
```

**Note**: the exact shape of `wafer_run::Wafer::builder()` and the `WaferBlock` trait may differ from what's shown. Check `wafer-run` sources via `cargo doc` to confirm method names; adjust until the crate compiles. If `wafer-run` doesn't expose a public builder, fall back to whichever construction API it does expose. The point of the example is to prove a non-`SolobaseBuilder` consumer works, so expose any gaps as issues to resolve before the framework ships.

- [ ] **Step 4: Create a minimal README**

`examples/minimal-browser/README.md`:

```markdown
# minimal-browser

Smallest possible consumer of `solobase-browser`. Demonstrates that the
framework can be used without pulling in `solobase` or `solobase-core`.

Build:

```bash
cargo build -p minimal-browser --target wasm32-unknown-unknown
```

This crate is a CI smoke test; it is not intended to be deployed.
```

- [ ] **Step 5: Build the example**

```bash
cargo build -p minimal-browser --target wasm32-unknown-unknown
```

Expected: compiles. If it fails due to `solobase` or `solobase-core` being pulled in transitively, the framework has leaked coupling — fix `solobase-browser/Cargo.toml` before proceeding.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml examples/minimal-browser/
git commit -m "test(solobase-browser): minimal-browser example verifying framework independence"
```

---

## Task 23: Remove `crates/solobase-web-bundle/`

**Files:**
- Delete: `crates/solobase-web-bundle/` (entire crate)
- Modify: `Cargo.toml` (workspace — remove member)

- [ ] **Step 1: Verify nothing depends on `solobase-web-bundle`**

```bash
grep -r "solobase-web-bundle" crates/ examples/ --include="*.toml" --include="Makefile" --include="*.rs" --include="*.md" | grep -v target | grep -v "\.worktrees"
```

Expected: no matches (after Task 20's Makefile rewrite, nothing references it). If there are matches, fix the references first.

- [ ] **Step 2: Remove the workspace member**

In the root `Cargo.toml`, delete the `"crates/solobase-web-bundle"` entry from the `members` array.

- [ ] **Step 3: Delete the crate**

```bash
git rm -r crates/solobase-web-bundle
```

- [ ] **Step 4: Verify workspace still builds**

Run: `cargo check --workspace`
Expected: no errors. `solobase-browser`, `solobase-web`, `minimal-browser`, and the rest of the workspace compile cleanly.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml
git commit -m "chore: remove crates/solobase-web-bundle (migrated to solobase-browser/tools/bundle)"
```

---

## Task 24: Final workspace sanity + tests

- [ ] **Step 1: Run the full test suite**

```bash
cargo test --workspace 2>&1 | tail -30
```

Expected: all tests pass. In particular:

- `solobase-browser` unit tests (hash, build_id, manifest, rename, template, assets) all pass.
- `solobase-browser` integration test (`bundle_integration`) passes.
- Other workspace members' tests are unaffected.

- [ ] **Step 2: Run wasm32 builds for both consumers**

```bash
cargo check -p solobase-browser --target wasm32-unknown-unknown
cargo check -p solobase-web --target wasm32-unknown-unknown
cargo check -p minimal-browser --target wasm32-unknown-unknown
```

Expected: all three compile.

- [ ] **Step 3: Run one more production build end-to-end**

```bash
cd crates/solobase-web
make clean
make build
```

Expected: `pkg/` output identical in shape to Task 20 step 2 (hashed assets, rendered templates, no `__` leaks).

- [ ] **Step 4: Spot-check for leftover references**

```bash
grep -rn "solobase_web_bundle" crates/ examples/ --include="*.rs" --include="*.toml" --include="Makefile" | grep -v target | grep -v "\.worktrees"
grep -rn "solobase-web-bundle" crates/ examples/ --include="*.rs" --include="*.toml" --include="Makefile" --include="*.md" | grep -v target | grep -v "\.worktrees"
```

Expected: no matches (all migration references are gone).

- [ ] **Step 5: No commit**

This is a verification task only. If anything fails, create a follow-up fix commit; otherwise nothing to commit.

---

## Self-Review Checklist

- [ ] **Spec coverage**:
  - New `solobase-browser` crate with services + bundler + assets → Tasks 1, 2, 3, 4, 5–14.
  - Service factories (`make_*_service`) → Tasks 6–11.
  - `db_init`, `store_wafer`, `dispatch_request`, `is_initialized` → Tasks 13, 14.
  - `static_assets`, `write_to` → Task 4.
  - `export-assets` bin → Task 15.
  - `solobase-web` migration → Tasks 16–21.
  - `examples/minimal-browser` smoke test → Task 22.
  - Workspace cleanup → Task 23.
  - sql.js vendored, not hashed → Task 3 (vendor) + existing PR #4 bundler behavior (sql.js stays in REWRITES but consumer is now solobase-browser; if the decision to drop sql.js from hashing needs a code change, add it as a step to Task 2).
- [ ] **Placeholder scan**: every step has complete code or concrete commands. No "TBD", "TODO", "similar to Task N". Trait-path exact imports are flagged as "confirm via grep" where the current code path was implicit.
- [ ] **Type consistency**: `make_database_service`, `make_storage_service`, `make_network_service`, `make_crypto_service`, `make_console_logger`, `make_sw_asset_loader` names are used identically in Tasks 6–11, Task 17 (solobase-web), and Task 22 (minimal-browser). The factories all return `Arc<dyn …>`.
- [ ] **Commit hygiene**: every task ends with a commit. 24 commits total; each maps 1:1 to a plan task.
