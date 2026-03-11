# solobase-core: Shared Platform Abstraction

## Status: COMPLETE

All migration steps have been implemented. The shared pipeline is used by both
the Cloudflare Worker and the native standalone binary.

## Problem

The Cloudflare adapter and native binary duplicate significant logic:
- JWT validation + auth meta extraction
- Routing table (path → block with feature gates and admin checks)
- Feature flag detection (`auth_enabled()`, `files_enabled()`, etc.)
- Crypto (argon2 password hashing + HMAC-SHA256 JWT)
- HTTP → Message field mapping

Both platforms run the same solobase blocks but have separate request pipelines. Changes to routing, auth, or feature gating must be made in two places.

## Solution

A `solobase-core` crate containing the shared request pipeline. CF and native
both call `solobase_core::handle_request()` for routing, JWT validation, feature
gates, and block dispatch. Platform-specific concerns (HTTP handling, middleware,
block instantiation) stay in each adapter.

## Crate Structure

```
solobase/crates/solobase-core/src/
├── lib.rs          — public API, re-exports
├── pipeline.rs     — shared request pipeline (handle_request)
├── routing.rs      — path → block routing table + feature gates + admin check
├── features.rs     — FeatureConfig trait (Send + Sync)
└── crypto.rs       — argon2 + HMAC-SHA256 JWT (wasm32-compatible)
```

## Architecture After Migration

### Cloudflare adapter (`solobase-cloudflare`)

```
HTTP Request → Worker #[event(fetch)]
  → Resolve tenant from hostname (KV lookup)
  → Create CloudflareContext (D1, R2, JWT)
  → Convert Worker Request → Message
  → solobase_core::handle_request()
    → JWT validation + auth meta
    → Routing table + feature gates + admin check
    → SolobaseBlockFactory creates block → block.handle()
  → Convert Result_ → Worker Response + CORS
```

### Native adapter (`solobase`)

```
HTTP Request → @wafer/http-listener
  → @wafer/infra flow (CORS, security headers, rate limiting)
  → @solobase/router block
    → Extract auth from Authorization header or auth_token cookie
    → solobase_core::handle_request()
      → JWT validation + auth meta
      → Routing table + feature gates + admin check
      → NativeBlockFactory returns shared Arc<dyn Block> → block.handle()
  → @wafer/web fallback (SPA serving for non-API paths)
```

Key difference: the native adapter keeps the WAFER flow engine for middleware
(`@wafer/infra`) and SPA serving (`@wafer/web`), but all API routing goes
through the shared `solobase-core` pipeline via the `@solobase/router` block.

## Shared Routing Table

```rust
const ROUTES: &[Route] = &[
    Route { prefix: "/health",                  requires_admin: false, block_id: BlockId::System },
    Route { prefix: "/nav",                     requires_admin: false, block_id: BlockId::System },
    Route { prefix: "/auth/",                   requires_admin: false, block_id: BlockId::Auth },
    Route { prefix: "/admin/settings/",         requires_admin: true,  block_id: BlockId::Admin },
    Route { prefix: "/admin/storage/",          requires_admin: true,  block_id: BlockId::Files },
    Route { prefix: "/admin/",                  requires_admin: true,  block_id: BlockId::Admin },
    Route { prefix: "/storage/",                requires_admin: false, block_id: BlockId::Files },
    Route { prefix: "/b/products",              requires_admin: false, block_id: BlockId::Products },
    // ... etc
];
```

Both CF and native use this same table. The native binary no longer needs
individual per-feature flow definitions for routing.

## Block Factory Pattern

```rust
pub trait BlockFactory: Send + Sync {
    fn create(&self, block_id: BlockId) -> Arc<dyn Block>;
}
```

- **CF adapter** (`SolobaseBlockFactory`): Creates fresh `Arc<dyn Block>` per request.
  CF Workers are stateless, so no state sharing needed.
- **Native adapter** (`NativeBlockFactory`): Returns clones of pre-created `Arc<dyn Block>`
  instances. Stateful blocks (e.g. AuthBlock with rate limiter) share state across requests.

## What Stays Platform-Specific

| Concern | Cloudflare | Native |
|---|---|---|
| Entry point | `#[event(fetch)]` Worker | `#[tokio::main]` + WAFER runtime |
| HTTP types | `worker::Request/Response` | WAFER `@wafer/http-listener` |
| HTTP ↔ Message conversion | `worker_request_to_message()` | WAFER HTTP block |
| Middleware (CORS, headers) | Manual `add_cors_headers()` | `@wafer/infra` flow |
| Database block | D1Block (JS FFI) | solobase/sqlite, solobase/postgres |
| Storage block | R2Block (JS FFI) | solobase/local-storage, solobase/s3 |
| Crypto implementation | `solobase_core::crypto` (manual HMAC) | `wafer-core` Argon2JwtCryptoService (jsonwebtoken) |
| Tenant resolution | KV lookup by subdomain | Single tenant from app.json |
| SPA serving | N/A | `@wafer/web` block |
| Control plane | CF-specific `/_control/` API | N/A |

## What's Shared via solobase-core

| Concern | Module |
|---|---|
| JWT validation + auth meta extraction | `crypto::extract_auth_meta()` |
| Routing table + feature gates + admin check | `routing::route_to_block()` |
| Feature flag trait | `features::FeatureConfig` |
| Argon2 password hashing | `crypto::hash_password()` / `verify_password()` |
| HMAC-SHA256 JWT (wasm32-compatible) | `crypto::jwt_sign()` / `jwt_verify()` |
| Request pipeline orchestration | `pipeline::handle_request()` |

## Migration Steps — COMPLETED

1. ✅ Create `solobase-core` crate (compiles for wasm32 + native)
2. ✅ Extract crypto into `solobase_core::crypto`
3. ✅ Define `FeatureConfig` trait in `solobase_core::features`
4. ✅ Move routing table into `solobase_core::routing`
5. ✅ Build `handle_request()` pipeline in `solobase_core::pipeline`
6. ~~ Define `Platform` trait ~~ — Skipped: parameters passed individually (simpler)
7. ✅ Refactor CF adapter: `SolobaseBlockFactory` + call shared pipeline
8. ✅ Refactor native adapter: `@solobase/router` block + `NativeBlockFactory` + shared instances
9. ✅ Simplify flows: `site-main` routes to `@solobase/router` instead of per-feature flows

## Legacy Compatibility

The individual per-feature flow files (auth.rs, admin.rs, etc.) are preserved for
backwards compatibility with the `blocks.json` configuration mode. When using
`app.json` (the preferred mode), only the simplified `site-main` flow is registered.
