# WAFER Runtime Architecture

## Overview

WAFER is Solobase's block-and-chain runtime. It provides typed platform services and composable blocks for building applications.

## Core Concepts

Two main abstractions:

1. **Platform Services** — infrastructure primitives provided by the runtime (database, storage, logger, crypto, config). Implementations are swappable via provider registration.
2. **Blocks** — all business logic. Each block is self-contained: owns its routes, its logic, its data access. No separate "service layer".

Plus **Chains** for composition — wiring blocks together, route matching, auth pipelines.

## Project Structure

```
wafer/go/               # Wafer runtime (separate Go module)
  services/              # Platform service interfaces + implementations
    database/            # Generic CRUD (Get/List/Create/Update/Delete)
    storage/             # File storage service
    logger/              # Structured logging
    crypto/              # Hash/sign/verify
    config/              # Key-value config
  bridge/                # HTTP <-> wafer message bridge
  manifest/              # Block manifest loader
  migrate/               # Schema migration
  schema/                # Schema types + SQLite adapter
  waferconfig/          # Config-driven bootstrap

blocks/                  # Feature blocks
  auth/                  # Authentication (login, signup, tokens, API keys)
  users/                 # User management
  iam/                   # Roles and permissions
  database/              # Database admin (table inspection)
  custom_tables/         # User-defined tables
  storage/               # File storage (buckets, objects, upload/download)
  cloudstorage/          # Cloud storage extensions (shares, quotas)
  logs/                  # System logging
  monitoring/            # Dashboard metrics
  settings/              # App settings
  legalpages/            # Legal pages (privacy, terms)
  products/              # Products, pricing, payments
  system/                # System info
  wafer/                # Wafer admin panel

infra/                   # Infrastructure blocks (middleware)
  auth.go                # JWT/API key authentication
  cors.go                # CORS headers
  iam.go                 # Role-based access control
  rate_limit.go          # Rate limiting
  security_headers.go    # Security headers
  readonly.go            # Read-only mode guard

chains/                  # Chain definitions (blocks.go, chains.go)
adapters/                # Adapter implementations
  crypto/                # Crypto service implementation (Argon2, JWT)
  auth/oauth/            # OAuth providers (Google, Microsoft, Facebook)
  database/sqlite/       # SQLite database creation
  repos/                 # Storage repository (V1, used by storage block)
  storage/               # File storage providers (local, S3)

core/                    # Shared utilities
builds/native/            # Go deployment entry point
builds/wasm/             # WASM deployment entry point
frontend/                # Admin UI (Preact)
```

## Platform Services

Typed interfaces on `ctx.Services()`:

```go
type Services struct {
    Database  database.Service
    Storage   storage.Service
    Logger    logger.Service
    Crypto    crypto.Service
    Config    config.Service
}
```

### Database Service

```go
type Service interface {
    Get(ctx, collection, id) (*Record, error)
    List(ctx, collection, *ListOptions) (*RecordList, error)
    Create(ctx, collection, data map[string]any) (*Record, error)
    Update(ctx, collection, id, data map[string]any) (*Record, error)
    Delete(ctx, collection, id) error
}
```

Blocks never write SQL. They call generic methods. Create does NOT take an ID — set `data["id"]` in the map.

Helpers in `wafer/go/services/database/helpers.go`: `GetByField`, `Upsert`, `ListAll`, `PaginatedList`, `SoftDelete`, `DeleteByField`.

### Crypto Service

```go
type Service interface {
    Hash(password string) (string, error)
    CompareHash(password, hash string) error
    Sign(claims map[string]any, expiry time.Duration) (string, error)
    Verify(token string) (map[string]any, error)
    RandomBytes(n int) ([]byte, error)
}
```

JWT secret is encapsulated — callers don't pass it.

## Blocks

Blocks implement the `Block` interface: `Info()`, `Handle()`, `Lifecycle()`.

### V2 Block Pattern

All blocks use `ctx.Services().Database` directly. Zero constructor dependencies.

```go
func NewMyBlock() *MyBlock {
    b := &MyBlock{}
    b.router = wafer.NewRouter()
    b.router.Retrieve("/items", b.handleList)
    b.router.Create("/items", b.handleCreate)
    return b
}

func (b *MyBlock) Lifecycle(ctx wafer.Context, evt wafer.LifecycleEvent) error {
    if evt.Type == wafer.Init {
        // Initialize from platform services
        b.db = ctx.Services().Database
    }
    return nil
}
```

### Response Helpers

- `wafer.JSONRespond(msg, status, data)` — generic JSON response
- `wafer.Error(msg, status, code, message)` — error response
- `wafer.NewResponse(msg, status).SetCookie().SetHeader().JSON()` — builder pattern

### Router Actions

- `Retrieve()` → GET, `Create()` → POST, `Update()` → PUT/PATCH, `Delete()` → DELETE

### Block Manifests

Each block has a `block.json` declaring collections, fields, indexes, and required services.

## Chains

Chains compose blocks with auth levels:

- `http-infra` — CORS, security headers, rate limiting
- `auth-pipe` — JWT/API key authentication, IAM role checking
- `admin-pipe` — admin-only routes (requires "admin" role)
- `protected-pipe` — authenticated user routes

The bridge auto-registers HTTP routes from chains: `bridge.AutoRegister(mux, runtime)`.

## Normalized Meta Schema

Blocks use transport-agnostic metadata:

- **Request**: `req.action`, `req.resource`, `req.param.*`, `req.query.*`
- **Auth**: `auth.user_id`, `auth.user_email`, `auth.user_roles`
- **Response**: `resp.status`, `resp.content_type`, `resp.header.*`, `resp.set_cookie.*`

## Configuration

Provider registration happens at startup via `waferconfig.RegisterProvider()`. The `WaferConfig` struct configures services and blocks:

```go
waferCfg := &waferconfig.WaferConfig{
    Services: waferconfig.ServiceConfig{
        Database: &waferconfig.ProviderConfig{
            Provider: "database/core/sqlite",
            Config:   map[string]any{"_sqldb": sqlDB},
        },
        Crypto: &waferconfig.ProviderConfig{
            Provider: "crypto/core/standard",
            Config:   map[string]any{"_jwt_secret": jwtSecret},
        },
    },
    Blocks: []string{"users-feature", "logs-feature", "settings-feature"},
}
```
