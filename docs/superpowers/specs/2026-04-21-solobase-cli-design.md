# Solobase CLI — Design

**Date:** 2026-04-21
**Status:** Draft
**Phase:** E (follows Phase D LLM service extraction)
**Repo affected:** `solobase` (new crate); downstream consumers `solobase-web` and `gizza-ai` migrate off Makefile/justfile

## Goal

Consolidate the three variants of browser-app build glue into one CLI. Today every solobase-browser consumer hand-rolls a Makefile or justfile that invokes the same sequence: `wasm-pack build` → `cargo run -p solobase-browser --bin export-assets` → optional file overlays → optional `python3 -m http.server` for serve. Each consumer also threads app-specific args (`--app-name`, `--app-title`, `--boot-redirect`, `--extra-bypass-prefix`) through those invocations by hand.

After Phase E:

- Consumers run `solobase build`, `solobase dev`, or `solobase serve`.
- Per-consumer invariants live in a single `solobase.toml` at repo root.
- No Makefile, no justfile. Native builds (`solobase-server`) stay on `cargo build` — out of scope.

## Architecture

A new crate `crates/solobase-cli` in the solobase workspace, producing a single `[[bin]]` named `solobase`. The CLI is a thin wrapper that shells out to `wasm-pack`, `cargo run -p solobase-browser --bin export-assets`, `wafer` (for skill blocks), and `python3 -m http.server`. It does not reimplement any of those tools; when upstream adds a flag we pass it through rather than mirror it.

### Crate layout

```
crates/solobase-cli/
  Cargo.toml
  src/
    main.rs           # clap parser + subcommand dispatch
    config.rs         # solobase.toml loader + validation
    build.rs          # `build` subcommand pipeline
    dev.rs            # thin wrapper: `build --dev`
    serve.rs          # `serve` subcommand: build + http.server
    skills.rs         # skill-block auto-discovery + wafer delegation
    cmd.rs            # std::process::Command helpers + error wrapping
  tests/
    config_toml.rs    # parser unit + rejection cases
    build_args.rs     # pure arg-construction functions
    skills_discovery.rs  # temp-dir synthesized blocks
    integration_smoke.rs # per-consumer fixture → dist/ snapshot
```

### Distribution

**Phase E:** locally installable via `cargo install --path crates/solobase-cli`. CI adds the same install step. No crates.io publish; that's a follow-up when we have external consumers.

**Not in the library.** `solobase-cli` does not expose a library surface. It is a binary-only crate, so its deps (clap, toml, anyhow, glob) stay out of the `solobase` library's dependency graph.

## Configuration: `solobase.toml`

One file per consumer, at repo root. Resolved by walking up from cwd to the first enclosing dir that contains it — same as `Cargo.toml`.

```toml
# Required.
[app]
name = "solobase-web"           # → export-assets --app-name
title = "Solobase"              # → export-assets --app-title
boot_redirect = "/b/system/"    # → export-assets --boot-redirect

# Optional.
[assets]
extra_bypass_prefix = ["/foo.js", "/bar.css"]   # → export-assets --extra-bypass-prefix (comma-joined)

# Optional — copied into dist/ AFTER export-assets, overwriting framework defaults.
[[assets.overlay]]
from = "site/index.html"        # relative to consumer repo root
to   = "index.html"             # relative to <dist_dir>

# Optional — wasm-pack tuning.
[wasm]
out_dir = "pkg"                 # default "pkg"
```

### Rules

- `solobase.toml` not found by walking up from cwd → error: `no solobase.toml found in <cwd> or any parent directory`. Exit code 2.
- Missing `[app]` → error with a pointer to the field. Exit code 2.
- Missing `[assets]` / `[wasm]` → those features simply no-op.
- Unknown top-level keys or table names → warning printed to stderr, not error. Unknown keys inside a known table (e.g., `[app].color`) are strict errors (typos should fail CI).
- No env-var substitution, no includes, no conditionals. If those become needed, revisit in a follow-up spec.

### Not in the config

- `--dev` / `--release` — CLI flag, not config (developers pick per invocation).
- `--repo-dir` — derived from the workspace root walk.
- Skill-block paths — auto-discovered from `blocks/*/Cargo.toml`.

## Subcommands

### `solobase build [--release]`

Pipeline:

1. Walk up from cwd to find `solobase.toml`; load + validate.
2. **Skill discovery.** If `blocks/*/Cargo.toml` exists, iterate each and run `wafer build` in that directory. Fail fast on any child error with a one-line `skill build failed: blocks/<name>` plus the child's stderr. Skip this step if no `blocks/` directory.
3. `wasm-pack build --target web {--release | --dev} --out-dir <wasm.out_dir>`.
4. `cargo run -p solobase-browser --release --bin export-assets -- <dist_dir>/ --repo-dir <repo_root> --app-name <app.name> --app-title <app.title> --boot-redirect <app.boot_redirect> [--extra-bypass-prefix a,b,c] [--dev]`.
5. For each `[[assets.overlay]]` entry, copy `<from>` → `<dist_dir>/<to>`. Overwrites whatever export-assets wrote.
6. Print a one-line summary, e.g. `built solobase-web (release) → pkg/`.

`<dist_dir>` = `<wasm.out_dir>` in v1. Consumers who want a separate assembly path can request an `[output].dir` field in a follow-up.

### `solobase dev`

Alias for `solobase build --dev`. Same pipeline with the dev profile, which skips wasm-opt and content-hashing.

### `solobase serve [--port 8080]`

Runs `build --dev`, then `python3 -m http.server <port> -d <dist_dir>`. No file watcher in v1 — if you want live rebuild run `solobase dev` in one terminal and `solobase serve` in another. Built-in watching is a v2 decision.

## Error handling

Every child-process failure aborts the pipeline immediately. The CLI prints exactly:

```
error: <step> failed
  command: <arg0> <arg1> ...
  exit code: <n>
  --- stderr ---
  <child stderr>
```

No partial continuation, no retries, no cleanup of half-built `dist/`. The CLI exits with the child's exit code so CI detects the failure correctly.

Config errors (`solobase.toml` parse failures, missing `[app]`, unknown strict keys) print with a file:line pointer from the TOML error and exit with code 2.

## Testing

### Unit tests (fast, in-process)

- **`config.rs`**: valid load, rejection cases (missing `[app]`, unknown keys inside a known table, bad glob, relative paths in `overlay.from`).
- **`build.rs`**: pure command-string construction — given a config + dev/release, produce the exact arg vectors for `wasm-pack` and `export-assets`. No shell-out; tests verify the args. This is where regressions from upstream flag changes will surface first.
- **`skills.rs`**: skill-block discovery against temp-dir fixtures with synthetic `blocks/<name>/Cargo.toml` files.

### Integration smoke (per-consumer)

One integration test per consumer in `crates/solobase-cli/tests/integration_smoke.rs`. Each sets up a tmpdir with a fixture `solobase.toml`, runs the CLI in-process (via `std::process::Command` on the built binary), and asserts the produced `dist/` contains the expected set of filenames (not contents — wasm-pack output varies by toolchain). Two test cases:

- solobase-web-style (no overlays, no extra-bypass, no skills).
- gizza-ai-style (overlays, extra-bypass, synthetic skill block).

### No end-to-end browser test in this crate

Playwright suites in `solobase-web` and `gizza-ai` cover runtime behavior; duplicating them here is out of scope.

## Consumer migration

### solobase-web

- Delete `crates/solobase-web/Makefile`.
- Add `crates/solobase-web/solobase.toml`:
  ```toml
  [app]
  name = "solobase-web"
  title = "Solobase"
  boot_redirect = "/b/system/"
  ```
- CI step changes from `make dev` / `make build` to `solobase dev` / `solobase build`.

### gizza-ai

- Delete `justfile` (or keep only the `test` rule that invokes Playwright).
- Add `solobase.toml`:
  ```toml
  [app]
  name = "gizza-ai"
  title = "Gizza AI"
  boot_redirect = "/"

  [assets]
  extra_bypass_prefix = ["/gizza-app.js", "/gizza.css"]

  [[assets.overlay]]
  from = "site/index.html"
  to   = "index.html"

  [[assets.overlay]]
  from = "site/gizza-app.js"
  to   = "gizza-app.js"

  [[assets.overlay]]
  from = "site/gizza.css"
  to   = "gizza.css"
  ```
- Skill-block pre-build handled by CLI auto-discovery; no config entry needed.

### solobase-server

Out of scope. `cargo build -p solobase-server` continues as today; `solobase-cli` is browser-only in v1.

## PR ordering

1. **Solobase PR.** Lands `solobase-cli` crate + tests. No consumer changes yet — the existing Makefile/justfile in each consumer keep working until they're migrated.
2. **solobase-web migration PR** (solobase repo). Deletes Makefile, adds `solobase.toml`, updates CI.
3. **gizza-ai migration PR** (separate repo). Deletes justfile (or trims to `test`), adds `solobase.toml`, updates CI.

Each migration is independent; neither blocks the other.

## Non-goals (explicit)

- No file watcher / live reload (`solobase serve` runs one build then static-serves).
- No native target subsumed (`cargo build` stays for solobase-server).
- No crates.io publish in this phase.
- No parallel coexistence with Makefile/justfile; migrated consumers delete the old files.
- No env-var substitution or includes in `solobase.toml`.
- No `solobase clean` / `solobase test` subcommands (YAGNI — the former is `rm -rf pkg/`, the latter is consumer-specific).

## Open questions

None at spec-writing time. All design decisions have a definite answer above.
