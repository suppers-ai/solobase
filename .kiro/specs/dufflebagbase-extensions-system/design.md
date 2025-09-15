# DuffleBagBase Extensions System - Technical Design Document

## Overview

The DuffleBagBase Extensions System implements a compile-time plugin architecture that leverages Go's type system and interface-based design to provide extensibility without sacrificing performance or deployment simplicity. This design prioritizes:

- **Type Safety**: Compile-time verification of extension compatibility
- **Performance**: Zero runtime overhead compared to native code
- **Security**: Controlled access to system resources through well-defined interfaces
- **Developer Experience**: Clear APIs with comprehensive tooling support
- **Operational Simplicity**: Single binary deployment with embedded extensions

## Design Principles

1. **Interface Segregation**: Extensions interact only through well-defined interfaces
2. **Dependency Injection**: Core services are injected, not globally accessed
3. **Fail-Safe Defaults**: Extensions fail gracefully without affecting core functionality
4. **Resource Isolation**: Each extension operates in its own namespace (database schemas, configuration)
5. **Audit and Observability**: All extension actions are logged and monitored

## Architecture

### Core Components

#### 1. Extension Registry
The central registry that manages all extensions and their lifecycle:

```go
package extensions

import (
    "context"
    "sync"
    "github.com/gorilla/mux"
    "github.com/suppers-ai/logger"
)

type ExtensionRegistry struct {
    mu         sync.RWMutex
    extensions map[string]Extension
    hooks      map[HookType][]HookRegistration
    routes     []RouteRegistration
    middleware []MiddlewareRegistration
    config     *ExtensionConfig
    logger     logger.Logger
    services   *ExtensionServices
    
    // Runtime state
    enabled    map[string]bool
    status     map[string]*ExtensionStatus
    metrics    map[string]*ExtensionMetrics
    
    // Error handling
    errorHandler ExtensionErrorHandler
    panicHandler ExtensionPanicHandler
}

// Thread-safe operations
func (r *ExtensionRegistry) Register(ext Extension) error
func (r *ExtensionRegistry) Enable(name string) error
func (r *ExtensionRegistry) Disable(name string) error
func (r *ExtensionRegistry) Get(name string) (Extension, bool)
func (r *ExtensionRegistry) List() []ExtensionMetadata
```

#### 2. Extension Interface
The primary interface that all extensions must implement:

```go
type Extension interface {
    // Metadata
    Metadata() ExtensionMetadata
    
    // Lifecycle hooks
    Initialize(ctx context.Context, services *ExtensionServices) error
    Start(ctx context.Context) error
    Stop(ctx context.Context) error
    Health(ctx context.Context) (*HealthStatus, error)
    
    // Registration methods
    RegisterRoutes(router ExtensionRouter) error
    RegisterMiddleware() []MiddlewareRegistration
    RegisterHooks() []HookRegistration
    RegisterTemplates() []TemplateRegistration
    RegisterStaticAssets() []StaticAssetRegistration
    
    // Configuration
    ConfigSchema() json.RawMessage
    ValidateConfig(config json.RawMessage) error
    ApplyConfig(config json.RawMessage) error
    
    // Database
    DatabaseSchema() string // Returns schema name
    Migrations() []Migration
    
    // Permissions
    RequiredPermissions() []Permission
}

type ExtensionMetadata struct {
    Name         string   `json:"name"`
    Version      string   `json:"version"`
    Description  string   `json:"description"`
    Author       string   `json:"author"`
    License      string   `json:"license"`
    Homepage     string   `json:"homepage"`
    Dependencies []string `json:"dependencies"`
    Tags         []string `json:"tags"`
    MinVersion   string   `json:"min_dufflebag_version"`
    MaxVersion   string   `json:"max_dufflebag_version"`
}
```

#### 3. Extension Services
Provides controlled access to core application services:

```go
type ExtensionServices struct {
    // Core services with controlled access
    db          database.Database
    auth        *auth.Service
    logger      logger.Logger
    storage     *services.StorageService
    config      *config.Config
    collections *services.CollectionsService
    stats       *services.StatsService
    
    // Extension-specific context
    extensionName string
    schemaName    string
}

// Controlled access methods that enforce extension boundaries
func (s *ExtensionServices) Database() ExtensionDatabase
func (s *ExtensionServices) Auth() ExtensionAuth
func (s *ExtensionServices) Logger() ExtensionLogger
func (s *ExtensionServices) Storage() ExtensionStorage
func (s *ExtensionServices) Config() ExtensionConfig

// ExtensionDatabase provides schema-isolated database access
type ExtensionDatabase interface {
    Query(ctx context.Context, query string, args ...interface{}) (*sql.Rows, error)
    Exec(ctx context.Context, query string, args ...interface{}) (sql.Result, error)
    Transaction(ctx context.Context, fn func(ExtensionTx) error) error
    // Queries are automatically prefixed with extension schema
}

// ExtensionAuth provides controlled auth access
type ExtensionAuth interface {
    GetUser(ctx context.Context, userID string) (*auth.User, error)
    ValidateToken(ctx context.Context, token string) (*auth.Claims, error)
    CheckPermission(ctx context.Context, userID string, permission string) bool
    // Cannot create/delete users or modify core permissions
}
```

#### 4. Hook System
Enables extensions to extend existing functionality:

```go
type HookType string

const (
    HookPreRequest   HookType = "pre_request"
    HookPostRequest  HookType = "post_request"
    HookPreAuth      HookType = "pre_auth"
    HookPostAuth     HookType = "post_auth"
    HookPreDatabase  HookType = "pre_database"
    HookPostDatabase HookType = "post_database"
)

type HookContext struct {
    Request    *http.Request
    Response   http.ResponseWriter
    Data       map[string]interface{}
    Services   *ExtensionServices
    Extension  string
}

type HookFunc func(ctx context.Context, hookCtx *HookContext) error
```

### Extension Discovery and Loading

#### Build-Time Registration
Extensions will be registered at build time using Go's init() functions and build tags:

```go
// +build extension_example

package example

import "github.com/suppers-ai/dufflebagbase/extensions"

func init() {
    extensions.Register(&ExampleExtension{})
}
```

#### Extension Directory Structure
```
extensions/
├── core/                 # Core extension system
│   ├── registry.go       # Extension registry implementation
│   ├── interfaces.go     # Core interfaces and types
│   ├── services.go       # Service wrappers with access control
│   ├── hooks.go          # Hook system implementation
│   ├── middleware.go     # Middleware management
│   ├── router.go         # Safe routing wrapper
│   ├── config.go         # Configuration management
│   ├── database.go       # Database isolation layer
│   ├── metrics.go        # Performance monitoring
│   ├── errors.go         # Error handling
│   └── testing.go        # Testing utilities
├── builtin/              # Built-in extensions (always included)
│   ├── api_docs/         # OpenAPI/Swagger documentation
│   ├── metrics/          # Prometheus metrics endpoint
│   └── health/           # Health check extensions
├── official/             # Official supported extensions
│   ├── analytics/        # Analytics and reporting
│   ├── audit_log/        # Enhanced audit logging
│   ├── backup/           # Backup and restore
│   ├── oauth/            # Additional OAuth providers
│   └── webhooks/         # Webhook management
├── community/            # Community-contributed extensions
│   └── example/
│       ├── extension.go      # Main extension implementation
│       ├── handlers/         # HTTP handlers
│       ├── services/         # Business logic
│       ├── models/           # Data models
│       ├── templates/        # Templ templates
│       ├── static/           # CSS/JS/images
│       ├── migrations/       # SQL migrations
│       ├── config.yaml       # Default configuration
│       ├── README.md         # Documentation
│       └── extension_test.go # Tests
└── generated.go          # Auto-generated extension registry
```

## Components and Interfaces

### 1. Extension Router
Provides a safe interface for extensions to register routes:

```go
type ExtensionRouter interface {
    HandleFunc(path string, handler http.HandlerFunc) RouteRegistration
    Handle(path string, handler http.Handler) RouteRegistration
    PathPrefix(prefix string) ExtensionRouter
    Use(middleware ...mux.MiddlewareFunc)
    
    // Restricted methods that require permissions
    RequireAuth(handler http.Handler) http.Handler
    RequireRole(role string, handler http.Handler) http.Handler
}

type RouteRegistration struct {
    Extension string
    Path      string
    Methods   []string
    Handler   http.Handler
    Protected bool
    Roles     []string
}
```

### 2. Configuration Management
Extensions can define their own configuration schemas:

```go
type ExtensionConfig struct {
    Enabled    map[string]bool                    `yaml:"enabled"`
    Config     map[string]map[string]interface{} `yaml:"config"`
    BuildTags  []string                          `yaml:"build_tags"`
    LoadOrder  []string                          `yaml:"load_order"`
}

type ConfigValidator interface {
    ValidateConfig(config interface{}) error
    DefaultConfig() interface{}
    ConfigSchema() interface{}
}
```

### 3. Database Integration
Extensions can define their own database schemas and migrations:

```go
type Migration struct {
    Version     string
    Description string
    Up          string
    Down        string
    Extension   string
}

type ExtensionDatabase interface {
    CreateSchema(ctx context.Context, schemaName string) error
    RunMigrations(ctx context.Context, extension string, migrations []Migration) error
    GetMigrationStatus(ctx context.Context, extension string) ([]MigrationStatus, error)
}
```

### 4. Middleware Integration
Extensions can register middleware that integrates with the existing pipeline:

```go
type MiddlewareRegistration struct {
    Extension string
    Name      string
    Priority  int
    Handler   mux.MiddlewareFunc
    Paths     []string // Optional: specific paths to apply to
}

type ExtensionMiddleware interface {
    Name() string
    Priority() int
    Handler() mux.MiddlewareFunc
    ApplyToPaths() []string
}
```

## Data Models

### Extension Metadata
```go
type ExtensionMetadata struct {
    Name         string            `json:"name"`
    Version      string            `json:"version"`
    Description  string            `json:"description"`
    Author       string            `json:"author"`
    License      string            `json:"license"`
    Homepage     string            `json:"homepage"`
    Dependencies []string          `json:"dependencies"`
    Tags         []string          `json:"tags"`
    Enabled      bool              `json:"enabled"`
    LoadOrder    int               `json:"load_order"`
    Config       map[string]interface{} `json:"config"`
}
```

### Extension Status and Health
```go
type ExtensionStatus struct {
    Name        string                 `json:"name"`
    Version     string                 `json:"version"`
    Enabled     bool                   `json:"enabled"`
    Loaded      bool                   `json:"loaded"`
    Health      HealthStatus           `json:"health"`
    Error       string                 `json:"error,omitempty"`
    LoadedAt    time.Time              `json:"loaded_at,omitempty"`
    Resources   ExtensionResources     `json:"resources"`
    Endpoints   []EndpointInfo         `json:"endpoints"`
    Metrics     ExtensionMetrics       `json:"metrics"`
}

type HealthStatus struct {
    Status      string                 `json:"status"` // "healthy", "degraded", "unhealthy"
    Message     string                 `json:"message,omitempty"`
    Checks      []HealthCheck          `json:"checks,omitempty"`
    LastChecked time.Time              `json:"last_checked"`
}

type ExtensionResources struct {
    Routes      int                    `json:"routes"`
    Middleware  int                    `json:"middleware"`
    Hooks       int                    `json:"hooks"`
    Templates   int                    `json:"templates"`
    Assets      int                    `json:"static_assets"`
    Migrations  int                    `json:"migrations"`
}

type ExtensionMetrics struct {
    RequestCount    int64              `json:"request_count"`
    ErrorCount      int64              `json:"error_count"`
    AverageLatency  time.Duration      `json:"average_latency"`
    P95Latency      time.Duration      `json:"p95_latency"`
    P99Latency      time.Duration      `json:"p99_latency"`
    MemoryUsage     int64              `json:"memory_bytes"`
    GoroutineCount  int                `json:"goroutine_count"`
    DatabaseQueries int64              `json:"database_queries"`
    CacheHitRate    float64            `json:"cache_hit_rate"`
}
```

## Error Handling

### Extension Isolation
Each extension will be wrapped with recovery middleware to prevent panics from crashing the main application:

```go
func (r *ExtensionRegistry) wrapExtensionHandler(ext Extension, handler http.HandlerFunc) http.HandlerFunc {
    return func(w http.ResponseWriter, req *http.Request) {
        defer func() {
            if err := recover(); err != nil {
                r.logger.Error(req.Context(), "Extension panic recovered",
                    logger.String("extension", ext.Name()),
                    logger.Any("error", err))
                
                // Disable the extension
                r.disableExtension(ext.Name())
                
                // Return error response
                http.Error(w, "Extension error", http.StatusInternalServerError)
            }
        }()
        
        handler(w, req)
    }
}
```

### Error Reporting
Extensions will have access to structured error reporting:

```go
type ExtensionError struct {
    Extension string
    Type      string
    Message   string
    Context   map[string]interface{}
    Timestamp time.Time
}

func (r *ExtensionRegistry) reportError(ext string, err error, context map[string]interface{}) {
    extErr := &ExtensionError{
        Extension: ext,
        Type:      "runtime_error",
        Message:   err.Error(),
        Context:   context,
        Timestamp: time.Now(),
    }
    
    r.logger.Error(context.Background(), "Extension error",
        logger.String("extension", ext),
        logger.Err(err),
        logger.Any("context", context))
}
```

## Testing Strategy

### Extension Testing Framework
Provide testing utilities for extension developers:

```go
type ExtensionTestSuite struct {
    Registry *ExtensionRegistry
    Services *ExtensionServices
    Router   *mux.Router
    DB       database.Database
}

func NewExtensionTestSuite() *ExtensionTestSuite {
    // Setup test environment
}

func (ts *ExtensionTestSuite) LoadExtension(ext Extension) error {
    // Load extension in test environment
}

func (ts *ExtensionTestSuite) TestRequest(method, path string, body io.Reader) *httptest.ResponseRecorder {
    // Make test request
}
```

### Integration Testing
Extensions will be tested against the main application:

```go
func TestExtensionIntegration(t *testing.T) {
    suite := NewExtensionTestSuite()
    
    // Load extension
    ext := &ExampleExtension{}
    err := suite.LoadExtension(ext)
    require.NoError(t, err)
    
    // Test extension endpoints
    resp := suite.TestRequest("GET", "/api/v1/example/test", nil)
    assert.Equal(t, http.StatusOK, resp.Code)
}
```

### Mock Services
Provide mock implementations for testing:

```go
type MockExtensionServices struct {
    DB     *MockDatabase
    Auth   *MockAuth
    Logger *MockLogger
}

func NewMockExtensionServices() *MockExtensionServices {
    return &MockExtensionServices{
        DB:     NewMockDatabase(),
        Auth:   NewMockAuth(),
        Logger: NewMockLogger(),
    }
}
```

## Security Considerations

### Extension Sandboxing
- Extensions run in the same process but with limited access to core services
- Database access is restricted to extension-specific schemas
- File system access is limited to designated directories
- Network access can be controlled through configuration

### Permission System
Extensions must declare required permissions:

```go
type ExtensionPermissions struct {
    DatabaseSchemas []string `json:"database_schemas"`
    APIEndpoints    []string `json:"api_endpoints"`
    FileAccess      []string `json:"file_access"`
    NetworkAccess   bool     `json:"network_access"`
}
```

### Code Review Process
- Community extensions go through a review process
- Automated security scanning for common vulnerabilities
- Dependency analysis for known security issues
- Code signing for verified extensions

## Performance Considerations

### Lazy Loading
Extensions are loaded only when needed:

```go
func (r *ExtensionRegistry) getExtension(name string) (Extension, error) {
    if ext, exists := r.extensions[name]; exists {
        return ext, nil
    }
    
    // Load extension on demand
    return r.loadExtension(name)
}
```

### Resource Monitoring
Track extension resource usage:

```go
type ExtensionMetrics struct {
    RequestCount    int64         `json:"request_count"`
    AverageLatency  time.Duration `json:"average_latency"`
    ErrorRate       float64       `json:"error_rate"`
    MemoryUsage     int64         `json:"memory_usage"`
    DatabaseQueries int64         `json:"database_queries"`
}
```

### Caching
Extensions can use the application's caching layer:

```go
type ExtensionCache interface {
    Get(ctx context.Context, key string) (interface{}, error)
    Set(ctx context.Context, key string, value interface{}, ttl time.Duration) error
    Delete(ctx context.Context, key string) error
    Clear(ctx context.Context, pattern string) error
}
```

## Build System Integration

### Build System Integration

#### Build Tags and Conditional Compilation
```bash
# Build with all official extensions
go build -tags "extensions_official" -o dufflebagbase

# Build with specific extensions
go build -tags "ext_analytics,ext_audit,ext_webhooks" -o dufflebagbase

# Build with community extensions
go build -tags "extensions_official,extensions_community" -o dufflebagbase-full

# Minimal build (only built-in extensions)
go build -o dufflebagbase-minimal

# Development build with hot-reload support
go build -tags "extensions_dev" -o dufflebagbase-dev
```

#### Makefile Integration
```makefile
# Extension management targets
.PHONY: extensions-list extensions-build extensions-test

extensions-list:
	@go run tools/list-extensions.go

extensions-build:
	@go generate ./extensions/...
	@go build -tags "$(EXTENSIONS)" -o dufflebagbase

extensions-test:
	@go test -tags "$(EXTENSIONS)" ./extensions/...

extensions-validate:
	@go run tools/validate-extensions.go
```

### Extension Discovery
Automatic discovery of extensions during build:

```go
//go:generate go run tools/extension-discovery.go

// Generated file: extensions/generated.go
package extensions

func init() {
    // Auto-generated extension registrations
    Register(&analytics.Extension{})
    Register(&reporting.Extension{})
}
```

### Dependency Management and Versioning

#### Extension Module Structure
```go
// go.mod for a community extension
module github.com/dufflebagbase/ext-analytics

go 1.21

require (
    github.com/suppers-ai/dufflebagbase v1.0.0
    github.com/prometheus/client_golang v1.16.0
    github.com/a-h/templ v0.2.543
)

// Version constraints in extension.go
func (e *AnalyticsExtension) Metadata() ExtensionMetadata {
    return ExtensionMetadata{
        Name:        "analytics",
        Version:     "1.2.0",
        MinVersion:  "1.0.0", // Minimum DuffleBagBase version
        MaxVersion:  "2.0.0", // Maximum DuffleBagBase version
        // ...
    }
}
```

#### Compatibility Matrix
```yaml
# extensions-compatibility.yaml
compatibility:
  dufflebagbase:
    "1.0.x":
      analytics: ["1.0.0", "1.1.0", "1.2.0"]
      audit_log: ["1.0.0", "1.0.1"]
      webhooks: ["0.9.0", "1.0.0"]
    "1.1.x":
      analytics: ["1.2.0", "1.3.0"]
      audit_log: ["1.1.0", "1.2.0"]
      webhooks: ["1.0.0", "1.1.0"]
```

## Implementation Phases

### Phase 1: Core Infrastructure (Weeks 1-2)
- Extension interfaces and registry
- Service wrappers with access control
- Basic lifecycle management
- Configuration system

### Phase 2: Integration Points (Weeks 3-4)
- Route registration with Gorilla Mux
- Middleware pipeline integration
- Hook system implementation
- Database schema isolation

### Phase 3: Developer Experience (Weeks 5-6)
- CLI tools for extension management
- Testing framework and utilities
- Documentation generator
- Example extensions

### Phase 4: Production Hardening (Weeks 7-8)
- Performance monitoring and metrics
- Resource limits and quotas
- Security audit and penetration testing
- Load testing with multiple extensions

### Phase 5: Ecosystem Development (Ongoing)
- Extension marketplace
- Community contribution guidelines
- Extension certification process
- Official extension suite