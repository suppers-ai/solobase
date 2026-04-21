# Solobase CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a `solobase` CLI binary that replaces the Makefile/justfile boilerplate in every solobase-browser consumer (solobase-web, gizza-ai) with `solobase build` / `solobase dev` / `solobase serve`, driven by a per-consumer `solobase.toml`.

**Architecture:** New `crates/solobase-cli` in the solobase workspace, bin-only. The CLI is a thin wrapper that shells out to `wasm-pack`, `cargo run -p solobase-browser --bin export-assets`, `wafer` (for skill-block discovery), and `python3 -m http.server` for `serve`. Pure-function arg construction (`build_wasm_pack_args`, `build_export_assets_args`) gets unit-tested; the pipeline gets an integration test per fixture.

**Tech Stack:** Rust, clap, serde, toml, anyhow, glob. Dev: tempfile.

**Spec:** `docs/superpowers/specs/2026-04-21-solobase-cli-design.md`

---

## File structure

```
crates/solobase-cli/
  Cargo.toml
  src/
    main.rs        # clap parser + dispatch
    config.rs      # solobase.toml loader + validation
    build.rs       # build subcommand: pipeline + arg construction
    serve.rs       # serve subcommand (delegates to build + http.server)
    skills.rs      # skill-block auto-discovery + wafer delegation
    cmd.rs         # std::process::Command helpers with structured errors
  tests/
    integration_smoke.rs  # end-to-end fixture tests
```

Unit tests live inline in each module under `#[cfg(test)] mod tests`. The only `tests/` file is the integration smoke.

The workspace root `Cargo.toml` adds `crates/solobase-cli` to the `members` array.

---

## Task 1: Scaffold the crate

**Files:**
- Create: `crates/solobase-cli/Cargo.toml`
- Create: `crates/solobase-cli/src/main.rs`
- Modify: `Cargo.toml` (workspace root) — add `"crates/solobase-cli"` to `members`

- [ ] **Step 1.1: Write the Cargo.toml**

```toml
[package]
name = "solobase-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Build / dev / serve CLI for solobase-browser consumers"

[[bin]]
name = "solobase"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
glob = "0.3"
serde = { version = "1", features = ["derive"] }
toml = "0.8"

[dev-dependencies]
tempfile = "3"

[lints]
workspace = true
```

- [ ] **Step 1.2: Write a minimal main.rs**

```rust
fn main() -> anyhow::Result<()> {
    println!("solobase-cli: stub");
    Ok(())
}
```

- [ ] **Step 1.3: Add the crate to the workspace**

In the workspace root `Cargo.toml`, append `"crates/solobase-cli",` to the `[workspace] members = [...]` array, keeping alphabetical order.

- [ ] **Step 1.4: Verify it compiles**

Run: `cargo check -p solobase-cli`
Expected: clean build.

- [ ] **Step 1.5: Commit**

```bash
git add Cargo.toml crates/solobase-cli
git commit -m "feat(solobase-cli): scaffold bin-only crate"
```

---

## Task 2: Config types

**Files:**
- Create: `crates/solobase-cli/src/config.rs`
- Modify: `crates/solobase-cli/src/main.rs` — add `mod config;`

The config types mirror `solobase.toml`. `deny_unknown_fields` on tables catches typos in config keys. The top-level is intentionally lenient (unknown top-level tables warn rather than error — the spec says so).

- [ ] **Step 2.1: Write the failing test**

Append to `crates/solobase-cli/src/config.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    #[serde(default)]
    pub assets: AssetsConfig,
    #[serde(default)]
    pub wasm: WasmConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub name: String,
    pub title: String,
    pub boot_redirect: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AssetsConfig {
    #[serde(default)]
    pub extra_bypass_prefix: Vec<String>,
    #[serde(default)]
    pub overlay: Vec<OverlayEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayEntry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct WasmConfig {
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
}

fn default_out_dir() -> String {
    "pkg".to_string()
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            out_dir: default_out_dir(),
        }
    }
}

pub fn parse(toml_text: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(toml_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let input = r#"
[app]
name = "solobase-web"
title = "Solobase"
boot_redirect = "/b/system/"
"#;
        let cfg = parse(input).unwrap();
        assert_eq!(cfg.app.name, "solobase-web");
        assert_eq!(cfg.app.title, "Solobase");
        assert_eq!(cfg.app.boot_redirect, "/b/system/");
        assert_eq!(cfg.assets.extra_bypass_prefix, Vec::<String>::new());
        assert!(cfg.assets.overlay.is_empty());
        assert_eq!(cfg.wasm.out_dir, "pkg");
    }
}
```

- [ ] **Step 2.2: Add module to main.rs**

At the top of `crates/solobase-cli/src/main.rs`, before `fn main`:

```rust
mod config;
```

- [ ] **Step 2.3: Run the test**

Run: `cargo test -p solobase-cli --lib config::tests::parse_minimal_config`
Expected: PASS.

- [ ] **Step 2.4: Add the full-config test**

Append inside `mod tests`:

```rust
#[test]
fn parse_full_config() {
    let input = r#"
[app]
name = "gizza-ai"
title = "Gizza AI"
boot_redirect = "/"

[assets]
extra_bypass_prefix = ["/gizza-app.js", "/gizza.css"]

[[assets.overlay]]
from = "site/index.html"
to = "index.html"

[[assets.overlay]]
from = "site/gizza-app.js"
to = "gizza-app.js"

[wasm]
out_dir = "dist"
"#;
    let cfg = parse(input).unwrap();
    assert_eq!(cfg.app.name, "gizza-ai");
    assert_eq!(
        cfg.assets.extra_bypass_prefix,
        vec!["/gizza-app.js".to_string(), "/gizza.css".to_string()]
    );
    assert_eq!(cfg.assets.overlay.len(), 2);
    assert_eq!(cfg.assets.overlay[0].from, "site/index.html");
    assert_eq!(cfg.assets.overlay[0].to, "index.html");
    assert_eq!(cfg.wasm.out_dir, "dist");
}
```

- [ ] **Step 2.5: Add the rejection test**

Append:

```rust
#[test]
fn reject_unknown_field_in_app() {
    let input = r#"
[app]
name = "x"
title = "y"
boot_redirect = "/"
color = "red"
"#;
    let err = parse(input).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("color"), "expected error to mention 'color', got: {msg}");
}

#[test]
fn reject_missing_app() {
    let input = r#"
[assets]
extra_bypass_prefix = []
"#;
    let err = parse(input).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("app"), "expected error to mention 'app', got: {msg}");
}
```

- [ ] **Step 2.6: Run the tests**

Run: `cargo test -p solobase-cli --lib config::tests`
Expected: 4 pass.

- [ ] **Step 2.7: Commit**

```bash
git add crates/solobase-cli/src
git commit -m "feat(solobase-cli): add solobase.toml parser + validation"
```

---

## Task 3: Config file resolver

**Files:**
- Modify: `crates/solobase-cli/src/config.rs`

Walks up from a starting directory to find the first enclosing `solobase.toml`. Returns the parsed config + the absolute path to the repo root (the directory containing the file).

- [ ] **Step 3.1: Add the failing test**

Append inside `crates/solobase-cli/src/config.rs`'s `mod tests`:

```rust
#[test]
fn find_config_walks_up() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("solobase.toml"),
        r#"
[app]
name = "x"
title = "y"
boot_redirect = "/"
"#,
    )
    .unwrap();
    let nested = root.join("sub/dir");
    fs::create_dir_all(&nested).unwrap();

    let (cfg, repo_root) = find_and_load(&nested).unwrap();
    assert_eq!(cfg.app.name, "x");
    // Use canonicalize to normalize macOS /var/folders -> /private/var/folders differences.
    assert_eq!(
        repo_root.canonicalize().unwrap(),
        root.canonicalize().unwrap()
    );
}

#[test]
fn find_config_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let err = find_and_load(tmp.path()).unwrap_err().to_string();
    assert!(err.contains("solobase.toml"));
    assert!(err.contains("no"));
}
```

- [ ] **Step 3.2: Run the test, confirm it fails**

Run: `cargo test -p solobase-cli --lib config::tests::find_config_walks_up`
Expected: FAIL (`find_and_load` not defined).

- [ ] **Step 3.3: Implement find_and_load**

Add to `crates/solobase-cli/src/config.rs` (above `#[cfg(test)]`):

```rust
use std::path::{Path, PathBuf};

/// Walk up from `start` looking for `solobase.toml`; parse and return
/// `(config, repo_root)` where `repo_root` is the directory that contains
/// the file.
pub fn find_and_load(start: &Path) -> anyhow::Result<(Config, PathBuf)> {
    let start = start
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("canonicalize {start:?}: {e}"))?;
    let mut cur: &Path = &start;
    loop {
        let candidate = cur.join("solobase.toml");
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)
                .map_err(|e| anyhow::anyhow!("read {candidate:?}: {e}"))?;
            let cfg = parse(&text)
                .map_err(|e| anyhow::anyhow!("parse {candidate:?}: {e}"))?;
            return Ok((cfg, cur.to_path_buf()));
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => {
                return Err(anyhow::anyhow!(
                    "no solobase.toml found in {start:?} or any parent directory"
                ));
            }
        }
    }
}
```

- [ ] **Step 3.4: Run both tests**

Run: `cargo test -p solobase-cli --lib config::tests`
Expected: all pass.

- [ ] **Step 3.5: Commit**

```bash
git add crates/solobase-cli/src/config.rs
git commit -m "feat(solobase-cli): walk-up config resolver"
```

---

## Task 4: cmd.rs — command runner

**Files:**
- Create: `crates/solobase-cli/src/cmd.rs`
- Modify: `crates/solobase-cli/src/main.rs` — add `mod cmd;`

One helper: runs a `std::process::Command`, streams stdio to the parent, and formats a structured error on non-zero exit.

- [ ] **Step 4.1: Write the failing test**

Create `crates/solobase-cli/src/cmd.rs`:

```rust
use std::process::Command;

/// Error message shape when a child process exits non-zero.
///
/// ```text
/// error: <step> failed
///   command: <arg0> <arg1> ...
///   exit code: <n>
///   --- stderr ---
///   <child stderr>
/// ```
pub fn format_child_error(step: &str, cmd: &Command, exit_code: Option<i32>, stderr: &str) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    let cmd_line = if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    };
    let code = exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "<signal>".to_string());
    format!(
        "error: {step} failed\n  command: {cmd_line}\n  exit code: {code}\n  --- stderr ---\n{stderr}",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_message_shape() {
        let mut cmd = Command::new("wasm-pack");
        cmd.arg("build").arg("--target").arg("web");
        let msg = format_child_error("wasm-pack build", &cmd, Some(101), "boom\n");
        assert!(msg.contains("error: wasm-pack build failed"));
        assert!(msg.contains("command: wasm-pack build --target web"));
        assert!(msg.contains("exit code: 101"));
        assert!(msg.contains("--- stderr ---"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn format_error_unknown_exit_code() {
        let cmd = Command::new("sleep");
        let msg = format_child_error("sleep", &cmd, None, "");
        assert!(msg.contains("exit code: <signal>"));
    }
}
```

- [ ] **Step 4.2: Add module to main.rs**

At the top of `crates/solobase-cli/src/main.rs`:

```rust
mod cmd;
```

- [ ] **Step 4.3: Run the tests**

Run: `cargo test -p solobase-cli --lib cmd::tests`
Expected: 2 pass.

- [ ] **Step 4.4: Add the runner**

Append to `crates/solobase-cli/src/cmd.rs` (above `#[cfg(test)]`):

```rust
/// Run `cmd` inheriting stdio, and map non-zero exits to an `anyhow::Error`
/// with the structured shape documented on `format_child_error`.
///
/// `step` is a short label for the error message (e.g., `"wasm-pack build"`).
pub fn run(step: &str, mut cmd: Command) -> anyhow::Result<()> {
    // We want the child's stderr to appear live for the user AND to be
    // available in the error message. Simplest: let it inherit stderr and
    // don't capture — the user sees it. On failure, the message says
    // "see above" via the stderr tail pattern: we include a note that the
    // child's stderr was streamed already. For small failure modes the
    // user already saw everything.
    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("spawn {step}: {e}"))?;
    if status.success() {
        return Ok(());
    }
    // No captured stderr — it was streamed. Use a shorter message.
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    let cmd_line = if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    };
    let code = status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "<signal>".to_string());
    Err(anyhow::anyhow!(
        "error: {step} failed\n  command: {cmd_line}\n  exit code: {code}\n  (stderr streamed above)"
    ))
}
```

- [ ] **Step 4.5: Verify it compiles**

Run: `cargo check -p solobase-cli`
Expected: clean.

- [ ] **Step 4.6: Commit**

```bash
git add crates/solobase-cli/src/cmd.rs crates/solobase-cli/src/main.rs
git commit -m "feat(solobase-cli): structured child-process error + runner"
```

---

## Task 5: skills.rs — skill-block discovery

**Files:**
- Create: `crates/solobase-cli/src/skills.rs`
- Modify: `crates/solobase-cli/src/main.rs` — add `mod skills;`

Auto-discovers every `blocks/<name>/Cargo.toml` under the repo root and produces a list of paths. Separately, a `build_all` function runs `wafer build` in each dir.

- [ ] **Step 5.1: Write the failing test**

Create `crates/solobase-cli/src/skills.rs`:

```rust
use std::path::{Path, PathBuf};

/// Return each absolute path to a skill-block crate under `<repo_root>/blocks/`.
/// A skill block is a directory directly under `blocks/` that contains a
/// `Cargo.toml`. Non-directories and directories without `Cargo.toml` are
/// skipped silently.
pub fn discover(repo_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let blocks_dir = repo_root.join("blocks");
    if !blocks_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&blocks_dir)
        .map_err(|e| anyhow::anyhow!("read {blocks_dir:?}: {e}"))?
    {
        let entry = entry.map_err(|e| anyhow::anyhow!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn no_blocks_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let found = discover(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn finds_blocks_with_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("blocks/alpha")).unwrap();
        fs::write(root.join("blocks/alpha/Cargo.toml"), "[package]\nname = \"alpha\"\n").unwrap();
        fs::create_dir_all(root.join("blocks/beta")).unwrap();
        fs::write(root.join("blocks/beta/Cargo.toml"), "[package]\nname = \"beta\"\n").unwrap();

        // Directory with no Cargo.toml is skipped.
        fs::create_dir_all(root.join("blocks/empty")).unwrap();

        let found = discover(root).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found[0].ends_with("blocks/alpha"));
        assert!(found[1].ends_with("blocks/beta"));
    }
}
```

- [ ] **Step 5.2: Add module to main.rs**

At the top of `crates/solobase-cli/src/main.rs`:

```rust
mod skills;
```

- [ ] **Step 5.3: Run tests**

Run: `cargo test -p solobase-cli --lib skills::tests`
Expected: 2 pass.

- [ ] **Step 5.4: Add the build-all helper**

Append to `crates/solobase-cli/src/skills.rs` (above `#[cfg(test)]`):

```rust
use std::process::Command;

/// For each discovered skill-block dir, run `wafer build` in that directory.
/// Fails fast on the first error with a `skill build failed: blocks/<name>`
/// prefix.
pub fn build_all(repo_root: &Path) -> anyhow::Result<()> {
    let blocks = discover(repo_root)?;
    for block_dir in blocks {
        let short = block_dir
            .strip_prefix(repo_root)
            .unwrap_or(&block_dir)
            .display();
        let step = format!("skill build failed: {short}");
        let mut cmd = Command::new("wafer");
        cmd.arg("build").current_dir(&block_dir);
        crate::cmd::run(&step, cmd)?;
    }
    Ok(())
}
```

- [ ] **Step 5.5: Verify compile**

Run: `cargo check -p solobase-cli`
Expected: clean.

- [ ] **Step 5.6: Commit**

```bash
git add crates/solobase-cli/src
git commit -m "feat(solobase-cli): skill-block auto-discovery + wafer build"
```

---

## Task 6: build.rs — pure arg construction

**Files:**
- Create: `crates/solobase-cli/src/build.rs`
- Modify: `crates/solobase-cli/src/main.rs` — add `mod build;`

Two pure functions that take a `&Config` + a `BuildProfile` enum (Dev/Release) and return the exact arg vectors we'll pass to `wasm-pack` and `export-assets`. Unit-tested without shelling out.

- [ ] **Step 6.1: Write the failing test**

Create `crates/solobase-cli/src/build.rs`:

```rust
use crate::config::Config;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Dev,
    Release,
}

/// Construct the `wasm-pack build` arg vector.
///
/// Example: ["build", "--target", "web", "--release", "--out-dir", "pkg"]
pub fn wasm_pack_args(cfg: &Config, profile: BuildProfile) -> Vec<String> {
    let mut out = vec!["build".into(), "--target".into(), "web".into()];
    match profile {
        BuildProfile::Release => out.push("--release".into()),
        BuildProfile::Dev => out.push("--dev".into()),
    }
    out.push("--out-dir".into());
    out.push(cfg.wasm.out_dir.clone());
    out
}

/// Construct the `cargo run -p solobase-browser --release --bin export-assets -- <args>`
/// arg vector (everything AFTER `cargo run ... --`). `dist_dir` is usually
/// the same as `cfg.wasm.out_dir`. `repo_root` is the absolute path to the
/// consumer repo root (the dir that contains `solobase.toml`).
pub fn export_assets_args(
    cfg: &Config,
    repo_root: &Path,
    dist_dir: &Path,
    profile: BuildProfile,
) -> Vec<String> {
    let mut out = vec![
        format!("{}/", dist_dir.display()),
        "--repo-dir".into(),
        repo_root.display().to_string(),
        "--app-name".into(),
        cfg.app.name.clone(),
        "--app-title".into(),
        cfg.app.title.clone(),
        "--boot-redirect".into(),
        cfg.app.boot_redirect.clone(),
    ];
    if !cfg.assets.extra_bypass_prefix.is_empty() {
        out.push("--extra-bypass-prefix".into());
        out.push(cfg.assets.extra_bypass_prefix.join(","));
    }
    if profile == BuildProfile::Dev {
        out.push("--dev".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn minimal_cfg() -> Config {
        crate::config::parse(
            r#"
[app]
name = "solobase-web"
title = "Solobase"
boot_redirect = "/b/system/"
"#,
        )
        .unwrap()
    }

    #[test]
    fn wasm_pack_release_args() {
        let cfg = minimal_cfg();
        let args = wasm_pack_args(&cfg, BuildProfile::Release);
        assert_eq!(
            args,
            vec![
                "build".to_string(),
                "--target".into(),
                "web".into(),
                "--release".into(),
                "--out-dir".into(),
                "pkg".into(),
            ]
        );
    }

    #[test]
    fn wasm_pack_dev_args() {
        let cfg = minimal_cfg();
        let args = wasm_pack_args(&cfg, BuildProfile::Dev);
        assert!(args.contains(&"--dev".to_string()));
        assert!(!args.contains(&"--release".to_string()));
    }

    #[test]
    fn export_assets_minimal() {
        let cfg = minimal_cfg();
        let repo = PathBuf::from("/repo");
        let dist = PathBuf::from("pkg");
        let args = export_assets_args(&cfg, &repo, &dist, BuildProfile::Release);
        assert_eq!(args[0], "pkg/");
        assert!(args.iter().any(|a| a == "--repo-dir"));
        assert!(args.iter().any(|a| a == "/repo"));
        assert!(args.iter().any(|a| a == "--app-name"));
        assert!(args.iter().any(|a| a == "solobase-web"));
        assert!(args.iter().any(|a| a == "--boot-redirect"));
        assert!(args.iter().any(|a| a == "/b/system/"));
        assert!(!args.iter().any(|a| a == "--extra-bypass-prefix"));
        assert!(!args.iter().any(|a| a == "--dev"));
    }

    #[test]
    fn export_assets_with_bypass_and_dev() {
        let cfg: Config = crate::config::parse(
            r#"
[app]
name = "gizza-ai"
title = "Gizza AI"
boot_redirect = "/"

[assets]
extra_bypass_prefix = ["/gizza-app.js", "/gizza.css"]
"#,
        )
        .unwrap();
        let args = export_assets_args(&cfg, Path::new("/repo"), Path::new("dist"), BuildProfile::Dev);
        assert_eq!(args[0], "dist/");
        let bp_ix = args.iter().position(|a| a == "--extra-bypass-prefix").unwrap();
        assert_eq!(args[bp_ix + 1], "/gizza-app.js,/gizza.css");
        assert!(args.contains(&"--dev".to_string()));
    }
}
```

- [ ] **Step 6.2: Add module to main.rs**

At the top of `crates/solobase-cli/src/main.rs`:

```rust
mod build;
```

- [ ] **Step 6.3: Run tests**

Run: `cargo test -p solobase-cli --lib build::tests`
Expected: 4 pass.

- [ ] **Step 6.4: Commit**

```bash
git add crates/solobase-cli/src
git commit -m "feat(solobase-cli): pure arg-construction for wasm-pack + export-assets"
```

---

## Task 7: build.rs — pipeline

**Files:**
- Modify: `crates/solobase-cli/src/build.rs`

The pipeline calls skill discovery → `wasm-pack` → `cargo run -p solobase-browser --bin export-assets` → overlay copies. All errors surface via `cmd::run`. No unit tests here — the pipeline is tested end-to-end in Task 10.

- [ ] **Step 7.1: Add the pipeline**

Append to `crates/solobase-cli/src/build.rs` (above `#[cfg(test)]`):

```rust
use std::path::PathBuf;
use std::process::Command;

/// Run the full build pipeline for `cfg`. `repo_root` is the directory that
/// contains `solobase.toml`.
///
/// Steps:
/// 1. Skill-block auto-discovery — runs `wafer build` in each `blocks/*/`.
/// 2. `wasm-pack build ...`.
/// 3. `cargo run -p solobase-browser --release --bin export-assets -- ...`.
/// 4. Apply `[[assets.overlay]]` — copy each `from` → `<dist>/<to>`.
///
/// On success prints a one-line summary to stdout.
pub fn run(cfg: &Config, repo_root: &PathBuf, profile: BuildProfile) -> anyhow::Result<()> {
    // 1. Skill blocks.
    crate::skills::build_all(repo_root)?;

    // 2. wasm-pack.
    let mut wp = Command::new("wasm-pack");
    wp.args(wasm_pack_args(cfg, profile)).current_dir(repo_root);
    crate::cmd::run("wasm-pack build", wp)?;

    // 3. export-assets.
    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    let mut ea = Command::new("cargo");
    ea.args([
        "run",
        "-p",
        "solobase-browser",
        "--release",
        "--bin",
        "export-assets",
        "--",
    ])
    .args(export_assets_args(cfg, repo_root, &dist_dir, profile))
    .current_dir(repo_root);
    crate::cmd::run("export-assets", ea)?;

    // 4. Overlays.
    for overlay in &cfg.assets.overlay {
        let src = repo_root.join(&overlay.from);
        let dst = dist_dir.join(&overlay.to);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("create dir {parent:?}: {e}"))?;
        }
        std::fs::copy(&src, &dst)
            .map_err(|e| anyhow::anyhow!("overlay {src:?} → {dst:?}: {e}"))?;
    }

    // 5. Summary.
    let profile_label = match profile {
        BuildProfile::Dev => "dev",
        BuildProfile::Release => "release",
    };
    println!(
        "built {} ({}) → {}",
        cfg.app.name,
        profile_label,
        cfg.wasm.out_dir
    );
    Ok(())
}
```

- [ ] **Step 7.2: Verify compile**

Run: `cargo check -p solobase-cli`
Expected: clean.

- [ ] **Step 7.3: Commit**

```bash
git add crates/solobase-cli/src/build.rs
git commit -m "feat(solobase-cli): build pipeline composes skills + wasm-pack + export-assets + overlays"
```

---

## Task 8: serve.rs — serve subcommand

**Files:**
- Create: `crates/solobase-cli/src/serve.rs`
- Modify: `crates/solobase-cli/src/main.rs` — add `mod serve;`

Runs a dev build, then shells out to `python3 -m http.server <port> -d <dist>`.

- [ ] **Step 8.1: Add serve.rs**

Create `crates/solobase-cli/src/serve.rs`:

```rust
use crate::build::BuildProfile;
use crate::config::Config;
use std::path::PathBuf;
use std::process::Command;

pub fn run(cfg: &Config, repo_root: &PathBuf, port: u16) -> anyhow::Result<()> {
    // Dev build first.
    crate::build::run(cfg, repo_root, BuildProfile::Dev)?;

    let dist_dir = repo_root.join(&cfg.wasm.out_dir);
    println!(
        "serving {} on http://localhost:{}",
        dist_dir.display(),
        port
    );

    let mut cmd = Command::new("python3");
    cmd.arg("-m")
        .arg("http.server")
        .arg(port.to_string())
        .arg("-d")
        .arg(dist_dir);
    // http.server only exits on signal — this call blocks until the user
    // hits Ctrl+C, which exit-codes 130. Treat that as success.
    match cmd.status() {
        Ok(status) if status.success() || status.code() == Some(130) => Ok(()),
        Ok(status) => Err(anyhow::anyhow!(
            "python3 -m http.server exited {:?}",
            status.code()
        )),
        Err(e) => Err(anyhow::anyhow!("spawn python3: {e}")),
    }
}
```

- [ ] **Step 8.2: Add module to main.rs**

At the top of `crates/solobase-cli/src/main.rs`:

```rust
mod serve;
```

- [ ] **Step 8.3: Verify compile**

Run: `cargo check -p solobase-cli`
Expected: clean.

- [ ] **Step 8.4: Commit**

```bash
git add crates/solobase-cli/src
git commit -m "feat(solobase-cli): serve subcommand (dev build + http.server)"
```

---

## Task 9: main.rs — clap wiring

**Files:**
- Modify: `crates/solobase-cli/src/main.rs`

Replace the stub `main` with a clap parser that dispatches to the three subcommands. Config is loaded once from cwd for every subcommand.

- [ ] **Step 9.1: Write main.rs**

Replace the contents of `crates/solobase-cli/src/main.rs` with:

```rust
mod build;
mod cmd;
mod config;
mod serve;
mod skills;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "solobase", about = "Build / dev / serve for solobase-browser consumers", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the consumer app. Release profile by default.
    Build {
        /// Use the release profile (default). Mutually exclusive with --dev.
        #[arg(long, default_value_t = false)]
        release: bool,
        /// Use the dev profile (skips wasm-opt + content-hashing).
        #[arg(long, default_value_t = false, conflicts_with = "release")]
        dev: bool,
    },
    /// Alias for `build --dev`.
    Dev,
    /// Build (dev) then serve `dist_dir` over http.
    Serve {
        /// TCP port. Defaults to 8080.
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    let (cfg, repo_root) = config::find_and_load(&cwd)?;
    match cli.command {
        Commands::Build { release: _, dev } => {
            let profile = if dev {
                build::BuildProfile::Dev
            } else {
                build::BuildProfile::Release
            };
            build::run(&cfg, &repo_root, profile)?;
        }
        Commands::Dev => build::run(&cfg, &repo_root, build::BuildProfile::Dev)?,
        Commands::Serve { port } => serve::run(&cfg, &repo_root, port)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}
```

- [ ] **Step 9.2: Verify compile**

Run: `cargo check -p solobase-cli`
Expected: clean.

- [ ] **Step 9.3: Sanity-check CLI help output**

Run: `cargo run -p solobase-cli -- --help`
Expected: output lists `build`, `dev`, `serve` subcommands. Non-interactive assertion is the next task; for now just eyeball it.

- [ ] **Step 9.4: Commit**

```bash
git add crates/solobase-cli/src/main.rs
git commit -m "feat(solobase-cli): wire clap parser with build/dev/serve"
```

---

## Task 10: Integration smoke

**Files:**
- Create: `crates/solobase-cli/tests/integration_smoke.rs`
- Create: `crates/solobase-cli/tests/fixtures/solobase-web-style/solobase.toml`
- Create: `crates/solobase-cli/tests/fixtures/gizza-ai-style/solobase.toml`
- Create: `crates/solobase-cli/tests/fixtures/gizza-ai-style/site/index.html`
- Create: `crates/solobase-cli/tests/fixtures/gizza-ai-style/site/gizza-app.js`
- Create: `crates/solobase-cli/tests/fixtures/gizza-ai-style/blocks/example/Cargo.toml`

The integration test doesn't execute a full browser build (would require `wasm-pack` + a real wasm crate). It exercises the CLI's **parse + find + arg-construction** path by running `solobase build --dev` with a `SOLOBASE_CLI_DRY_RUN=1` env var that short-circuits right before spawning children.

We add the dry-run guard to the pipeline step to make this test possible without spawning `wasm-pack` / `cargo run` / `wafer`.

- [ ] **Step 10.1: Add the dry-run guard**

Modify `crates/solobase-cli/src/build.rs`'s `run` function. Immediately after printing the summary, leave it as-is. Add a check at the TOP of `run` that prints the resolved configuration when `SOLOBASE_CLI_DRY_RUN=1` and returns early:

Replace the top of the `pub fn run(...)` body with:

```rust
pub fn run(cfg: &Config, repo_root: &PathBuf, profile: BuildProfile) -> anyhow::Result<()> {
    if std::env::var("SOLOBASE_CLI_DRY_RUN").as_deref() == Ok("1") {
        // Emit a machine-readable summary instead of spawning children.
        let skills = crate::skills::discover(repo_root)?;
        let wp = wasm_pack_args(cfg, profile);
        let dist_dir = repo_root.join(&cfg.wasm.out_dir);
        let ea = export_assets_args(cfg, repo_root, &dist_dir, profile);
        let overlays: Vec<String> = cfg
            .assets
            .overlay
            .iter()
            .map(|o| format!("{}->{}", o.from, o.to))
            .collect();
        println!(
            "DRY_RUN\napp={}\nprofile={:?}\nskills={}\nwasm_pack={:?}\nexport_assets={:?}\noverlays={:?}",
            cfg.app.name, profile, skills.len(), wp, ea, overlays
        );
        return Ok(());
    }

    // 1. Skill blocks.
    crate::skills::build_all(repo_root)?;
    // ... existing code ...
```

Keep the rest of the function unchanged.

- [ ] **Step 10.2: Create the solobase-web fixture**

Create `crates/solobase-cli/tests/fixtures/solobase-web-style/solobase.toml`:

```toml
[app]
name = "solobase-web"
title = "Solobase"
boot_redirect = "/b/system/"
```

- [ ] **Step 10.3: Create the gizza-ai fixture**

Create `crates/solobase-cli/tests/fixtures/gizza-ai-style/solobase.toml`:

```toml
[app]
name = "gizza-ai"
title = "Gizza AI"
boot_redirect = "/"

[assets]
extra_bypass_prefix = ["/gizza-app.js", "/gizza.css"]

[[assets.overlay]]
from = "site/index.html"
to = "index.html"

[[assets.overlay]]
from = "site/gizza-app.js"
to = "gizza-app.js"
```

Create `crates/solobase-cli/tests/fixtures/gizza-ai-style/site/index.html`:

```html
<!DOCTYPE html><html><body>gizza</body></html>
```

Create `crates/solobase-cli/tests/fixtures/gizza-ai-style/site/gizza-app.js`:

```js
// fixture
```

Create `crates/solobase-cli/tests/fixtures/gizza-ai-style/blocks/example/Cargo.toml`:

```toml
[package]
name = "example"
version = "0.1.0"
```

(Real crate is not built in dry-run; we only check that `discover` finds it.)

- [ ] **Step 10.4: Write the integration test**

Create `crates/solobase-cli/tests/integration_smoke.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

fn cli_path() -> PathBuf {
    // `CARGO_BIN_EXE_solobase` is set by Cargo for tests in crates with a
    // bin of that name.
    PathBuf::from(env!("CARGO_BIN_EXE_solobase"))
}

fn run_in(fixture_dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let bin = cli_path();
    let output = Command::new(&bin)
        .args(args)
        .current_dir(fixture_dir)
        .env("SOLOBASE_CLI_DRY_RUN", "1")
        .output()
        .expect("spawn solobase");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code(),
    )
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn solobase_web_style_dry_run() {
    let (stdout, _stderr, code) = run_in(&fixture("solobase-web-style"), &["build"]);
    assert_eq!(code, Some(0), "expected success, got {code:?}");
    assert!(stdout.contains("DRY_RUN"));
    assert!(stdout.contains("app=solobase-web"));
    assert!(stdout.contains("skills=0"));
    assert!(stdout.contains("overlays=[]"));
}

#[test]
fn gizza_ai_style_dry_run() {
    let (stdout, stderr, code) = run_in(&fixture("gizza-ai-style"), &["dev"]);
    assert_eq!(code, Some(0), "expected success, got {code:?}\nstderr={stderr}");
    assert!(stdout.contains("DRY_RUN"));
    assert!(stdout.contains("app=gizza-ai"));
    assert!(stdout.contains("skills=1")); // discovers blocks/example
    // Two overlays, separated by commas in the Vec debug output.
    assert!(stdout.contains("site/index.html->index.html"));
    assert!(stdout.contains("site/gizza-app.js->gizza-app.js"));
    assert!(stdout.contains("extra-bypass-prefix"));
}

#[test]
fn missing_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let (_stdout, stderr, code) = run_in(tmp.path(), &["build"]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("no solobase.toml found"));
}
```

- [ ] **Step 10.5: Run the integration test**

Run: `cargo test -p solobase-cli --test integration_smoke`
Expected: 3 pass.

- [ ] **Step 10.6: Commit**

```bash
git add crates/solobase-cli
git commit -m "test(solobase-cli): integration-smoke tests for build/dev + fixtures"
```

---

## Task 11: Verify full test suite

**Files:** none — verification only.

- [ ] **Step 11.1: Run the full suite**

Run: `cargo test -p solobase-cli`
Expected: all unit + integration tests pass.

- [ ] **Step 11.2: Run `cargo clippy` for the crate**

Run: `cargo clippy -p solobase-cli -- -D warnings`
Expected: zero warnings.

- [ ] **Step 11.3: If clippy reports issues, fix them and commit**

If clippy is clean, this step is a no-op. If not, apply its suggestions and commit with `chore(solobase-cli): clippy fixes`.

---

## Task 12: Open the PR

**Files:** none — git operations only.

- [ ] **Step 12.1: Push the branch**

```bash
git push -u origin feat/solobase-cli
```

- [ ] **Step 12.2: Open the PR**

```bash
gh pr create --repo suppers-ai/solobase --base main --head feat/solobase-cli \
  --title "feat: solobase-cli — unified build/dev/serve for solobase-browser consumers (Phase E)" \
  --body "$(cat <<'EOF'
## Summary
Adds a new `crates/solobase-cli` crate producing a `solobase` binary. Replaces the per-consumer Makefile/justfile boilerplate with `solobase build` / `solobase dev` / `solobase serve`, driven by a per-consumer `solobase.toml` at repo root.

Consumer migrations (solobase-web Makefile, gizza-ai justfile) happen in follow-up PRs; this PR only lands the CLI.

### Pipeline
1. Skill-block auto-discovery — runs \`wafer build\` in each \`blocks/*/\` crate found under the consumer repo.
2. \`wasm-pack build --target web --[release|dev] --out-dir <wasm.out_dir>\`.
3. \`cargo run -p solobase-browser --release --bin export-assets -- <dist>/ --repo-dir <repo> --app-name <app> --app-title <title> --boot-redirect <path> [--extra-bypass-prefix ...] [--dev]\`.
4. \`[[assets.overlay]]\` — copy consumer-specific files into \`dist/\` after framework defaults.

### Test plan
- [x] \`cargo test -p solobase-cli\` (unit + integration; dry-run via \`SOLOBASE_CLI_DRY_RUN=1\`)
- [x] \`cargo clippy -p solobase-cli -- -D warnings\`
- [ ] Manual: \`cargo install --path crates/solobase-cli\` then run \`solobase build\` in a fresh solobase-web tree against a temporary solobase.toml; full wasm-pack pipeline exercised.

### Non-goals (explicit)
- Migrations of existing consumers (separate PRs).
- File watcher / live reload.
- crates.io publish.
- Subsuming native \`cargo build\` for solobase-server.

Spec: \`docs/superpowers/specs/2026-04-21-solobase-cli-design.md\`

🤖 Generated with [Claude Code](https://claude.ai/code)
EOF
)"
```

- [ ] **Step 12.3: Record the PR URL**

Note the URL printed by `gh pr create`. Share with the human so they can review.

---

## Post-merge: consumer migrations (separate plans)

Once this PR is merged, two small follow-up PRs land the consumer migrations:

1. **solobase-web**: delete `Makefile`, add `solobase.toml`. Update CI to run `solobase dev` / `solobase build` instead of `make dev` / `make build`.
2. **gizza-ai**: delete `justfile` (or trim to a `test` rule), add `solobase.toml`. Update CI likewise.

Each migration is trivial — one file replaced, a few CI lines updated. They don't need their own plan documents; the consumer migration section in the spec is sufficient guidance.
