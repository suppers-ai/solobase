# Wave 2 Group B — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-B` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-B` then `gh pr create --title "fix(rbp-B): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group B of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-B` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-core/blocks: admin + utility blocks

### Critical
- `crates/solobase-core/src/blocks/fastembed.rs:71-72` — Two `.expect("just set")` / `.expect("other thread set it")` calls in production code. **Fix:** Replace with `match self.service.get() { Some(svc) => Ok(svc.as_ref()), None => Err(...) }` or `.get_or_try_init()`.
- `crates/solobase-core/src/blocks/admin/pages/settings.rs:59` — `unreachable!()` on the dispatch arm. **Fix:** Default to `email::settings_body(...)` (same as the `tab` normalization above) instead of panicking.
- `crates/solobase-core/src/blocks/admin/pages/database.rs:441,443` — `std::time::Instant::now()` panics on wasm32-unknown-unknown (no system clock); admin runs under cloudflare workers. **Fix:** Use `helpers::now_millis()` (already wasm-safe via chrono) and compute elapsed in ms.
- `crates/solobase-core/src/blocks/admin/pages/blocks.rs:364` — `block_opt.unwrap()` after `if block_opt.is_none() { return ... }`. **Fix:** Refactor to `let Some(block) = block_opt else { return ui::html_response(markup_for_unloaded); };`.
- `crates/solobase-core/src/blocks/helpers.rs:89` — `write!(s, "{:02x}", b).unwrap()`. **Fix:** `let _ = write!(...)` with a `// SAFETY: writing to String never errors` note.
- `crates/solobase-core/src/blocks/rate_limit.rs:222` — `db::query_raw` raw-SQL call in block code (non-exception path). **Fix:** Replace with `db::list_all` + a `Filter` (same builders the SQL already uses).
- `crates/solobase-core/src/blocks/admin/pages/blocks.rs:33-49` and `:152-153` — `block_enabled` is a `HashMap<String,bool>`; iteration order is randomized per process. **Fix:** Sort `block_settings_rows` (or collect into `BTreeMap`) before iteration; sort `all_blocks` deterministically after the append. _(Partially addressed in PR #155 cleanup.)_
- `crates/solobase-core/src/blocks/admin/pages/users.rs:214-231` — `users_table` does N+1 queries (one `list_all` per user shown for roles). **Fix:** Issue one `list_all` filtered by `user_id IN (...)` using an `InOp` filter, then bucket roles by `user_id`.
- `crates/solobase-core/src/blocks/admin/pages/dashboard.rs:372-392` — 8+ sequential DB roundtrips per dashboard load (D1 amplification concern). **Fix:** Issue independent count queries concurrently with `tokio::join!`.

### High
- `crates/solobase-core/src/blocks/admin/pages/database.rs:331-344` — Stable column ordering is computed by linear `.iter().any()` in an inner loop — O(n²). **Fix:** Use a `HashSet<String>` for membership + a `Vec<String>` for ordering.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:592-601` — `_sort, _order` destructuring tuple shape suggests the row representation should be a struct. **Fix:** Refactor `all_rows: Vec<PermRow>` with named fields.
- `crates/solobase-core/src/blocks/admin/custom_tables.rs:248-256,275-283` — `format!("{e}")` then `.contains("not found")` for error-code dispatch. **Fix:** `Err(e) if e.code == ErrorCode::NotFound => err_not_found(...)`.
- `crates/solobase-core/src/blocks/storage.rs:61,255` — Two `wrap_grants.write().unwrap()` / `read().unwrap()` calls. **Fix:** `.unwrap_or_else(|e| e.into_inner())` (same pattern used in `rate_limit.rs:133`).
- `crates/solobase-core/src/blocks/admin/pages/blocks.rs:53` — `let mut all_blocks = registered_blocks.clone();` clones the full `Vec<BlockInfo>` just to extend it. **Fix:** Drop the intermediate clone.
- `crates/solobase-core/src/blocks/email.rs:512-521` — Hand-rolled `Base64Encoder` reimplements `base64`/`base64ct` which is already a workspace dep used by `wafer-block-crypto`. **Fix:** Use `base64ct::Base64::encode_string`.
- `crates/solobase-core/src/blocks/email.rs:502-510` — Hand-rolled `url_encode` next to `helpers::url_path_encode`. **Fix:** Add a `form_url_encode` to helpers and delete this copy.
- `crates/solobase-core/src/blocks/admin/pages/database.rs:36-39` — Third copy of percent-encode (`pct_encode`). **Fix:** Consolidate on a single helper module.
- `crates/solobase-core/src/blocks/storage.rs:286-291` — Drains every event into `Vec<StreamEvent>` before replaying — buffers entire storage GET body in memory, defeating streaming. **Fix:** Pass-through forward chunks as they arrive.
- `crates/solobase-core/src/blocks/network.rs:53-90` — Same buffering pattern as storage; whole HTTP response sits in memory before reaching caller.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:286-300` — Cubic nested `@for` loop (`blocks × grants × config_keys`). **Fix:** Precompute `HashMap<String, Vec<&Grant>>` keyed by resource pattern outside the maud closure.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:171-178` — `var_map.get(&var.key).cloned()` per row clones `(String, bool)` when callers only need `&str`. **Fix:** Borrow via `var_map.get(&var.key).map(|(v,s)| (v.as_str(), *s))`.

### Medium
- `crates/solobase-core/src/blocks/crud.rs:113,143` — `.unwrap_or("").to_string()` immediately after `strip_prefix` allocates even though `id` is only used by reference. **Fix:** Keep as `&str`.
- `crates/solobase-core/src/blocks/admin/pages/users.rs:255` — `user_roles.get(&record.id).cloned().unwrap_or_default()` clones `Vec<String>` just for iteration. **Fix:** `.get().map(Vec::as_slice).unwrap_or(&[])`.
- `crates/solobase-core/src/blocks/admin/iam.rs:267-271` — `if let Ok(records) = existing` swallows the DB error case silently. **Fix:** Match and `return err_internal(...)` on the error arm.
- `crates/solobase-core/src/blocks/admin/users.rs:178-200` — `handle_delete` falls back on `path.strip_prefix("/admin/users/")` returning everything after, including trailing segments. **Fix:** After stripping, take only the first `/`-bounded segment.
- `crates/solobase-core/src/blocks/admin/users.rs:121` — `serde_json::to_value(&record).unwrap_or_default()` silently turns a serialization failure into `Null`. **Fix:** Match and surface internal error.
- `crates/solobase-core/src/blocks/admin/pages/users.rs:314-385` — `user_row_fragment` duplicates the row-rendering logic of `users_table`'s inner loop verbatim. **Fix:** Extract a `single_user_row(record, roles, current_uid)` helper.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:639-871` — Three `#[allow(dead_code)]` functions. **Fix:** Delete.
- `crates/solobase-core/src/blocks/admin/database.rs:126-132` — `#[allow(dead_code)] fn message(&self)`. **Fix:** If not in use, delete.
- `crates/solobase-core/src/blocks/admin/custom_tables.rs:148-153` — Branch on `starts_with("custom_")` repeated four times. **Fix:** Pull into `fn full_table_name(name: &str) -> String`.
- `crates/solobase-core/src/blocks/email.rs:217-292` — Massive `match req.template.as_str()` block holding HTML literals inline (~80 lines per arm). **Fix:** Extract per-template renderers.
- `crates/solobase-core/src/blocks/admin/pages/dashboard.rs:299` — `record.data.get("email").and_then(...)` pattern in two places; `RecordExt::str_field` exists. **Fix:** Use `record.str_field("email")`.
- `crates/solobase-core/src/blocks/email.rs:393` — `trimmed.split_once('@').unwrap()`. **Fix:** `let Some((local, domain)) = trimmed.split_once('@') else { return Err(...) };`.
- `crates/solobase-core/src/blocks/email.rs:520` — `String::from_utf8(buf).unwrap_or_default()`. **Fix:** Replace with `base64ct` (the issue vanishes).

### Low
- `crates/solobase-core/src/blocks/admin/pages/blocks.rs:671,696,723` — Three `TODO(cloud):` comments without an issue link.
- `crates/solobase-core/src/blocks/admin/wafer_info.rs:17-39` — Hand-maintained static list of blocks plus "In a real implementation, this would query the Wafer runtime" comment. **Fix:** Replace body with `ctx.registered_blocks()`.
- `crates/solobase-core/src/blocks/errors.rs:45` — `pub fn as_str(&self)` and `pub fn status_code(&self)` lack doc comments.
- `crates/solobase-core/src/blocks/admin/pages/email.rs:10` — `EMAIL_SETTINGS_KEYS: &[(&str, &str, &str, &str, bool)]` five-tuple with positional meaning. **Fix:** `struct EmailSettingField { key, label, help, default, sensitive }`.
- `crates/solobase-core/src/blocks/admin/pages/blocks.rs:323,393` — `block.name.replace('/', "--")` repeated; helper would help.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:401-411` — Magic-string ladder repeated at lines 452-458. **Fix:** Single `fn human_resource_type(rt: &str) -> &'static str`.
- `crates/solobase-core/src/blocks/admin/iam.rs:317-318` — `count > 0` then early return makes seed idempotent on row count; fragile if rows partially exist.
- `crates/solobase-core/src/blocks/system.rs:54-98` — Five `_ if path.starts_with(...) && path.ends_with(...) =>` arms — order-sensitive. **Fix:** Replace with a dispatch table.
- `crates/solobase-core/src/blocks/admin/pages/permissions.rs:232-305` — Inline JS as `PreEscaped(r#"..."#)`; consider a static asset.
- `crates/solobase-core/src/blocks/storage.rs:117-121` — `access_type_for_op` returns `"read"`/`"write"` as strings. **Fix:** Return a `Access::Read | Access::Write` enum.

---
