# CI/CD GitHub Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single CI workflow with a three-workflow GitHub Actions setup: fast PR checks, full post-merge verification, and tag-triggered releases.

**Architecture:** Three workflow files (`ci.yml`, `ci-main.yml`, `release.yml`) each triggered by different events. PR checks run 5 parallel jobs for fast feedback. Post-merge adds cross-compilation and coverage. Tag push triggers artifact build + GitHub Release creation.

**Tech Stack:** GitHub Actions, Rust (cargo, clippy, rustfmt, cargo-audit, cargo-llvm-cov), wasm-pack, Node 20 (for SDK checks)

**Spec:** `docs/superpowers/specs/2026-04-16-ci-cd-github-actions-design.md`

---

## File Structure

- **Replace:** `.github/workflows/ci.yml` — current single CI workflow, replaced with PR-only checks
- **Create:** `.github/workflows/ci-main.yml` — post-merge verification (cross-platform + coverage)
- **Modify:** `.github/workflows/release.yml` — change trigger from `workflow_dispatch` to tag push `v*`
- **Unchanged:** `.github/workflows/deploy-demo.yml` — not touched by this plan

---

### Task 1: Replace `ci.yml` with PR-Only Checks

**Files:**
- Replace: `.github/workflows/ci.yml`

This replaces the current `ci.yml` entirely. The new version runs 5 parallel jobs on PRs targeting `main`.

- [ ] **Step 1: Replace `.github/workflows/ci.yml` with the new PR workflow**

```yaml
name: CI

on:
  pull_request:
    branches: [main]
    paths:
      - 'crates/**'
      - 'packages/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/ci.yml'

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Format & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: check

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

  test:
    name: Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: test

      - name: Run tests
        run: cargo test --workspace

  wasm:
    name: WASM Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: wasm

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM
        run: cd crates/solobase-web && wasm-pack build --target web --release

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  sdk:
    name: TypeScript SDK
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Check for SDK changes
        id: sdk-changes
        run: |
          CHANGED=$(git diff --name-only ${{ github.event.pull_request.base.sha }}..HEAD -- packages/solobase-js/)
          if [ -z "$CHANGED" ]; then
            echo "skip=true" >> "$GITHUB_OUTPUT"
          else
            echo "skip=false" >> "$GITHUB_OUTPUT"
          fi

      - name: Setup Node
        if: steps.sdk-changes.outputs.skip != 'true'
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Checkout wafer-run (for local dependency)
        if: steps.sdk-changes.outputs.skip != 'true'
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install and build SDK
        if: steps.sdk-changes.outputs.skip != 'true'
        working-directory: packages/solobase-js
        run: |
          npm ci
          npm run build
          npm run lint
```

- [ ] **Step 2: Verify the YAML is valid**

Run from the solobase repo root:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML valid"
```
Expected: `YAML valid`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: replace ci.yml with parallel PR-only checks

Split into 5 parallel jobs: check (fmt+clippy), test, wasm, audit, sdk.
Only triggers on pull_request, not push. Adds concurrency controls
to cancel stale runs."
```

---

### Task 2: Create `ci-main.yml` for Post-Merge Verification

**Files:**
- Create: `.github/workflows/ci-main.yml`

This workflow runs on push to `main` and includes cross-platform compilation + coverage on top of the standard checks.

- [ ] **Step 1: Create `.github/workflows/ci-main.yml`**

```yaml
name: CI Main

on:
  push:
    branches: [main]
    paths:
      - 'crates/**'
      - 'packages/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/ci-main.yml'

concurrency:
  group: ci-main-${{ github.sha }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Format & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: check

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

  test:
    name: Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: test

      - name: Run tests
        run: cargo test --workspace

  wasm:
    name: WASM Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: wasm

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM
        run: cd crates/solobase-web && wasm-pack build --target web --release

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  cross-compile:
    name: Cross-Compile (${{ matrix.name }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            name: linux-amd64
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            name: linux-arm64
            cross: true
          - target: aarch64-apple-darwin
            os: macos-latest
            name: darwin-arm64
          - target: x86_64-apple-darwin
            os: macos-latest
            name: darwin-amd64
          - target: x86_64-pc-windows-gnu
            os: ubuntu-latest
            name: windows-amd64
            cross: true

    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: cross-${{ matrix.name }}

      - name: Install cross-compilation tools (Linux ARM64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y -qq gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
          echo '[target.aarch64-unknown-linux-gnu]' >> ~/.cargo/config.toml
          echo 'linker = "aarch64-linux-gnu-gcc"' >> ~/.cargo/config.toml

      - name: Install cross-compilation tools (Windows)
        if: matrix.target == 'x86_64-pc-windows-gnu'
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y -qq gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64

      - name: Build
        run: cargo build -p solobase --release --target ${{ matrix.target }}

  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: coverage

      - name: Generate coverage
        run: cargo llvm-cov --workspace --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          token: ${{ secrets.CODECOV_TOKEN }}
          fail_ci_if_error: false
```

- [ ] **Step 2: Verify the YAML is valid**

Run from the solobase repo root:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci-main.yml'))" && echo "YAML valid"
```
Expected: `YAML valid`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci-main.yml
git commit -m "ci: add ci-main.yml for post-merge verification

Runs on push to main. Includes all PR checks plus cross-platform
compilation (5 targets) and code coverage upload to Codecov."
```

---

### Task 3: Update `release.yml` to Trigger on Tags

**Files:**
- Modify: `.github/workflows/release.yml`

Change the trigger from `workflow_dispatch` with a manual version input to automatic triggering on `v*` tag pushes. The version is derived from the tag name.

- [ ] **Step 1: Replace the contents of `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build (${{ matrix.name }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            name: solobase-linux-amd64
            ext: tar.gz
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            name: solobase-linux-arm64
            ext: tar.gz
            cross: true
          - target: aarch64-apple-darwin
            os: macos-latest
            name: solobase-darwin-arm64
            ext: tar.gz
          - target: x86_64-apple-darwin
            os: macos-latest
            name: solobase-darwin-amd64
            ext: tar.gz
          - target: x86_64-pc-windows-gnu
            os: ubuntu-latest
            name: solobase-windows-amd64
            ext: zip
            cross: true

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Checkout wafer-run
        run: git clone --depth 1 https://github.com/wafer-run/wafer-run.git "$GITHUB_WORKSPACE/../wafer-run"

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools (Linux ARM64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y -qq gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
          echo '[target.aarch64-unknown-linux-gnu]' >> ~/.cargo/config.toml
          echo 'linker = "aarch64-linux-gnu-gcc"' >> ~/.cargo/config.toml

      - name: Install cross-compilation tools (Windows)
        if: matrix.target == 'x86_64-pc-windows-gnu'
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y -qq gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64

      - name: Build
        run: cargo build -p solobase --release --target ${{ matrix.target }}

      - name: Package (tar.gz)
        if: matrix.ext == 'tar.gz'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf /tmp/${{ matrix.name }}.tar.gz solobase
          echo "ASSET=/tmp/${{ matrix.name }}.tar.gz" >> $GITHUB_ENV

      - name: Package (zip)
        if: matrix.ext == 'zip'
        run: |
          cd target/${{ matrix.target }}/release
          zip /tmp/${{ matrix.name }}.zip solobase.exe
          echo "ASSET=/tmp/${{ matrix.name }}.zip" >> $GITHUB_ENV

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}
          path: ${{ env.ASSET }}

  release:
    name: Publish Release
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          VERSION="${GITHUB_REF_NAME}"
          FILES=$(find artifacts -type f \( -name '*.tar.gz' -o -name '*.zip' \))
          gh release create "$VERSION" $FILES \
            --repo ${{ github.repository }} \
            --title "$VERSION" \
            --generate-notes
```

Key changes from the current `release.yml`:
- Trigger: `workflow_dispatch` with manual version input → `push.tags: v*`
- Version: `v${{ inputs.version }}` → `$GITHUB_REF_NAME` (the tag itself, e.g. `v0.1.0`)
- Release notes: hardcoded template → `--generate-notes` (GitHub auto-generates from PR titles since last release)

- [ ] **Step 2: Verify the YAML is valid**

Run from the solobase repo root:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML valid"
```
Expected: `YAML valid`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: trigger releases on v* tag push instead of workflow_dispatch

Tag push (e.g. git tag v0.1.0 && git push origin v0.1.0) now
triggers the full release build. Uses --generate-notes for
auto-generated release notes from merged PRs."
```

---

### Task 4: Verify All Workflows Locally

**Files:** None (verification only)

- [ ] **Step 1: Validate all three workflow files parse correctly**

```bash
cd solobase
for f in .github/workflows/ci.yml .github/workflows/ci-main.yml .github/workflows/release.yml; do
  python3 -c "import yaml; yaml.safe_load(open('$f'))" && echo "$f: valid" || echo "$f: INVALID"
done
```

Expected output:
```
.github/workflows/ci.yml: valid
.github/workflows/ci-main.yml: valid
.github/workflows/release.yml: valid
```

- [ ] **Step 2: Verify `deploy-demo.yml` was not modified**

```bash
cd solobase && git diff .github/workflows/deploy-demo.yml
```

Expected: no output (no changes)

- [ ] **Step 3: Review the final file list**

```bash
cd solobase && ls -la .github/workflows/
```

Expected files:
- `ci.yml` — PR checks (modified)
- `ci-main.yml` — post-merge verification (new)
- `release.yml` — tag-triggered release (modified)
- `deploy-demo.yml` — unchanged

- [ ] **Step 4: Final commit with all workflows if any uncommitted changes remain**

```bash
cd solobase && git status .github/workflows/
```

If clean, skip. Otherwise:
```bash
git add .github/workflows/
git commit -m "ci: finalize workflow files"
```

---

### Task 5: Configure GitHub Branch Protection (Manual)

**Files:** None — this is GitHub UI/API configuration, not code.

These settings must be configured in the GitHub repository settings after the workflows are pushed to `main`.

- [ ] **Step 1: Go to Settings → Branches → Add branch protection rule for `main`**

Configure:
- Branch name pattern: `main`
- **Require a pull request before merging**: enabled
  - Required approvals: 1
  - Dismiss stale pull request approvals when new commits are pushed: enabled
- **Require status checks to pass before merging**: enabled
  - Required checks: `Format & Lint`, `Tests`, `WASM Build`
  - NOT required (informational): `Security Audit`, `TypeScript SDK`
- **Require branches to be up to date before merging**: enabled
- **Do not allow bypassing the above settings**: enabled

- [ ] **Step 2: Verify by opening a test PR**

Open a trivial PR (e.g. whitespace change) and confirm:
- All 5 CI jobs appear in the checks tab
- The 3 required checks block merge until green
- The 2 informational checks show but don't block
