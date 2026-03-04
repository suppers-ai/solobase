# Plan: Generic `web` Block for Solobase

## Context

The solobase-site is currently a Hugo + Express + Terraform stack. Rather than building site-specific blocks for marketing pages, the idea is to create a **generic, reusable `web` block** that serves static files from a configurable build directory. Any website (marketing, docs, SPA, etc.) just builds to a folder and the block serves it — through the full WAFFLE flow pipeline, so it gets security headers, CORS, rate limiting, and monitoring for free.

This block will live in `solobase/blocks/web/` and follow existing block conventions.

---

## Implementation

### Step 1: Create `solobase/blocks/web/block.go`

The main block file with config, constructor, `Info()`, `Handle()`, and `Lifecycle()`.

**Config struct** — accepts constructor params (not env vars) so multiple instances can coexist:

```go
type WebConfig struct {
    Dir             string // Required. Root directory with built site (e.g., "./sites/marketing/dist")
    Prefix          string // URL prefix to strip (e.g., "/site"). Default: ""
    SPAMode         bool   // Fallback to index.html for unknown paths. Default: false
    IndexFile       string // Directory index filename. Default: "index.html"
    CacheMaxAge     int    // Cache max-age for normal assets (seconds). Default: 3600
    ImmutableMaxAge int    // Cache max-age for hashed assets (seconds). Default: 31536000
}
```

**Constructor** — `NewWebBlock(cfg WebConfig) *WebBlock` — applies defaults for `IndexFile`, `CacheMaxAge`, `ImmutableMaxAge`.

**`Info()`** — returns `BlockName = "web-feature"`, interface `"http.handler"`, singleton mode (also allows `PerNode` for multi-instance use).

**`Lifecycle(Init)`** — resolves `Dir` to absolute path via `filepath.Abs()`, validates directory exists with `os.Stat()`. Stores resolved `absRoot` on the struct.

**`Handle()`** — rejects non-retrieve (non-GET) requests with 405, strips `Prefix` from `msg.Path()`, delegates to `serveFile()`. No `waffle.Router` needed — single-purpose block.

### Step 2: Create `solobase/blocks/web/serve.go`

File serving logic, content-type detection, path security.

**`serveFile(msg, reqPath) Result`:**

1. Default `/` or empty path to `/{IndexFile}`
2. `filepath.Clean()` the path to prevent traversal
3. Block dotfiles (segments starting with `.`) unless `AllowDotFiles` is set
4. Join with `absRoot`, then `filepath.EvalSymlinks()` to resolve symlinks
5. Verify resolved path is still within `absRoot` (symlink escape prevention)
6. If path is a directory, append `IndexFile`
7. `os.ReadFile()` the file
8. Detect content-type via `mime.TypeByExtension()`, fall back to `http.DetectContentType()`
9. Set cache headers, return via `waffle.NewResponse(msg, 200).SetHeader(...).Body(data, contentType)`

**`handleNotFound(msg, reqPath) Result`:**

- If `SPAMode` is true: read and serve `{absRoot}/index.html` with `Cache-Control: no-cache`
- Otherwise: `waffle.ErrNotFound(msg, "not found")`

**`detectContentType(filePath, data) string`:**

- Try `mime.TypeByExtension()` first (more reliable)
- Add `; charset=utf-8` for `text/*` types
- Fall back to `http.DetectContentType()` for unknown extensions

### Step 3: Create `solobase/blocks/web/cache.go`

Cache header strategy:

- **HTML files** (`text/html`): `Cache-Control: no-cache` (always revalidate)
- **Hashed assets** (files in `/assets/`, `/_next/static/`, `/static/js/`, `/static/css/` with hash-like patterns in the filename): `Cache-Control: public, max-age=31536000, immutable`
- **Everything else**: `Cache-Control: public, max-age=3600`

The `isHashedAsset(reqPath) bool` function detects fingerprinted files by checking:

- Is the file in a known hashed-asset directory?
- Does the filename contain a hash segment? (e.g., `main.a1b2c3d4.js` or `style-BkZ3xQ.css`)

### Step 4: Create `solobase/blocks/web/block_test.go`

Unit tests using `waffletest` helpers. No database needed.

Test helper: `setupTestSite(t) string` — creates a temp directory tree:

```
index.html
about.html
css/style.css
assets/main.abc123.js
subdir/index.html
.env (dotfile)
```

Test helper: create block with `NewWebBlock(cfg)`, init via `waffletest.InitBlock()`.

**Test cases:**

1. `TestServeRoot` — GET `/` returns `index.html`
2. `TestServeStaticFile` — GET `/about.html` returns correct content + `text/html` type
3. `TestServeNestedFile` — GET `/css/style.css` returns CSS with `text/css` content type
4. `TestServeHashedAsset` — GET `/assets/main.abc123.js` returns JS with `immutable` cache header
5. `TestPathTraversal` — GET `/../../../etc/passwd` returns 404
6. `TestDotFileBlocked` — GET `/.env` returns 404
7. `TestSPAFallback` — SPA mode: GET `/nonexistent/route` returns `index.html` with 200
8. `TestSPADisabled` — Non-SPA mode: GET `/nonexistent/route` returns 404
9. `TestNonGetMethod` — POST request returns 405
10. `TestCacheHeadersHTML` — HTML gets `no-cache`
11. `TestCacheHeadersNormal` — CSS gets `public, max-age=3600`
12. `TestDirectoryServesIndex` — GET `/subdir/` serves `subdir/index.html`
13. `TestLifecycleInitMissingDir` — Init with nonexistent dir returns error
14. `TestLifecycleInitNotADir` — Init with file path returns error

### Step 5: Wire into `solobase/flows/flows.go`

**In `registerFeatureBlocks()`** — conditionally register the web block when `WEB_ROOT` env var is set:

```go
if webRoot := env.GetEnv("WEB_ROOT"); webRoot != "" {
    prefix := env.GetEnvOrDefault("WEB_PREFIX", "")
    spaMode := env.GetEnv("WEB_SPA") == "true"
    w.RegisterBlock(web.BlockName, web.NewWebBlock(web.WebConfig{
        Dir:     webRoot,
        Prefix:  prefix,
        SPAMode: spaMode,
    }))
}
```

**In `buildFeatureFlows()`** — conditionally add the web flow:

```go
if w.HasBlock(web.BlockName) {
    prefix := env.GetEnvOrDefault("WEB_PREFIX", "")
    addPublicFlow(w, "web-site", web.BlockName, "Static website serving",
        []waffle.HTTPRoute{{Path: prefix + "/", PathPrefix: true}})
}
```

Add a new `addPublicFlow` helper (similar to existing `addAdminFlow` but without `admin-pipe`):

```go
func addPublicFlow(w *waffle.Waffle, flowID, blockName, summary string, routes []waffle.HTTPRoute) {
    w.AddFlow(waffle.Flow{
        ID:      flowID,
        Summary: summary,
        Config:  waffle.FlowConfig{OnError: "stop"},
        HTTP:    &waffle.HTTPRouteDef{Routes: routes},
        Root: &waffle.Node{
            Flow: "http-infra",
            Next:  []*waffle.Node{{Block: blockName}},
        },
    })
}
```

Add import for `web` package and `env` package.

### Step 6: Register factory in `solobase/builds/native/blocks.go`

```go
waffleconfig.RegisterBlock(web.BlockName, func(cfg map[string]any) (waffle.Block, error) {
    dir, _ := cfg["dir"].(string)
    if dir == "" {
        return nil, fmt.Errorf("web block requires 'dir' config")
    }
    prefix, _ := cfg["prefix"].(string)
    spaMode, _ := cfg["spa_mode"].(bool)
    indexFile, _ := cfg["index_file"].(string)
    return web.NewWebBlock(web.WebConfig{
        Dir:       dir,
        Prefix:    prefix,
        SPAMode:   spaMode,
        IndexFile: indexFile,
    }), nil
})
```

Add import for the `web` package.

---

## Files Summary

### New files (4)

| File | Purpose |
|------|---------|
| `solobase/blocks/web/block.go` | Block struct, config, constructor, Info, Handle, Lifecycle |
| `solobase/blocks/web/serve.go` | File serving, path security, content-type detection |
| `solobase/blocks/web/cache.go` | Cache header strategy, hashed asset detection |
| `solobase/blocks/web/block_test.go` | Unit tests (14 cases) |

### Modified files (2)

| File | Change |
|------|--------|
| `solobase/flows/flows.go` | Import web, register block + flow conditionally, add `addPublicFlow` helper |
| `solobase/builds/native/blocks.go` | Import web, register waffleconfig factory |

### Reference files (patterns to follow)

| File | Why |
|------|-----|
| `solobase/blocks/legalpages/block.go` | Closest block pattern to follow |
| `waffle-go/helpers.go` | Response builder API: `NewResponse`, `SetHeader`, `Body`, `ErrNotFound` |
| `waffle-go/waffletest/waffletest.go` | Test helpers: `Retrieve()`, `InitBlock()`, `Status()`, `ResponseBody()` |
| `waffle-go/match.go` | Path matching: `/**` wildcard, `{var}` extraction |

---

## Verification

1. **Unit tests**: `cd /workspaces/workspace && go test ./solobase/blocks/web/...`
2. **Build check**: `cd /workspaces/workspace && go build ./solobase/...`
3. **Vet check**: `cd /workspaces/workspace && go vet ./solobase/...`
4. **Full test suite**: `cd /workspaces/workspace && go test ./...` — ensure no regressions
5. **Manual smoke test** (optional): Create a temp site dir, set `WEB_ROOT=./testsite`, run the binary, curl a few paths
