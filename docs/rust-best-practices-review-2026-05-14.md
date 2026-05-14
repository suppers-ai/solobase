# Solobase Rust Best-Practices Review — 2026-05-14

Findings from a whole-repo review against Apollo GraphQL's [Rust Best Practices Handbook](https://github.com/apollographql/rust-best-practices), interpreted through the project rules in `CLAUDE.md` (no `panic!`/`unwrap`/`expect` in production code, no sync bridges, no raw SQL in block code outside the documented exceptions, terse comments, etc.).

Reviewed at commit `bcf96ce` (post-PR-#155 merge). Total: **~253 findings** (44 critical / 67 high / 82 medium / 60 low) across all seven solobase crates.

This is a **report, not a plan** — every finding is a concrete file:line observation with a one-line fix recommendation. Triage and prioritise as a separate exercise.

## Contents

- [Cross-cutting patterns](#cross-cutting-patterns)
- [solobase-core: root modules](#solobase-core-root-modules)
- [solobase-core/blocks: admin + utility blocks](#solobase-coreblocks-admin--utility-blocks)
- [solobase-core/blocks: auth + auth_ui](#solobase-coreblocks-auth--auth_ui)
- [solobase-core/blocks: products + files + legalpages + userportal](#solobase-coreblocks-products--files--legalpages--userportal)
- [solobase-core/blocks: vector + llm + messages](#solobase-coreblocks-vector--llm--messages)
- [solobase + solobase-native](#solobase--solobase-native)
- [solobase-browser + solobase-web + solobase-cloudflare](#solobase-browser--solobase-web--solobase-cloudflare)
- [Methodology](#methodology)

---

## Cross-cutting patterns

These themes recur across multiple crates. Fixing them at the root tends to clear out a long tail of individual findings.

### 1. `unwrap`/`expect` in production code paths

The project rule is "no `panic!`/`unwrap`/`expect` outside test code". Reviewers found violations in every crate. The biggest concentrations:

- **Lock-poisoning panics on hot paths.** `RwLock::write().expect("poisoned")` / `Mutex::lock().expect("poisoned")` in `llm::providers::mod` (9 sites), `auth::cache::OrgAdminCache`, `cache::TtlCache`, `solobase-cloudflare::database::schema_cache`, `solobase-browser::vector::service`. The standard fix is `.unwrap_or_else(|e| e.into_inner())` — poisoned data is recoverable for read-mostly caches.
- **`.expect()` on SSE-chunk wire shapes.** Every chunk decoder in `llm::providers::{openai, anthropic}` panics on a wafer-core wire-shape rename; one byte of bad data crashes the streaming task. The root-cause fix lives upstream in `wafer-core` (export typed `ChunkDelta::*` constructors).
- **`.expect()` during boot.** `solobase-native` factories (`make_sqlite_database_service`, `make_local_storage_service`, `make_jwt_crypto_service`, `serve_until_shutdown`, `init_tracing`), `solobase/src/cli/server.rs:84,105,169`, `crypto::derive_block_jwt_key`, `fastembed::ensure_initialized`. Boot panics surface as opaque stack traces rather than structured CLI errors.

### 2. Errors silently swallowed into success or default

A pattern that repeats in nearly every crate:

```rust
let _ = db::exec_raw(...).await;                       // stripe webhook completion
let rows = update(...).await.unwrap_or(0);             // stripe CAS update
serde_json::to_string(&v).unwrap_or_default();         // token JSON encoding
match db::get_by_field(...) { Err(_) => create(...) }  // signup, sync_user
```

Highest-impact instances:
- `products/stripe.rs:130,351,411,456,509,538,898` — Stripe webhook handlers swallow DB errors and return 200, so Stripe stops retrying.
- `auth_ui/api/signup.rs:109-126`, `auth_ui/api/sync_user.rs:35-56`, `auth/repo/jwt_blocklist.rs:66-71` — every `Err(_)` is treated as `NotFound`, including WRAP denials and DB outages. The right pattern is `match err.code { ErrorCode::NotFound => …, _ => propagate }`.
- `solobase-browser/src/database.rs:288-289,367,611,643` and `solobase-cloudflare/src/{convert.rs:35, network_service.rs:36-48}` — same pattern in WASM clients, where a failed JSON parse returns `Vec::new()` or `"[]"` and callers mistake transient errors for missing data.

### 3. HashMap iteration order in user-visible output

`HashMap` iteration is randomized per-process via SipHash. `admin/pages/blocks.rs` was already fixed during PR #155 cleanup, but the same pattern is latent elsewhere:

- `solobase-browser/src/database.rs:454,482,531,544,726` — INSERT/UPDATE column order varies per call, breaking prepared-statement caches (the sibling `solobase-cloudflare/src/database.rs` already sorts).
- `auth/cache.rs:24` — `Key = (String, String, String)`, allocating three Strings per lookup, hashed by the same randomized hasher.

The architectural root-cause fix lives in **wafer-run**: `Runtime.blocks_snapshot` is built via `self.blocks.values().map(b.info()).collect()`. Sort there and the admin/blocks workaround can be retired.

### 4. `SOLOBASE_*` env-var filter strips legitimate app config

`solobase-native::env::filter_app_env_vars` strips every key starting with `SOLOBASE_`, but per CLAUDE.md `SOLOBASE_SHARED__*` is **app config**, not infra. PR #155 worked around this by reading `BOOTSTRAP_ADMIN_*` directly from `std::env::var`; the underlying filter is still wrong. Two fixes possible:

1. Invert the rule: keep keys that contain `__` (the project's marker for app/block config).
2. Distinguish at the call site: drop only the keys `InfraConfig::from_env` consumed.

Same module also reads `std::env::var("SOLOBASE_BLOCK_ENABLED")` directly inside `load_block_settings`, bypassing the filter altogether — the filter contract has already drifted in practice.

### 5. Per-request allocations in routing/UI hot paths

The biggest offenders sit in code that runs on every request:

- `solobase-core/src/routing.rs:233,243,307` — `format!("suppers-ai/{}", …)` per route check.
- `solobase-core/src/ui/sidebar.rs:71` — `format!("{}/", item.href)` per nav item per render.
- `auth_ui/api/login.rs:54-61` — extra `db::get` after `find_by_email` already returned the row (one extra D1 read per login).
- `llm/routes.rs:116,387,418` and `llm/migrations.rs:127,231` — `db::list_all(PROVIDERS_TABLE, vec![])` on every chat request even though `ProviderLlmService.inner.providers` already caches it.
- `auth/mod.rs:87-106` — `get_user_roles` issues two DB queries per request that needs roles.

These compound with the active D1-amplification work tracked in `d1-amplification-active.md`.

### 6. Token storage encoding is inconsistent across the auth surface

Every auth-token table is meant to store `sha256(raw)`, but:

- `auth/repo/pats.rs:95,143,173,199` — PAT `token_hash` is serialised as a **JSON byte array** (`[12,34,…]`), while `sessions`/`tokens`/`bootstrap_tokens` hex-encode.
- `auth_ui/api/forgot_password.rs:38-50`, `reset_password.rs:41-50`, `verify.rs:46-83`, `signup.rs:140-194` — `users.reset_token` and `users.verification_token` are stored **plaintext**. Any DB-read primitive becomes a password-reset oracle.

The fix is one canonical helper (`sha256_hex(token)`) called at every write and every lookup site.

### 7. Blocking `std::process::Command` inside `async fn`

`solobase/src/cli/flows/embed_native.rs:71`, `flows/embed_cloudflare.rs:88,99`, `helpers/cloudflare/build.rs:14-24` all spawn long-running child processes (`wrangler dev`, `cargo build`) with the blocking `std::process::Command`. Inside a tokio runtime this pins a worker thread for the entire dev session. Replace with `tokio::process::Command`.

### 8. Documentation, dead code, and stale comments

Lower-severity but pervasive: `auth_ui/api/mod.rs:5` and `auth_ui/mod.rs:19-25` still describe themselves as scaffolds that panic with `unimplemented!()`; `admin/pages/permissions.rs:639-871` and `admin/database.rs:126-132` carry `#[allow(dead_code)]` blocks; `routes.rs:279` references TODOs without issue links (also `products/mod.rs:235`, `files/mod.rs:177`, `admin/pages/blocks.rs:671,696,723`).

### 9. WASM-specific concerns

- `solobase-browser/src/runtime.rs:46-67` uses `*const Wafer` across `.await` with only a `debug_assert!` guarding the invariant. Promote `store_wafer` to return `Err` on double-set.
- `solobase-cloudflare/src/storage.rs:83,137` fabricates `last_modified: chrono::Utc::now()` on every `get`/`list`; consumers using it for cache freshness see garbage.
- `solobase-browser/src/crypto.rs:108-118` implements constant-time compare via `HMAC == HMAC` — works, but `subtle::ConstantTimeEq` is already in the dep tree.
- Several files have `unsafe impl Send/Sync` for wasm32 single-threaded usage without `// SAFETY:` comments (`solobase-browser/src/{database,storage,crypto,network,logger,vector/service,llm/service}.rs`).

---

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

## Methodology

- Reviewed at commit `bcf96ce` (post-PR-#155 merge).
- Reference: [Apollo GraphQL Rust Best Practices Handbook](https://github.com/apollographql/rust-best-practices) chapters 1–9 (idioms, clippy, performance, error handling, testing, generics & dispatch, type state, comments vs docs, pointers).
- Seven parallel review agents, scoped by cohesive module:
  - **A** — `solobase-core` root modules (`crypto.rs`, `pipeline.rs`, `builder.rs`, `helpers/`, `ui/`).
  - **B** — `solobase-core/blocks/admin/` + `blocks/*.rs` utility blocks.
  - **C** — `solobase-core/blocks/auth/` + `auth_ui/`.
  - **D** — `solobase-core/blocks/{products,files,legalpages,userportal}/`.
  - **E** — `solobase-core/blocks/{vector,llm,messages}/`.
  - **F** — `solobase/` (CLI) + `solobase-native/`.
  - **G** — `solobase-browser/` + `solobase-web/` + `solobase-cloudflare/`.
- Severity scheme:
  - **Critical**: real bug — panic/unwrap on production path, security issue, swallowed error that should propagate, race.
  - **High**: clear inefficiency in hot path, missing error context, blocking IO in async, non-determinism in user-visible output, fail-open patterns.
  - **Medium**: idiomatic — unnecessary clones, `&String` params, manual loops, `unwrap_or` with expensive arg, dead code with `#[allow]`, magic-string ladders.
  - **Low**: missing `///` docs on public API, TODO without issue ref, stale comments, naming clarity.
- Skipped: anything rustfmt/clippy-default catches; speculative "refactor whole module" suggestions; test code unless tests themselves had correctness bugs.
