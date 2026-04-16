# CI/CD GitHub Actions Design for Solobase

## Context

Solobase is an open-source project that will accept PRs from external contributors. The repo is a Rust workspace with 4 crates (`solobase`, `solobase-core`, `solobase-native`, `solobase-web`) and 2 TypeScript packages (`@solobase/sdk`, `solobase-web` JS wrapper). It cross-compiles to Linux amd64/arm64, macOS amd64/arm64, and Windows amd64. The `solobase-web` crate compiles to WASM via `wasm-pack`.

The repo depends on `wafer-run` as a sibling directory (path dependencies in Cargo.toml). CI must clone `wafer-run` to build.

## Branching Model: GitHub Flow

- **PRs target `main`** — fast feedback, standard open-source convention
- **Pushes to `main`** — full cross-platform verification
- **Tags (`v*`)** — trigger release artifact builds + GitHub Release creation
- No long-lived `development` or `staging` branches

## Workflow Structure

Three workflow files, replacing the current `ci.yml` and leaving `release.yml` to be triggered by tags:

### 1. `ci.yml` — PR Checks

**Trigger:** `pull_request` targeting `main`

**Path filter:** `crates/**`, `packages/**`, `Cargo.toml`, `Cargo.lock`

**Jobs (run in parallel):**

#### `check` — Formatting + Linting
- `cargo fmt --all -- --check`
- `cargo clippy --workspace -- -D warnings`
- Runs on `ubuntu-latest`
- Requires cloning `wafer-run` (clippy does full compilation)
- Uses `Swatinem/rust-cache@v2` for caching
- Fastest job — gives instant feedback on style issues

#### `test` — Native Tests
- `cargo test --workspace`
- Runs on `ubuntu-latest`
- Requires cloning `wafer-run` (shallow clone, `--depth 1`)

#### `wasm` — WASM Build Verification
- Installs `wasm32-unknown-unknown` target
- Installs `wasm-pack`
- Runs `wasm-pack build --target web --release` in `crates/solobase-web`
- Does NOT run the full Makefile (no sql.js download, no static file copy) — just verifies the Rust→WASM compilation succeeds
- Runs on `ubuntu-latest`

#### `audit` — Dependency Security Audit
- `cargo audit`
- Uses `rustsec/audit-check@v2` action (avoids reinstalling `cargo-audit` every run)
- Runs on `ubuntu-latest`
- **Not a required check** — advisory-only, so a new CVE in a transitive dep doesn't block all PRs

#### `sdk` — TypeScript SDK Check
- Installs Node 20
- Runs `npm ci && npm run build && npm run lint` in `packages/solobase-js`
- Runs on `ubuntu-latest`
- Only triggers when `packages/solobase-js/**` changes (path filter on the job)

### 2. `ci-main.yml` — Post-Merge Verification

**Trigger:** `push` to `main`

**Path filter:** `crates/**`, `packages/**`, `Cargo.toml`, `Cargo.lock`

Runs everything from `ci.yml` plus:

#### `cross-compile` — Full Platform Matrix
- **Strategy matrix:**
  - `x86_64-unknown-linux-gnu` on `ubuntu-latest`
  - `aarch64-unknown-linux-gnu` on `ubuntu-latest` (cross-compile with `gcc-aarch64-linux-gnu`)
  - `aarch64-apple-darwin` on `macos-latest`
  - `x86_64-apple-darwin` on `macos-latest`
  - `x86_64-pc-windows-gnu` on `ubuntu-latest` (cross-compile with `mingw-w64`)
- Builds `cargo build -p solobase --release --target $TARGET`
- Artifacts are NOT uploaded (that's the release workflow's job)
- Purpose: catch platform-specific compilation failures before tagging a release

#### `coverage` — Code Coverage
- Uses `cargo-llvm-cov` to generate lcov report
- Uploads to Codecov
- Runs only on push to main (not on PRs — saves time for contributors)

### 3. `release.yml` — Tag-Triggered Release

**Trigger:** `push` of tags matching `v*`

Replaces the current `workflow_dispatch`-based release. Same build matrix and artifact packaging as the existing `release.yml`, but triggered automatically by tagging.

**Jobs:**

#### `build` — Cross-Platform Artifact Build
- Same 5-target matrix as current `release.yml`
- Packages binaries as `.tar.gz` (Linux/macOS) or `.zip` (Windows)
- Uploads artifacts

#### `release` — Create GitHub Release
- Downloads all artifacts
- Creates GitHub Release with tag name, download links, and platform list
- Uses `GITHUB_TOKEN` (no extra secrets needed)

## Security for Public Repos

### Preventing CI Abuse
- **`pull_request` (not `pull_request_target`)** — PRs from forks run in a restricted context with no access to secrets. This is the default and correct behavior.
- **No secrets in PR jobs** — the PR workflow doesn't need any. Codecov token is only used in the main-push workflow.
- **Concurrency controls** — each PR gets one CI run at a time; pushing a new commit cancels the in-flight run:
  ```yaml
  concurrency:
    group: ci-${{ github.ref }}
    cancel-in-progress: true
  ```

### Required Status Checks
Configure these as **required** in GitHub branch protection for `main`:
- `check` (fmt + clippy)
- `test`
- `wasm`

Configure these as **not required** (informational):
- `audit` (new CVEs shouldn't block unrelated PRs)
- `sdk` (only relevant when TS files change)

### Branch Protection Rules for `main`
- Require PR reviews (at least 1 approval from maintainer)
- Require status checks to pass before merging
- Require branches to be up to date before merging
- Dismiss stale PR reviews on new pushes
- Do not allow bypassing the above settings (even for admins)

## Caching Strategy

All Rust jobs use `Swatinem/rust-cache@v2` with these settings:
- **Key prefix per job** — so `check`, `test`, `wasm`, and `cross-compile` don't thrash each other's caches
- **Shared cache on `main`** — PR runs read from the `main` branch cache (rust-cache does this automatically)
- Cache includes `~/.cargo/registry`, `~/.cargo/git`, and `target/`

## Dependency: wafer-run

All Rust jobs need `wafer-run` cloned as a sibling:
```yaml
- name: Checkout wafer-run
  run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"
```

This is a public repo shallow clone — no tokens needed, fast (<5s).

## What This Replaces

| Current file | What happens |
|---|---|
| `ci.yml` | Replaced by `ci.yml` (PR) + `ci-main.yml` (push to main) |
| `release.yml` | Updated to trigger on `v*` tags instead of `workflow_dispatch` |
| `deploy-demo.yml` | Unchanged (still deploys WASM demo on push to main) |

## Future Considerations (Not In Scope)

- **MSRV (Minimum Supported Rust Version) check** — pin and test against a specific Rust version once stable users exist
- **Benchmark regression CI** — `cargo bench` comparison against main (add when performance-sensitive code stabilizes)
- **Auto-labeling PRs** — label by path (`crates/solobase-web` → `wasm`, `packages/` → `sdk`)
- **Dependabot** — automated dependency update PRs
