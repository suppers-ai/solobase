# Wave 2 Group A — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-A` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-A` then `gh pr create --title "fix(rbp-A): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group A of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-A` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-core: root modules

### Critical
- `crates/solobase-core/src/crypto.rs:160` — `.expect("HKDF expand")` in production path (`derive_block_jwt_key` called from request flow). **Fix:** propagate the error — return `Result<String, String>` and bubble via `?`.
- `crates/solobase-core/src/crypto.rs:81-88` — `jwt_sign` swallows `to_string` failure with `unwrap_or_default()` and signing failure by returning `String::new()`, producing a structurally broken token that the caller can't distinguish from success. **Fix:** change signature to `Result<String, String>` and propagate via `?`.
- `crates/solobase-core/src/ui/components.rs:491,497` — `pagination`/page math divides by `per_page` with no guard; `per_page == 0` triggers integer overflow (debug) / wrong output (release). **Fix:** clamp `per_page = per_page.max(1)` at function entry.

### High
- `crates/solobase-core/src/crypto.rs:73` — `chrono::Duration::seconds(expiry.as_secs() as i64)` silently narrows u64→i64 with no check. **Fix:** use `i64::try_from(expiry.as_secs()).map_err(...)?` or clamp to `i64::MAX`.
- `crates/solobase-core/src/pipeline.rs:166` — `(now_millis() - start_ms) as i64` silently casts a u128/u64 subtraction; if `now_millis()` ever regresses (clock skew on suspend), this wraps. **Fix:** use `saturating_sub` and `try_into`.
- `crates/solobase-core/src/migration_helper.rs:144-167` — DB write failures (update/create/list) are downgraded to `warn!` and the function returns `Ok(())`, so the next request will re-apply the migration but never report the persistence failure to the caller. **Fix:** at least bubble `Err` from `list`/`update`/`create` when the table exists; the "fresh-install / table missing" case stays best-effort, real DB errors should propagate.
- `crates/solobase-core/src/features.rs:101-103` — `to_config_json` swallows serialization failure to `"{}"`, which silently loses migration-gate state on transport. **Fix:** return `Result<String, serde_json::Error>` (serializing `HashMap<String, BlockState>` can't realistically fail, so `.expect` is also acceptable here — pick one and stop hiding).
- `crates/solobase-core/src/routing.rs:233,243,307` — `format!("suppers-ai/{}", short_name)` allocates a `String` on every routing check and `routes_config` rebuild. Routing is per-request hot. **Fix:** return `&'static str` from a `block_id_full_name` helper using `concat!` or a `const` table.
- `crates/solobase-core/src/ui/sidebar.rs:71` — `current_path.starts_with(&format!("{}/", item.href))` allocates a `String` per nav item per request render. **Fix:** check `current_path.strip_prefix(&item.href).is_some_and(|r| r.starts_with('/') || r.is_empty())` — no allocation.

### Medium
- `crates/solobase-core/src/cache.rs:35,49,56` — `lock().expect("TtlCache poisoned")` in production path. Mutex poisoning is recoverable; for an isolate-level cache the right thing is `unwrap_or_else(|e| e.into_inner())`. **Fix:** swap to `into_inner()` on poison so a single panic in one fetcher doesn't permanently brick the cache.
- `crates/solobase-core/src/cache.rs:37` — `loaded_at.as_ref().map_or(false, |t| t.elapsed() < self.ttl)` — clippy `unnecessary_map_or` / readable `.is_some_and`. **Fix:** `loaded_at.is_some_and(|t| t.elapsed() < self.ttl)`.
- `crates/solobase-core/src/builder.rs:266` — `Arc::new("suppers-ai/admin".to_string())` allocates a String + Arc for a constant identifier. **Fix:** pass `Arc<str>` from a `const &'static str`, or change the consumer to take `&str`.
- `crates/solobase-core/src/crypto.rs:75` — `payload = claims.clone()` clones the whole HashMap to add 2 keys. **Fix:** take `claims: HashMap<...>` by value (the caller currently builds a fresh map every time anyway).
- `crates/solobase-core/src/crypto.rs:243` — `.unwrap_or("").to_string()` — building an owned `String` just to test `.is_empty()` later. **Fix:** keep as `&str` until you actually need to set meta.
- `crates/solobase-core/src/crypto.rs:256-265` — `roles` allocated as `String` even when only used for `set_meta(... &str)`. **Fix:** thread a `Cow<'_, str>` or just emit two branches setting meta directly without intermediate allocation.
- `crates/solobase-core/src/migration_helper.rs:99` — `BlockSettings::from_config_json` is called on every `apply_if_blessed` and re-parses the full JSON to read a single block's state. **Fix:** parse once at boot, or expose a `state_for(json, block_name)` helper that returns early without materializing all blocks.
- `crates/solobase-core/src/migration_helper.rs:178-205` — `split_statements` allocates a fresh `String` per statement and char-iterates a potentially-large embedded SQL on every block init. **Fix:** return `Vec<&str>` of byte-range slices.
- `crates/solobase-core/src/ui/mod.rs:289-291` — `format!(r#"{{"showToast":...}}"#, toast_message, toast_type)` injects unescaped strings into a JSON payload that lands in an HTTP header. **Fix:** call `serde_json::to_string` on the trigger object — a message containing `"` or `\` produces a malformed `HX-Trigger` and a possible header-injection vector.
- `crates/solobase-core/src/ui/components.rs:312-316` — `button()` returns `PreEscaped` of hand-built HTML with `extra_attrs` inserted verbatim. Caller-supplied attributes are not escaped. **Fix:** type the extra-attrs as a slice of `(name, value)` and escape values via `html_escape`, or rename to make the unsafety obvious (`button_unchecked`).
- `crates/solobase-core/src/ui/components.rs:345-351` — `html_escape` does 4 sequential allocating `replace` calls. **Fix:** single-pass escape into a `String::with_capacity`.
- `crates/solobase-core/src/ui/assets.rs:62` — `format!("{}\n{}\n{}\n{}\n{}\n", ...)` over five `&str` constants reallocates; called from `css()` then again from `css_url()`. **Fix:** wrap `css_bundle()` body in `static BUNDLE: OnceLock<String>`.
- `crates/solobase-core/src/features.rs:81-84` — `is_enabled` builds `format!("suppers-ai/{short_name}")` per call. **Fix:** require full names at the API boundary or precompute.
- `crates/solobase-core/src/builder.rs:140,202` — `extra_block`/`block_config` take `&str` then `to_string` it; callers almost always own a `String`. **Fix:** take `impl Into<String>` like `add_route` already does (consistency).

### Low
- `crates/solobase-core/src/crypto.rs:170-175` — `META_AUTH_JTI` / `META_AUTH_EXP` are `pub const` with no `///` doc explaining lifecycle.
- `crates/solobase-core/src/builder.rs:33-75` — `SolobaseBuilder` struct fields lack `///` docs; most setters do.
- `crates/solobase-core/src/pipeline.rs:30-39` — `pub async fn handle_request` has no `# Errors` / `# Panics` / `# Examples` section.
- `crates/solobase-core/src/ui/components.rs:262-296` — public enums `BtnVariant`/`CtrlSize`/`BadgeVariant` are `pub` without `#[non_exhaustive]`.
- `crates/solobase-core/src/migration_helper.rs:74,79` — `format!("ddl failed on `{trimmed}`: {e}")` could use `inspect_err` + `map_err` to surface tracing context.
- `crates/solobase-core/src/routing.rs:55-62` — `RouteAccess::Public` is a public enum without `#[non_exhaustive]`.
- `crates/solobase-core/src/ui/templates.rs:413-414` — `_components_keep_alive(_: components::BtnVariant)` with `#[allow(dead_code)]` is a code-smell shim; if `components` is genuinely unused here, drop the `use` and the function.
- `crates/solobase-core/src/builder.rs:246-251` — Six identical `.ok_or("database service required")?` patterns; compress to a helper or typed `BuilderError` enum.
- `crates/solobase-core/src/flows/mod.rs:15-17` — Doc comment claims `# Panics` but the function never panics; misleading.

---
