# Wave 2 Group G — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-G` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-G` then `gh pr create --title "fix(rbp-G): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group G of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-G` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-browser + solobase-web + solobase-cloudflare

### Critical
- `crates/solobase-cloudflare/src/convert.rs:35` — `req_clone.bytes().await.unwrap_or_default()` silently turns a body-read error into an empty body. POST/PUT requests silently corrupted. **Fix:** propagate via `?`.
- `crates/solobase-browser/src/database.rs:35` — `serde_json::to_string(params).unwrap_or_else(|_| "[]".to_string())` silently changes WHERE/SET semantics on serialization error. **Fix:** return `Result<String, DatabaseError>`.
- `crates/solobase-browser/src/database.rs:288-289` — `existing_columns` silently swallows `db_query_raw` + JSON parse failures. Callers issue `ALTER TABLE ADD COLUMN` for every column on next write → "duplicate column name" errors mask real failure. **Fix:** propagate.
- `crates/solobase-browser/src/network.rs:35-37` — `JSON::stringify(&js_val).map(|s| s.as_string().unwrap_or_default()).unwrap_or_default()` collapses two distinct failure modes into an empty string. **Fix:** return `NetworkError::RequestError` carrying the JS error message.
- `crates/solobase-cloudflare/src/network_service.rs:36-40` — `if let Ok(headers) = worker_req.headers_mut() { for ... { let _ = headers.set(k, v); } }` silently drops the entire header block on failure. **Fix:** unwrap `headers_mut()?` and `.set(k, v).map_err(...)?`.
- `crates/solobase-cloudflare/src/network_service.rs:48` — `resp.bytes().await.unwrap_or_default()` returns empty body on read error. **Fix:** `.map_err(|e| NetworkError::RequestError(format!("read body: {e}")))?`.

### High
- `crates/solobase-cloudflare/src/database.rs:111,133` — `.expect("schema cache poisoned")` on `std::sync::Mutex`. **Fix:** `.lock().unwrap_or_else(|p| p.into_inner())`.
- `crates/solobase-browser/src/vector/service.rs:84,104` — Same rule violation on `Mutex`. **Fix:** `unwrap_or_else(|p| p.into_inner())`, or `RefCell` (single-threaded wasm).
- `crates/solobase-browser/src/vector/service.rs:196,216` — `keyword_query.unwrap()` after early-return check. **Fix:** `let Some(kq) = keyword_query.as_deref() else { return Err(...) };`.
- `crates/solobase-browser/src/vector/embedding.rs:40` — `Self::new().expect("default model is always valid")` in `Default::default()` panics if model id is removed from the registry. **Fix:** drop the `Default` impl.
- `crates/solobase-browser/src/llm/openai_codec.rs:65` — `serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".into())` silently rewrites tool-call args. **Fix:** propagate `Err(LlmError::BackendError(...))`.
- `crates/solobase-browser/src/database.rs:454,482,531,544,726` — `data.keys()` / `data.iter()` iterated in HashMap order, producing different INSERT/UPDATE column orderings. `solobase-cloudflare/src/database.rs:303,348,506` already sorts. **Fix:** sort entries by key before building SQL.
- `crates/solobase-browser/src/database.rs:367,611,643` — `serde_json::from_str(&...).unwrap_or_default()` after a successful SQL query swallows parse errors → counts/sums silently lie. **Fix:** `map_err(...)?`.
- `crates/solobase-browser/src/runtime.rs:46-67` — `*const Wafer` taken across `.await`, only guarded by `debug_assert!`. **Fix:** make `store_wafer` return `Err` on already-set (or use `OnceCell`).
- `crates/solobase-cloudflare/src/storage.rs:83,137` — `last_modified: chrono::Utc::now()` on every `get`/`list` response. Callers using `last_modified` for cache freshness get garbage. **Fix:** read R2's actual `obj.uploaded()` timestamp, or return `Option<DateTime<Utc>>` in `ObjectInfo`.

### Medium
- `crates/solobase-web/src/config.rs:26-66,128-132,146-154,169-172` — Every `bridge::db_exec_raw(...)` is `let _ = ...`. A failing CREATE TABLE silently breaks seeding. **Fix:** propagate to `initialize()` which already returns `Result<(), JsValue>`.
- `crates/solobase-browser/src/database.rs:482-494` — Builds `Vec<&String>` then maps to two more `Vec<String>` allocations. **Fix:** single loop populating columns, placeholders, params in lockstep.
- `crates/solobase-browser/src/vector/service.rs:188-191,218-221` — `candidates.iter().map(|(id, v, _m)| (id.clone(), v.clone())).collect::<Vec<(String, Vec<f32>)>>()` clones every vector + id. For a 1k-row index with 768-dim vectors that's ~3MB cloned per query. **Fix:** `score::top_k` to take borrows.
- `crates/solobase-browser/src/database.rs:21-25` — `sanitize_ident` returns `String` for every call; allocations unnecessary when input is already alphanumeric. **Fix:** `Cow<'_, str>`.
- `crates/solobase-cloudflare/src/network_service.rs:23` — Unknown HTTP method silently falls back to `Method::Get`. **Fix:** return `NetworkError::RequestError`.
- `crates/solobase-browser/src/storage.rs:24-33` — `jsvalue_to_string` treats anything that isn't a string-or-null-or-undefined as a thrown error. **Fix:** make the bridge contract explicit via `Result<JsValue, JsValue>`.
- `crates/solobase-cloudflare/src/database.rs:50-70` — `add_missing_columns` swallows every ALTER TABLE error with `let _ = ...`. **Fix:** match on "duplicate column" string explicitly.
- `crates/solobase-browser/src/crypto.rs:108-118` — Constant-time compare via `HMAC == HMAC` works but is non-obvious. **Fix:** `subtle::ConstantTimeEq` (already in dep tree).
- `crates/solobase-browser/src/database.rs:73-74` — `std::iter::repeat("?").take(arr.len()).collect()`. **Fix:** `vec!["?"; n].join(", ")`.
- `crates/solobase-cloudflare/src/database.rs:264,266` — `total_count.unwrap_or(records.len() as i64)` masks "skip_count" intent. **Fix:** `Option<i64>`.
- `crates/solobase-browser/src/llm/openai_codec.rs:252-254`, `llm/catalog.rs:35-38`, `image/catalog.rs:39-44` — `let mut c = X::default(); c.field = ...;` pattern for `#[non_exhaustive]` types. **Fix:** upstream builder in wafer-core.

### Low
- `crates/solobase-browser/src/runtime.rs:30-36` — `store_wafer` uses `debug_assert!` only. Promote to `Result<(), StoreError>`.
- `crates/solobase-cloudflare/src/lib.rs:236-251` — `#[cfg(test)] mod api_surface` is unreachable (wasm32-only crate). Delete or move to a `target_arch` doctest.
- `crates/solobase-browser/src/asset_loader.rs:1-11` — module-level comment says "currently not shipped". File an issue or remove.
- `crates/solobase-cloudflare/src/database.rs:686-693` — long block-comment explaining why tests are omitted; move rationale to issue.
- `crates/solobase-browser/src/llm/service.rs:131-138` — multi-paragraph rationale comment; move to ADR / design-doc link.
- `crates/solobase-browser/src/vector/service.rs:55-56,16` — manual `unsafe impl Send` without `// SAFETY:` comment. Same pattern in `database.rs:15-16`, `storage.rs:13-14`, `crypto.rs:35-36`, `network.rs:10-11`, `logger.rs:6-7`, and four `solobase-cloudflare` services.
- `crates/solobase-browser/src/database.rs:330` — `parse_rows` could be parameterised to short-circuit after one row when caller is `get()`.

---
