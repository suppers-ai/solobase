# solobase-web Asset Versioning Design

**Date:** 2026-04-18
**Status:** Design approved; pending implementation plan
**Surface area:** `crates/solobase-web/` (standalone static-site build) and `packages/solobase-web/` (npm package)

## Problem

`solobase-web` ships a Service Worker (`sw.js`) that loads a Rust-compiled WASM binary (`solobase_web_bg.wasm`) plus its wasm-bindgen glue (`solobase_web.js`). Today:

- `sw.js` imports `solobase_web.js` by a stable, unhashed URL.
- Rebuilding the Rust crate produces new WASM + glue bytes at the same URLs.
- The browser's SW update check compares `sw.js` bytes only. When only the WASM changes, `sw.js` is byte-identical → no SW update fires → users keep running old WASM until their HTTP cache evicts it.
- No cache-busting on any asset. Deployment targets are expected to be arbitrary (GitHub Pages, Cloudflare, S3, local files), so we cannot rely on server-side `Cache-Control` headers.

We need a versioning strategy that guarantees:

1. Any code change (Rust or JS) produces a new SW that the browser installs.
2. Long-lived assets (WASM, glue, sql.js) can be cached aggressively without risk of staleness.
3. No server-side configuration is required. It works on any static host.
4. Developers consuming `solobase-web` as an npm package can integrate with their own bundler's asset pipeline.

## Non-Goals

- Staged rollouts, A/B, or emergency rollback infrastructure.
- Opt-in update banner UX (silent-on-next-navigation is the chosen UX for the standalone site).
- Safari ≤ 15.3 fallback (`updateViaCache: 'none'` has been supported since Safari 15.4).
- Pre-caching assets into a `Cache` storage. Browser HTTP cache + hashed URLs suffice.
- Hashing `loader.js`, `index.html`, or `sw.js` themselves — these are stable entry points.

## Chosen Approach

**Content-hashed filenames for long-lived assets, with a templated `sw.js` that embeds the hashed paths at build time.** Entry points (`index.html`, `loader.js`, `sw.js`) stay at stable URLs. The browser's SW update check then Just Works: any code change produces a new hashed import path inside `sw.js` → byte diff → SW install → `skipWaiting` + `clients.claim` (already present) rolls out silently.

Two alternatives were considered and rejected:

- **Build-ID stamp with unhashed filenames + `?v=` query strings** — simpler, but query-string cache keys are unreliable on some static hosts (S3 default config strips them); no-op rebuilds churn caches.
- **Runtime version manifest fetched on SW install** — most flexible, but adds a round-trip and code complexity for rollback/rollout features we don't need.

## Architecture

### Surface 1 — Standalone `crates/solobase-web/pkg/` (static-site deployment)

After build:

```
pkg/
├── index.html                            (stable, from template)
├── loader.js                             (stable, hand-written)
├── sw.js                                 (stable URL; bytes change per build, from template)
├── ai-bridge.js                          (stable, hand-written)
├── manifest.json                         (stable, hand-written)
├── asset-manifest.json                   (build metadata — logical → hashed paths)
├── solobase_web-<hash>.js                (wasm-bindgen glue, renamed)
├── solobase_web_bg-<hash>.wasm           (compiled Rust, renamed)
├── solobase_web.d.ts                     (types; not loaded by browser)
├── snippets/<wasm-pack-hash>/bridge.js   (already content-addressed by wasm-pack)
├── sql-wasm-esm-<hash>.js                (sql.js ESM wrapper, renamed)
└── sql-wasm-<hash>.wasm                  (sql.js binary, renamed)
```

Hashing uses the first 8 hex characters of SHA-256 of the file bytes.

### Surface 2 — npm package `packages/solobase-web/`

The package does not apply hashing. It ships wasm-pack output unmodified and exposes update-lifecycle helpers:

```
packages/solobase-web/dist/
├── index.js                 (composable API)
├── worker.js                (SW entrypoint)
├── update.js                (new — main-thread helpers)
├── *.d.ts
└── wasm/
    ├── solobase_web.js
    ├── solobase_web_bg.wasm
    └── snippets/…
```

Consumer bundlers (Vite, Rollup, webpack 5, esbuild) hash the `.wasm` at their own build time via the `new URL('solobase_web_bg.wasm', import.meta.url)` pattern that wasm-pack's `--target web` emits.

## Build Pipeline

### New: `crates/solobase-web-bundle/` (binary crate)

A small Rust binary that runs after `wasm-pack build` and performs the hashing pass. Rust (not shell) because the tool must rewrite string literals inside `solobase_web.js` — fragile in `sed`, trivial and testable in Rust.

Responsibilities:

1. For each asset in the hash set (`solobase_web.js`, `solobase_web_bg.wasm`, `sql-wasm.js`, `sql-wasm.wasm`, `sql-wasm-esm.js`):
   a. Compute SHA-256 of file bytes; truncate to 8 hex chars.
   b. Rename `foo.ext` → `foo-<hash>.ext`.
2. Rewrite embedded cross-references:
   - Inside the renamed `solobase_web.js`, replace the literal `'solobase_web_bg.wasm'` with `'solobase_web_bg-<hash>.wasm'`.
   - Inside the renamed `sql-wasm-esm.js`, replace the literal `'sql-wasm.wasm'` with `'sql-wasm-<hash>.wasm'`.
   - Assert each expected reference exists before rewriting; fail loudly if wasm-pack output shape changes in future toolchain versions.
3. Leave `snippets/<wasm-pack-hash>/` untouched — wasm-pack has already content-addressed this path.
4. Emit `pkg/asset-manifest.json`:
   ```json
   {
     "buildId": "a1b2c3d4",
     "assets": {
       "solobase_web.js":       "/solobase_web-a1b2c3d4.js",
       "solobase_web_bg.wasm":  "/solobase_web_bg-e5f6a7b8.wasm",
       "sql-wasm-esm.js":       "/sql-wasm-esm-c9d0e1f2.js"
     }
   }
   ```
5. Render `js/sw.js.tmpl` and `js/index.html.tmpl` to `pkg/sw.js` and `pkg/index.html`, substituting `__BUILD_ID__` and one `__<LOGICAL_NAME>__` placeholder per manifest entry.

`buildId` format: the 8-char git short SHA from `git rev-parse --short=8 HEAD`. If the working tree is dirty, append `-dirty`. If the build is run outside a git repo, fall back to a SHA-256-8 of the concatenated asset hashes. Determinism holds: identical source commit → identical `buildId` → identical `sw.js` bytes → no unnecessary SW churn.

Determinism: given identical input bytes, the tool produces byte-identical output. No-op rebuilds do not invalidate caches.

### Templates

`js/sw.js.tmpl` replaces `js/sw.js`. The only diff from today is the import line and a build-id comment at the top:

```js
// @generated build: __BUILD_ID__
import init, { initialize, handle_request } from '__WASM_JS__';
// ... rest unchanged: ensureInitialized, handleLocalLlm, fetch handler ...
```

`js/index.html.tmpl` replaces `js/index.html`. Adds a fallback cache hint but is otherwise identical:

```html
<meta http-equiv="Cache-Control" content="no-cache">
```

The meta hint is not binding on most caches; it's a belt-and-braces signal for local dev servers and naive proxies. The real guarantee comes from `index.html` referencing hashed assets by path.

### Makefile

```makefile
build: pkg/sql-wasm-esm.js
    wasm-pack build --target web --release --out-dir pkg
    cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
    cargo run -p solobase-web-bundle --release -- pkg/

dev: pkg/sql-wasm-esm.js
    wasm-pack build --target web --dev --out-dir pkg
    cp js/loader.js js/ai-bridge.js js/manifest.json pkg/
    cargo run -p solobase-web-bundle --release -- pkg/ --dev
```

`--dev` mode skips hashing (files keep canonical names) so local iteration isn't cluttered by per-build hash churn. The templates are still rendered so the structure matches prod.

## SW Update Flow

No changes to the existing logic in `sw.js` (the hand-written parts — `ensureInitialized`, `handleLocalLlm`, fetch handler, local-LLM bridge). Only the import line changes, and one line changes in `loader.js`:

`loader.js`:

```js
const registration = await navigator.serviceWorker.register('/sw.js', {
    type: 'module',
    scope: '/',
    updateViaCache: 'none',      // <-- new: bypass HTTP cache for SW update check
});
```

This is the single host-agnostic guarantee that protects the SW update check from misconfigured CDN caching. Combined with hashed filenames, no server-side `Cache-Control` headers are required.

Step-by-step update trace:

1. User has a tab open; old SW controls it; old WASM is in memory.
2. User navigates (link click, reload, new tab).
3. Browser fetches `/sw.js` with `updateViaCache: 'none'` → hits origin fresh.
4. New `sw.js` has a different `__WASM_JS__` path → byte diff detected.
5. Browser installs the new SW → `install` event → `skipWaiting()` (already in sw.js:23).
6. `activate` event → `clients.claim()` (already in sw.js:28) → new SW now controls the tab.
7. Current page's DOM is still old; that's fine — silent UX.
8. Any new fetch from the page goes through the new SW.
9. First fetch triggers `ensureInitialized()` → `init()` loads the new hashed `_bg.wasm` from origin → `initialize()` boots the new runtime.
10. Old hashed URLs are never referenced again; they evict from HTTP cache naturally.

### Cache-bypass list

sw.js:125-146 has a bypass list in the `fetch` event handler that returns early for specific paths (entry-point files, `/pkg/`, `/sql-`). This list only applies to fetches originating from controlled *pages* — fetches the SW itself makes (its ES module imports of `solobase_web.js`, `init()`'s fetch of `solobase_web_bg.wasm`) do not pass through `self.addEventListener('fetch', …)` at all, so they need no exemption.

The list should be kept in sync regardless, for correctness when pages load assets. Update the rules to:

- Keep `startsWith('/sql-')` — still covers `sql-wasm-<hash>.js` and `sql-wasm-<hash>.wasm`.
- Replace `startsWith('/pkg/')` (dead in prod, where `pkg/` is the site root) with two prefixes covering the hashed assets if a page ever links to them directly: `startsWith('/solobase_web')` and `startsWith('/snippets/')`.
- Leave `/sw.js`, `/loader.js`, `/ai-bridge.js`, `/manifest.json`, `/index.html`, `/` unchanged.

## npm Package Update Lifecycle

### New behavior: `worker.ts` does not auto-`skipWaiting` on install

Current `worker.ts` calls `self.skipWaiting()` inside `install`. For a library that consumers embed in their own apps, this is too aggressive — consumers may want to show a confirmation UI before updating. The new default: install and activate, but wait for an explicit `{ type: 'skip-waiting' }` message before pre-empting the old SW.

```ts
self.addEventListener('install', (event) => {
  event.waitUntil(initialize());
  // No skipWaiting by default — consumers opt in via postMessage.
});

self.addEventListener('message', (event) => {
  if (event.data?.type === 'skip-waiting') self.skipWaiting();
  if (event.data?.type === 'solobase:config' && Array.isArray(event.data.routes)) {
    routes = event.data.routes;
  }
});
```

Consumers wanting the old aggressive behavior post `{ type: 'skip-waiting' }` from their main thread once, at registration time. One line.

### New module: `src/update.ts` (main-thread helpers)

```ts
export function registerWithUpdates(
  scriptURL: string,
  opts?: { scope?: string; type?: 'classic' | 'module' },
): Promise<UpdateHandle>;

export interface UpdateHandle {
  registration: ServiceWorkerRegistration;
  /** Fires when a new SW is installed and waiting to activate. Returns unsubscribe. */
  onUpdateReady(cb: (apply: () => Promise<void>) => void): () => void;
  /** Force an update check. Wraps registration.update(). */
  checkForUpdate(): Promise<void>;
}
```

Implementation (~40 lines): listens to `registration.updatefound` and the new worker's `statechange`; filters out the first-install case (no existing `navigator.serviceWorker.controller`); exposes `apply()` which posts `{ type: 'skip-waiting' }` to the waiting worker and resolves on `controllerchange`.

Consumers can wire this to any UX pattern:

- Silent on next nav — don't call `apply()`; user's next navigation picks it up once they reload.
- Auto-reload — call `apply()` immediately, then `location.reload()` on `controllerchange`.
- Toast — surface the availability via `onUpdateReady`, call `apply()` when the user clicks.

### Package versioning

This is a breaking change to the SW lifecycle default. Bump `packages/solobase-web/package.json` to `0.2.0`. Add a CHANGELOG entry.

### README updates

New sections covering:

- How bundlers handle the WASM URL (brief examples for Vite, webpack, esbuild).
- How to use `registerWithUpdates` with each of the three common UX patterns.
- Note that the standalone `pkg/` site uses silent-on-next-nav and why.

## Testing

### Unit tests: `solobase-web-bundle`

- Given a fixture `pkg/` with known contents:
  - Assert each hashed filename matches expected SHA-256-8.
  - Assert `asset-manifest.json` matches expected shape.
  - Assert templated `sw.js` and `index.html` contain hashed paths and no unresolved `__` placeholders.
- Determinism test: run the tool twice on identical input → byte-identical output.
- Negative test: if `solobase_web.js` lacks the expected `'solobase_web_bg.wasm'` literal, fail with a clear error.

### Build integration tests

- In CI, after `make build`:
  - Every URL referenced by `pkg/sw.js` and `pkg/index.html` resolves to a file on disk.
  - Lint: grep `pkg/sw.js` and `pkg/index.html` for literal `__` placeholder patterns → fail build if any remain.

### Browser update flow (Playwright)

- Serve `pkg/` via `make serve`.
- Load `/`; wait for SW active; record `sw.js` URL and the hashed `_bg.wasm` URL pulled from the Network panel.
- Trigger a small Rust source change; rebuild.
- Reload page; assert: new `sw.js` bytes, `controllerchange` fires, new hashed `_bg.wasm` fetched, old hashed URLs no longer requested.
- Negative test: rebuild with no source changes → no new SW installed, no new fetches on reload.

### npm package tests

- Fixture app in `packages/solobase-web/test-fixtures/vite-app/` that imports `@solobase/web/worker`.
- CI builds the fixture; asserts the Vite output contains a hashed `.wasm` filename and a reference to it in the bundled SW code.
- Unit tests for `registerWithUpdates` with a mocked `navigator.serviceWorker`.

## Rollout

Four independent, revertable steps:

1. Land `crates/solobase-web-bundle/` with unit tests. No behavior change.
2. Convert `js/sw.js` → `js/sw.js.tmpl`, add `js/index.html.tmpl`, update Makefile to run the bundler. Ship. Standalone site now has proper versioning.
3. Refactor `packages/solobase-web/src/worker.ts` lifecycle and add `src/update.ts`. Bump to `0.2.0`. Update `README.md`.
4. Add Playwright test for the update flow, gated in CI.

Each step is independently shippable. A revert of step 2 only requires restoring the hand-written `sw.js` and removing the bundler invocation from the Makefile.

## Risks

- **wasm-pack output shape change**: the string-rewrite in the bundler tool assumes `solobase_web.js` contains a literal `'solobase_web_bg.wasm'`. A future wasm-pack version could use a different encoding. Mitigation: assert the literal exists before rewriting; fail the build with a clear error if not.
- **Relative `snippets/` resolution after rename**: `solobase_web.js` imports `./snippets/<hash>/bridge.js` relative to its own URL. After we rename `solobase_web.js` to include a hash, the relative path still resolves correctly (same directory). Verified during implementation with a browser test.
- **Dev-mode drift**: `--dev` mode skips hashing, so the structural shape of `pkg/` differs between dev and prod. The templates still render the same set of imports — just to canonical names in dev — so `sw.js` behavior is identical. Playwright tests run against the prod build.

## Summary

Hashed filenames for long-lived assets + templated `sw.js` carrying those hashes + `updateViaCache: 'none'` on `register()` gives us a correct, host-agnostic SW update story with zero server configuration. The existing `skipWaiting` + `clients.claim` flow in `sw.js` delivers silent-on-next-navigation UX for the standalone site. The npm package ships clean wasm-pack output for consumer bundlers to hash, and exposes update-lifecycle helpers so consumers can pick their own UX.
