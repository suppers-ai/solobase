# Wave 2 Group C — Work Scope

You are fixing findings below in this worktree (`rbp/wave2-C` branch). Aim for the **Critical** and **High** items in this session; **Medium** and **Low** are stretch goals — include if scope allows, otherwise note them as "deferred to follow-up PR" in the PR body.

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
5. Push: `git push -u origin rbp/wave2-C` then `gh pr create --title "fix(rbp-C): rust best-practices remediation — <one-line summary>" --body "<see template>"`.

PR body template:

```markdown
Wave 2 group C of the 2026-05-14 Rust best-practices remediation. Builds on Wave 1a (#165) + Wave 1b (#167).

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
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-C` then `gh pr create`.
- Do NOT skip pre-commit hooks.

When done, report back with the PR URL and a one-paragraph summary of what shipped.

---

(Below: verbatim review section from `docs/rust-best-practices-review-2026-05-14.md`.)

## solobase-core/blocks: auth + auth_ui

### Critical
- `crates/solobase-core/src/blocks/auth_ui/api/forgot_password.rs:38-50` and `api/reset_password.rs:41-50` — password-reset token stored and looked up **as plaintext** in `users.reset_token`. A DB read primitive (admin SQL explorer, backup leak, log dump, any other block with `read` grant over `suppers_ai__auth__users`) becomes a password-reset oracle. **Fix:** store `sha256_hex(reset_token)` in `users.reset_token`; look up by SHA-256 of the supplied token.
- `crates/solobase-core/src/blocks/auth_ui/api/verify.rs:46-83` and `signup.rs:140-194` — email-verification token similarly stored plaintext in `users.verification_token`. Same oracle as above. **Fix:** sha256 before write/lookup.
- `crates/solobase-core/src/blocks/auth_ui/api/refresh.rs:201-240` — `resign_refresh_with_family` omits the `iss` claim that `generate_tokens` stamps and the same handler enforces at line 73-76. After one rotation, the new refresh JWT has no `iss`, and the next refresh attempt fails forever. **Fix:** include `iss: expected_issuer(ctx).await` in `refresh_claims`.
- `crates/solobase-core/src/blocks/auth_ui/api/signup.rs:109-126` — `email_already_taken` is computed from `db::get_by_field(...).is_ok()`. Any non-NotFound DB error (WRAP denial, connection blip) is collapsed to "email is free". **Fix:** match on `ErrorCode::NOT_FOUND` specifically, then go through `users::find_by_email` typed path.
- `crates/solobase-core/src/blocks/auth_ui/api/sync_user.rs:35-56` — Same error-collapsing footgun: any non-NotFound DB error causes a duplicate user insert. **Fix:** match on `ErrorCode::NOT_FOUND`; surface other errors as `err_internal`.
- `crates/solobase-core/src/blocks/auth/repo/pats.rs:95,143,173,199` — `token_hash` (`Vec<u8>`) is passed to `json!(...)` which serialises as a JSON array of numbers. Every other token table hex-encodes. **Fix:** hex-encode like `sessions.rs`/`tokens.rs` (`json!(hex_encode(&new.token_hash))`).

### High
- `crates/solobase-core/src/blocks/auth/cache.rs:55,71,79,88` — `Mutex.lock().expect("OrgAdminCache mutex poisoned")` in `verify_org_admin`'s hot path. **Fix:** `lock().unwrap_or_else(|e| e.into_inner())`.
- `crates/solobase-core/src/blocks/auth/repo/jwt_blocklist.rs:66-71` — `contains` returns `false` on *any* DB error, not just NotFound. A backend hiccup silently re-enables a logged-out JWT until expiry. **Fix:** match on `ErrorCode::NOT_FOUND` to return `false`, propagate other errors.
- `crates/solobase-core/src/blocks/auth_ui/oauth/start.rs:101-112` — `client_id` and `redirect_uri` interpolated into the provider auth URL without urlencoding. **Fix:** urlencode every interpolation site uniformly.
- `crates/solobase-core/src/blocks/auth_ui/api/login.rs:54-61` — after `find_by_email` returned the row, handler does a *second* `db::get(ctx, USERS_TABLE, &u.id)`. Extra D1 read per login. **Fix:** carry the `UserRow` from `users::find_by_email` and read `disabled`/`email_verified` off it.
- `crates/solobase-core/src/blocks/auth_ui/api/login.rs:32` — `users::find_by_email(...).await.ok().flatten()` swallows real errors (WRAP denial, DB outage) into "no such user" → "invalid credentials". **Fix:** propagate non-NotFound errors via `err_internal`.
- `crates/solobase-core/src/blocks/auth/mod.rs:87-106` — `get_user_roles` does *two* DB calls every request that needs roles. **Fix:** combine into one, drop legacy `USER_ROLES_TABLE` read once Plan A2 closes.
- `crates/solobase-core/src/blocks/auth/mod.rs:124-164` — `ensure_admin_role` reads `SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL` on **every** authenticated path that mints tokens. **Fix:** early-return when config is unset before the second `db::create`; or hoist the config read to once per `AuthBlock::init`.
- `crates/solobase-core/src/blocks/auth_ui/oauth/callback.rs:159-170` — `tracing::error!(... body_preview = ...)` logs up to 200 chars of the OAuth provider response; typically contains email and provider IDs. **Fix:** redact or hash the preview.
- `crates/solobase-core/src/blocks/auth/service.rs:234,248,275` — `row.expires_at.as_str() < now_iso().as_str()`. String comparison of ISO-8601 timestamps fails on mixed timezone formats (`+00:00` vs `Z`). **Fix:** parse both sides with `chrono::DateTime::parse_from_rfc3339`.
- `crates/solobase-core/src/blocks/auth_ui/api/logout.rs:46` — `unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(1))` for blocklist `expires_at`. If `ACCESS_TOKEN_LIFETIME_SECS` is extended past 1 day, blocklist row evicts while JWT is still valid. **Fix:** fall back to `now + access_token_lifetime_secs(ctx)`.

### Medium
- `crates/solobase-core/src/blocks/auth_ui/api/refresh.rs:52-57` — `unwrap_or("").to_string()` then check `is_empty()`. **Fix:** `let Some(user_id) = claims.get("user_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) else { ... };`.
- `crates/solobase-core/src/blocks/auth/cache.rs:24` — `Key = (String, String, String)`. Every `get`/`insert` allocates three Strings. **Fix:** `(Arc<str>, Arc<str>, Arc<str>)`.
- `crates/solobase-core/src/blocks/auth/repo/orgs.rs:107-113,144,167` — `is_reserved` JSON value mixes `json!(0)` and `json!(false)`. **Fix:** use `json!(false)` consistently.
- `crates/solobase-core/src/blocks/auth/bootstrap.rs:87-122` — `bootstrap_with_email_password` builds the user row by hand with 11 inserts into a HashMap rather than calling `users::insert`. **Fix:** widen `users::NewUser` to include the legacy fields.
- `crates/solobase-core/src/blocks/auth_ui/api/signup.rs:286-289` — `is_common_password` does `to_ascii_lowercase()` allocation per signup. **Fix:** `COMMON_PASSWORDS.iter().any(|p| p.eq_ignore_ascii_case(pw))`.
- `crates/solobase-core/src/blocks/auth/mod.rs:337-347` — `helpers::urlencode` is a hand-rolled percent-encoder. The `url` crate is already a dep. **Fix:** `url::form_urlencoded::byte_serialize`.
- `crates/solobase-core/src/blocks/auth_ui/api/refresh.rs:135-139,128` — `email = user.str_field("email").to_string()` reads from raw `db::get` Record. **Fix:** use `users::find_by_id` typed path.
- `crates/solobase-core/src/blocks/auth/repo/users.rs:108-119` — `is_email_verified` returns `RepoError::Db` for missing users instead of `Ok(false)` as the doc claims. **Fix:** make code match doc.
- `crates/solobase-core/src/blocks/auth/mod.rs:380-413` — `authenticate_api_key` silently returns on every error path including real DB outages. **Fix:** `tracing::warn!` on DB error paths.

### Low
- `crates/solobase-core/src/blocks/auth/mod.rs:73` — `pub(crate) mod helpers` exposes secret-handling functions without `///` doc-comments. Add canonical `auth_method` values.
- `crates/solobase-core/src/blocks/auth/repo/pats.rs:37-47` — `decode_bytes` accepts `Value::String(s).as_bytes()` for the token_hash column; too permissive.
- `crates/solobase-core/src/blocks/auth_ui/mod.rs:221,237,256,270,288` — five identical TODO comments. Convert to a single tracked issue.
- `crates/solobase-core/src/blocks/auth_ui/api/mod.rs:5` — module doc claims "every function panics with `unimplemented!()`"; stale.
- `crates/solobase-core/src/blocks/auth_ui/mod.rs:19-25` — same stale "scaffold" wording.
- `crates/solobase-core/src/blocks/auth/repo/cli_codes.rs:39-49,90,129` — `decode_bytes` defined but unused at module level. Encoding-mismatch will trip if/when a caller appears.
- `crates/solobase-core/src/blocks/auth/service.rs:106-108` — three sha256 helpers in the auth tree (`hash_token`, `bootstrap::sha256`, `helpers::sha256_hex`). **Fix:** consolidate.
- `crates/solobase-core/src/blocks/auth/repo/tokens.rs:48` — `let id = uuid::Uuid::now_v7().to_string()` then `db::create` likely generates its own id. Verify which is canonical.

---
