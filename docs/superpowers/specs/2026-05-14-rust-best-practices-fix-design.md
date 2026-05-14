# Rust Best-Practices Review Fix — Design

**Date:** 2026-05-14
**Status:** Design approved; pending implementation plan
**Source review:** `docs/rust-best-practices-review-2026-05-14.md` (commit `bcf96ce`)

## Problem

The 2026-05-14 whole-repo review against Apollo GraphQL's Rust Best Practices Handbook produced ~253 findings (44 Critical / 67 High / 82 Medium / 60 Low) across seven solobase crates. The review is a report, not a plan: every finding is a `file:line` observation with a one-line fix recommendation, but they are unsequenced, some have their true root cause in upstream repos (`wafer-core`, `wafer-run`, `wafer-sql-utils`), and several cross-cutting patterns (token hashing, lock-poisoning, hashmap-order non-determinism, env-filter, env-smuggling) would conflict if fixed in parallel without coordination.

The user wants every finding fixed, including the Low tier, using parallel subagents in git worktrees. The challenge is sequencing the work so subagents don't fight over the same files and so the cross-cutting concerns get one canonical fix instead of seven divergent ones.

## Non-Goals

- New features. This is a remediation pass; no behaviour additions.
- Test infrastructure changes (the review explicitly skipped test code unless tests themselves had correctness bugs).
- Speculative refactors not tied to a review finding.
- Replacing rustfmt/clippy-default catches (those are already enforced via pre-commit).
- Performance benchmarking. We trust the review's hot-path identification; we don't establish before/after micro-benchmarks.
- Solving the cross-repo `wafer-run` D1-amplification work tracked in `d1-amplification-active.md` (a separate initiative; only the items overlapping with review findings are touched here).

## Chosen Approach

**Three sequential waves: upstream root-cause fixes → solobase shared foundations → seven parallel per-crate-group worktrees.** Each wave gates on the previous so that downstream agents have stable helpers to call into and don't reinvent cross-cutting fixes.

### Wave 0 — Upstream root-cause fixes (3 PRs across `wafer-core`, `wafer-run`, `wafer-sql-utils`)

These exist in sibling repos. Each is a small, independent change consumed by Wave 1 or Wave 2.

**W0-a — `wafer-core`** (typed constructors so SSE decoders stop panicking):
- `ChunkDelta::{text, tool_call_start, tool_call_arguments, tool_call_end, usage}` constructors — replaces `serde_json::from_value(...).expect("wire shape")` in `llm/providers/{openai,anthropic}.rs`.
- `TokenUsage::new(input, output)` — replaces `TokenUsage::default()` + field mutation on `#[non_exhaustive]` type.
- `ModelStatus::{ready, error}` constructors — replaces `.expect("ModelStatus wire shape")` in `llm/providers/mod.rs:343`.

**W0-b — `wafer-run`** (determinism):
- Sort `Runtime.blocks_snapshot` by block id before returning. Eliminates the SipHash randomness that forced PR #155's admin/blocks workaround.

**W0-c — `wafer-sql-utils`** (builders the review's fixes need):
- `Filter::InOp` for `WHERE col IN (...)` — required by users-table N+1 fix.
- `aggregate::coalesce` — required by products/handlers manual null-coalesce loop.
- `upsert::cas_update` — required by files/share access-count atomicity fix.

### Wave 1 — Solobase shared foundations (1 serial PR on `main`)

Lands after Wave 0 merges. Bumps `Cargo.lock` to pick up W0-a/b/c. Then in a single coordinated PR:

- **Canonical token hasher.** `solobase-core::crypto::sha256_hex(&str) -> String` (or `auth::helpers::sha256_hex`; pick one location, consolidate the three existing copies — `auth::service::hash_token`, `auth::bootstrap::sha256`, `auth::helpers::sha256_hex`). Every call site in Wave 2-C uses this.
- **Token-storage migration.** SQL migration that:
  - Hashes `users.reset_token` and `users.verification_token` in place (rows with non-hex-length values get hashed; already-hashed rows are no-ops; check by `length=64 AND HEX-only`).
  - Re-encodes `pats.token_hash` from JSON byte-array → hex string with the same idempotent guard.
  - All future writes go through `sha256_hex`. Reads compare hashed input against stored hash.
- **Env-filter fix.** `solobase-native::env::filter_app_env_vars` keeps any key containing `__` (the project's app/block-config marker per CLAUDE.md). The PR #155 `auto_bootstrap_if_needed` workaround can then read via the filter again.
- **Drop CLI env smuggling.** `solobase/src/main.rs:59,70` currently calls `std::env::set_var("SOLOBASE_RUN_MIGRATIONS", "1")`. Rust 2024 makes this unsound. Thread `run_migrations: bool` through `cli::dispatch → server::run` explicitly.

These four items are coupled (the token hasher is used by the migration; the env filter fix is what unblocks bootstrap reading shared config; the run-migrations refactor changes a function signature on the same boot path), so they ship together. Anything that *isn't* coupled stays in Wave 2.

### Wave 2 — Seven parallel per-crate-group worktrees (7 PRs)

Each subagent owns one cohesive crate group, branched off `main` after Wave 1 merges. Each fixes every Critical/High/Medium/Low finding in its scope. The groups align with the original review's parallel scan boundaries (A–G in the methodology section of the review doc), which were chosen for cohesion:

| Group | Scope | Approx findings |
|-------|------------------------------------------------------|---|
| **A** | `solobase-core` root (`crypto.rs`, `pipeline.rs`, `builder.rs`, `helpers/`, `ui/`, `routing.rs`, `features.rs`, `migration_helper.rs`, `cache.rs`) | ~30 |
| **B** | `solobase-core/blocks/admin/**` + utility blocks (`storage.rs`, `rate_limit.rs`, `network.rs`, `email.rs`, `fastembed.rs`, `crud.rs`, `system.rs`, `errors.rs`) | ~35 |
| **C** | `solobase-core/blocks/{auth,auth_ui}/**` — consumes Wave 1 `sha256_hex` helper | ~40 |
| **D** | `solobase-core/blocks/{products,files,legalpages,userportal}/**` | ~40 |
| **E** | `solobase-core/blocks/{vector,llm,messages}/**` — consumes Wave 0 wafer-core types | ~35 |
| **F** | `solobase/**` + `solobase-native/**` (CLI + native runtime) | ~25 |
| **G** | `solobase-browser/**` + `solobase-web/**` + `solobase-cloudflare/**` (WASM clients) | ~30 |

Each agent's brief:
- Worktree at `~/.worktrees/rbp-<group>-2026-05-14`, branch `rbp/<group>-2026-05-14`, off `origin/main`.
- Review section extracted to `WORK_SCOPE.md` inside the worktree so the agent has the exact `file:line` list without re-reading the 449-line master doc.
- Use Wave 0/1 helpers; do not reinvent them.
- For findings explicitly cross-referenced to another group (e.g. shared helpers in `solobase-core/blocks/helpers.rs`), the *owning* group fixes it; consumers reference the fix.
- Verification before declaring done: `cargo check -p <crate>`, `cargo clippy -p <crate> -- -D warnings`, `cargo test -p <crate>`, `cargo fmt --check`. For groups touching wasm crates (G), also `cargo check --target wasm32-unknown-unknown -p <crate>` for the wasm-only crates.
- Output: one PR per group, titled `chore(<group>): rust best-practices remediation (<crit>/<high>/<med>/<low>)`.

## Architecture

```
Wave 0 (upstream, parallel)
  ├── wafer-core PR    ──┐
  ├── wafer-run PR     ──┼──> merge ──> Wave 1
  └── wafer-sql-utils PR ┘

Wave 1 (solobase, serial)
  └── Cargo.lock bump + sha256_hex + token migration + env filter + run-migrations refactor
      └── merge ──> Wave 2

Wave 2 (solobase, parallel — 7 worktrees)
  ├── A: solobase-core root
  ├── B: admin + utility blocks
  ├── C: auth + auth_ui
  ├── D: products + files + legalpages + userportal
  ├── E: vector + llm + messages
  ├── F: solobase + solobase-native
  └── G: solobase-browser + solobase-web + solobase-cloudflare
```

Branch / PR strategy:
- Wave 0 PRs land independently in their respective repos.
- Wave 1 PR lands on `solobase/main`.
- Wave 2 PRs all branch off the post-Wave-1 `solobase/main` and merge serially as they finish. If two Wave 2 PRs touch overlapping files (rare given the cohesion split), the later PR rebases.

## Data Flow / Coordination

- **Helper availability.** The `sha256_hex` helper is required by Wave 2-C (auth/auth_ui token hashing). Wave 2-C must not start before Wave 1 merges.
- **Type availability.** The wafer-core `ChunkDelta` / `TokenUsage::new` / `ModelStatus` constructors are required by Wave 2-E (llm providers). Wave 2-E must not start before Wave 0-a merges and Wave 1's `Cargo.lock` bump lands.
- **SQL builders.** `Filter::InOp` is required by Wave 2-B (users-table N+1) and Wave 2-D (products bulk lookups). Both gate on Wave 0-c.
- **Cross-group references.** When a review entry in one group points at code owned by another (e.g. group F's note about consolidating with `crypto.rs` which belongs to group A), the owning group fixes it. Cross-group agents reference `WORK_SCOPE.md` ownership column to know which side acts.
- **No shared file edits.** The crate boundaries in the group table do not overlap. Each agent edits only files in their scope.

## Error Handling / Risks

- **Wave 0 upstream review lag.** If `wafer-core` or `wafer-run` PRs need slow review, Wave 1 blocks. Mitigation: file all three Wave 0 PRs simultaneously; if any stalls more than two days, ship Wave 2 with explicit `TODO(wave0): use ChunkDelta::text once wafer-core#NNN lands` comments and a follow-up doc, rather than blocking everything. Token-hashing migration in Wave 1 has no upstream dep — it lands regardless.
- **Token-migration data risk.** Hashing in place is destructive. Migration must be idempotent (already-hashed rows are no-ops) and tested against fixture DBs (cloudflare D1 fixture + sqlite local fixture). Verification: `SELECT length(reset_token), reset_token FROM users` post-migration shows only 64-char hex strings.
- **Subagent context drift.** 35–40 findings per agent is at the upper end of useful context. Mitigation: `WORK_SCOPE.md` is the curated, line-numbered list (not the full review). Each agent gets a "complete the checklist" framing, not "read the master review."
- **Worktree clobber.** Working tree currently has 52 unstaged playwright snapshot files (Playwright baselines). Worktrees branched off `origin/main` will not see them, but the user should not switch back into `/workspace/solobase` mid-flow and accidentally commit them via "stage all."
- **Cross-crate clippy regressions.** A fix in Wave 2-A's `crypto.rs` could newly warn in a Wave 2-C call site. Mitigation: Wave 2 PRs merge serially; each rebased PR re-runs `cargo clippy --workspace` against `main` before merging, not just its own crate.
- **PR review burden.** Seven Wave-2 PRs plus a Wave-1 PR plus three Wave-0 PRs = 11 PRs to review. The user signed up for this; we mitigate fatigue by tagging each PR's `(crit/high/med/low)` counts in the title so reviewers can prioritise.

## Testing Strategy

- **Per-crate baseline.** Each Wave 2 agent runs `cargo test -p <crate>` before declaring done. Existing tests must continue to pass.
- **New tests for security-critical fixes.** The token-hashing migration in Wave 1 ships with explicit unit tests: a test that `sha256_hex` is stable, a regression test that a plaintext lookup against the post-migration `users.reset_token` returns `None`, a test that the migration is idempotent (running it twice is a no-op).
- **No new tests for cosmetic fixes.** Low-tier doc/dead-code findings don't add tests.
- **Clippy as enforcement.** `cargo clippy -- -D warnings` gates each PR. This catches the bulk of Medium-tier idiom fixes for free.
- **End-to-end smoke.** After Wave 1 merges, run the full Playwright suite locally once to confirm token-hash migration didn't break login / signup / password-reset flows. Wave 2 PRs do not require Playwright re-runs unless they touch auth_ui or admin pages.

## Open Questions

None — design is concrete enough to plan against. The plan-writing step will produce the per-wave task breakdown including the exact subagent prompts for Wave 2.
