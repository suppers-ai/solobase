# Rust Best-Practices Remediation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 253 findings from `docs/rust-best-practices-review-2026-05-14.md` across seven solobase crates plus upstream root-cause fixes in `wafer-core` and `wafer-sql-utils`.

**Architecture:** Three sequential waves — (0) upstream root-cause fixes in sibling repos, (1) one serial solobase PR adding shared helpers and the token-storage migration, (2) seven parallel per-crate-group worktrees each fixing all findings in their scope. Worktrees branched off `origin/main` post-Wave-1, one PR per group. See `docs/superpowers/specs/2026-05-14-rust-best-practices-fix-design.md` for the full design.

**Tech Stack:** Rust, `wafer-core`/`wafer-run`/`wafer-sql-utils` (sibling repos), `gh` CLI for PRs, `git worktree`.

**Pre-flight assumptions:**
- Repos `/workspace/solobase` and `/workspace/wafer-run` are at clean `main` HEADs that you control.
- `gh auth status` is healthy in both repos.
- `cargo` and the project's pre-commit hooks work in both repos.
- The reference doc `solobase/docs/rust-best-practices-review-2026-05-14.md` is committed and available.

---

## Wave 0 — Upstream root-cause fixes

Three PRs in `wafer-run/` (which contains `wafer-core`, `wafer-run`, and `wafer-sql-utils` as a Cargo workspace). All three are independent and can be raised in parallel.

### Task 0.1: Worktree setup for Wave 0

**Files:**
- Create: `~/.worktrees/wafer-rbp-core/` (worktree, branch `rbp/wafer-core-constructors`)
- Create: `~/.worktrees/wafer-rbp-run/` (worktree, branch `rbp/wafer-run-sort-blocks-snapshot`)
- Create: `~/.worktrees/wafer-rbp-sqlutils/` (worktree, branch `rbp/wafer-sql-utils-coalesce`)

- [ ] **Step 1: Verify wafer-run is clean**

```bash
cd /workspace/wafer-run && git fetch origin && git status --short
```
Expected: empty output (clean working tree). If not clean, stop and tell the user.

- [ ] **Step 2: Create three worktrees**

```bash
cd /workspace/wafer-run
git worktree add -b rbp/wafer-core-constructors          ~/.worktrees/wafer-rbp-core      origin/main
git worktree add -b rbp/wafer-run-sort-blocks-snapshot   ~/.worktrees/wafer-rbp-run       origin/main
git worktree add -b rbp/wafer-sql-utils-coalesce         ~/.worktrees/wafer-rbp-sqlutils  origin/main
git worktree list
```
Expected: three new worktrees listed.

- [ ] **Step 3: Commit**

No commit. Setup task.

### Task 0.2: Wave 0-a — wafer-core constructors

**Files:**
- Modify: `crates/wafer-core/src/interfaces/llm/service.rs` (in `~/.worktrees/wafer-rbp-core/`)
- Modify: `crates/wafer-core/src/interfaces/image/service.rs`
- Test: `crates/wafer-core/src/interfaces/llm/service.rs` (add unit tests at bottom)

The `ChunkDelta` and `TokenUsage` types are `#[non_exhaustive]` and `ModelStatus` is exhaustive but already has `::ready()`. Solobase llm providers can't currently build the tool-call variants from outside the crate without serde round-trips (see `solobase/crates/solobase-core/src/blocks/llm/providers/openai.rs:327-360` for the comment that *literally says* "Until wafer-core grows explicit constructors…"). Add them.

- [ ] **Step 1: Write the failing test**

Append to `crates/wafer-core/src/interfaces/llm/service.rs`, in the existing `#[cfg(test)] mod tests { ... }` block (find the existing block; if none, create one at file end):

```rust
#[test]
fn chat_chunk_tool_call_constructors_build_expected_variants() {
    let start = ChatChunk::tool_call_start("call_1", "search");
    assert!(matches!(
        start.delta,
        ChunkDelta::ToolCallStart { ref id, ref name } if id == "call_1" && name == "search"
    ));
    assert!(start.finish_reason.is_none());

    let args = ChatChunk::tool_call_arguments("call_1", "{\"q\":");
    assert!(matches!(
        args.delta,
        ChunkDelta::ToolCallArguments { ref id, ref arguments_delta }
            if id == "call_1" && arguments_delta == "{\"q\":"
    ));

    let done = ChatChunk::tool_call_complete("call_1");
    assert!(matches!(done.delta, ChunkDelta::ToolCallComplete { ref id } if id == "call_1"));

    let usage = ChatChunk::usage(TokenUsage::new(10, 20));
    assert_eq!(usage.delta, ChunkDelta::Empty);
    assert_eq!(usage.usage.as_ref().map(|u| u.input_tokens), Some(10));
    assert_eq!(usage.usage.as_ref().map(|u| u.output_tokens), Some(20));
}

#[test]
fn token_usage_new_builder_sets_required_fields_and_defaults() {
    let u = TokenUsage::new(7, 11);
    assert_eq!(u.input_tokens, 7);
    assert_eq!(u.output_tokens, 11);
    assert!(u.cached_tokens.is_none());
    assert!(u.reasoning_tokens.is_none());

    let u2 = TokenUsage::new(1, 2).with_cached(3).with_reasoning(4);
    assert_eq!(u2.cached_tokens, Some(3));
    assert_eq!(u2.reasoning_tokens, Some(4));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd ~/.worktrees/wafer-rbp-core
cargo test -p wafer-core --lib chat_chunk_tool_call_constructors_build_expected_variants
cargo test -p wafer-core --lib token_usage_new_builder_sets_required_fields_and_defaults
```
Expected: both fail with "no associated item named `tool_call_start` / `tool_call_arguments` / `tool_call_complete` / `usage` / `new` / `with_cached` / `with_reasoning`".

- [ ] **Step 3: Add the constructors**

Edit `crates/wafer-core/src/interfaces/llm/service.rs`. In the existing `impl ChatChunk { … }` block (around line 257), append after the existing `pub fn delta(...)`:

```rust
    /// A chunk announcing the start of a tool call.
    pub fn tool_call_start(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            delta: ChunkDelta::ToolCallStart { id: id.into(), name: name.into() },
            finish_reason: None,
            usage: None,
        }
    }

    /// A chunk carrying an incremental tool-call arguments delta.
    pub fn tool_call_arguments(id: impl Into<String>, arguments_delta: impl Into<String>) -> Self {
        Self {
            delta: ChunkDelta::ToolCallArguments {
                id: id.into(),
                arguments_delta: arguments_delta.into(),
            },
            finish_reason: None,
            usage: None,
        }
    }

    /// A chunk announcing the end of a tool call.
    pub fn tool_call_complete(id: impl Into<String>) -> Self {
        Self {
            delta: ChunkDelta::ToolCallComplete { id: id.into() },
            finish_reason: None,
            usage: None,
        }
    }

    /// A meta-only chunk carrying just a usage update.
    pub fn usage(usage: TokenUsage) -> Self {
        Self {
            delta: ChunkDelta::Empty,
            finish_reason: None,
            usage: Some(usage),
        }
    }
```

Add a `TokenUsage` impl after the struct definition (around line 256). Because `TokenUsage` is `#[non_exhaustive]`, also accept building with cached / reasoning via fluent methods:

```rust
impl TokenUsage {
    /// Construct with input + output token counts. cached/reasoning default to None.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cached_tokens: None,
            reasoning_tokens: None,
        }
    }

    /// Set the cached-tokens field.
    pub fn with_cached(mut self, cached: u32) -> Self {
        self.cached_tokens = Some(cached);
        self
    }

    /// Set the reasoning-tokens field.
    pub fn with_reasoning(mut self, reasoning: u32) -> Self {
        self.reasoning_tokens = Some(reasoning);
        self
    }
}
```

`ModelStatus::ready()` already exists at `interfaces/llm/service.rs:374` and `interfaces/image/service.rs:127`. Add an `error` constructor next to each existing `ready`:

```rust
    /// A failed-state status carrying an error message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            // …match the existing struct fields; copy from `ready()` and set the
            // appropriate `loaded`/`error` fields. Field shape: read the struct
            // definition immediately above the impl; ModelStatus has
            // `loaded: bool` and `error: Option<String>` (read service.rs around
            // line 104 for image, line ~360 for llm to confirm).
            loaded: false,
            error: Some(message.into()),
            ..Default::default()
        }
    }
```

Note: `ModelStatus` may not have `Default` — if `cargo check` complains, read the struct fields and spell out every one. Read `interfaces/llm/service.rs` around line 350-380 and `interfaces/image/service.rs` around line 104-145 to confirm the exact shape before writing.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd ~/.worktrees/wafer-rbp-core
cargo test -p wafer-core --lib chat_chunk_tool_call_constructors_build_expected_variants
cargo test -p wafer-core --lib token_usage_new_builder_sets_required_fields_and_defaults
cargo check --workspace
cargo clippy -p wafer-core -- -D warnings
```
Expected: tests PASS, workspace check clean, clippy clean.

- [ ] **Step 5: Commit and push**

```bash
cd ~/.worktrees/wafer-rbp-core
git add crates/wafer-core/src/interfaces/llm/service.rs crates/wafer-core/src/interfaces/image/service.rs
git commit -m "$(cat <<'EOF'
feat(wafer-core): tool-call chunk + token-usage + model-status constructors

Adds ChatChunk::{tool_call_start, tool_call_arguments, tool_call_complete,
usage}, TokenUsage::new/with_cached/with_reasoning, and ModelStatus::error.

Consumers (solobase llm providers) currently build #[non_exhaustive] variants
via serde round-trips with .expect("wire shape"); this lets them construct
directly.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git push -u origin rbp/wafer-core-constructors
```

- [ ] **Step 6: Open PR**

```bash
cd ~/.worktrees/wafer-rbp-core
gh pr create --title "feat(wafer-core): typed constructors for ChunkDelta / TokenUsage / ModelStatus" --body "$(cat <<'EOF'
## Summary
- Adds `ChatChunk::{tool_call_start, tool_call_arguments, tool_call_complete, usage}` so consumers can build `#[non_exhaustive]` tool-call variants directly.
- Adds `TokenUsage::new(input, output)` plus fluent `with_cached` / `with_reasoning`.
- Adds `ModelStatus::error(msg)` matching the existing `ready()`.

## Why
Solobase LLM SSE decoders today round-trip through `serde_json::from_value(...).expect("wire shape")` on every chunk (see `solobase/crates/solobase-core/src/blocks/llm/providers/openai.rs:327-360` — comment literally says "Until wafer-core grows explicit constructors…"). Unblocks the W2-E remediation in the 2026-05-14 rust best-practices review.

## Test plan
- [ ] `cargo test -p wafer-core` passes including the two new tests
- [ ] `cargo clippy -p wafer-core -- -D warnings` clean

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
Capture the PR URL — record it inline in this plan under "Wave 0 PR URLs" at the end before moving on.

### Task 0.3: Wave 0-b — wafer-run sort blocks_snapshot

**Files:**
- Modify: `crates/wafer-run/src/runtime/lifecycle.rs:18` (in `~/.worktrees/wafer-rbp-run/`)
- Modify: `crates/wafer-run/src/runtime.rs:310`
- Test: `crates/wafer-run/src/runtime/lifecycle.rs` (bottom)

`blocks_snapshot` is built from `HashMap.values()`, which iterates in SipHash-randomised order. Sort by `BlockInfo.full_name()` (or whatever the canonical ordering field is — check `BlockInfo`'s shape first; the review says block id) so admin pages and any consumer get deterministic output.

- [ ] **Step 1: Inspect BlockInfo to find the canonical sort key**

```bash
cd ~/.worktrees/wafer-rbp-run
grep -rn "pub struct BlockInfo\|impl BlockInfo" crates/wafer-run/src/block.rs crates/wafer-core/src/ 2>/dev/null | head
```
Pick the field that's the stable id (likely `id`, `full_name`, or `name`). For the rest of this task, assume `info.id` — if it's named differently, substitute throughout.

- [ ] **Step 2: Write the failing test**

Append to `crates/wafer-run/src/runtime/lifecycle.rs` in a `#[cfg(test)] mod tests` block (create if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn blocks_snapshot_is_sorted_by_id() {
        // Build a Runtime-shaped struct or fake whose .blocks HashMap has
        // entries inserted in non-alphabetic order, then call the snapshot
        // builder and assert ids come out alphabetically.
        //
        // If a public test seam doesn't exist, write a unit test against the
        // pure sort helper extracted in Step 3.
        let mut input: Vec<&str> = vec!["zeta", "alpha", "mu"];
        input.sort();
        assert_eq!(input, vec!["alpha", "mu", "zeta"]);
        // Replace this stub with a real assertion against the snapshot builder
        // once Step 3 extracts it.
    }
}
```

- [ ] **Step 3: Extract and sort**

Edit `crates/wafer-run/src/runtime/lifecycle.rs:18`. Replace:

```rust
self.blocks_snapshot = Arc::new(self.blocks.values().map(|b| b.info()).collect());
```

with:

```rust
self.blocks_snapshot = Arc::new(sorted_snapshot(self.blocks.values().map(|b| b.info())));
```

And edit `crates/wafer-run/src/runtime.rs:310`. Replace:

```rust
self.blocks.values().map(|b| b.info()).collect()
```

with:

```rust
sorted_snapshot(self.blocks.values().map(|b| b.info()))
```

Add a free function next to one of the call sites (top of `runtime/lifecycle.rs` is fine — both files import from `runtime/lifecycle.rs` already if needed, or add to `runtime.rs` and re-export):

```rust
/// Collect `BlockInfo`s into a Vec sorted by their stable id, so consumers
/// (admin pages, snapshot consumers) see deterministic order regardless of
/// the underlying HashMap's SipHash randomisation.
pub(crate) fn sorted_snapshot(iter: impl IntoIterator<Item = crate::block::BlockInfo>) -> Vec<crate::block::BlockInfo> {
    let mut v: Vec<_> = iter.into_iter().collect();
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v
}
```

Now rewrite the test in Step 2 to actually call `sorted_snapshot`:

```rust
#[test]
fn sorted_snapshot_orders_by_id() {
    use crate::block::BlockInfo;
    let infos = vec![
        // Construct three BlockInfo values with ids "zeta", "alpha", "mu".
        // BlockInfo is likely #[non_exhaustive]; use whatever constructor /
        // builder exists in `crates/wafer-run/src/block.rs`. If none, mark
        // this test #[ignore = "needs BlockInfo test seam"] and file
        // a follow-up note in the PR description.
    ];
    let out = sorted_snapshot(infos);
    let ids: Vec<&str> = out.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, vec!["alpha", "mu", "zeta"]);
}
```

- [ ] **Step 4: Verify**

```bash
cd ~/.worktrees/wafer-rbp-run
cargo test -p wafer-run --lib sorted_snapshot_orders_by_id
cargo check --workspace
cargo clippy -p wafer-run -- -D warnings
```
Expected: PASS / clean. If the BlockInfo test seam is missing, the test will be `#[ignore]` — that's acceptable, mention it in the PR description.

- [ ] **Step 5: Commit, push, PR**

```bash
cd ~/.worktrees/wafer-rbp-run
git add crates/wafer-run/src/runtime.rs crates/wafer-run/src/runtime/lifecycle.rs
git commit -m "$(cat <<'EOF'
fix(wafer-run): sort blocks_snapshot deterministically

HashMap.values() iterates in SipHash-randomised order, which leaks into
Runtime.blocks_snapshot and any admin UI / consumer that derives ordering
from it. Sort by BlockInfo.id at snapshot construction; downstream
consumers no longer need per-call workarounds (e.g. solobase admin/blocks).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git push -u origin rbp/wafer-run-sort-blocks-snapshot

gh pr create --title "fix(wafer-run): sort blocks_snapshot deterministically" --body "$(cat <<'EOF'
## Summary
- Wrap `self.blocks.values().map(|b| b.info())` in `sorted_snapshot()` so `Runtime.blocks_snapshot` and the snapshot returned by `Runtime::registered_blocks()` are sorted by `BlockInfo.id`.
- Eliminates the SipHash-driven non-determinism that drove PR #155's admin/blocks workaround in solobase.

## Test plan
- [ ] `cargo test -p wafer-run` passes (one new unit test on `sorted_snapshot`)
- [ ] `cargo clippy -p wafer-run -- -D warnings` clean

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
Record PR URL.

### Task 0.4: Wave 0-c — wafer-sql-utils coalesce aggregate

**Files:**
- Modify: `crates/wafer-sql-utils/src/aggregate.rs` (in `~/.worktrees/wafer-rbp-sqlutils/`)
- Test: `crates/wafer-sql-utils/src/aggregate.rs` (`#[cfg(test)] mod tests`)

`FilterOp::In` and `query::build_update_where` already exist (review used the wrong names). The only genuinely missing builder is `COALESCE(...)` for products/handlers null-coalesce loops.

- [ ] **Step 1: Inspect the file for the existing `AggFunc` enum**

```bash
cd ~/.worktrees/wafer-rbp-sqlutils
sed -n '130,170p' crates/wafer-sql-utils/src/aggregate.rs
```
Note the existing variants. `AggFunc` lives at line 136; `AggregateColumn` at line 146. We extend both.

- [ ] **Step 2: Write the failing test**

Append to `crates/wafer-sql-utils/src/aggregate.rs` `mod tests` (create if missing):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agg_func_coalesce_emits_coalesce_function_call() {
        let col = AggregateColumn {
            field: "price".to_string(),
            func: AggFunc::Coalesce(serde_json::json!(0)),
            alias: Some("price_or_zero".to_string()),
        };
        let sql = render_agg_for_test(&col, Backend::Sqlite);
        assert!(sql.contains("COALESCE("), "expected COALESCE() in {sql}");
        assert!(sql.contains("price"));
        assert!(sql.contains("0"));
    }
}

/// Test-only helper: feed a single AggregateColumn through the renderer that
/// `build_grouped_query` uses and return the SQL string of just that column.
/// If no such helper exists, instead exercise `build_grouped_query` end-to-end
/// with one column.
#[cfg(test)]
fn render_agg_for_test(col: &AggregateColumn, backend: Backend) -> String {
    let cfg = GroupedQueryConfig {
        table: "items".to_string(),
        group_by: vec!["category".to_string()],
        aggregates: vec![col.clone()],
        filters: vec![],
        sort: vec![],
        limit: None,
        offset: None,
    };
    build_grouped_query(&cfg, backend).0
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd ~/.worktrees/wafer-rbp-sqlutils
cargo test -p wafer-sql-utils --lib agg_func_coalesce_emits_coalesce_function_call
```
Expected: FAIL — `AggFunc::Coalesce` not defined.

- [ ] **Step 4: Add the variant**

Edit `crates/wafer-sql-utils/src/aggregate.rs`. Extend `AggFunc`:

```rust
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    // ... existing variants ...
    /// COALESCE(col, fallback). Fallback is the JSON value that becomes the
    /// SQL literal substituted for NULL.
    Coalesce(serde_json::Value),
}
```

Then in the SQL-emitting code (find where `AggFunc::Sum` is rendered — likely inside `build_grouped_query` around line 222), add a `Coalesce(default)` arm that emits `Expr::expr(Func::cust("COALESCE").args([Expr::col(...), Expr::val(json_to_sea_value(default))]))` (or whatever the sea_query idiom is in this file — match existing style).

If sea_query exposes a `Func::coalesce` directly, prefer that.

- [ ] **Step 5: Verify**

```bash
cd ~/.worktrees/wafer-rbp-sqlutils
cargo test -p wafer-sql-utils
cargo check --workspace
cargo clippy -p wafer-sql-utils -- -D warnings
```
Expected: all pass, clean.

- [ ] **Step 6: Commit, push, PR**

```bash
cd ~/.worktrees/wafer-rbp-sqlutils
git add crates/wafer-sql-utils/src/aggregate.rs
git commit -m "$(cat <<'EOF'
feat(wafer-sql-utils): COALESCE aggregate

Adds AggFunc::Coalesce(default) so consumers can fold null-handling into
the aggregate-builder pipeline instead of post-processing rows in Rust.

Unblocks the handlers/products null-coalesce loop cleanup in the 2026-05-14
rust best-practices review.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git push -u origin rbp/wafer-sql-utils-coalesce

gh pr create --title "feat(wafer-sql-utils): COALESCE aggregate" --body "$(cat <<'EOF'
## Summary
- Add `AggFunc::Coalesce(default)` rendering to `COALESCE(col, default)`.

## Test plan
- [ ] `cargo test -p wafer-sql-utils` passes (new test asserts emitted SQL)
- [ ] `cargo clippy -p wafer-sql-utils -- -D warnings` clean

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
Record PR URL.

### Task 0.5: Merge Wave 0

- [ ] **Step 1: Wait for review approval on all three PRs**

This is a human gate. The PRs may be merged in any order — they don't depend on each other.

- [ ] **Step 2: Confirm all three merged**

```bash
cd /workspace/wafer-run && git fetch origin && git log --oneline origin/main | head -5
```
Expected: see the three `rbp/*` merge commits.

- [ ] **Step 3: Clean up worktrees**

```bash
cd /workspace/wafer-run
git worktree remove ~/.worktrees/wafer-rbp-core
git worktree remove ~/.worktrees/wafer-rbp-run
git worktree remove ~/.worktrees/wafer-rbp-sqlutils
```

---

## Wave 1 — Solobase shared foundations (1 serial PR)

One worktree, one PR. Lands all the cross-cutting fixes so Wave 2 parallel agents can call into stable helpers.

### Task 1.1: Worktree setup for Wave 1

- [ ] **Step 1: Verify solobase is clean**

```bash
cd /workspace/solobase && git fetch origin && git status --short
```
Expected: empty. If not clean, stop — the design-spec commit (43e53ba on main) absorbed the user's prior staged work; do not let further unrelated changes leak into Wave 1.

- [ ] **Step 2: Create the worktree**

```bash
cd /workspace/solobase
git worktree add -b rbp/wave1-foundations ~/.worktrees/solobase-rbp-wave1 origin/main
```

### Task 1.2: Bump wafer-run dependency

**Files:**
- Modify: `Cargo.toml` (workspace root in `~/.worktrees/solobase-rbp-wave1/`)
- Modify: `Cargo.lock`

- [ ] **Step 1: Check how wafer-run is declared**

```bash
cd ~/.worktrees/solobase-rbp-wave1
grep -n "wafer-core\|wafer-run\|wafer-sql-utils" Cargo.toml
```

- [ ] **Step 2: Update to a commit at-or-after the three Wave 0 merges**

If declared by git rev, bump the rev to the post-Wave-0 `main` SHA. If declared by path or workspace, just `cargo update -p wafer-core -p wafer-run -p wafer-sql-utils`.

```bash
cargo update -p wafer-core -p wafer-run -p wafer-sql-utils
cargo check --workspace
```
Expected: builds clean with the new constructors / sort / Coalesce variant available.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump wafer-core / wafer-run / wafer-sql-utils for Wave 0 constructors"
```

### Task 1.3: Add canonical `sha256_hex` helper

**Files:**
- Inspect first: `crates/solobase-core/src/blocks/auth/service.rs:106-108` (existing `hash_token` / `bootstrap::sha256` / `helpers::sha256_hex` triplicates)
- Modify (consolidate target): pick *one* — `crates/solobase-core/src/blocks/auth/helpers.rs`
- Modify (callers): all three existing copies become re-exports or are deleted
- Test: `crates/solobase-core/src/blocks/auth/helpers.rs`

- [ ] **Step 1: Read existing implementations**

```bash
cd ~/.worktrees/solobase-rbp-wave1
grep -rn "fn sha256\|fn hash_token" crates/solobase-core/src/blocks/auth/
```
Read each. They should already all be `sha2::Sha256::digest(input).iter().fold(...)` style. If they differ in any way (different encoding, different input handling) that difference IS a bug — the canonical one returns lowercase hex of `sha256(input.as_bytes())`.

- [ ] **Step 2: Write the failing test**

Append to `crates/solobase-core/src/blocks/auth/helpers.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_known_value() {
        // sha256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_hex_empty_string() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
```

- [ ] **Step 3: Run test to verify it fails or passes against the existing helper**

```bash
cargo test -p solobase-core --lib sha256_hex_known_value
```
If PASS — the canonical helper is already there. Skip to Step 5.
If FAIL — make it pass:

- [ ] **Step 4: Write or harden the canonical implementation**

In `crates/solobase-core/src/blocks/auth/helpers.rs`:

```rust
use sha2::{Digest, Sha256};

/// Canonical SHA-256 hex digest of `input`. Lowercase, 64 chars.
/// Used everywhere we store a "hash of a secret token" (PATs, session ids,
/// reset tokens, verification tokens).
pub fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut s = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(s, "{byte:02x}");
    }
    s
}
```

- [ ] **Step 5: Consolidate**

Find every other definition (`auth/service.rs:106-108`, `auth/bootstrap.rs:sha256`, anywhere else `grep` finds in `crates/solobase-core/src/blocks/auth/`). Delete each one. Update each caller to import `crate::blocks::auth::helpers::sha256_hex`.

```bash
grep -rn "fn sha256\|fn hash_token" crates/solobase-core/src/blocks/auth/
```
Expected after: only the one in `helpers.rs` remains.

- [ ] **Step 6: Verify**

```bash
cargo test -p solobase-core --lib sha256_hex
cargo check --workspace
cargo clippy -p solobase-core -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/solobase-core/src/blocks/auth/
git commit -m "refactor(auth): consolidate sha256_hex canonical helper"
```

### Task 1.4: Token-storage migration

**Files:**
- Read first: existing migration runner pattern under `crates/solobase-core/src/blocks/auth*/migrations.rs` (or wherever auth schema versioning lives)
- Create: new migration file (e.g. `crates/solobase-core/src/blocks/auth_ui/migrations/NNNN_hash_user_tokens.sql` — match the existing naming scheme)
- Modify: every write/read site for `users.reset_token`, `users.verification_token`, `pats.token_hash`

- [ ] **Step 1: Map existing migration scheme**

```bash
cd ~/.worktrees/solobase-rbp-wave1
find crates/solobase-core/src/blocks/auth crates/solobase-core/src/blocks/auth_ui -name "migrations*" -o -name "*.sql" 2>/dev/null
```
Inspect what the project does for additive migrations. There is one — see the review's `migration_helper.rs:144-167` reference. Follow that file's pattern exactly.

- [ ] **Step 2: Write the migration**

The migration must be **idempotent**. A row whose token is already 64 lowercase hex chars must not be re-hashed. Use a `WHERE length(reset_token) != 64 OR reset_token GLOB '*[^0-9a-f]*'` style guard. For SQLite + D1 syntax this is:

```sql
-- 2026-05-14 hash plaintext user reset/verification tokens.
-- Idempotent: rows already 64 lowercase-hex chars are skipped.

UPDATE users
   SET reset_token = lower(hex(sha256(reset_token)))
 WHERE reset_token IS NOT NULL
   AND reset_token != ''
   AND (length(reset_token) != 64
        OR reset_token GLOB '*[^0-9a-f]*');

UPDATE users
   SET verification_token = lower(hex(sha256(verification_token)))
 WHERE verification_token IS NOT NULL
   AND verification_token != ''
   AND (length(verification_token) != 64
        OR verification_token GLOB '*[^0-9a-f]*');

-- PAT token_hash was serialised as JSON byte-array; backfill to hex.
-- Detected by leading '[' character.
UPDATE pats
   SET token_hash = lower(hex(sha256(token_hash)))
 WHERE token_hash IS NOT NULL
   AND substr(token_hash, 1, 1) = '[';
```

If SQLite-on-D1 doesn't expose `sha256()` natively, the migration needs to be a Rust function executed against the rows rather than pure SQL. Check what runner is in use:

```bash
grep -rn "fn migrate\|fn apply\|run_migration" crates/solobase-core/src/migration_helper.rs crates/solobase-core/src/blocks/auth*/
```
If the project's migration runner is Rust-side, write a Rust migration that:
1. `db::list_all(USERS_TABLE, vec![])` (or paginated equivalent)
2. for each row: if `reset_token` is set and not already hex64, `update(... reset_token = sha256_hex(reset_token))`
3. same for `verification_token`
4. Then for `pats`: load each row, if `token_hash` starts with `[`, parse the JSON array, sha256 the bytes, hex-encode, update.

The exact mechanism depends on the existing runner — read it first.

- [ ] **Step 3: Update every WRITE site to use the helper**

Sites enumerated in the review:
- `crates/solobase-core/src/blocks/auth_ui/api/forgot_password.rs:38-50` — write `sha256_hex(&token)` to `users.reset_token`, return the *plain* token to the email-link.
- `crates/solobase-core/src/blocks/auth_ui/api/reset_password.rs:41-50` — lookup by `sha256_hex(&supplied_token)`.
- `crates/solobase-core/src/blocks/auth_ui/api/verify.rs:46-83` — same pattern for `verification_token`.
- `crates/solobase-core/src/blocks/auth_ui/api/signup.rs:140-194` — write hashed verification_token on signup.
- `crates/solobase-core/src/blocks/auth/repo/pats.rs:95,143,173,199` — `token_hash` serialised as `hex_encode(&new.token_hash)` instead of `json!(new.token_hash)`.

Edit each to call `sha256_hex` on write and lookup. Show the patch for the first one as the template; agents should follow the pattern for the rest.

`forgot_password.rs` (around line 38):
```rust
// Before:
// let token = generate_token();
// db::update(... set users.reset_token = token ...)

// After:
let raw = generate_token();
let hashed = sha256_hex(&raw);
db::update(... set users.reset_token = hashed ...);
// Send `raw` in the email; never write it back.
```

`reset_password.rs` (around line 41):
```rust
// Before:
// let row = db::get_by_field(USERS_TABLE, "reset_token", &supplied)?;

// After:
let hashed = sha256_hex(&supplied);
let row = db::get_by_field(USERS_TABLE, "reset_token", &hashed)?;
```

Apply the same shape at every other site.

- [ ] **Step 4: Tests**

Add unit tests at `crates/solobase-core/src/blocks/auth_ui/api/reset_password.rs` (or wherever feels natural) confirming that:
- a plaintext lookup against a stored-hashed value returns `None`
- a `sha256_hex(plain)` lookup against the same stored value returns the row

If wiring full tests against the live DB is hard, at minimum add a unit test on `sha256_hex` (already in Task 1.3) and a property-style test that `len(sha256_hex(x)) == 64`.

```bash
cargo test -p solobase-core
cargo check --workspace
cargo clippy -p solobase-core -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-core/src/blocks/auth crates/solobase-core/src/blocks/auth_ui
git commit -m "$(cat <<'EOF'
fix(auth): hash plaintext reset/verification/PAT tokens at rest

users.reset_token and users.verification_token were stored plaintext, so
any DB-read primitive (admin SQL explorer, backup leak, log dump, any
block with a read grant on suppers_ai__auth__users) became a password
reset oracle. pats.token_hash was JSON-byte-array serialised, the rest of
the auth surface hex-encodes.

All three now go through auth::helpers::sha256_hex on write; lookups
compare sha256(supplied) against the stored hash. Migration is idempotent
on already-hashed rows.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.5: Fix `solobase-native::env::filter_app_env_vars`

**Files:**
- Modify: `crates/solobase-native/src/env.rs:38-44`
- Test: `crates/solobase-native/src/env.rs`

- [ ] **Step 1: Read the current implementation**

```bash
sed -n '30,60p' crates/solobase-native/src/env.rs
```

Per CLAUDE.md: `SOLOBASE_SHARED__*` and `{ORG}__{BLOCK}__*` are app config. `SOLOBASE_*` without `__` is infra. The current filter strips every `SOLOBASE_*`, killing legitimate app config — PR #155's `auto_bootstrap_if_needed` reads `BOOTSTRAP_ADMIN_*` directly via `std::env::var` to dodge this.

- [ ] **Step 2: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k,v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn filter_keeps_shared_app_config() {
        let input = env(&[
            ("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL", "admin@example.com"),
            ("SOLOBASE_DATABASE_URL", "sqlite://x"), // infra, drop
            ("SUPPERS_AI__AUTH__JWT_SECRET", "abc"),  // block-scoped app config, keep
        ]);
        let out = filter_app_env_vars(input);
        assert!(out.contains_key("SOLOBASE_SHARED__AUTH__BOOTSTRAP_ADMIN_EMAIL"));
        assert!(out.contains_key("SUPPERS_AI__AUTH__JWT_SECRET"));
        assert!(!out.contains_key("SOLOBASE_DATABASE_URL"));
    }
}
```

- [ ] **Step 3: Make it pass**

Change the rule to "keep keys containing `__`":

```rust
pub fn filter_app_env_vars(vars: HashMap<String, String>) -> HashMap<String, String> {
    vars.into_iter()
        .filter(|(k, _)| k.contains("__"))
        .collect()
}
```

- [ ] **Step 4: Verify and clean up the PR #155 workaround**

```bash
grep -n "std::env::var" crates/solobase-native/src/env.rs crates/solobase-core/src/blocks/auth/bootstrap.rs
```
Where `auto_bootstrap_if_needed` (or whatever PR #155 introduced) reads `BOOTSTRAP_ADMIN_*` via `std::env::var`, that can now go through the filter again. Update those call sites.

```bash
cargo test -p solobase-native
cargo check --workspace
cargo clippy -p solobase-native -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/solobase-native crates/solobase-core/src/blocks/auth
git commit -m "$(cat <<'EOF'
fix(native): env filter keeps SOLOBASE_SHARED__* + block-scoped keys

Rule changes from 'strip every SOLOBASE_* key' to 'keep every key
containing __'. Per CLAUDE.md, SOLOBASE_SHARED__* and {ORG}__{BLOCK}__*
are app config; only plain SOLOBASE_* (no __) is infra.

Removes the std::env::var workaround PR #155 added to bypass this filter.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.6: Drop `set_var("SOLOBASE_RUN_MIGRATIONS", "1")` smuggle

**Files:**
- Modify: `crates/solobase/src/main.rs:59,70`
- Modify: `crates/solobase/src/cli/server.rs` (consumer of the env var)
- Modify: any other `Cli` dispatch points

- [ ] **Step 1: Read the current shape**

```bash
grep -rn "SOLOBASE_RUN_MIGRATIONS\|run_migrations" crates/solobase/src/
```

- [ ] **Step 2: Add an explicit field**

Thread `run_migrations: bool` through the dispatch chain. The CLI subcommand that previously set the env var now sets the bool directly; `server::run` takes the bool as a parameter instead of reading the env.

```rust
// Before (main.rs):
// std::env::set_var("SOLOBASE_RUN_MIGRATIONS", "1");
// cli::dispatch(...).await?;

// After:
let run_migrations = matches!(cli.command, Some(Commands::Migrate { .. }));
cli::dispatch(cli, RunOptions { run_migrations }).await?;
```

Update `server::run` signature; remove `std::env::var("SOLOBASE_RUN_MIGRATIONS")` reads.

- [ ] **Step 3: Test**

```bash
cargo check --workspace
cargo test -p solobase
cargo clippy -p solobase -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add crates/solobase
git commit -m "refactor(cli): thread run_migrations explicitly, drop env smuggle (Rust 2024)"
```

### Task 1.7: Open the Wave 1 PR

- [ ] **Step 1: Push**

```bash
cd ~/.worktrees/solobase-rbp-wave1
git push -u origin rbp/wave1-foundations
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --title "chore(rbp): wave 1 shared foundations (sha256_hex, token migration, env filter, run-migrations refactor)" --body "$(cat <<'EOF'
## Summary
First wave of the 2026-05-14 Rust best-practices remediation.
- Canonical `sha256_hex` helper consolidating three pre-existing copies.
- Idempotent migration hashing `users.reset_token`, `users.verification_token`, `pats.token_hash` (was plaintext / JSON-byte-array).
- `solobase-native::env::filter_app_env_vars` now keeps any `__`-containing key (drops the PR #155 std::env::var workaround).
- Threads `run_migrations: bool` explicitly through CLI dispatch instead of `std::env::set_var` (Rust 2024 makes the latter unsound).
- Bumps wafer-core / wafer-run / wafer-sql-utils to pick up Wave 0.

## Test plan
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] Manual: login → request password reset → reset link arrives → consume link → succeeds (validates hashed-token round trip in the new code, not just the migration)
- [ ] Manual: re-run migration → no rows changed (idempotency)

## Follow-up
Wave 2 (seven parallel per-crate-group PRs) starts after this merges.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
Record PR URL.

### Task 1.8: Merge Wave 1

- [ ] **Step 1: Wait for review and merge**

Human gate.

- [ ] **Step 2: Clean up worktree**

```bash
cd /workspace/solobase
git fetch origin
git log --oneline origin/main | head -3
git worktree remove ~/.worktrees/solobase-rbp-wave1
```

---

## Wave 2 — Seven parallel per-crate-group worktrees

Each group is dispatched as a fresh subagent in its own worktree. The agents work in parallel; each produces one PR.

### Task 2.0: Pre-flight — extract per-group WORK_SCOPE files

Each worktree gets a `WORK_SCOPE.md` containing **only** the review's section for that group, with paths normalised to the worktree-relative form. This is the agent's single source of truth — it does not re-read the master 449-line review.

- [ ] **Step 1: Create seven worktrees**

```bash
cd /workspace/solobase
git fetch origin
for grp in A B C D E F G; do
  git worktree add -b "rbp/wave2-$grp" "$HOME/.worktrees/solobase-rbp-wave2-$grp" origin/main
done
git worktree list | grep wave2
```
Expected: seven worktrees.

- [ ] **Step 2: Slice the review into WORK_SCOPE files**

Source: `/workspace/solobase/docs/rust-best-practices-review-2026-05-14.md` (on `main`, post-Wave-1). For each group, copy the corresponding section verbatim into `WORK_SCOPE.md` at the root of the worktree.

| Group | Review section heading in source doc |
|-------|--------------------------------------|
| A | `## solobase-core: root modules` |
| B | `## solobase-core/blocks: admin + utility blocks` |
| C | `## solobase-core/blocks: auth + auth_ui` |
| D | `## solobase-core/blocks: products + files + legalpages + userportal` |
| E | `## solobase-core/blocks: vector + llm + messages` |
| F | `## solobase + solobase-native` |
| G | `## solobase-browser + solobase-web + solobase-cloudflare` |

For each group:
```bash
# Example for group A — adapt the awk range or sed range for each section.
awk '/^## solobase-core: root modules$/,/^---$/' \
    /workspace/solobase/docs/rust-best-practices-review-2026-05-14.md \
    > ~/.worktrees/solobase-rbp-wave2-A/WORK_SCOPE.md
```

The `awk` pattern matches from the section header to the next `---` separator. Verify each `WORK_SCOPE.md` is non-empty and ends at the section break — `wc -l` should be roughly proportional to the section's size in the master doc.

Prepend each `WORK_SCOPE.md` with a short instruction header:

```markdown
# Wave 2 Group <X> — Work Scope

You are fixing every finding below (Critical / High / Medium / Low) in this worktree. Cross-cutting helpers (sha256_hex, env filter, sorted blocks_snapshot, ChunkDelta constructors, COALESCE aggregate) already landed in Waves 0 and 1 — use them. Do NOT reinvent them.

Verification before declaring done:
- `cargo check --workspace`
- `cargo clippy -p <each touched crate> -- -D warnings`
- `cargo test -p <each touched crate>`
- `cargo fmt --check`
- For wasm-only crates: `cargo check --target wasm32-unknown-unknown -p <crate>`

Commit in small batches grouped by finding-cluster (e.g. "fix(auth): hash PAT token_hash"). One PR for the whole group at the end.

---

(Below: verbatim review section.)
```

Commit each `WORK_SCOPE.md`:
```bash
for grp in A B C D E F G; do
  cd "$HOME/.worktrees/solobase-rbp-wave2-$grp"
  git add WORK_SCOPE.md
  git commit -m "chore(rbp-$grp): work scope for wave 2 group $grp"
done
```

### Task 2.1–2.7: Dispatch seven parallel subagents

These seven tasks run in parallel. Each is launched with one `Agent` tool call in a **single** message containing all seven calls.

For each group `<X>` in {A, B, C, D, E, F, G}, dispatch:

```
Agent({
  description: "Wave 2 group <X> rust best-practices fixes",
  subagent_type: "general-purpose",
  prompt: <<EOF
You are working in the worktree at ~/.worktrees/solobase-rbp-wave2-<X> on branch rbp/wave2-<X>.

Your work scope is in WORK_SCOPE.md at the root of the worktree. Read it now.

Your job: fix every finding (Critical, High, Medium, Low) in WORK_SCOPE.md. Use the project conventions in solobase/CLAUDE.md (no panic/unwrap/expect in production, no sync bridges, no raw SQL outside the documented exceptions, no hardcoded domain values in blocks, terse comments — comments only where the WHY is non-obvious).

Cross-cutting helpers are already merged in Wave 1: `crate::blocks::auth::helpers::sha256_hex`, the new env-filter behaviour in solobase-native, sorted blocks_snapshot in wafer-run, ChatChunk::tool_call_* and TokenUsage::new in wafer-core, and AggFunc::Coalesce in wafer-sql-utils. Use them; do not reinvent.

Process:
1. Read WORK_SCOPE.md end-to-end before you start, so you see the whole picture.
2. Group nearby findings (same file or same module) into logical commits. One commit per cluster, not one per finding.
3. After each commit, run `cargo check --workspace` and `cargo clippy -p <touched crate> -- -D warnings`. Fix anything new before moving on.
4. When the entire scope is fixed, run the full verification:
   - `cargo check --workspace`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo test -p <each touched crate>`
   - `cargo fmt --check`
   - For wasm crates in scope (group G only): `cargo check --target wasm32-unknown-unknown -p <crate>`
5. Open the PR with `gh pr create`. Title format: `chore(rbp-<X>): rust best-practices remediation (<crit>/<high>/<med>/<low>)`. Body must list each commit with a one-line summary of the cluster.

Constraints:
- Stay inside your worktree. Do not touch files outside the crate scope your WORK_SCOPE.md describes.
- If a finding cross-references code in another group (e.g. consolidating with crypto.rs from group A), skip that finding and note it in the PR body under "Deferred to <other group>".
- If you find a finding's premise is now obsolete (Wave 1 already fixed it), note that in the PR body under "Already fixed by Wave 1" and skip.
- Do NOT amend commits or force-push. Always create new commits.
- Do NOT skip pre-commit hooks.
- Do NOT push to main directly. Only `git push -u origin rbp/wave2-<X>` then `gh pr create`.

Report back when the PR is open. Include the PR URL and a one-paragraph summary of what shipped.
EOF
})
```

Replace `<X>` and the bullets in `<crit>/<high>/<med>/<low>` per group. The seven Agent calls go in a single message so they run concurrently.

- [ ] **Task 2.1: Dispatch group A** — solobase-core root
- [ ] **Task 2.2: Dispatch group B** — admin + utility blocks
- [ ] **Task 2.3: Dispatch group C** — auth + auth_ui
- [ ] **Task 2.4: Dispatch group D** — products + files + legalpages + userportal
- [ ] **Task 2.5: Dispatch group E** — vector + llm + messages
- [ ] **Task 2.6: Dispatch group F** — solobase + solobase-native CLI
- [ ] **Task 2.7: Dispatch group G** — solobase-browser + solobase-web + solobase-cloudflare

### Task 2.8: Review subagent PRs

- [ ] **Step 1: For each returned PR**

Pull the PR locally, run `cargo test --workspace` and a manual smoke test. Don't trust the subagent's verification claim — re-run.

- [ ] **Step 2: Merge serially**

Merge in dependency order. Group A's helpers may be referenced by other groups, so merge A first if its PR is ready. Each subsequent PR rebases on the updated `main` if there are conflicts.

```bash
cd /workspace/solobase
git fetch origin
git checkout main
git pull
# Inspect each PR via `gh pr view <num> --web` or `gh pr diff <num>`.
```

- [ ] **Step 3: Clean up worktrees**

```bash
for grp in A B C D E F G; do
  cd /workspace/solobase
  git worktree remove "$HOME/.worktrees/solobase-rbp-wave2-$grp"
done
```

---

## Wave 0 / Wave 1 / Wave 2 PR URL register

Track here as PRs open so the user can audit one place.

- Wave 0-a (wafer-core constructors): _to fill_
- Wave 0-b (wafer-run sort): _to fill_
- Wave 0-c (wafer-sql-utils coalesce): _to fill_
- Wave 1 (solobase foundations): _to fill_
- Wave 2-A: _to fill_
- Wave 2-B: _to fill_
- Wave 2-C: _to fill_
- Wave 2-D: _to fill_
- Wave 2-E: _to fill_
- Wave 2-F: _to fill_
- Wave 2-G: _to fill_
