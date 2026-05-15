# Wave 2 Group E — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-E` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-E` then `gh pr create --title "fix(rbp-E): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group E of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-E` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-core/blocks: vector + llm + messages

### Critical
- `crates/solobase-core/src/blocks/llm/providers/mod.rs:57,68,89,104,143,151,319,333,353` — `RwLock::read()/write()` calls use `.expect("provider svc lock poisoned")` in production. A poisoned lock from any panicking writer brings down chat, model listing, status checks. **Fix:** propagate via `map_err`, or fall back to `lock.into_inner()` after `PoisonError`.
- `crates/solobase-core/src/blocks/llm/providers/mod.rs:343` — `serde_json::from_value(v).expect("ModelStatus wire shape")` in the `status()` hot path. **Fix:** add a typed `ModelStatus::error(msg)` constructor in wafer-core (root-cause), or `unwrap_or_else(|_| ModelStatus::ready())` with `tracing::error!`.
- `crates/solobase-core/src/blocks/llm/providers/openai.rs:334,339,342,351,359` and `providers/anthropic.rs:497,509,517,524` — `ChatChunk` builders all `.expect("ChatChunk wire shape should round-trip")` on every SSE frame. A single wafer-core wire-shape rename turns every chat response into a panic per chunk. **Fix:** make wafer-core export explicit constructors; return `LlmError::BackendError` from the decoder rather than panicking.
- `crates/solobase-core/src/blocks/llm/providers/mod.rs:57` — `reqwest::Client::builder().build().expect("…")` at service construction. **Fix:** return `Result<Self, LlmError>` from `ProviderLlmService::new()`.
- `crates/solobase-core/src/blocks/llm/providers/openai.rs:439-449` and `anthropic.rs:381-384` — `TokenUsage::default()` then field-by-field mutation on a `#[non_exhaustive]` type. **Fix:** use `TokenUsage::new(input, output)` in wafer-core.

### High
- `crates/solobase-core/src/blocks/llm/routes.rs:201-228` — `handle_chat` buffers the entire LLM response into `String content` before returning JSON. **Fix:** add `max_response_bytes` config or cap; use streaming sibling at line 285.
- `crates/solobase-core/src/blocks/llm/routes.rs:224-228` — `model_used` is always returned as `""` due to a dead branch. Clients get an empty model field. **Fix:** thread `chat_req.model` through `dispatch_chat`.
- `crates/solobase-core/src/blocks/llm/routes.rs:281-283` — `handle_chat_stream` silently drops assistant message persistence (TODO at line 279). **Fix:** snapshot auth/thread fields, persist on stream end; or document loudly.
- `crates/solobase-core/src/blocks/llm/routes.rs:116,387,418` and `migrations.rs:127,231` — every `db::list_all(PROVIDERS_TABLE, vec![])` pulls the whole provider table on each request. **Fix:** read from `ProviderLlmService.inner.providers` cache.
- `crates/solobase-core/src/blocks/vector/pages.rs:528` — `body.vector.clone()` clones the full query vector on every search. **Fix:** `body.vector.take()`.
- `crates/solobase-core/src/blocks/vector/pages.rs:789` — `vclient::embed(ctx, embedding_block, chunks.clone()).await` clones the full chunk text list. **Fix:** move `chunks`, return alongside vectors.
- `crates/solobase-core/src/blocks/llm/providers/anthropic.rs:381-384` and `openai.rs:441-449` — `TokenUsage::default()` then field mutation; fragile under future field additions.
- `crates/solobase-core/src/blocks/llm/mod.rs:79-84` — `serde_json::to_vec(&...).unwrap_or_default()` silently sends `b""` as the inter-block call body. **Fix:** return `Result`; surface encode failures.
- `crates/solobase-core/src/blocks/llm/providers/anthropic.rs:365` — `tool_blocks` `Vec` grows unbounded across a long stream. Malicious server emitting a billion `content_block_start` events = OOM. **Fix:** cap `tool_blocks.len()` (e.g. reject `index > 1024`).
- `crates/solobase-core/src/blocks/messages/a2a.rs:54` — `params.get(...).and_then(...).map(|s| s.to_string())` on every JSON-RPC call. **Fix:** deserialize into typed structs per method.

### Medium
- `crates/solobase-core/src/blocks/llm/routes.rs:150,153` — Redundant clone of `body.thread_id` when `body` is owned. **Fix:** destructure.
- `crates/solobase-core/src/blocks/llm/routes.rs:797` — `model_ids` cloned twice. **Fix:** assign to `cfg.models` once, clone from there.
- `crates/solobase-core/src/blocks/llm/migrations.rs:127` — `db::list_all(LEGACY_TABLE, vec![])` fired on every `Init`. **Fix:** one-shot marker.
- `crates/solobase-core/src/blocks/llm/schema.rs:65` — `cfg.models.iter().map(|m| Value::String(m.clone()))` clones every string. **Fix:** `config_into_row(cfg: ProviderConfig)` that consumes.
- `crates/solobase-core/src/blocks/llm/providers/mod.rs:72-79` — `p.name.clone()` twice per provider. **Fix:** bind once.
- `crates/solobase-core/src/blocks/messages/service.rs:75-98` — four near-identical filter-push blocks. **Fix:** helper `maybe_eq(field, &Option<String>) -> Option<Filter>`.
- `crates/solobase-core/src/blocks/vector/pages.rs:546-551` — `match` clones two `Option<String>`. **Fix:** `body.keyword_query.take()`.
- `crates/solobase-core/src/blocks/llm/providers/openai.rs:454` — `if !content.is_empty()` is unnecessary; OpenAI never emits empty deltas.
- `crates/solobase-core/src/blocks/llm/providers/anthropic.rs:380-385` — Only build `TokenUsage` when at least one field is `Some`.
- `crates/solobase-core/src/blocks/messages/a2a.rs:181` — `result.records.iter().map(context_to_task)` clones every metadata/parent_id. **Fix:** consume via `.into_iter()`.

### Low
- `crates/solobase-core/src/blocks/llm/routes.rs:279` — `TODO(llm-phase-b-task-14): wire assistant persistence` paired with correctness regression.
- `crates/solobase-core/src/blocks/vector/pages.rs:651-660` — `embedding_block_for_model(_model_id: &str)` ignores its argument.
- `crates/solobase-core/src/blocks/llm/providers/config.rs` — public types lack `#[non_exhaustive]`.
- `crates/solobase-core/src/blocks/llm/mod.rs:229-235` — `#[allow(dead_code)] pub(super) async fn get_default_provider_id` for unfinished Phase B tasks. **Fix:** delete or link issue.
- `crates/solobase-core/src/blocks/llm/migrations.rs:38` — `LEGACY_TABLE` is now reference-only post-migration.
- `crates/solobase-core/src/blocks/vector/pages.rs:177` — `body.model.as_deref().unwrap_or(DEFAULT_MODEL).to_string()` allocates a fresh `String` even when `body.model` was `Some`. **Fix:** `Cow<str>` path.

---
