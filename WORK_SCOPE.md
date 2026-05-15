# Wave 2 Group F — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-F` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

Cross-cutting helpers landed in Waves 0 and 1 — use them, do **not** reinvent:

- `crate::blocks::auth::helpers::sha256_hex` (TEXT-column secret-token hashing)
- `solobase-native::env::filter_app_env_vars` (keeps keys containing `__`)
- `wafer_run` already exposes sorted `blocks_snapshot` and `Runtime::registered_blocks`
- `wafer-core` has `ChatChunk::{tool_call_start, tool_call_arguments, tool_call_complete, usage}` and `TokenUsage::new`/`with_cached`/`with_reasoning`
- `wafer-sql-utils` has `AggFunc::Coalesce(default)`
- `cli::server::run` now takes `run_migrations: bool` explicitly (no more `std::env::set_var` smuggle)

Some findings may have been silently fixed by intervening merges (PRs #124-#133 in solobase main migrated many `db::list` callsites to `db::list_sorted` / `db::list_all`). If a finding's premise no longer holds against current `main`, note "Already fixed by PR #N" in the PR body and skip.

Project conventions (from `solobase/CLAUDE.md` and workspace `CLAUDE.md`):
- No `panic!` / `unwrap` / `expect` in production paths (tests OK).
- No `poll_once` / `block_on` sync bridges. If something is async, callers stay async.
- No raw SQL (`exec_raw` / `query_raw`) outside the documented exceptions (admin SQL explorer, migration runners, test fixtures).
- No hardcoded domain values — use `ConfigVar`.
- Comments only where the *why* is non-obvious. Don't write what the code already says.
- `// TODO` without a linked issue is a smell; add a `(#NNN)` reference if you can find one.

Process:

1. Read this whole file first, then read the touched files. Cluster nearby findings (same file or same module) into one logical commit. **One commit per cluster, not one per finding.**
2. After each cluster commit, run:
   ```
   cargo check -p <touched-crate>
   cargo clippy --workspace --exclude solobase-web --exclude solobase-cloudflare 2>&1 | grep '^error' | head
   ```
   Fix any new errors before moving on. CI runs `cargo clippy --workspace --exclude solobase-web --exclude solobase-cloudflare` without `-D warnings`, so warnings don't block but errors do.
3. Run **`cargo +nightly fmt --all`** before every commit. The CI Format & Lint job runs `cargo +nightly fmt -- --check`; stable `rustfmt` doesn't catch all rules (memory: `wafer-run-nightly-fmt`).
4. When the scope is complete:
   - `cargo test -p <each touched crate>` — green.
   - `cargo clippy --workspace --exclude solobase-web --exclude solobase-cloudflare` — no new errors from this PR (some pre-existing warnings exist; don't worry about those).
   - `cargo +nightly fmt --all -- --check` — clean.
   - For native compilation: `crates/solobase-web/pkg/solobase_web_bg.wasm` must exist as a stub or the build fails. `touch crates/solobase-web/pkg/solobase_web_bg.wasm` if missing. **Do not commit this stub** (memory: `solobase-web-wasm-build-broken`).
5. Push: `git push -u origin rbp/wave2-F` then `gh pr create --title "fix(rbp-F): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group F of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

## Summary
- <1-3 bullet points describing the shape of the changes>

## Findings addressed
- <line/file>: <one-line description> — <commit short-sha>
- ...

## Deferred to follow-up
- <Medium/Low items not in this PR>
- <Cross-group items: "Deferred to group X">
- <Already fixed by PR #N>

## Test plan
- [ ] `cargo test -p <crate>` green
- [ ] `cargo clippy --workspace --exclude solobase-web --exclude solobase-cloudflare` no new errors
- [ ] `cargo +nightly fmt --all -- --check` clean

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

Constraints:
- Stay inside your worktree. Do not touch files outside the crate(s) named in your scope.
- If a finding cross-references code outside this group (e.g. consolidating with a helper in group A), skip it and note "Deferred to group X" in the PR body.
- Do NOT amend commits or force-push. Always create new commits.
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-F` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase + solobase-native

### Critical
- `crates/solobase/src/cli/server.rs:84` — `.build().expect(...)` panics deep in boot if WAFER fails to build. **Fix:** propagate via `?` — `e.context("failed to build solobase runtime")`.
- `crates/solobase/src/cli/server.rs:105` — `wafer.start().await.expect(...)` on a recoverable startup failure. **Fix:** `.await.context("failed to start WAFER runtime")?`.
- `crates/solobase/src/cli/server.rs:169,191,196,215-218` — `stmt.prepare(...).expect(...)` panics if the DB is locked / schema mismatched during boot. **Fix:** return `anyhow::Result` from `seed_and_load_variables` / `seed_auto_generated`.
- `crates/solobase/src/cli/server.rs:226` — `getrandom::getrandom(&mut bytes).expect(...)` panics during boot if entropy source unavailable. **Fix:** propagate via `?`.
- `crates/solobase-native/src/database.rs:18,30` and `storage.rs:10,40` — `make_sqlite_database_service`, `make_postgres_database_service`, `make_local_storage_service`, `make_s3_storage_service` all panic on failure from library crates. **Fix:** return `Result<Arc<dyn Service>, _>`.
- `crates/solobase-native/src/serve.rs:46,53` — `tokio::signal::ctrl_c().await.expect(...)` in a library helper. A failed signal install makes the process unkillable. **Fix:** propagate; make `serve_until_shutdown` return `Result<()>`.
- `crates/solobase/src/main.rs:59,70` — `std::env::set_var("SOLOBASE_RUN_MIGRATIONS", "1")` smuggles a CLI flag through process env. Violates "no magic / implicit mapping" rule; becomes unsafe in Rust 2024. **Fix:** thread `run_migrations: bool` explicitly through dispatch → server::run.

### High
- `crates/solobase/src/cli/server.rs:48-51` — `vars.get(...).cloned().unwrap_or_default()` silently substitutes an empty JWT secret. Security-relevant fail-open. **Fix:** bail with context.
- `crates/solobase/src/cli/server_config.rs:67` — `.prepare(...).unwrap()` in `load_block_settings`. **Fix:** be consistent with the function's tolerant style elsewhere.
- `crates/solobase/src/cli/helpers/http_server.rs:18` — Fixed 1024-byte read buffer; typical browser request line + Cookie header exceeds this. **Fix:** `BufReader::read_until` until `\r\n\r\n`.
- `crates/solobase/src/cli/helpers/http_server.rs:30-33` — `dir.join(path.trim_start_matches('/'))` accepts `..`-paths. Path-traversal in the dev tool. **Fix:** reject `..` components or canonicalize and verify `starts_with(dir)`.
- `crates/solobase/src/cli/server.rs:32` — `std::env::args_os().count() == 1` invokes `Cli::default()` which ignores verb-level flags. UX foot-gun. **Fix:** parse `["solobase","serve"]`.
- `crates/solobase/src/main.rs:45` — `std::env::set_current_dir(&ctx.cwd)?` mutates global process state. **Fix:** plumb `repo_root` into `load_dotenv` explicitly.
- `crates/solobase-native/src/env.rs:38-44` — `filter_app_env_vars` strips every `SOLOBASE_*` prefix, including `SOLOBASE_SHARED__*` which is **app config** per CLAUDE.md. _This is the underlying bug that PR #155 worked around in `auto_bootstrap_if_needed`._ **Fix:** keep `SOLOBASE_SHARED__*` (or drop only infra keys = those without `__`).
- `crates/solobase-native/src/log_init.rs:46` — `.expect("failed to create OTLP span exporter")` — OTLP misconfig at boot crashes hard. **Fix:** return `Result<()>`.

### Medium
- `crates/solobase/src/cli/server.rs:67-84` — `SolobaseBuilder::new()...build()` chain calls factories that each independently panic. Fold into single `?`-bubbled chain once factories return `Result`.
- `crates/solobase/src/cli/cmd.rs:18-22` — `Vec<String>` allocation per arg just to format dry-run line. **Fix:** `write!` directly.
- `crates/solobase/src/cli/helpers/cloudflare/env.rs:67-119` — Six identical 4-arg `env_or` calls. **Fix:** `&[(&str,&str)]` table + `for` loop.
- `crates/solobase/src/cli/config.rs:80-107` — `find_and_load` returns `anyhow::Result` from library code. Prefer `thiserror` for matchable variants.
- `crates/solobase/src/cli/helpers/blocks.rs:33` — Build-loop swallows inner `cmd::run` errors with limited context.
- `crates/solobase/src/cli/helpers/cloudflare/deploy.rs:23-45` — `walk_files` does subprocess work inside recursion. Restructure for future retry/concurrency.
- `crates/solobase/src/cli/server.rs:144-158` — Raw SQL `CREATE TABLE IF NOT EXISTS variables` duplicates what `wafer-block-sqlite` defines. **Fix:** delegate to migration file or shared schema helper.
- `crates/solobase/src/cli/flows/embed_native.rs:73` — `std::process::exit(...)` inside tokio runtime bypasses drop of the runtime. **Fix:** propagate via `Result` / `ExitCode`.
- `crates/solobase/src/cli/flows/embed_native.rs:71` — Blocking `std::process::Command::spawn` inside `async fn`. **Fix:** `tokio::process::Command`.
- `crates/solobase/src/cli/flows/embed_cloudflare.rs:88,99` — Same blocking `Command::status()` for `wrangler dev` (long-running, freezes tokio thread).
- `crates/solobase/src/cli/helpers/cloudflare/build.rs:14-24` — Same pattern for `cargo build`.
- `crates/solobase/src/cli/server_config.rs:15` — `HashSet<String>` built for membership checks. **Fix:** `HashSet<&str>`.
- `crates/solobase/src/cli/helpers/cloudflare/wrangler.rs:121-136` — `deep_merge` locked to `toml::Value`; consider generic over Value-like type for testability.

### Low
- `crates/solobase/src/cli/helpers/cloudflare/env.rs:50` — `pub fn parse`, `pub fn load`, `pub fn require_api_token` lack `# Errors` doc sections.
- `crates/solobase-native/src/database.rs:14` — Doc admits panics; library best practice forbids.
- `crates/solobase/src/cli/helpers/http_server.rs:9` — `pub async fn serve_static` lacks doc + `# Errors`.
- `crates/solobase/src/cli/server_config.rs:33` — `pub type BlockDefault` undocumented except in code comment.
- `crates/solobase-native/src/serve.rs:24` — `register_http_listener` silently fails if called after start. Consider `Result<(), AlreadyStartedError>`.
- `crates/solobase/src/cli/server.rs:198-202` — `if !key.is_empty()` guard hides real DB-corruption case.
- `crates/solobase/src/cli/helpers/wasm.rs:11-21` — `is_file()` follows symlinks; intentional but worth noting in doc.

---
