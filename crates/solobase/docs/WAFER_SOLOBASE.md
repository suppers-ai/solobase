# Solobase + WAFER Integration

How Solobase uses WAFER to provide a block-based Backend-as-a-Service platform.

---

## Overview

Solobase is a BaaS (Backend as a Service) built on WAFER-Go. Every feature is a **block** - a self-contained unit with its own backend logic and optional standalone UI. Blocks compose into flows for request processing.

```
┌─────────────────────────────────────────────────────────────┐
│                        SOLOBASE                              │
│                                                              │
│  Blocks (blocks/)                                           │
│  ┌──────────────────────────────────────────────┐           │
│  │ auth    system    users     database          │           │
│  │ storage iam       settings  logs              │           │
│  │ shares  wafer    custom_tables               │           │
│  │ userportal  products  cloudstorage  legalpages│           │
│  └──────────────────────────────────────────────┘           │
│                                                              │
│  Infrastructure (infra/)     Flows (flows/)                 │
│  ┌──────────────────────┐   ┌─────────────────┐            │
│  │ auth  cors  iam      │   │ http-infra      │            │
│  │ monitoring  rate_limit│   │ auth-pipe       │            │
│  │ readonly  security   │   │ admin-pipe      │            │
│  └──────────────────────┘   │ protected-pipe  │            │
│                              └─────────────────┘            │
├─────────────────────────────────────────────────────────────┤
│                       WAFER-GO (wafer/go/)                 │
│            (Runtime, Bridge, WASM Loader)                    │
├─────────────────────────────────────────────────────────────┤
│                      WAFER SPEC                             │
│          (Blocks, Flows, Interfaces, Registry)              │
└─────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Block-Based Structure

Each feature in Solobase is a block. Blocks with UI serve standalone HTML pages (Preact + HTM). Navigation between blocks is full page navigation.

```
solobase/
├── blocks/                    # All feature blocks (handlers + services + UI)
│   ├── auth/                  #   Authentication + login UI
│   │   ├── frontend/          #     Login page (Preact)
│   │   ├── auth.go            #     Login/signup/logout handlers
│   │   ├── apikeys.go         #     API key handlers
│   │   ├── service.go         #     AuthService
│   │   ├── token_service.go   #     TokenService
│   │   ├── apikey_service.go  #     APIKeyService
│   │   └── routes.go          #     Route registration
│   ├── system/                #   Dashboard, nav, health, metrics + UI
│   │   ├── frontend/          #     Dashboard page (Preact)
│   │   ├── dashboard.go       #     Dashboard stats handler
│   │   ├── nav.go             #     /api/nav endpoint
│   │   ├── health.go          #     Health check
│   │   ├── metrics_service.go #     MetricsService
│   │   └── routes.go          #     Route registration
│   ├── users/                 #   User management + UI
│   │   ├── frontend/          #     Users page (Preact)
│   │   └── service.go         #     UserService
│   ├── database/              #   Database admin + UI
│   │   ├── frontend/          #     Database page (Preact)
│   │   └── service.go         #     DatabaseService
│   ├── storage/               #   Storage management + UI
│   │   ├── frontend/          #     Storage page (Preact)
│   │   ├── service.go         #     StorageService
│   │   ├── bucket_service.go  #     Bucket operations
│   │   ├── token_service.go   #     Storage token operations
│   │   └── stats_service.go   #     Storage stats
│   ├── iam/                   #   Identity & Access Management + UI
│   │   └── frontend/          #     IAM page (Preact)
│   ├── settings/              #   App settings + UI
│   │   ├── frontend/          #     Settings page (Preact)
│   │   └── service.go         #     SettingsService
│   ├── logs/                  #   Log viewer + UI
│   │   ├── frontend/          #     Logs page (Preact)
│   │   ├── service.go         #     LogsService
│   │   └── db_logger.go       #     Database logger
│   ├── shares/                #   Share link management
│   │   └── service.go         #     SharesService
│   ├── wafer/                #   Wafer admin (flows + blocks browser)
│   │   ├── frontend/          #     Wafer admin page (Preact)
│   │   └── handlers.go        #     Flow/block listing handlers
│   ├── custom_tables/         #   Custom table management
│   ├── userportal/            #   User-facing portal (login, profile, etc.)
│   │   └── frontend/          #     Multi-page Preact app (own Vite build)
│   ├── products/              #   Products & pricing
│   ├── cloudstorage/          #   Cloud storage quotas & bandwidth
│   └── legalpages/            #   Legal pages editor
│
├── flows/                     # Wafer flow definitions
│   ├── flows.go               #   BuildAll, base flows, feature flows
│   └── blocks.go              #   Block registration + Deps struct
│
├── infra/                     # Infrastructure blocks (wafer blocks)
│   ├── auth.go                #   JWT/cookie/API key validation
│   ├── cors.go                #   CORS headers
│   ├── iam.go                 #   Role-based access control
│   ├── monitoring.go          #   Request monitoring/metrics
│   ├── rate_limit.go          #   Rate limiting
│   ├── readonly.go            #   Read-only mode guard
│   ├── security_headers.go    #   Security response headers
│   └── register.go            #   RegisterAll helper
│
├── adapter/                   # Wafer adapter for mux routers
│   └── router_block.go        #   RouterBlock wraps *mux.Router as wafer block
│
├── wafer/                    # WAFER runtime (separate Go sub-module)
│   ├── go/                    #   wafer-go module (own go.mod)
│   │   ├── bridge/            #     HTTP-to-message bridge
│   │   ├── adapter/           #     Block adapters (handler, service)
│   │   ├── wasm/              #     WASM block loader
│   │   ├── wafer.go          #     Runtime entry point
│   │   ├── executor.go        #     Flow executor
│   │   ├── registry.go        #     Block/flow registry
│   │   └── types.go           #     Core types (Block, Flow, Node, Message)
│   └── spec/                  #   WAFER specification docs
│
├── frontend/                  # Vite build project (config + output only)
│   ├── vite.config.ts         #   Multi-page build config (inputs from blocks)
│   ├── tailwind.config.js
│   ├── src/app.css            #   Shared global styles
│   └── build/                 #   Vite output (embedded via go:embed)
│
├── shared/                    # Shared packages (npm workspaces)
│   ├── types/                 #   @solobase/types (TypeScript types)
│   ├── utils/                 #   @solobase/utils (formatters, colors)
│   └── ui/                    #   @solobase/ui (shared Preact components)
│
├── models/                    # Domain models (user, settings, logs, etc.)
├── interfaces/                # Core interfaces (Database, Logger, Storage, etc.)
├── constants/                 # App-wide constants (roles, errors, limits)
├── config/                    # App configuration
├── schema/                    # Database schema definitions
├── pkg/                       # Shared Go utilities
│   ├── auth/                  #   Auth helpers (context, hash, models)
│   ├── uuid/                  #   UUID generation
│   ├── envutil/               #   Environment variable helpers
│   ├── httputil/              #   HTTP client helpers
│   ├── apptime/               #   Time utilities
│   ├── dynamicfields/         #   Dynamic field validation
│   └── formulaengine/         #   Formula/expression evaluator
│
├── adapters/                  # External integrations
│   ├── auth/                  #   JWT, OAuth providers
│   ├── repos/                 #   Database repositories (SQLite, WASM)
│   ├── storage/               #   File storage backends (local, S3)
│   ├── crypto/                #   Hashing, JWT signing
│   ├── database/              #   Database adapters (SQLite, WASM)
│   ├── iam/                   #   IAM implementations
│   └── mailer/                #   Email sending
│
├── internal/                  # Internal packages
│   ├── logger/                #   Logging infrastructure
│   └── database/              #   Database config
│
├── builds/
│   ├── go/                    #   Standard Go binary build
│   └── wasm/                  #   TinyGo WASM build
│
├── go.work                    # Go workspace (links root + wafer/go)
├── solobase.go                # Main app wiring
└── ui.go                      # UI embed + serving (go:embed frontend/build)
```

### How Blocks Work

Each block is a Go package that is registered with the wafer runtime and composed into flows. Requests flow through flows of infrastructure blocks before reaching the feature block:

1. **Registers as a wafer block** via `RouterBlock` adapter (wraps a `*mux.Router`)
2. **Handles requests** using its own colocated service layer
3. **Serves its UI** as a standalone HTML page (if it has one)

```
HTTP Request → Bridge (httpToMessage)
  → Flow: http-infra (security-headers → cors → readonly → rate-limit → monitoring)
    → admin-pipe (auth-block → iam-block{role:admin})
      → feature block (e.g. database-feature)
        → Bridge (writeHTTPResponse)

Examples:
  GET  /admin/database     → http-infra → admin-pipe → database-feature (serves HTML)
  POST /api/auth/login     → http-infra → auth-feature (validates credentials)
  GET  /api/dashboard/stats → http-infra → auth-pipe → system-feature (returns JSON)
```

### UI Architecture

Block UIs use **Preact + HTM** (no JSX, minimal runtime ~4KB).

- Each block with UI has a standalone HTML page
- Pages share common components via `@solobase/ui` (tree-shaken at build time)
- Navigation sidebar loads items from `/api/nav` (dynamic, extensible)
- Auth is checked on every page load (cookie-based, no shared SPA state)
- Block UIs are built together (Vite multi-page build, auto code-split)
- User portal has its own separate Vite build in `blocks/userportal/frontend/`

```
┌──────────────────────────────────────────────┐
│ BlockShell (from @solobase/ui)               │
│ ┌────────────┐  ┌─────────────────────────┐  │
│ │  Sidebar   │  │  Block Page Content     │  │
│ │            │  │                         │  │
│ │  Dashboard │  │  (each block renders    │  │
│ │  Users     │  │   its own Preact        │  │
│ │  Database ←│  │   components here)      │  │
│ │  Storage   │  │                         │  │
│ │  IAM       │  │                         │  │
│ │  Logs      │  │                         │  │
│ │  Settings  │  │                         │  │
│ │            │  │                         │  │
│ └────────────┘  └─────────────────────────┘  │
└──────────────────────────────────────────────┘
```

---

## Core Blocks

### Auth Block (UI)

Handles authentication - login, signup, logout, session management, API keys. Includes the login page UI at `/admin/login`.

**UI components:** Login form, OAuth provider buttons, error display
**Flows:**
- `auth-login`: Validates credentials, issues session cookie
- `auth-signup`: Creates new user account
- `auth-check`: Verifies session (used by other blocks)

**API:**
```
POST /api/auth/login      → Login with email/password
POST /api/auth/signup     → Create account
POST /api/auth/logout     → End session
GET  /api/auth/me         → Get current user + roles
```

### Admin Guard (admin-pipe flow)

The `admin-pipe` flow in `flows/flows.go` composes the `auth-block` (JWT/cookie/API key validation from `infra/auth.go`) and the `iam-block` (role check from `infra/iam.go` with `{role: "admin"}`). All admin feature flows include `admin-pipe` before the feature block.

### System Block (UI)

Admin dashboard at `/admin`, plus navigation API, health check, and system metrics.

**UI components:** Stat cards, activity charts (Chart.js), system metrics
**API:**
```
GET /api/nav              → Navigation items (used by sidebar)
GET /api/dashboard/stats  → Dashboard statistics
GET /api/health           → Health check
GET /api/metrics          → System metrics
```
**Dependencies:** admin-pipe (admin routes), auth-pipe (protected routes)

### Users Block (UI)

User management at `/admin/users`.

**UI components:** User list table, edit modal, delete confirmation
**API:** `GET/PATCH/DELETE /api/admin/users/*`
**Dependencies:** admin-pipe flow

### Database Block (UI)

Database admin at `/admin/database`.

**UI components:** Table sidebar, SQL editor, query results table, database stats
**API:**
```
GET  /api/admin/database/tables           → List tables
GET  /api/admin/database/tables/:t/columns → Table columns
POST /api/admin/database/query            → Execute SQL
```
**Dependencies:** admin-pipe flow

### Storage Block (UI)

File storage management at `/admin/storage`.

**UI components:** File explorer, bucket selector, breadcrumb, upload/create/delete/rename/preview modals
**API:**
```
GET    /api/storage/buckets             → List buckets
GET    /api/storage/buckets/:b/objects  → List objects
POST   /api/storage/buckets/:b/upload   → Upload file
POST   /api/storage/buckets/:b/folders  → Create folder
DELETE /api/storage/buckets/:b/objects/:id → Delete
```
**Dependencies:** admin-pipe (for admin view), auth-pipe (for user view)

### IAM Block (UI)

Identity & Access Management at `/admin/iam`.

**UI components:** Roles manager, policies manager, audit log, role/policy CRUD modals
**API:** `GET/POST/DELETE /api/admin/iam/*`
**Dependencies:** admin-pipe flow

### Settings Block (UI)

App settings at `/admin/settings`.

**UI components:** Settings form (app name, signup toggle, mailer, storage provider, etc.)
**API:** `GET /api/settings`, `PATCH /api/admin/settings`
**Dependencies:** admin-pipe flow

### Logs Block (UI)

Activity logs at `/admin/logs`.

**UI components:** Log table with filters, export button
**API:** `GET /api/admin/logs`
**Dependencies:** admin-pipe flow

### Navigation API (part of system block)

The system block provides the `/api/nav` endpoint that returns available navigation items based on user role and enabled extensions. The sidebar component in `@solobase/ui` fetches from this endpoint.

```json
GET /api/nav → [
  { "title": "Dashboard", "href": "/admin", "icon": "layout-dashboard" },
  { "title": "Users", "href": "/admin/users", "icon": "users" },
  { "title": "Database", "href": "/admin/database", "icon": "database" },
  { "title": "Storage", "href": "/admin/storage", "icon": "hard-drive" },
  { "title": "IAM", "href": "/admin/iam", "icon": "shield" },
  { "title": "Logs", "href": "/admin/logs", "icon": "file-text" },
  { "title": "Settings", "href": "/admin/settings", "icon": "settings" }
]
```

Blocks register their own nav items via `WithAdminUI()` during block registration.

---

## Additional Blocks

These blocks add domain-specific functionality. They live alongside the core blocks in `blocks/` and are registered as wafer blocks with their own flows.

### User Portal (`blocks/userportal/`)

User-facing authentication portal and profile management. Has its own separate Vite build in `blocks/userportal/frontend/`.

**Pages:** Login, signup, profile, OAuth callback, checkout, products
**Dependencies:** Auth block

### Products (`blocks/products/`)

E-commerce system for product catalog, pricing, and checkout.

**API:** Product CRUD, groups, pricing templates, purchases, webhooks
**Dependencies:** Auth block, storage block (for product images)

### Cloud Storage (`blocks/cloudstorage/`)

Cloud storage with quotas, bandwidth tracking, and sharing.

**API:** Quota management, bandwidth tracking, storage hooks
**Dependencies:** Storage block

### Legal Pages (`blocks/legalpages/`)

Terms of service and privacy policy editor.

**API:** Public terms/privacy endpoints, admin editor endpoints
**Dependencies:** Admin guard (admin-pipe flow)

---

## Shared UI Package (@solobase/ui)

Shared Preact + HTM components used by all blocks. Installed as an npm workspace dependency, tree-shaken at build time.

### Components

| Component | Description |
|-----------|-------------|
| `BlockShell` | Page wrapper with sidebar, header, auth check, toast container |
| `Sidebar` | Navigation sidebar (items from `/api/nav`) |
| `Button` | Primary, secondary, danger, ghost, link variants |
| `Modal` | Overlay dialog with keyboard handling |
| `ConfirmDialog` | Confirmation modal with cancel/confirm |
| `Toast` | Auto-dismissing notification |
| `DataTable` | Sortable, paginated data table |
| `PageHeader` | Page title with optional actions |
| `Pagination` | Table pagination controls |
| `EmptyState` | Placeholder for empty lists |
| `LoadingSpinner` | Loading indicator |
| `StatCard` | Metric display card |
| `StatusBadge` | Status indicator badge |
| `TabNavigation` | Tab switcher |
| `SearchInput` | Search with debounce |
| `ExportButton` | CSV/JSON export |

### State Management

Uses **@preact/signals** (reactive, ~1KB):

```ts
// Auth state (shared across all block pages)
import { signal, computed } from '@preact/signals';

export const authState = signal({ user: null, roles: [], loading: true });
export const isAuthenticated = computed(() => !!authState.value.user);
export const currentUser = computed(() => authState.value.user);
```

### API Client

Cookie-based authentication (httpOnly cookies). No tokens stored client-side.

```ts
import { api } from '@solobase/ui';

const users = await api.get('/admin/users');
const result = await api.post('/admin/database/query', { query: 'SELECT * FROM users' });
```

---

## Build & Embed Pipeline

### Frontend Build

Block UIs are built together as a Vite multi-page application. Page sources live colocated in each block's `frontend/` directory, but are compiled by a single Vite project in `frontend/`:

```
npm run build (in frontend/)
  → Vite root is repo root, inputs from blocks/*/frontend/
  → Builds 9 HTML pages + shared asset chunks
  → Output: frontend/build/
      blocks/auth/frontend/index.html
      blocks/system/frontend/index.html
      blocks/users/frontend/index.html
      blocks/database/frontend/index.html
      blocks/storage/frontend/index.html
      blocks/iam/frontend/index.html
      blocks/settings/frontend/index.html
      blocks/logs/frontend/index.html
      blocks/wafer/frontend/index.html
      assets/
        shared-abc123.js      ← Preact + shared components (auto-split)
        dashboard-def456.js   ← Dashboard-specific code
        style-xyz.css         ← Shared Tailwind CSS
```

### Go Embed

```go
// ui.go
//go:embed all:frontend/build/*
var uiFiles embed.FS

// Each block's UI page is served by name
func (app *App) ServeBlockUI(blockName string) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        uiFS, _ := fs.Sub(uiFiles, "frontend/build")
        data, _ := fs.ReadFile(uiFS, "blocks/"+blockName+"/frontend/index.html")
        w.Header().Set("Content-Type", "text/html; charset=utf-8")
        w.Write(data)
    }
}
```

### WASM Build

```bash
# Build frontend, copy to WASM build dir, compile with TinyGo
make build-wasm
```

The Preact build uses `assets/` (not SvelteKit's `_app/`), so no TinyGo-specific renaming is needed.

### Bundle Size

| Component | Size (gzipped) |
|-----------|---------------|
| Preact | ~3KB |
| HTM | ~700B |
| @preact/signals | ~1KB |
| Shared chunks (components, Tailwind) | ~15-25KB |
| Per-page unique code | ~5-15KB |

Each page loads ~25-45KB total. Significantly smaller than a monolithic SPA.

---

## Flow Examples

These are the actual flows defined in `flows/flows.go`.

### Base Flows (reusable infrastructure compositions)

```
Flow: http-infra
  security-headers → cors → readonly-guard → rate-limit → monitoring

Flow: auth-pipe
  auth-block (validates JWT/cookie/API key, sets auth.* meta)

Flow: admin-pipe
  auth-block → iam-block {role: "admin"}

Flow: protected-pipe
  auth-block (alias for auth-pipe)
```

### Auth Flow (mixed public/protected)

```
Flow: auth
  Routes: POST /auth/login, POST /auth/signup, GET /auth/me, etc.
  http-infra →
    ├── POST:/auth/login      → auth-feature (public, no auth needed)
    ├── POST:/auth/signup     → auth-feature (public)
    ├── *:/auth/oauth/**      → auth-feature (public, OAuth flow)
    └── *:/auth/**            → auth-pipe → auth-feature (protected)
```

### Admin-Only Flows (simple pattern)

Admin-only features use a helper: `http-infra → admin-pipe → feature-block`.

```
Flow: users       → http-infra → admin-pipe → users-feature
Flow: database    → http-infra → admin-pipe → database-feature
Flow: logs        → http-infra → admin-pipe → logs-feature
Flow: iam-admin   → http-infra → admin-pipe → iam-feature
Flow: wafer-admin → http-infra → admin-pipe → wafer-admin-feature
```

### System Flow (mixed public/protected/admin)

```
Flow: system
  Routes: /health, /dashboard/stats, /nav, /admin/system/*, /admin/metrics
  http-infra →
    ├── GET:/health           → system-feature (public)
    ├── GET:/dashboard/stats  → auth-pipe → system-feature (protected)
    ├── GET:/nav              → auth-pipe → system-feature (protected)
    └── *:/admin/**           → admin-pipe → system-feature (admin)
```

### Storage Flow (mixed public/protected/admin)

```
Flow: storage
  Routes: /storage/direct/{token}, /storage/*, /admin/storage/*
  http-infra →
    ├── GET:/storage/direct/{token} → storage-feature (public, direct download)
    ├── *:/admin/storage/**         → admin-pipe → storage-feature (admin)
    └── *:/storage/**               → auth-pipe → storage-feature (protected)
```

---

## Adding a New Block

### 1. Create the Go block

```go
// blocks/myfeature/handlers.go
package myfeature

import (
    "encoding/json"
    "net/http"
    "github.com/gorilla/mux"
)

// RegisterAdminRoutes registers routes WITHOUT the /admin/ prefix.
// The RouterBlock adapter adds the prefix via subrouter.
func RegisterAdminRoutes(r *mux.Router, svc *MyFeatureService) {
    r.HandleFunc("/myfeature/data", handleGetData(svc)).Methods("GET", "OPTIONS")
}

func handleGetData(svc *MyFeatureService) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        data, _ := svc.GetData(r.Context())
        json.NewEncoder(w).Encode(data)
    }
}
```

```go
// blocks/myfeature/service.go
package myfeature

type MyFeatureService struct {
    // dependencies
}
```

### 2. Create the UI page (if needed)

The frontend lives inside the block directory:

```
blocks/myfeature/
├── handlers.go
├── service.go
├── routes.go
└── frontend/          ← Preact page colocated with Go code
    ├── index.html     ← Entry HTML
    ├── main.ts        ← render(MyFeaturePage, #app)
    └── MyFeaturePage.ts
```

```ts
// main.ts
import { render } from 'preact';
import { html } from '@solobase/ui';
import { MyFeaturePage } from './MyFeaturePage';
import '@app/app.css';

render(html`<${MyFeaturePage} />`, document.getElementById('app')!);
```

```ts
// MyFeaturePage.ts
import { html } from '@solobase/ui';
import { BlockShell, DataTable, PageHeader } from '@solobase/ui';
import { api } from '@solobase/ui';

export function MyFeaturePage() {
  // Fetch data, render UI
  return html`
    <${BlockShell} title="My Feature">
      <${PageHeader} title="My Feature" />
      <${DataTable} columns=${columns} data=${data} />
    <//>
  `;
}
```

### 3. Add to Vite config

```ts
// frontend/vite.config.ts
input: {
  // ...existing pages
  myfeature: resolve(__dirname, '../blocks/myfeature/frontend/index.html'),
}
```

### 4. Register block and flow

In `flows/blocks.go`, register the block as a `RouterBlock`:

```go
// flows/blocks.go — inside registerFeatureBlocks()
const blockMyFeature = "myfeature-feature"

myRouter := mux.NewRouter()
myfeature.RegisterAdminRoutes(myRouter.PathPrefix("/admin").Subrouter(), deps.MyFeatureService)
w.RegisterBlock(blockMyFeature, adapter.NewRouterBlock(blockMyFeature, "My feature routes", myRouter).
    WithAdminUI(wafer.AdminUIInfo{Path: "/admin/myfeature", Icon: "star", Title: "My Feature"}))
```

In `flows/flows.go`, add the flow using the `addAdminFlow` helper:

```go
// flows/flows.go — inside buildFeatureFlows()
addAdminFlow(w, "myfeature", blockMyFeature, "My feature",
    []wafer.HTTPRoute{{Path: "/admin/myfeature", PathPrefix: true}})
```

This creates: `http-infra → admin-pipe → myfeature-feature`, with the nav item automatically registered via `WithAdminUI`.

---

## CLI Integration

```bash
# Run Solobase
solobase run

# Create new block
solobase new block my-cache --interface cache@v1

# Create new app from template
solobase new app my-api --template api-with-auth

# View logs
solobase logs --follow

# Build for deployment
make build          # Go binary with embedded UIs
make build-wasm     # WASM module for Cloudflare Workers
```

---

## Roadmap

### Phase 1: Core
- [x] Auth block (login, signup, session) + login UI
- [x] Admin guard (admin-pipe flow)
- [x] System block (dashboard, nav, health, metrics) + UI
- [x] Users block + UI
- [x] Database block + UI
- [x] Storage block + UI
- [x] IAM block + UI
- [x] Settings block + UI
- [x] Logs block + UI
- [x] Block-based frontend (Preact + HTM migration)
- [x] Colocated frontend (page sources in block directories)
- [x] Refactor: blocks at top level, services inside blocks, infra blocks

### Phase 2: WAFER Runtime
- [x] Flow execution engine
- [x] Infrastructure blocks (auth, cors, iam, monitoring, rate-limit, readonly, security-headers)
- [x] HTTP bridge (httpToMessage / writeHTTPResponse)
- [x] Block registry browser
- [ ] Connection blocks (MQTT, WebSocket, Cron)
- [ ] Visual flow editor

### Phase 3: Monitoring
- [ ] Prometheus metrics
- [ ] Log aggregation
- [ ] Distributed tracing
- [ ] Alerting

### Phase 4: Enterprise
- [ ] Multi-tenant support
- [ ] Audit logging
- [ ] SSO integration

---

## License

MIT License
