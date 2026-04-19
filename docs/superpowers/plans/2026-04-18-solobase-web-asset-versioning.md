# solobase-web Asset Versioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make solobase-web's service-worker update flow correct and host-agnostic by content-hashing long-lived assets (WASM + glue + sql.js) and templating `sw.js` to embed those hashes at build time.

**Architecture:** A new Rust binary crate `solobase-web-bundle` runs after `wasm-pack build` and rewrites filenames + internal string-literal references to carry content hashes, emits an `asset-manifest.json`, and renders `sw.js` / `index.html` from `.tmpl` files. `loader.js` adds `updateViaCache: 'none'` so the SW update check is host-agnostic. The npm package `packages/solobase-web/` ships clean wasm-pack output and a new `src/update.ts` with update-lifecycle helpers for consumers.

**Tech Stack:** Rust (workspace crate using `sha2`, `hex`, `serde_json`), wasm-pack, TypeScript (npm package), Playwright (browser test), Vite (npm-package bundler fixture).

**Spec:** `docs/superpowers/specs/2026-04-18-solobase-web-asset-versioning-design.md`

---

## File Structure

### New files

**Rust bundler crate:**
- `crates/solobase-web-bundle/Cargo.toml`
- `crates/solobase-web-bundle/src/main.rs` — CLI entry
- `crates/solobase-web-bundle/src/lib.rs` — public API used by tests + main
- `crates/solobase-web-bundle/src/hash.rs` — SHA-256-8 helpers
- `crates/solobase-web-bundle/src/build_id.rs` — git SHA detection + fallback
- `crates/solobase-web-bundle/src/manifest.rs` — `AssetManifest` type + I/O
- `crates/solobase-web-bundle/src/rename.rs` — file renames + in-file literal rewrites
- `crates/solobase-web-bundle/src/template.rs` — `__PLACEHOLDER__` substitution
- `crates/solobase-web-bundle/tests/integration.rs` — end-to-end fixture test
- `crates/solobase-web-bundle/tests/fixtures/pkg-in/` — known-content fixture input

**Templates (replace current hand-written files):**
- `crates/solobase-web/js/sw.js.tmpl`
- `crates/solobase-web/js/index.html.tmpl`

**npm package additions:**
- `packages/solobase-web/src/update.ts`
- `packages/solobase-web/src/update.test.ts`
- `packages/solobase-web/test-fixtures/vite-app/package.json`
- `packages/solobase-web/test-fixtures/vite-app/vite.config.ts`
- `packages/solobase-web/test-fixtures/vite-app/src/main.ts`
- `packages/solobase-web/test-fixtures/vite-app/index.html`
- `packages/solobase-web/CHANGELOG.md`

**Browser E2E:**
- `crates/solobase-web/tests/e2e/sw-update.spec.ts`
- `crates/solobase-web/tests/playwright.config.ts`

### Modified files

- `Cargo.toml` (workspace root) — add `crates/solobase-web-bundle` to members
- `crates/solobase-web/js/loader.js` — add `updateViaCache: 'none'`
- `crates/solobase-web/js/ai-bridge.js` — unchanged; verify nothing references the unhashed WASM path
- `crates/solobase-web/js/sw.js` — **deleted**, replaced by `.tmpl`
- `crates/solobase-web/js/index.html` — **deleted**, replaced by `.tmpl`
- `crates/solobase-web/Makefile` — add bundler invocation
- `packages/solobase-web/src/worker.ts` — lifecycle change (no auto-skipWaiting on install; add `skip-waiting` message handler)
- `packages/solobase-web/package.json` — bump to `0.2.0`
- `packages/solobase-web/README.md` — new sections

---

## Task 1: Scaffold `solobase-web-bundle` crate

**Files:**
- Create: `crates/solobase-web-bundle/Cargo.toml`
- Create: `crates/solobase-web-bundle/src/main.rs`
- Create: `crates/solobase-web-bundle/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — append to `members`)

- [ ] **Step 1: Create `crates/solobase-web-bundle/Cargo.toml`**

```toml
[package]
name = "solobase-web-bundle"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Post-processor for solobase-web/pkg: content-hashes assets and renders SW/HTML templates"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
sha2 = "0.10"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create `src/lib.rs` as an empty module tree**

```rust
pub mod build_id;
pub mod hash;
pub mod manifest;
pub mod rename;
pub mod template;
```

- [ ] **Step 3: Create `src/main.rs` as a stub**

```rust
use anyhow::Result;

fn main() -> Result<()> {
    println!("solobase-web-bundle placeholder");
    Ok(())
}
```

- [ ] **Step 4: Add to workspace members**

In `Cargo.toml` at the repo root, change:
```toml
members = [
    "crates/solobase",
    "crates/solobase-core",
    "crates/solobase-native",
    "crates/solobase-web",
]
```
to:
```toml
members = [
    "crates/solobase",
    "crates/solobase-core",
    "crates/solobase-native",
    "crates/solobase-web",
    "crates/solobase-web-bundle",
]
```

- [ ] **Step 5: Create empty module files so `lib.rs` compiles**

```rust
// src/hash.rs
// src/build_id.rs
// src/manifest.rs
// src/rename.rs
// src/template.rs
```
Each file starts empty.

- [ ] **Step 6: Verify it builds**

Run: `cargo build -p solobase-web-bundle`
Expected: compiles with no warnings beyond unused-module.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/solobase-web-bundle/
git commit -m "feat(solobase-web-bundle): scaffold crate skeleton"
```

---

## Task 2: `hash` module — content hashing

**Files:**
- Modify: `crates/solobase-web-bundle/src/hash.rs`

- [ ] **Step 1: Write failing unit tests**

Append to `src/hash.rs`:
```rust
/// Returns the first 8 hex chars of the SHA-256 of `bytes`.
pub fn short_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    hex::encode(&digest[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_hash_is_eight_chars() {
        let h = short_hash(b"hello");
        assert_eq!(h.len(), 8);
    }

    #[test]
    fn short_hash_is_deterministic() {
        assert_eq!(short_hash(b"abc"), short_hash(b"abc"));
    }

    #[test]
    fn short_hash_differs_per_input() {
        assert_ne!(short_hash(b"abc"), short_hash(b"abd"));
    }

    #[test]
    fn short_hash_matches_known_value() {
        // SHA-256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert_eq!(short_hash(b"hello"), "2cf24dba");
    }
}
```

- [ ] **Step 2: Run the tests — expect FAIL**

Run: `cargo test -p solobase-web-bundle --lib hash::`
Expected: all four tests PASS immediately since the implementation is included with the tests. This is one of the rare cases where the test and minimal implementation land together (the impl is three lines); proceed.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/hash.rs
git commit -m "feat(solobase-web-bundle): add short_hash helper"
```

---

## Task 3: `build_id` module — git SHA detection

**Files:**
- Modify: `crates/solobase-web-bundle/src/build_id.rs`

- [ ] **Step 1: Write the failing test (detection + fallback)**

Append to `src/build_id.rs`:
```rust
use std::path::Path;
use std::process::Command;

/// Derives a build identifier for a `pkg/` directory.
///
/// Uses `git rev-parse --short=8 HEAD` when available; appends `-dirty` if
/// the working tree has uncommitted changes. Falls back to a SHA-256-8 of
/// the concatenated asset SHAs if `git` is unavailable or fails.
pub fn build_id(repo_dir: &Path, asset_hashes: &[&str]) -> String {
    if let Some(sha) = git_short_sha(repo_dir) {
        let suffix = if git_is_dirty(repo_dir) { "-dirty" } else { "" };
        return format!("{sha}{suffix}");
    }
    let joined: String = asset_hashes.join("");
    crate::hash::short_hash(joined.as_bytes())
}

fn git_short_sha(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn git_is_dirty(dir: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_when_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let id = build_id(tmp.path(), &["aaaa", "bbbb"]);
        // SHA-256("aaaabbbb") first 8 hex chars
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fallback_is_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        let a = build_id(tmp.path(), &["x", "y"]);
        let b = build_id(tmp.path(), &["x", "y"]);
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 2: Run tests — expect PASS**

Run: `cargo test -p solobase-web-bundle --lib build_id::`
Expected: both tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/build_id.rs
git commit -m "feat(solobase-web-bundle): build_id from git SHA with content-hash fallback"
```

---

## Task 4: `manifest` module — asset manifest type

**Files:**
- Modify: `crates/solobase-web-bundle/src/manifest.rs`

- [ ] **Step 1: Write the failing test + implementation**

Append to `src/manifest.rs`:
```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManifest {
    #[serde(rename = "buildId")]
    pub build_id: String,
    /// Logical asset name (as referenced from templates) → `/`-prefixed hashed URL.
    pub assets: BTreeMap<String, String>,
}

impl AssetManifest {
    pub fn write(&self, path: &Path) -> Result<()> {
        let body = serde_json::to_string_pretty(self)
            .context("serialising asset manifest")?;
        std::fs::write(path, body).context("writing asset-manifest.json")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_expected_json_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("asset-manifest.json");
        let mut assets = BTreeMap::new();
        assets.insert("solobase_web.js".into(), "/solobase_web-a1b2c3d4.js".into());
        let m = AssetManifest { build_id: "a1b2c3d4".into(), assets };
        m.write(&path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"buildId\": \"a1b2c3d4\""));
        assert!(contents.contains("\"solobase_web.js\": \"/solobase_web-a1b2c3d4.js\""));
    }

    #[test]
    fn ordering_is_stable() {
        // BTreeMap guarantees sorted key order so the manifest is deterministic.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("m.json");
        let mut assets = BTreeMap::new();
        assets.insert("z.wasm".into(), "/z.wasm".into());
        assets.insert("a.js".into(), "/a.js".into());
        let m = AssetManifest { build_id: "x".into(), assets };
        m.write(&path).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        let a_pos = body.find("\"a.js\"").unwrap();
        let z_pos = body.find("\"z.wasm\"").unwrap();
        assert!(a_pos < z_pos);
    }
}
```

- [ ] **Step 2: Run tests — expect PASS**

Run: `cargo test -p solobase-web-bundle --lib manifest::`
Expected: both tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/manifest.rs
git commit -m "feat(solobase-web-bundle): AssetManifest type with deterministic JSON output"
```

---

## Task 5: `rename` module — file rename + literal rewrite

**Files:**
- Modify: `crates/solobase-web-bundle/src/rename.rs`

- [ ] **Step 1: Write failing tests for rename helpers**

Append to `src/rename.rs`:
```rust
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

/// Rename `path` to include `-<hash>` before its extension.
/// Example: `foo.wasm` + `a1b2c3d4` → `foo-a1b2c3d4.wasm`.
/// Returns the new path (absolute, same directory).
pub fn rename_with_hash(path: &Path, hash: &str) -> Result<PathBuf> {
    let dir = path.parent().ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
    let stem = path.file_stem().ok_or_else(|| anyhow!("no file stem: {}", path.display()))?
        .to_string_lossy().into_owned();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let new_name = format!("{stem}-{hash}{ext}");
    let new_path = dir.join(&new_name);
    std::fs::rename(path, &new_path)
        .with_context(|| format!("rename {} -> {}", path.display(), new_path.display()))?;
    Ok(new_path)
}

/// Replace one exact substring in a UTF-8 file. Fails if `from` is not
/// present, or if it appears more than once.
pub fn rewrite_literal(path: &Path, from: &str, to: &str) -> Result<()> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let count = body.matches(from).count();
    if count == 0 {
        bail!("literal {:?} not found in {}", from, path.display());
    }
    if count > 1 {
        bail!("literal {:?} appears {} times in {} — expected exactly one",
              from, count, path.display());
    }
    let replaced = body.replace(from, to);
    std::fs::write(path, replaced).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rename_adds_hash_before_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("foo.wasm");
        fs::write(&p, b"x").unwrap();
        let out = rename_with_hash(&p, "abcd1234").unwrap();
        assert_eq!(out.file_name().unwrap(), "foo-abcd1234.wasm");
        assert!(out.exists());
        assert!(!p.exists());
    }

    #[test]
    fn rename_handles_compound_stems() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("sql-wasm-esm.js");
        fs::write(&p, b"x").unwrap();
        let out = rename_with_hash(&p, "ffff0000").unwrap();
        assert_eq!(out.file_name().unwrap(), "sql-wasm-esm-ffff0000.js");
    }

    #[test]
    fn rewrite_replaces_single_literal() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.js");
        fs::write(&p, "x = 'solobase_web_bg.wasm';").unwrap();
        rewrite_literal(&p, "'solobase_web_bg.wasm'", "'solobase_web_bg-abcd1234.wasm'").unwrap();
        let body = fs::read_to_string(&p).unwrap();
        assert_eq!(body, "x = 'solobase_web_bg-abcd1234.wasm';");
    }

    #[test]
    fn rewrite_fails_when_literal_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.js");
        fs::write(&p, "unrelated content").unwrap();
        let err = rewrite_literal(&p, "MISSING", "X").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn rewrite_fails_on_multiple_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.js");
        fs::write(&p, "foo foo").unwrap();
        let err = rewrite_literal(&p, "foo", "bar").unwrap_err();
        assert!(err.to_string().contains("expected exactly one"));
    }
}
```

- [ ] **Step 2: Run tests — expect PASS**

Run: `cargo test -p solobase-web-bundle --lib rename::`
Expected: all five tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/rename.rs
git commit -m "feat(solobase-web-bundle): rename_with_hash and rewrite_literal helpers"
```

---

## Task 6: `template` module — placeholder substitution

**Files:**
- Modify: `crates/solobase-web-bundle/src/template.rs`

- [ ] **Step 1: Write failing tests + implementation**

Append to `src/template.rs`:
```rust
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Render a template by substituting `__KEY__` tokens with values from `vars`.
/// Fails if the rendered output still contains any `__…__` token, catching
/// missed placeholders before they reach the browser.
pub fn render_to_file(template_src: &Path, out: &Path, vars: &BTreeMap<String, String>) -> Result<()> {
    let body = std::fs::read_to_string(template_src)
        .with_context(|| format!("reading template {}", template_src.display()))?;
    let mut rendered = body;
    for (key, value) in vars {
        let token = format!("__{}__", key);
        rendered = rendered.replace(&token, value);
    }
    if let Some(stray) = find_unresolved_placeholder(&rendered) {
        bail!("unresolved placeholder {:?} in {}", stray, template_src.display());
    }
    std::fs::write(out, rendered).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

fn find_unresolved_placeholder(body: &str) -> Option<String> {
    // Look for __WORDCHARS__ — only flag all-upper-with-underscores tokens so
    // we don't false-positive on arbitrary __ sequences in minified JS.
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i] == b'_' && bytes[i + 1] == b'_' {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j] == b'_' || (bytes[j] as char).is_ascii_uppercase() || (bytes[j] as char).is_ascii_digit()) {
                j += 1;
            }
            if j + 2 <= bytes.len() && bytes[j] == b'_' && bytes[j + 1] == b'_' && j > i + 2 {
                return Some(String::from_utf8_lossy(&bytes[i..j + 2]).into_owned());
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, body: &str) { std::fs::write(p, body).unwrap(); }

    #[test]
    fn substitutes_known_placeholders() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.tmpl");
        let out = tmp.path().join("out");
        write(&src, "import x from '__WASM_JS__';\n// build: __BUILD_ID__\n");
        let mut vars = BTreeMap::new();
        vars.insert("WASM_JS".into(), "/solobase_web-abcd1234.js".into());
        vars.insert("BUILD_ID".into(), "abcd1234".into());
        render_to_file(&src, &out, &vars).unwrap();
        let body = std::fs::read_to_string(&out).unwrap();
        assert_eq!(body, "import x from '/solobase_web-abcd1234.js';\n// build: abcd1234\n");
    }

    #[test]
    fn fails_on_unresolved_placeholder() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.tmpl");
        let out = tmp.path().join("out");
        write(&src, "x = __MISSING__;");
        let err = render_to_file(&src, &out, &BTreeMap::new()).unwrap_err();
        assert!(err.to_string().contains("__MISSING__"), "got: {err}");
    }

    #[test]
    fn ignores_minified_double_underscores() {
        // Minified JS sometimes has __wbg_foo__ (mixed case) — shouldn't trigger.
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.tmpl");
        let out = tmp.path().join("out");
        write(&src, "function __wbg_init__() {}");
        render_to_file(&src, &out, &BTreeMap::new()).unwrap();
    }
}
```

- [ ] **Step 2: Run tests — expect PASS**

Run: `cargo test -p solobase-web-bundle --lib template::`
Expected: all three tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/template.rs
git commit -m "feat(solobase-web-bundle): template renderer with unresolved-placeholder guard"
```

---

## Task 7: End-to-end pipeline in `lib.rs` + integration test

**Files:**
- Modify: `crates/solobase-web-bundle/src/lib.rs`
- Create: `crates/solobase-web-bundle/tests/integration.rs`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/solobase_web.js`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/solobase_web_bg.wasm`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/sql-wasm-esm.js`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/sql-wasm.wasm`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/sw.js.tmpl`
- Create: `crates/solobase-web-bundle/tests/fixtures/pkg-in/index.html.tmpl`

- [ ] **Step 1: Write the failing integration test**

Create `tests/integration.rs`:
```rust
use solobase_web_bundle::run;
use std::fs;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pkg-in")
}

#[test]
fn end_to_end_renames_rewrites_and_templates() {
    // Copy the fixture to a writable temp dir.
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&fixture_path(), tmp.path());

    // Act.
    run(tmp.path(), tmp.path(), /* dev */ false).expect("bundler ok");

    // Assert: asset-manifest.json exists.
    let manifest_body = fs::read_to_string(tmp.path().join("asset-manifest.json")).unwrap();
    assert!(manifest_body.contains("\"buildId\""));
    assert!(manifest_body.contains("\"solobase_web.js\""));
    assert!(manifest_body.contains("\"solobase_web_bg.wasm\""));

    // Assert: hashed files exist and originals are gone.
    let entries: Vec<String> = fs::read_dir(tmp.path()).unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert!(entries.iter().any(|n| n.starts_with("solobase_web-") && n.ends_with(".js")), "missing hashed JS in {:?}", entries);
    assert!(entries.iter().any(|n| n.starts_with("solobase_web_bg-") && n.ends_with(".wasm")));
    assert!(!entries.iter().any(|n| n == "solobase_web.js"));
    assert!(!entries.iter().any(|n| n == "solobase_web_bg.wasm"));

    // Assert: rendered sw.js contains a hashed import and no __ placeholders.
    let sw = fs::read_to_string(tmp.path().join("sw.js")).unwrap();
    assert!(sw.contains("from '/solobase_web-"), "sw.js = {sw}");
    assert!(!sw.contains("__WASM_JS__"));
    assert!(!sw.contains("__BUILD_ID__"));

    // Assert: glue file's internal reference was rewritten.
    let glue_name = entries.iter().find(|n| n.starts_with("solobase_web-") && n.ends_with(".js")).unwrap();
    let glue = fs::read_to_string(tmp.path().join(glue_name)).unwrap();
    assert!(glue.contains("solobase_web_bg-"), "glue = {glue}");
    assert!(!glue.contains("'solobase_web_bg.wasm'"));
}

#[test]
fn deterministic_across_runs() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    copy_dir(&fixture_path(), tmp1.path());
    copy_dir(&fixture_path(), tmp2.path());
    solobase_web_bundle::run(tmp1.path(), tmp1.path(), false).unwrap();
    solobase_web_bundle::run(tmp2.path(), tmp2.path(), false).unwrap();

    let m1 = fs::read_to_string(tmp1.path().join("asset-manifest.json")).unwrap();
    let m2 = fs::read_to_string(tmp2.path().join("asset-manifest.json")).unwrap();
    // buildId comes from git in the repo; the `assets` field must match.
    let v1: serde_json::Value = serde_json::from_str(&m1).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&m2).unwrap();
    assert_eq!(v1.get("assets"), v2.get("assets"));
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    for entry in fs::read_dir(src).unwrap() {
        let e = entry.unwrap();
        let to = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            fs::create_dir_all(&to).unwrap();
            copy_dir(&e.path(), &to);
        } else {
            fs::copy(e.path(), to).unwrap();
        }
    }
}
```

- [ ] **Step 2: Create fixture input files**

`tests/fixtures/pkg-in/solobase_web.js`:
```js
// fake glue
export async function init() {
    const url = new URL('solobase_web_bg.wasm', import.meta.url);
    return fetch(url);
}
```

`tests/fixtures/pkg-in/solobase_web_bg.wasm` — any non-empty content, e.g. write `deadbeef` ASCII bytes.

`tests/fixtures/pkg-in/sql-wasm-esm.js`:
```js
var x = "sql-wasm.wasm";
```

`tests/fixtures/pkg-in/sql-wasm.wasm` — any non-empty content.

`tests/fixtures/pkg-in/sw.js.tmpl`:
```js
// @generated build: __BUILD_ID__
import init, { initialize, handle_request } from '__WASM_JS__';
```

`tests/fixtures/pkg-in/index.html.tmpl`:
```html
<!DOCTYPE html><html><head><meta http-equiv="Cache-Control" content="no-cache"></head>
<body><script src="/loader.js"></script></body></html>
```

- [ ] **Step 3: Run the integration test — expect FAIL**

Run: `cargo test -p solobase-web-bundle --test integration`
Expected: FAIL — `run` function doesn't exist yet.

- [ ] **Step 4: Implement `run` in `lib.rs`**

Replace `src/lib.rs` with:
```rust
pub mod build_id;
pub mod hash;
pub mod manifest;
pub mod rename;
pub mod template;

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Input files we content-hash. Keys are logical names used by templates.
const HASHED_ASSETS: &[(&str, &str)] = &[
    ("solobase_web.js", "solobase_web.js"),
    ("solobase_web_bg.wasm", "solobase_web_bg.wasm"),
    ("sql-wasm-esm.js", "sql-wasm-esm.js"),
    ("sql-wasm.wasm", "sql-wasm.wasm"),
];

/// Cross-references inside hashed files that we rewrite after renaming.
/// (source-logical, quote-char, target-logical)
/// The literal we search for is the quoted canonical target filename.
const REWRITES: &[(&str, char, &str)] = &[
    ("solobase_web.js", '\'', "solobase_web_bg.wasm"),
    ("sql-wasm-esm.js", '"', "sql-wasm.wasm"),
];

pub fn run(pkg_dir: &Path, repo_dir: &Path, dev: bool) -> Result<()> {
    if dev {
        return run_dev(pkg_dir);
    }
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
    let mut renamed: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();

    // 1. Compute hashes and rename each asset.
    for (logical, filename) in HASHED_ASSETS {
        let src = pkg_dir.join(filename);
        let bytes = std::fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
        let hash = hash::short_hash(&bytes);
        let new_path = rename::rename_with_hash(&src, &hash)?;
        hashes.insert((*logical).to_string(), hash);
        renamed.insert((*logical).to_string(), new_path);
    }

    // 2. Rewrite cross-references. The source file currently contains the
    //    canonical (un-hashed) target filename inside matching quotes;
    //    replace it with the hashed filename we just renamed to.
    for (source_logical, quote, target_logical) in REWRITES {
        let source_path = renamed.get(*source_logical)
            .ok_or_else(|| anyhow::anyhow!("missing renamed source: {source_logical}"))?;
        let old_name = HASHED_ASSETS.iter()
            .find(|(l, _)| *l == *target_logical).unwrap().1;
        let new_name = renamed.get(*target_logical)
            .ok_or_else(|| anyhow::anyhow!("missing renamed target: {target_logical}"))?
            .file_name().unwrap().to_string_lossy().into_owned();
        let old_literal = format!("{quote}{old_name}{quote}");
        let new_literal = format!("{quote}{new_name}{quote}");
        rename::rewrite_literal(source_path, &old_literal, &new_literal)?;
    }

    // 3. Compute buildId.
    let asset_hashes_ordered: Vec<&str> = HASHED_ASSETS.iter()
        .map(|(l, _)| hashes.get(*l).unwrap().as_str())
        .collect();
    let build_id = build_id::build_id(repo_dir, &asset_hashes_ordered);

    // 4. Write asset-manifest.json.
    let mut manifest_assets = BTreeMap::new();
    for (logical, _) in HASHED_ASSETS {
        let new_path = renamed.get(*logical).unwrap();
        let url = format!("/{}", new_path.file_name().unwrap().to_string_lossy());
        manifest_assets.insert((*logical).to_string(), url);
    }
    let manifest = manifest::AssetManifest {
        build_id: build_id.clone(),
        assets: manifest_assets.clone(),
    };
    manifest.write(&pkg_dir.join("asset-manifest.json"))?;

    // 5. Render templates.
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("BUILD_ID".to_string(), build_id);
    for (logical, url) in &manifest_assets {
        vars.insert(template_key(logical), url.clone());
    }
    render_if_exists(pkg_dir, "sw.js.tmpl", "sw.js", &vars)?;
    render_if_exists(pkg_dir, "index.html.tmpl", "index.html", &vars)?;

    Ok(())
}

fn run_dev(pkg_dir: &Path) -> Result<()> {
    // Skip hashing; just render templates with canonical names.
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("BUILD_ID".to_string(), "dev".to_string());
    for (logical, filename) in HASHED_ASSETS {
        vars.insert(template_key(logical), format!("/{filename}"));
    }
    render_if_exists(pkg_dir, "sw.js.tmpl", "sw.js", &vars)?;
    render_if_exists(pkg_dir, "index.html.tmpl", "index.html", &vars)?;
    Ok(())
}

fn render_if_exists(pkg_dir: &Path, src_name: &str, out_name: &str, vars: &BTreeMap<String, String>) -> Result<()> {
    let src = pkg_dir.join(src_name);
    if !src.exists() {
        return Ok(());
    }
    template::render_to_file(&src, &pkg_dir.join(out_name), vars)?;
    std::fs::remove_file(&src).ok();
    Ok(())
}

/// Convert a logical asset name into a template key.
/// `solobase_web.js`       → `WASM_JS`
/// `solobase_web_bg.wasm`  → `WASM_BIN`
/// `sql-wasm-esm.js`       → `SQL_JS`
/// `sql-wasm.wasm`         → `SQL_WASM`
fn template_key(logical: &str) -> String {
    match logical {
        "solobase_web.js" => "WASM_JS".into(),
        "solobase_web_bg.wasm" => "WASM_BIN".into(),
        "sql-wasm-esm.js" => "SQL_JS".into(),
        "sql-wasm.wasm" => "SQL_WASM".into(),
        other => panic!("unknown logical asset: {other}"),
    }
}
```

- [ ] **Step 5: Run the integration test — expect PASS**

Run: `cargo test -p solobase-web-bundle --test integration`
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/solobase-web-bundle/src/lib.rs crates/solobase-web-bundle/tests/
git commit -m "feat(solobase-web-bundle): end-to-end pipeline + integration tests"
```

---

## Task 8: CLI entry in `main.rs`

**Files:**
- Modify: `crates/solobase-web-bundle/src/main.rs`

- [ ] **Step 1: Replace `main.rs` with CLI**

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "solobase-web-bundle")]
#[command(about = "Content-hash solobase-web/pkg assets and render SW/HTML templates")]
struct Cli {
    /// Path to the `pkg/` directory produced by wasm-pack.
    pkg_dir: PathBuf,

    /// Repo root (used to read `git rev-parse` for the build id). Defaults to `pkg_dir`'s parent.
    #[arg(long)]
    repo_dir: Option<PathBuf>,

    /// Skip hashing; render templates with canonical filenames for fast local iteration.
    #[arg(long)]
    dev: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = cli.repo_dir.clone()
        .or_else(|| cli.pkg_dir.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| cli.pkg_dir.clone());
    solobase_web_bundle::run(&cli.pkg_dir, &repo, cli.dev)
}
```

- [ ] **Step 2: Smoke-test the CLI**

Run:
```bash
cargo build -p solobase-web-bundle --release
./target/release/solobase-web-bundle --help
```
Expected: usage text printed, exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/solobase-web-bundle/src/main.rs
git commit -m "feat(solobase-web-bundle): CLI with --dev flag"
```

---

## Task 9: Create `sw.js.tmpl` and `index.html.tmpl`

**Files:**
- Create: `crates/solobase-web/js/sw.js.tmpl`
- Create: `crates/solobase-web/js/index.html.tmpl`
- Delete: `crates/solobase-web/js/sw.js`
- Delete: `crates/solobase-web/js/index.html`

- [ ] **Step 1: Create `sw.js.tmpl`**

Copy the full current `crates/solobase-web/js/sw.js` verbatim. Then change **only** the first two lines from:

```js
// sw.js — Service Worker that runs Solobase via WASM
import init, { initialize, handle_request } from './solobase_web.js';
```

to:

```js
// @generated build: __BUILD_ID__ — Service Worker that runs Solobase via WASM
import init, { initialize, handle_request } from '__WASM_JS__';
```

Then locate the cache-bypass block (in the current file at sw.js:125-146, inside the `fetch` listener). Replace the list from:

```js
if (url.pathname === '/sw.js' ||
    url.pathname === '/loader.js' ||
    url.pathname === '/ai-bridge.js' ||
    url.pathname === '/manifest.json' ||
    url.pathname === '/index.html' ||
    url.pathname === '/' ||
    url.pathname.startsWith('/pkg/') ||
    url.pathname.startsWith('/sql-')) {
    return;
}
```

to:

```js
if (url.pathname === '/sw.js' ||
    url.pathname === '/loader.js' ||
    url.pathname === '/ai-bridge.js' ||
    url.pathname === '/manifest.json' ||
    url.pathname === '/index.html' ||
    url.pathname === '/' ||
    url.pathname === '/asset-manifest.json' ||
    url.pathname.startsWith('/solobase_web') ||
    url.pathname.startsWith('/snippets/') ||
    url.pathname.startsWith('/sql-')) {
    return;
}
```

- [ ] **Step 2: Create `index.html.tmpl`**

Copy the current `crates/solobase-web/js/index.html` verbatim, and add inside `<head>` a cache-hint meta tag:

```html
<meta http-equiv="Cache-Control" content="no-cache">
```

No other changes. No placeholders needed in `index.html` itself — `loader.js` is referenced by stable URL.

- [ ] **Step 3: Delete the old hand-written files**

```bash
git rm crates/solobase-web/js/sw.js
git rm crates/solobase-web/js/index.html
```

- [ ] **Step 4: Commit**

```bash
git add crates/solobase-web/js/sw.js.tmpl crates/solobase-web/js/index.html.tmpl
git commit -m "refactor(solobase-web): convert sw.js and index.html to templates"
```

---

## Task 10: Add `updateViaCache: 'none'` to `loader.js`

**Files:**
- Modify: `crates/solobase-web/js/loader.js`

- [ ] **Step 1: Update the register call**

Replace (loader.js:9-12):
```js
        const registration = await navigator.serviceWorker.register('/sw.js', {
            type: 'module',
            scope: '/',
        });
```
with:
```js
        const registration = await navigator.serviceWorker.register('/sw.js', {
            type: 'module',
            scope: '/',
            updateViaCache: 'none',
        });
```

- [ ] **Step 2: Commit**

```bash
git add crates/solobase-web/js/loader.js
git commit -m "fix(solobase-web): bypass HTTP cache for SW update check (updateViaCache: none)"
```

---

## Task 11: Wire bundler into the Makefile

**Files:**
- Modify: `crates/solobase-web/Makefile`

- [ ] **Step 1: Update `build` and `dev` targets**

Replace:
```makefile
# Build for production
build: pkg/sql-wasm-esm.js
	wasm-pack build --target web --release --out-dir pkg
	cp js/sw.js pkg/
	cp js/loader.js pkg/
	cp js/ai-bridge.js pkg/
	cp js/index.html pkg/

# Build for development
dev: pkg/sql-wasm-esm.js
	wasm-pack build --target web --dev --out-dir pkg
	cp js/sw.js pkg/
	cp js/loader.js pkg/
	cp js/ai-bridge.js pkg/
	cp js/index.html pkg/
```

with:
```makefile
# Build for production (content-hashes assets, renders templates)
build: pkg/sql-wasm-esm.js
	wasm-pack build --target web --release --out-dir pkg
	cp js/sw.js.tmpl pkg/
	cp js/index.html.tmpl pkg/
	cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-web-bundle --release -- pkg/ --repo-dir $(CURDIR)/../..

# Build for development (no hashing; canonical filenames)
dev: pkg/sql-wasm-esm.js
	wasm-pack build --target web --dev --out-dir pkg
	cp js/sw.js.tmpl pkg/
	cp js/index.html.tmpl pkg/
	cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
	cargo run -p solobase-web-bundle --release -- pkg/ --repo-dir $(CURDIR)/../.. --dev
```

Note: `js/manifest.json` is the PWA manifest. It already exists in the `js/` directory (verified) but the previous Makefile did not copy it to `pkg/` — `index.html` references it, so this is a minor pre-existing bug we fix here.

- [ ] **Step 2: Run a clean prod build**

Run:
```bash
cd crates/solobase-web
make clean
make build
```
Expected: build succeeds; `pkg/` contains hashed `solobase_web-*.js`, `solobase_web_bg-*.wasm`, `asset-manifest.json`, and `sw.js` / `index.html` with no `__…__` placeholders.

- [ ] **Step 3: Verify no placeholders leaked**

Run:
```bash
grep -E '__[A-Z_]+__' crates/solobase-web/pkg/sw.js crates/solobase-web/pkg/index.html || echo "OK — no placeholders"
```
Expected: prints `OK — no placeholders`.

- [ ] **Step 4: Verify sw.js imports a hashed URL**

Run:
```bash
head -2 crates/solobase-web/pkg/sw.js
```
Expected: output shows the build id and an import from `/solobase_web-<hash>.js`.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/Makefile
git commit -m "build(solobase-web): run solobase-web-bundle after wasm-pack"
```

---

## Task 12: Manual browser smoke test

- [ ] **Step 1: Serve the build**

In one terminal:
```bash
cd crates/solobase-web
make serve
```

- [ ] **Step 2: Open the site and verify the SW activates**

In a fresh browser tab, open `http://localhost:8080/` and open DevTools → Application → Service Workers. Verify:
- `/sw.js` is activated.
- Network panel shows `/sw.js` fetched with `sw: 1` (not from disk cache).
- Network panel shows `/solobase_web-<hash>.js` and `/solobase_web_bg-<hash>.wasm` fetched.
- Console shows the existing `[solobase-web] Runtime ready.` log.

- [ ] **Step 3: Trigger an update**

In another terminal:
```bash
cd /home/joris/Programs/suppers-ai/workspace/solobase
touch crates/solobase-web/src/lib.rs
cd crates/solobase-web && make build
```

Back in the browser, refresh. Verify:
- `/sw.js` was re-fetched (check its hash in the Application tab — the build-id comment changes).
- A new `/solobase_web_bg-<hash>.wasm` URL (different hash) was fetched.
- The old hashed URL is not re-requested.

- [ ] **Step 4: Document the result**

No commit needed unless you found a bug; this is a manual verification step. If the smoke test fails, investigate — do not proceed to Task 13.

---

## Task 13: Refactor `packages/solobase-web/src/worker.ts` lifecycle

**Files:**
- Modify: `packages/solobase-web/src/worker.ts`

- [ ] **Step 1: Replace the install handler**

Locate (worker.ts:44-46):
```ts
  self.addEventListener('install', (event) => {
    event.waitUntil(initialize().then(() => self.skipWaiting()));
  });
```

Replace with:
```ts
  self.addEventListener('install', (event) => {
    event.waitUntil(initialize());
    // NOTE: no skipWaiting here. Consumers opt in by posting
    // { type: 'skip-waiting' } from the main thread when they want to
    // apply an update. The standalone pkg/ site uses its own sw.js
    // which does call skipWaiting.
  });
```

- [ ] **Step 2: Extend the message handler**

Locate (worker.ts:52-56):
```ts
  self.addEventListener('message', (event) => {
    if (event.data?.type === 'solobase:config' && Array.isArray(event.data.routes)) {
      routes = event.data.routes;
    }
  });
```

Replace with:
```ts
  self.addEventListener('message', (event) => {
    if (event.data?.type === 'skip-waiting') {
      self.skipWaiting();
      return;
    }
    if (event.data?.type === 'solobase:config' && Array.isArray(event.data.routes)) {
      routes = event.data.routes;
    }
  });
```

- [ ] **Step 3: Build the package**

Run: `cd packages/solobase-web && npm run build:ts`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add packages/solobase-web/src/worker.ts
git commit -m "refactor(solobase-web/package): drop auto-skipWaiting; add skip-waiting message"
```

---

## Task 14: New module `src/update.ts` with update-lifecycle helpers

**Files:**
- Create: `packages/solobase-web/src/update.ts`
- Create: `packages/solobase-web/src/update.test.ts`
- Modify: `packages/solobase-web/src/index.ts` (re-export)

- [ ] **Step 1: Write failing unit tests**

Create `packages/solobase-web/src/update.test.ts`:
```ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { registerWithUpdates } from './update';

type Listener = (event: any) => void;

function makeFakeWorker() {
  const listeners: Record<string, Listener[]> = {};
  const postMessage = vi.fn();
  return {
    state: 'installing' as 'installing' | 'installed' | 'activating' | 'activated',
    postMessage,
    addEventListener(ev: string, cb: Listener) { (listeners[ev] ||= []).push(cb); },
    removeEventListener(ev: string, cb: Listener) {
      listeners[ev] = (listeners[ev] || []).filter(l => l !== cb);
    },
    _fire(ev: string, data: any = {}) { (listeners[ev] || []).forEach(l => l(data)); },
  };
}

function makeRegistration(installing: any = null, waiting: any = null) {
  const listeners: Record<string, Listener[]> = {};
  return {
    installing, waiting,
    update: vi.fn().mockResolvedValue(undefined),
    addEventListener(ev: string, cb: Listener) { (listeners[ev] ||= []).push(cb); },
    _fire(ev: string, data: any = {}) { (listeners[ev] || []).forEach(l => l(data)); },
  };
}

describe('registerWithUpdates', () => {
  beforeEach(() => {
    const waitingWorker = makeFakeWorker();
    const registration = makeRegistration(null, waitingWorker);
    (globalThis as any).navigator = {
      serviceWorker: {
        register: vi.fn().mockResolvedValue(registration),
        controller: { postMessage: vi.fn() },
        addEventListener: vi.fn(),
      },
    };
    (globalThis as any)._fakes = { waitingWorker, registration };
  });

  it('resolves to a handle exposing the registration', async () => {
    const handle = await registerWithUpdates('/sw.js');
    expect(handle.registration).toBe((globalThis as any)._fakes.registration);
  });

  it('does not fire updateReady on first install (no existing controller)', async () => {
    (globalThis as any).navigator.serviceWorker.controller = null;
    const handle = await registerWithUpdates('/sw.js');
    const cb = vi.fn();
    handle.onUpdateReady(cb);
    const { registration } = (globalThis as any)._fakes;
    const newWorker = makeFakeWorker();
    registration.installing = newWorker;
    registration._fire('updatefound');
    newWorker.state = 'installed';
    newWorker._fire('statechange');
    expect(cb).not.toHaveBeenCalled();
  });

  it('fires updateReady when a new worker installs while an old one controls', async () => {
    const handle = await registerWithUpdates('/sw.js');
    const cb = vi.fn();
    handle.onUpdateReady(cb);
    const { registration } = (globalThis as any)._fakes;
    const newWorker = makeFakeWorker();
    registration.installing = newWorker;
    registration._fire('updatefound');
    newWorker.state = 'installed';
    newWorker._fire('statechange');
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('apply() posts skip-waiting to the waiting worker', async () => {
    const handle = await registerWithUpdates('/sw.js');
    const cb = vi.fn();
    handle.onUpdateReady(cb);
    const { registration } = (globalThis as any)._fakes;
    const newWorker = makeFakeWorker();
    registration.installing = newWorker;
    registration._fire('updatefound');
    newWorker.state = 'installed';
    registration.waiting = newWorker;
    newWorker._fire('statechange');
    const apply = cb.mock.calls[0][0];
    apply();
    expect(newWorker.postMessage).toHaveBeenCalledWith({ type: 'skip-waiting' });
  });

  it('checkForUpdate() calls registration.update()', async () => {
    const handle = await registerWithUpdates('/sw.js');
    await handle.checkForUpdate();
    expect((globalThis as any)._fakes.registration.update).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cd packages/solobase-web && npx vitest run src/update.test.ts`
Expected: FAIL — `update.ts` doesn't exist.

If `vitest` isn't in `devDependencies` yet, add it: `npm i -D vitest @types/node` and re-run.

- [ ] **Step 3: Implement `src/update.ts`**

```ts
export interface UpdateHandle {
  registration: ServiceWorkerRegistration;
  /**
   * Subscribe to updates. The callback receives an `apply` function that
   * posts `skip-waiting` to the installed-but-waiting SW; call it when the
   * consumer is ready to switch over (e.g., after user clicks a toast).
   * Returns an unsubscribe function.
   */
  onUpdateReady(cb: (apply: () => Promise<void>) => void): () => void;
  /** Force an update check. Wraps `registration.update()`. */
  checkForUpdate(): Promise<void>;
}

export async function registerWithUpdates(
  scriptURL: string,
  opts?: { scope?: string; type?: WorkerType },
): Promise<UpdateHandle> {
  const registration = await navigator.serviceWorker.register(scriptURL, {
    scope: opts?.scope ?? '/',
    type: opts?.type ?? 'module',
    updateViaCache: 'none',
  });

  const callbacks = new Set<(apply: () => Promise<void>) => void>();

  registration.addEventListener('updatefound', () => {
    const installing = registration.installing;
    if (!installing) return;
    installing.addEventListener('statechange', () => {
      if (installing.state !== 'installed') return;
      // Only treat as "update" when there's an existing controller.
      if (!navigator.serviceWorker.controller) return;
      const apply = () => applyUpdate(registration);
      for (const cb of callbacks) cb(apply);
    });
  });

  return {
    registration,
    onUpdateReady(cb) {
      callbacks.add(cb);
      return () => callbacks.delete(cb);
    },
    async checkForUpdate() {
      await registration.update();
    },
  };
}

function applyUpdate(registration: ServiceWorkerRegistration): Promise<void> {
  const waiting = registration.waiting ?? registration.installing;
  if (!waiting) return Promise.resolve();
  return new Promise<void>((resolve) => {
    const onChange = () => {
      navigator.serviceWorker.removeEventListener('controllerchange', onChange);
      resolve();
    };
    navigator.serviceWorker.addEventListener('controllerchange', onChange);
    waiting.postMessage({ type: 'skip-waiting' });
  });
}
```

- [ ] **Step 4: Re-export from `src/index.ts`**

Append:
```ts
export { registerWithUpdates } from './update';
export type { UpdateHandle } from './update';
```

- [ ] **Step 5: Run tests — expect PASS**

Run: `cd packages/solobase-web && npx vitest run src/update.test.ts`
Expected: all five tests PASS.

- [ ] **Step 6: Build**

Run: `npm run build:ts`
Expected: compiles clean.

- [ ] **Step 7: Commit**

```bash
git add packages/solobase-web/src/update.ts packages/solobase-web/src/update.test.ts packages/solobase-web/src/index.ts
git commit -m "feat(solobase-web/package): add registerWithUpdates helper"
```

---

## Task 15: Vite fixture verifying bundler asset handling

**Files:**
- Create: `packages/solobase-web/test-fixtures/vite-app/package.json`
- Create: `packages/solobase-web/test-fixtures/vite-app/vite.config.ts`
- Create: `packages/solobase-web/test-fixtures/vite-app/index.html`
- Create: `packages/solobase-web/test-fixtures/vite-app/src/main.ts`
- Create: `packages/solobase-web/test-fixtures/vite-app/src/sw.ts`
- Create: `packages/solobase-web/test-fixtures/vite-app/verify.mjs`

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "vite-app-fixture",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vite build",
    "verify": "node verify.mjs"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  },
  "dependencies": {
    "solobase-web": "file:../../"
  }
}
```

- [ ] **Step 2: Create `vite.config.ts`**

```ts
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    outDir: 'dist',
    rollupOptions: {
      input: {
        main: './index.html',
        sw: './src/sw.ts',
      },
      output: {
        entryFileNames: (info) => info.name === 'sw' ? 'sw.js' : 'assets/[name]-[hash].js',
      },
    },
  },
});
```

- [ ] **Step 3: Create `index.html`**

```html
<!DOCTYPE html>
<html><head><title>fixture</title></head>
<body><script type="module" src="/src/main.ts"></script></body></html>
```

- [ ] **Step 4: Create `src/main.ts`**

```ts
import { registerWithUpdates } from 'solobase-web';

registerWithUpdates('/sw.js').then((handle) => {
  console.log('registered', handle.registration);
});
```

- [ ] **Step 5: Create `src/sw.ts`**

```ts
import { initialize, handleRequest } from 'solobase-web/worker';

self.addEventListener('install', (event: any) => {
  event.waitUntil(initialize());
});

self.addEventListener('fetch', (event: any) => {
  event.respondWith(handleRequest(event.request));
});
```

- [ ] **Step 6: Create `verify.mjs`**

```js
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const dist = 'dist';
const sw = readFileSync(join(dist, 'sw.js'), 'utf8');
const assets = readdirSync(join(dist, 'assets'));
const wasm = assets.find((n) => n.endsWith('.wasm'));
if (!wasm) {
  console.error('FAIL: no .wasm emitted in dist/assets');
  process.exit(1);
}
if (!wasm.match(/\.[a-z0-9]{8,}\.wasm$/)) {
  console.error(`FAIL: wasm filename not hashed: ${wasm}`);
  process.exit(1);
}
if (!sw.includes(wasm)) {
  console.error(`FAIL: sw.js does not reference the hashed wasm ${wasm}`);
  process.exit(1);
}
console.log(`OK: ${wasm} referenced from sw.js`);
```

- [ ] **Step 7: Build & verify**

Run:
```bash
cd packages/solobase-web
npm run build              # ensure the package is built so file: link works
cd test-fixtures/vite-app
npm install
npm run build
npm run verify
```

Expected: `OK: solobase_web_bg-<hash>.wasm referenced from sw.js`.

- [ ] **Step 8: Commit**

```bash
git add packages/solobase-web/test-fixtures/
git commit -m "test(solobase-web/package): vite fixture verifies bundler WASM hashing"
```

---

## Task 16: README + CHANGELOG + version bump

**Files:**
- Create: `packages/solobase-web/CHANGELOG.md`
- Modify: `packages/solobase-web/README.md`
- Modify: `packages/solobase-web/package.json`

- [ ] **Step 1: Create `CHANGELOG.md`**

```markdown
# Changelog

## 0.2.0

### Breaking changes

- `worker.ts` no longer calls `self.skipWaiting()` during `install`. Consumers who want the old behavior should post `{ type: 'skip-waiting' }` to the registration from the main thread after `register()` resolves, or (recommended) use the new `registerWithUpdates` helper.

### New

- `registerWithUpdates(scriptURL, opts?)` — registers the SW and returns a handle with `onUpdateReady` and `checkForUpdate` for wiring update UX.
- `UpdateHandle` type re-exported from the package root.

## 0.1.0

- Initial release.
```

- [ ] **Step 2: Update `README.md`**

Locate the existing content (or create a minimal README if none exists). Append the following sections:

```markdown
## Bundler integration

The package ships wasm-pack output unmodified at `dist/wasm/`. The embedded glue uses `new URL('solobase_web_bg.wasm', import.meta.url)`, which Vite, Rollup, webpack 5, and esbuild all detect and bundle as a hashed asset automatically.

### Vite / Rollup
No config required in typical setups. The `.wasm` will be emitted to `dist/assets/` with a content hash.

### webpack 5
Make sure `experiments.asyncWebAssembly` is enabled in your webpack config; it handles the URL pattern out of the box.

### esbuild
Add a `.wasm` file loader:

```js
build({
  loader: { '.wasm': 'file' },
  // ...
});
```

## Service Worker update lifecycle

The exported `worker.ts` does **not** call `skipWaiting()` during install so consumers can control when an update takes effect. Three common patterns:

### Silent (pick up on next navigation)

```ts
import { registerWithUpdates } from 'solobase-web';

await registerWithUpdates('/sw.js');
// Do nothing else. Next hard navigation uses the new SW.
```

### Auto-reload on update

```ts
import { registerWithUpdates } from 'solobase-web';

const handle = await registerWithUpdates('/sw.js');
handle.onUpdateReady(async (apply) => {
  await apply();
  location.reload();
});
```

### Toast + opt-in reload

```ts
import { registerWithUpdates } from 'solobase-web';

const handle = await registerWithUpdates('/sw.js');
handle.onUpdateReady((apply) => {
  showToast('New version available', async () => {
    await apply();
    location.reload();
  });
});
```
```

- [ ] **Step 3: Bump version**

In `packages/solobase-web/package.json`, change `"version": "0.1.0"` to `"version": "0.2.0"`.

- [ ] **Step 4: Commit**

```bash
git add packages/solobase-web/README.md packages/solobase-web/CHANGELOG.md packages/solobase-web/package.json
git commit -m "docs(solobase-web/package): document bundler + update lifecycle; bump to 0.2.0"
```

---

## Task 17: Playwright E2E for standalone SW update flow

**Files:**
- Create: `crates/solobase-web/tests/playwright.config.ts`
- Create: `crates/solobase-web/tests/e2e/sw-update.spec.ts`
- Create: `crates/solobase-web/package.json` (if not present — add minimal playwright-only `package.json`)

- [ ] **Step 1: Create `crates/solobase-web/package.json`** (only if it does not already exist)

```json
{
  "name": "solobase-web-tests",
  "private": true,
  "type": "module",
  "scripts": {
    "e2e": "playwright test"
  },
  "devDependencies": {
    "@playwright/test": "^1.40.0"
  }
}
```

Skip this step if a `package.json` already exists — add `@playwright/test` as a devDependency instead.

- [ ] **Step 2: Create `tests/playwright.config.ts`**

```ts
import { defineConfig, devices } from '@playwright/test';

const PORT = process.env.TEST_PORT ? parseInt(process.env.TEST_PORT) : 8080;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,        // tests share the served pkg/ dir
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  timeout: 60_000,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    serviceWorkers: 'allow',
  },
  projects: [
    { name: 'desktop-chrome', use: { ...devices['Desktop Chrome'] } },
  ],
});
```

- [ ] **Step 3: Create `tests/e2e/sw-update.spec.ts`**

```ts
import { test, expect } from '@playwright/test';
import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const PKG = join(__dirname, '../../pkg');

function readManifestBuildId(): string {
  const body = readFileSync(join(PKG, 'asset-manifest.json'), 'utf8');
  return JSON.parse(body).buildId as string;
}

test('a new build causes the SW to update and fetch new hashed WASM', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  const initialBuildId = readManifestBuildId();

  // Capture the currently loaded wasm URL via the Network panel.
  const initialWasm = await new Promise<string>((resolve) => {
    page.on('request', function listener(req) {
      if (req.url().match(/solobase_web_bg-[a-f0-9]+\.wasm/)) {
        page.off('request', listener);
        resolve(req.url());
      }
    });
    page.reload();
  });

  // Rebuild with a small change that alters the WASM output.
  execSync(
    `touch crates/solobase-web/src/lib.rs && cd crates/solobase-web && make build`,
    { cwd: join(__dirname, '../../../..'), stdio: 'inherit' },
  );

  const newBuildId = readManifestBuildId();
  expect(newBuildId).not.toBe(initialBuildId);

  // Reload; the new SW should install, activate, and fetch a new wasm URL.
  const newWasm = await new Promise<string>((resolve) => {
    page.on('request', function listener(req) {
      if (req.url().match(/solobase_web_bg-[a-f0-9]+\.wasm/) && req.url() !== initialWasm) {
        page.off('request', listener);
        resolve(req.url());
      }
    });
    page.reload();
  });

  expect(newWasm).not.toBe(initialWasm);
});

test('a no-op rebuild does not trigger a SW update', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  const buildId1 = readManifestBuildId();

  // Re-run make build without changing any source.
  execSync(
    `cd crates/solobase-web && make build`,
    { cwd: join(__dirname, '../../../..'), stdio: 'inherit' },
  );

  const buildId2 = readManifestBuildId();
  expect(buildId2).toBe(buildId1);
});
```

- [ ] **Step 4: Install & run**

In one terminal:
```bash
cd crates/solobase-web
npm install
make serve &
```

Wait for the server to print `Serving at http://localhost:8080`, then:

```bash
npx playwright install chromium
npm run e2e
```

Expected: both tests PASS. The "no-op" test may be skipped if the git working tree is dirty between runs (the `-dirty` suffix would change); ensure a clean tree before running.

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-web/tests/ crates/solobase-web/package.json
git commit -m "test(solobase-web): e2e verifies SW update on rebuild"
```

---

## Self-Review Checklist

Before marking the plan complete, verify:

- [ ] **Spec coverage:** every section of the spec maps to tasks above.
  - "Standalone `pkg/`" → Tasks 1–11.
  - "npm package" → Tasks 13–16.
  - "SW update flow" / `updateViaCache` → Task 10.
  - "Cache-bypass list" correction → Task 9, step 1.
  - "Testing / Rollout" → Tasks 7, 11, 15, 17.
- [ ] **No placeholders:** every step either runs a concrete command, shows full code, or is a concrete verification.
- [ ] **Type consistency:** `UpdateHandle` signature matches across Tasks 14, 16.
- [ ] **Commits every ≤2 tasks:** yes, each task ends with a commit.
