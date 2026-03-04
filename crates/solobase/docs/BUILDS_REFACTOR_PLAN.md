# Builds Refactor Plan

## Context

The wafer runtime architecture envisions thin build targets that compose services, blocks, and flows. Currently the root `package solobase` acts as both a shared library AND the app wiring layer (App struct, Config, lifecycle, routes, UI serving). `builds/go/main.go` is just 5 lines calling `solobase.Run()`. This inverts the intended architecture.

**Goal**: Root module becomes a pure library (blocks, flows, infra, core, adapters). Each build target owns its own wiring. Builds ARE the product — users clone/customize builds.

**After this refactor:**
```
solobase/                    # Root module — pure shared library
  blocks/                    # Feature blocks
  flows/                     # Flow topology + constants
  infra/                     # Infrastructure blocks
  core/                      # Shared utilities
  adapters/                  # Implementations (crypto, storage, oauth)
  wafer/go/                 # Runtime engine (separate module)
  frontend/                  # Frontend source + build output
  ui.go                      # Just exports UIFiles embed.FS
  blockmanifests.go          # Just exports BlockManifestsFS embed.FS

builds/
  native/                    # Standard Go server (the product)
    main.go                  # Entry point
    app.go                   # App struct, lifecycle
    config.go                # Config, env resolution
    builder.go               # New(), Run(), provider init
    blocks.go                # Block + provider registration
    routes.go                # HTTP routing, custom routes
    ui.go                    # Frontend serving
    hooks.go                 # Event hooks
    solobase_test.go         # Integration tests
    database/sqlite.go       # SQLite connection helper
    Makefile
    go.mod

  wasm/                      # WASM/edge build
    main.go                  # Self-contained wiring
    database/service.go      # WASM DB service
    go.mod

  configs/                   # Example wafer configs
    full.json                # All blocks, full UI
    minimal.json             # Auth + users + settings
    api-only.json            # No UI, headless API
```

---

## Phase 0: Prep (non-breaking changes in root)

### 0.1 — Export embed variables

| File | Change |
|------|--------|
| `blockmanifests.go` | `blockManifestsFS` → `BlockManifestsFS` |
| `ui.go` | `uiFiles` → `UIFiles` |
| `builder.go` | Update `defaultSchemaMigrator` reference |

### 0.2 — Move block implementations out of flows/

`flows/blocks.go` contains `oauthBlock` and `profileBlock` — actual block implementations mixed with wiring code. Move them to proper block packages:

| From | To |
|------|----|
| `oauthBlock` (flows/blocks.go:128-265) | `blocks/oauth/block.go` (new package) |
| `profileBlock` (flows/blocks.go:268-300) | `blocks/profile/block.go` (new package) |

### 0.3 — Split flows/blocks.go (topology vs wiring)

**Keep in `flows/blocks.go`:**
- Block name constants (exported: `BlockAuth`, `BlockUsers`, etc.)
- `Deps` struct

**Remove from `flows/blocks.go`:**
- `registerFeatureBlocks()` — moves to each build target
- `registerIfAbsent()` helper — moves with it
- Hook wiring (observability hooks, lines 110-124) — moves with it

**Modify `flows/flows.go`:**
- Rename `BuildAll()` → `BuildFlows()`
- Remove the `registerFeatureBlocks()` call
- `BuildFlows()` does: `infrablocks.RegisterAll()` + `buildBaseFlows()` + `buildFeatureFlows()`

---

## Phase 1: Create builds/native/

### 1.1 — Rename builds/go/ → builds/native/

- `git mv builds/go builds/native`
- Update `go.work`: `./builds/go` → `./builds/native`
- Update `builds/native/go.mod` module path
- Remove old `builds/native/example/` directory (replaced by configs/)
- Keep `builds/native/database/sqlite.go`

### 1.2 — Create builds/native/app.go

Move from `solobase.go`:
- `App` struct, `AppServices` struct
- `newApp()`, `Initialize()`, `Start()`, `Shutdown()`
- `setupRoutes()` — the main wiring function
- Accessors: `Router()`, `Handler()`, `SetupRouter()`, `DB()`, `Services()`, `Config()`, `GetAppID()`
- Type re-exports (`TableDefinition`, etc.) — drop these, no longer needed

### 1.3 — Create builds/native/config.go

Move from `builder.go`:
- `Config` struct (public API for build configuration)
- `Providers` struct
- `appConfig` internal struct
- `resolveConfig()`, `getEnvOrDefault()`

### 1.4 — Create builds/native/builder.go

Move from `builder.go`:
- `New()`, `MustNew()`, `Run()`
- `initializeProviders()` — crypto, storage, oauth, logger, env, file providers
- `defaultShutdownHandler()`, `defaultCleanupScheduler()`
- `defaultSchemaMigrator()` — uses imported `solobase.BlockManifestsFS`

### 1.5 — Create builds/native/blocks.go

Merge from `blockregistry.go` + `flows/blocks.go`:
- `registerBuiltinProviders()` — crypto/standard + database/passthrough
- `registerBuiltinBlocks()` — all block factory registrations
- `registerFeatureBlocks(w, deps)` — creates block instances, wires hooks
- `registerIfAbsent()` helper
- Observability hook wiring (logs DBLogger + monitoring collector)

### 1.6 — Create builds/native/routes.go

Move from `routes.go`:
- `RouteHandler` type, `customRoute` struct, `routeType` enum
- `RegisterPublicRoute()`, `RegisterProtectedRoute()`, `RegisterAdminRoute()`
- `registerCustomRoutes()`
- `registerUserPortalFrontend()`, `serveBlockSPA()`

### 1.7 — Create builds/native/ui.go

Move methods from `ui.go` (embed stays at root):
- `getUIFS()` — imports `solobase.UIFiles` from root module
- `ServeStaticAsset()`, `ServeBlockUI()`, `ServeUI()`

No `//go:embed` here — the embed lives in root where `frontend/build/` exists.

### 1.8 — Create builds/native/hooks.go

Move from `hooks.go`:
- `ServeEvent`, `APIEvent`, `ModelEvent` structs
- `ServeHook`, `APIHook`, `ModelHook` types
- `OnServe()`, `OnBeforeAPI()`, `OnAfterAPI()`, `OnModel()` methods

### 1.9 — Update builds/native/main.go

```go
package main

import "log"

func main() {
    if err := Run(); err != nil {
        log.Fatal(err)
    }
}
```

### 1.10 — Move tests

- `solobase_test.go` → `builds/native/solobase_test.go`
- Change `package solobase` → `package main`
- Update imports: use local types, import root only for embed vars

---

## Phase 2: Create builds/configs/

Example wafer.json-format configuration files:

### configs/full.json
All blocks enabled, SQLite, full admin UI — the standard Solobase deployment.

### configs/minimal.json
Auth + users + settings only — lightweight backend.

### configs/api-only.json
All blocks but `disable_ui: true` — headless API server.

Move existing `wafer.json` from root → `builds/configs/full.json`.

---

## Phase 3: Clean up root module

### 3.1 — Delete moved files

| Delete | Reason |
|--------|--------|
| `solobase.go` | → builds/native/app.go |
| `builder.go` | → builds/native/builder.go + config.go |
| `routes.go` | → builds/native/routes.go |
| `hooks.go` | → builds/native/hooks.go |
| `blockregistry.go` | → builds/native/blocks.go |
| `solobase_test.go` | → builds/native/solobase_test.go |
| `wafer.json` | → builds/configs/full.json |

### 3.2 — Simplify remaining root files

**`ui.go`** → just the embed export:
```go
package solobase

import "embed"

//go:embed all:frontend/build/*
var UIFiles embed.FS
```

**`blockmanifests.go`** → just the embed export:
```go
package solobase

import "embed"

//go:embed blocks/*/block.json
var BlockManifestsFS embed.FS
```

Root module is now a pure library.

---

## Phase 4: Update builds/wasm/

WASM currently imports `solobase.New(solobase.Config{...})`. After Phase 3, that API is gone.

- Remove root `solobase` import for App/Config/New
- Self-wire: create wafer runtime directly, register blocks locally, call `flows.BuildFlows()`
- Keep its own `//go:embed frontend/build` (TinyGo requires embed in main package)
- Keep its own provider initialization (already mostly self-contained)
- Has its own `registerFeatureBlocks()` (can exclude blocks not relevant to WASM)

---

## Phase 5: Verify

1. `go build ./...` from workspace root — root module compiles as pure library
2. `cd builds/native && go build .` — native binary compiles
3. `cd builds/native && go test ./...` — integration tests pass
4. `go vet ./...` — clean
5. Manual: start server from builds/native/, verify admin UI + API endpoints work

---

## Execution Order

The operations are sequenced to avoid breaking compilation at any step:

1. **Phase 0** — Additive/non-breaking changes in root (export vars, move blocks, split flows)
2. **Phase 1** — Create all builds/native/ files (root still has everything — dual existence)
3. **Verify** builds/native/ compiles and tests pass
4. **Phase 2** — Create builds/configs/
5. **Phase 4** — Update builds/wasm/ to self-wire
6. **Phase 3** — Delete wiring code from root (now safe — no consumers remain)
7. **Phase 5** — Final verification

---

## File Manifest

### Files to CREATE

| Path | Source |
|------|--------|
| `builds/native/app.go` | solobase.go |
| `builds/native/config.go` | builder.go (Config, Providers, resolveConfig) |
| `builds/native/builder.go` | builder.go (New, Run, initializeProviders) |
| `builds/native/blocks.go` | blockregistry.go + flows/blocks.go wiring |
| `builds/native/routes.go` | routes.go |
| `builds/native/ui.go` | ui.go (methods only) |
| `builds/native/hooks.go` | hooks.go |
| `builds/native/solobase_test.go` | solobase_test.go |
| `blocks/oauth/block.go` | flows/blocks.go (oauthBlock) |
| `blocks/profile/block.go` | flows/blocks.go (profileBlock) |
| `builds/configs/full.json` | wafer.json (expanded) |
| `builds/configs/minimal.json` | new |
| `builds/configs/api-only.json` | new |

### Files to MODIFY

| Path | Change |
|------|--------|
| `blockmanifests.go` | Export variable name |
| `ui.go` | Strip to just embed export |
| `flows/blocks.go` | Keep constants + Deps only |
| `flows/flows.go` | `BuildAll` → `BuildFlows`, remove registerFeatureBlocks call |
| `builds/native/go.mod` | Rename module path |
| `builds/native/main.go` | Simplify to call Run() |
| `builds/wasm/main.go` | Self-wire, remove root solobase dependency |
| `go.work` | `./builds/go` → `./builds/native` |

### Files to DELETE

| Path | Reason |
|------|--------|
| `solobase.go` | Moved to builds/native/ |
| `builder.go` | Moved to builds/native/ |
| `routes.go` | Moved to builds/native/ |
| `hooks.go` | Moved to builds/native/ |
| `blockregistry.go` | Moved to builds/native/ |
| `solobase_test.go` | Moved to builds/native/ |
| `wafer.json` | Moved to builds/configs/ |
| `builds/native/example/` | Replaced by builds/configs/ |
