# Project Structure

## Root Level
- `solobase.go` - Main application entry point and core App struct
- `go.mod/go.sum` - Go module dependencies
- `Makefile` - Build automation
- `README.md` - Project documentation
- `.env/.env.example` - Environment configuration

## Core Directories

### `/cmd`
Application entry points and binaries
- `cmd/solobase/main.go` - Main binary entry point
- `cmd/check-tables/` - Database utility tools

### `/internal`
Private application code (not importable by external packages)

#### `/internal/api`
HTTP API layer
- `handlers/` - Request handlers organized by domain (auth, database, users, etc.)
- `middleware/` - HTTP middleware (auth, CORS, logging, metrics, RBAC)
- `router/` - Route configuration and setup
- `routes/` - Route definitions
- `response/` - Response utilities

#### `/internal/core/services`
Business logic services
- `auth.go` - Authentication service
- `database.go` - Database operations
- `storage.go` - File storage service
- `user.go` - User management
- `settings.go` - Configuration management

#### `/internal/pkg`
Internal packages (reusable within project)
- `auth/` - Authentication implementation
- `database/` - Database adapters and models
- `storage/` - Storage providers (local, S3)
- `logger/` - Logging infrastructure
- `mailer/` - Email services

#### `/internal/data`
Data layer
- `models/` - Data models and DTOs
- `repositories/` - Data access patterns

#### `/internal/iam`
Identity and Access Management
- `service.go` - IAM business logic
- `handlers.go` - IAM HTTP handlers
- `models.go` - IAM data models
- `middleware.go` - Authorization middleware

### `/extensions`
Extension system
- `core/` - Extension framework and interfaces
- `official/` - Official extensions (analytics, cloudstorage, products, webhooks)
- `manager.go` - Extension lifecycle management
- `registry.go` - Extension registration
- `config.json` - Extension configuration
- `schema.json` - Extension schema definitions

### `/packages`
Standalone packages (can be imported externally)
- `auth/` - Authentication package
- `database/` - Database package
- `storage/` - Storage package
- `logger/` - Logging package
- `mailer/` - Email package
- `metrics/` - Metrics package
- `dynamicfields/` - Dynamic field handling
- `formulaengine/` - Formula calculation engine
- `image-tools/` - Image processing utilities

### `/ui`
Frontend Svelte application
- `src/lib/` - Reusable Svelte components
- `src/routes/` - SvelteKit routes and pages
- `static/` - Static assets
- `build/` - Production build output (embedded in Go binary)

### `/sdk`
Software Development Kits
- `typescript/` - TypeScript SDK with auto-generated types

### `/demo`
Demo and deployment examples
- `code/` - Demo setup code
- `deployment/` - Docker and deployment configurations

### `/constants`
Application constants
- `errors.go` - Error definitions
- `roles.go` - Role definitions
- `limits.go` - System limits
- `pagination.go` - Pagination constants

### `/utils`
Utility functions
- `database.go` - Database utilities
- `security.go` - Security helpers
- `validation.go` - Input validation

## File Organization Patterns

### Handler Organization
Handlers are grouped by domain in `/internal/api/handlers/`:
- Each domain has its own subdirectory
- Related handlers are co-located
- Clear separation of concerns

### Service Layer
Services in `/internal/core/services/` follow single responsibility:
- One service per domain
- Clear interfaces
- Dependency injection pattern

### Extension Structure
Extensions follow a standard pattern:
```
extensions/official/extensionname/
├── extension.go      # Main extension implementation
├── handlers.go       # HTTP handlers
├── models.go         # Data models
├── services.go       # Business logic
└── README.md         # Documentation
```

### Package Mirroring
The `/packages` directory mirrors `/internal/pkg` for external consumption:
- Same structure and interfaces
- Allows external projects to use Solobase components
- Maintains API compatibility

## Import Conventions
- Use full import paths: `github.com/suppers-ai/solobase/internal/...`
- Group imports: standard library, external packages, internal packages
- Internal packages should not import from `/packages` (use `/internal/pkg` instead)
- Extensions import from `/extensions/core` for framework interfaces