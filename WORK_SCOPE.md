# Wave 2 Group D — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-D` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-D` then `gh pr create --title "fix(rbp-D): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group D of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-D` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-core/blocks: products + files + legalpages + userportal

### Critical
- `crates/solobase-core/src/blocks/products/stripe.rs:130` — Stripe checkout swallows the CAS UPDATE error via `unwrap_or(0)`. **Fix:** match on the `Result` — propagate transport errors; only return 400 when the update returned 0 rows.
- `crates/solobase-core/src/blocks/products/stripe.rs:351` — Same in webhook completion: `db::exec_raw(...).await.unwrap_or(0)` then logs warn on 0 rows. A real DB error silently leaves the purchase un-completed and Stripe gets a 200, so it won't retry. **Fix:** on `Err`, return `err_internal` so Stripe retries.
- `crates/solobase-core/src/blocks/products/stripe.rs:411,456,509,538,898` — Subscription upsert + status updates use `let _ = db::exec_raw(...).await`. Failures dropped entirely. **Fix:** at minimum `.inspect_err(|e| tracing::error!(...))`; for `subscription.deleted`/`invoice.payment_failed` propagate 500.
- `crates/solobase-core/src/blocks/products/stripe.rs:380` — `sub_id = format!("sub_{}_{}", user_id, timestamp_millis())` used as PK but upsert conflicts on `user_id`. Two parallel webhooks race-insert. **Fix:** deterministic id (`format!("sub_{user_id}")`).
- `crates/solobase-core/src/blocks/products/stripe.rs:701` — `String::from_utf8_lossy(payload)` for HMAC signing corrupts the signature on non-UTF8 bytes. **Fix:** keep the signed buffer as `Vec<u8>`; concat `timestamp.as_bytes()`, `b"."`, `payload` directly.
- `crates/solobase-core/src/blocks/products/purchase.rs:153` — `(total_amount * 100.0).round() as i64` silently saturates on overflow / NaN. **Fix:** validate `total_amount.is_finite()` and bounds-check before the cast.
- `crates/solobase-core/src/blocks/products/purchase.rs:108` — `evaluate_formula(...).unwrap_or(0.0)` swallows formula errors. **Fix:** propagate as `err_bad_request("Pricing failed: {e}")`.
- `crates/solobase-core/src/blocks/files/share.rs:46` — `handle_direct_access` looks up by token without rate limit. Enables enumeration / DOS. **Fix:** add per-IP rate limit on `/storage/direct/*`; verify `crypto::verify(token)` before DB lookup.

### High
- `crates/solobase-core/src/blocks/products/stripe.rs:159-167` — Stripe form body built with `format!` interpolates `body.purchase_id` and pre-encoded URLs without URL-encoding `purchase_id`. **Fix:** URL-encode `body.purchase_id` (and `total_cents`/`currency`).
- `crates/solobase-core/src/blocks/products/handlers.rs:228,461` — `format!("%{}%", search)` for `Like` filter does not escape `%` or `_` in user input. **Fix:** escape `%` and `_` before formatting.
- `crates/solobase-core/src/blocks/files/storage.rs:347` — `body_bytes = input.collect_to_bytes().await` reads the entire upload into memory before quota check. **Fix:** stream the upload, check size as bytes arrive, abort at quota cap.
- `crates/solobase-core/src/blocks/files/storage.rs:380-405` — TOCTOU compensation is best-effort: cleanup `db::delete` on upload failure is only warned. On a transient DB blip, orphan `pending` row inflates quota usage forever. **Fix:** add a periodic sweeper for stale `pending` rows.
- `crates/solobase-core/src/blocks/files/share.rs:106` — Access-count increment is non-atomic: read-then-write. Two concurrent accesses with `max_access_count = 1` both succeed. **Fix:** CAS update via `build_update_where`.
- `crates/solobase-core/src/blocks/products/purchase.rs:212-217` — Purchase rollback is sequential and not transactional. If creation 5/10 fails, the `delete(purchase)` call itself can fail (warn only). **Fix:** wrap in a transaction or change status to `failed`.
- `crates/solobase-core/src/blocks/legalpages/pages.rs:700-744` — `archive_published` is called before the new doc is created; if create fails, previous published doc is already archived. **Fix:** archive *after* the new publish succeeds.
- `crates/solobase-core/src/blocks/products/stripe.rs:638` — `serde_json::to_vec(&body).unwrap_or_default()` silently signs/sends an empty webhook payload. **Fix:** log + return on Err.
- `crates/solobase-core/src/blocks/products/handlers.rs:551,699` — `or_insert(serde_json::json!(1))` hardcodes integer `1` as the default template's ID, but seeding uses UUIDv7 string ids. **Fix:** look up the default template by `name = "default"` once at startup.
- `crates/solobase-core/src/blocks/files/pages_user.rs:121` — N+1 query: `list_buckets_for_user` then per-bucket `db::count`. **Fix:** single aggregate via `wafer_sql_utils::aggregate` (GROUP BY bucket).
- `crates/solobase-core/src/blocks/products/stripe.rs:148,156` — `body.success_url.unwrap_or_else(|| format!(...))` interpolates user-supplied URLs without origin validation. **Fix:** validate that `success_url`/`cancel_url` are on `SOLOBASE_SHARED__FRONTEND_URL` origin.

### Medium
- `crates/solobase-core/src/blocks/products/stripe.rs:713` — `hmac_sha256_local` returns `Vec::new()` on crypto failure. **Fix:** `.inspect_err(|e| tracing::error!("hmac failure: {e}"))`.
- `crates/solobase-core/src/blocks/products/handlers.rs:872-887` — Manual null-coalesce loop on subscription rows. **Fix:** add `coalesce` support to `wafer-sql-utils`.
- `crates/solobase-core/src/blocks/products/pages.rs:534-584,688` — `SETTINGS_KEYS` is a duplicate of `config_keys` declared in `mod.rs:145-165`. **Fix:** derive settings rendering from `BlockInfo::config_keys()`. Same pattern in `legalpages/pages.rs:751` and `userportal/mod.rs:630`.
- `crates/solobase-core/src/blocks/products/purchase.rs:299` — `path.rsplit('/').next().unwrap_or("")` to extract purchase id; brittle. **Fix:** use explicit `strip_prefix` pattern.
- `crates/solobase-core/src/blocks/legalpages/pages.rs:592-650` — `handle_save` returns `ok_json` with `{"error": "..."}` on parse failure (200 + error key). **Fix:** return `err_bad_request`. Same at `handle_publish:655`, `handle_save_settings:887`, `products/pages.rs:680`.
- `crates/solobase-core/src/blocks/userportal/mod.rs:307-311` — `db::update(...).map_err(|e| err_internal(..., e.message))` loses structured error info. **Fix:** pass `e` directly.
- `crates/solobase-core/src/blocks/files/cloud.rs:86-108` — Duplicate of `is_bucket_access_denied` logic from `storage.rs:84`. **Fix:** call the storage helper or move to shared util.
- `crates/solobase-core/src/blocks/products/pricing.rs:194-216` — Hand-rolled `chars().collect::<Vec<char>>` then index-walk. **Fix:** `Peekable<Chars>`.
- `crates/solobase-core/src/blocks/products/handlers.rs:898-923` — Six awaited DB calls in `handle_stats`, sequential. **Fix:** `tokio::join!`.

### Low
- `crates/solobase-core/src/blocks/products/mod.rs:235` — `// TODO: Allowed(headers) discarded` without an issue ref. See also `files/mod.rs:177`.
- `crates/solobase-core/src/blocks/products/handlers.rs:1-15` — Module-level docs missing.
- `crates/solobase-core/src/blocks/files/pages_admin.rs:578-588` — `format_bytes` duplicated; candidate for `ui::components`.
- `crates/solobase-core/src/blocks/products/stripe.rs:22-27` — Inline `#[derive(serde::Deserialize)]` structs inside async fns; consider lifting to module scope for testability.
- `crates/solobase-core/src/blocks/legalpages/pages.rs:313-421` — 100+ line JS string constant inline. Prefer assets pipeline.

---
