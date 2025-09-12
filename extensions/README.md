# Solobase Extension System

A powerful, compile-time extension system for Solobase that allows third-party developers to extend the platform's functionality while maintaining security and isolation.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Creating Extensions](#creating-extensions)
- [Extension Lifecycle](#extension-lifecycle)
- [API Reference](#api-reference)
- [Security](#security)
- [Testing](#testing)
- [Examples](#examples)

## Overview

The Solobase Extension System provides a compile-time plugin architecture that allows developers to extend the platform with custom functionality. Extensions are compiled into the main binary, ensuring optimal performance while maintaining isolation through schema separation and security boundaries.

### Key Features

- **Compile-time Integration**: Extensions are compiled into the main binary for optimal performance
- **Schema Isolation**: Each extension gets its own database schema
- **Security Sandboxing**: Resource quotas, rate limiting, and permission management
- **Hot-reload Configuration**: Update extension settings without restarting
- **Comprehensive Metrics**: Built-in Prometheus metrics for monitoring
- **Migration Support**: Database migration management per extension
- **Hook System**: Extend existing functionality through hooks
- **Middleware Support**: Add custom middleware to request processing

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Main Application                │
├─────────────────────────────────────────────────┤
│              Extension Registry                  │
├──────┬──────┬──────┬──────┬──────┬──────┬──────┤
│Router│Hooks │Config│Migrat│Secur │Metric│Middle│
│      │      │      │ions  │ity   │s     │ware  │
├──────┴──────┴──────┴──────┴──────┴──────┴──────┤
│              Extension Services                  │
├──────┬──────┬──────┬──────┬──────┬──────┬──────┤
│  DB  │Auth  │Logger│Store │Config│Colls │Stats │
└──────┴──────┴──────┴──────┴──────┴──────┴──────┘
```

## Creating Extensions

### Basic Extension Structure

```go
package myextension

import (
    "context"
    "github.com/suppers-ai/solobase/extensions/core"
)

type MyExtension struct {
    services *core.ExtensionServices
    enabled  bool
}

func NewMyExtension() *MyExtension {
    return &MyExtension{
        enabled: true,
    }
}

// Implement Extension interface
func (e *MyExtension) Metadata() core.ExtensionMetadata {
    return core.ExtensionMetadata{
        Name:        "my-extension",
        Version:     "1.0.0",
        Description: "My custom extension",
        Author:      "Your Name",
        License:     "MIT",
    }
}

func (e *MyExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
    e.services = services
    services.Logger().Info(ctx, "Extension initializing")
    return nil
}

func (e *MyExtension) Start(ctx context.Context) error {
    e.services.Logger().Info(ctx, "Extension started")
    return nil
}

func (e *MyExtension) Stop(ctx context.Context) error {
    e.enabled = false
    return nil
}

func (e *MyExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
    return &core.HealthStatus{
        Status:  "healthy",
        Message: "Extension is running",
    }, nil
}
```

### Registering Routes

```go
func (e *MyExtension) RegisterRoutes(router core.ExtensionRouter) error {
    router.HandleFunc("/dashboard", e.handleDashboard)
    router.HandleFunc("/api/data", e.handleData)
    return nil
}

func (e *MyExtension) handleDashboard(w http.ResponseWriter, r *http.Request) {
    w.Write([]byte("Extension Dashboard"))
}
```

### Adding Hooks

```go
func (e *MyExtension) RegisterHooks() []core.HookRegistration {
    return []core.HookRegistration{
        {
            Extension: "my-extension",
            Name:      "process-upload",
            Type:      core.HookPostRequest,
            Priority:  10,
            Handler:   e.processUploadHook,
        },
    }
}

func (e *MyExtension) processUploadHook(ctx context.Context, hctx *core.HookContext) error {
    // Process upload data
    if uploadData, ok := hctx.Data["upload"]; ok {
        // Custom processing
    }
    return nil
}
```

### Database Migrations

```go
func (e *MyExtension) DatabaseSchema() string {
    return "ext_myextension"
}

func (e *MyExtension) Migrations() []core.Migration {
    return []core.Migration{
        {
            Version:     "001",
            Description: "Create initial tables",
            Extension:   "my-extension",
            Up: `
                CREATE SCHEMA IF NOT EXISTS ext_myextension;
                CREATE TABLE ext_myextension.data (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    name TEXT NOT NULL,
                    value JSONB,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
            `,
            Down: `DROP SCHEMA IF EXISTS ext_myextension CASCADE;`,
        },
    }
}
```

### Configuration Management

```go
func (e *MyExtension) ConfigSchema() json.RawMessage {
    schema := map[string]interface{}{
        "type": "object",
        "properties": map[string]interface{}{
            "enabled": map[string]interface{}{
                "type":        "boolean",
                "description": "Enable extension",
                "default":     true,
            },
            "apiKey": map[string]interface{}{
                "type":        "string",
                "description": "API key for external service",
            },
        },
    }
    data, _ := json.Marshal(schema)
    return data
}

func (e *MyExtension) ValidateConfig(config json.RawMessage) error {
    var cfg map[string]interface{}
    return json.Unmarshal(config, &cfg)
}

func (e *MyExtension) ApplyConfig(config json.RawMessage) error {
    var cfg map[string]interface{}
    if err := json.Unmarshal(config, &cfg); err != nil {
        return err
    }
    
    if enabled, ok := cfg["enabled"].(bool); ok {
        e.enabled = enabled
    }
    
    return nil
}
```

## Extension Lifecycle

1. **Registration**: Extension is registered with the registry
2. **Initialization**: Extension receives core services
3. **Migration**: Database migrations are run
4. **Start**: Extension is started and begins processing
5. **Running**: Extension handles requests and hooks
6. **Stop**: Extension is gracefully shutdown
7. **Unregister**: Extension is removed from registry

## API Reference

### Extension Interface

```go
type Extension interface {
    // Core lifecycle
    Metadata() ExtensionMetadata
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
    DatabaseSchema() string
    Migrations() []Migration
    
    // Security
    RequiredPermissions() []Permission
}
```

### Extension Services

```go
type ExtensionServices struct {
    DB() ExtensionDatabase          // Schema-isolated database access
    Auth() ExtensionAuth            // Authentication service
    Logger() ExtensionLogger        // Logging service
    Storage() ExtensionStorage      // File storage service
    Config() ExtensionConfig        // Configuration service
    Collections() ExtensionCollections // Collections service
    Stats() ExtensionStats          // Statistics service
}
```

### Hook Types

- `HookPreRequest`: Before request processing
- `HookPostRequest`: After request processing
- `HookPreResponse`: Before response is sent
- `HookPostResponse`: After response is sent
- `HookError`: On error occurrence
- `HookAuthentication`: During authentication
- `HookAuthorization`: During authorization

## Security

### Permission System

Extensions must declare required permissions:

```go
func (e *MyExtension) RequiredPermissions() []core.Permission {
    return []core.Permission{
        {
            Name:        "myext.manage",
            Description: "Manage extension data",
            Resource:    "myext_data",
            Actions:     []string{"create", "read", "update", "delete"},
        },
    }
}
```

### Resource Quotas

Extensions are subject to resource quotas:

- Maximum memory usage
- Maximum goroutines
- Maximum database connections
- Maximum storage space
- Request rate limiting

### Schema Isolation

Each extension operates in its own database schema:

```go
// Extension can only access its own schema
db := services.DB()
rows, err := db.Query(ctx, "SELECT * FROM data") // Automatically prefixed with extension schema
```

## Testing

### Unit Testing

```go
func TestMyExtension(t *testing.T) {
    suite := core.NewExtensionTestSuite(t)
    defer suite.Cleanup()
    
    ext := NewMyExtension()
    
    // Test registration
    err := suite.Registry.Register(ext)
    assert.NoError(t, err)
    
    // Test enabling
    err = suite.Registry.Enable("my-extension")
    assert.NoError(t, err)
    
    // Test route
    resp := suite.TestRequest("GET", "/ext/my-extension/dashboard", nil)
    assert.Equal(t, http.StatusOK, resp.Code)
}
```

### Integration Testing

```go
func TestExtensionIntegration(t *testing.T) {
    suite := core.NewExtensionTestSuite(t)
    defer suite.Cleanup()
    
    ext := NewMyExtension()
    suite.LoadExtension(ext)
    
    // Test with database
    ctx := context.Background()
    db := suite.Services.DB()
    
    _, err := db.Exec(ctx, "INSERT INTO data (name, value) VALUES ($1, $2)", 
        "test", map[string]interface{}{"key": "value"})
    assert.NoError(t, err)
}
```

### Benchmarking

```go
func BenchmarkExtension(b *testing.B) {
    suite := core.NewExtensionTestSuite(&testing.T{})
    defer suite.Cleanup()
    
    ext := NewMyExtension()
    suite.LoadExtension(ext)
    
    core.BenchmarkExtension(b, suite, "GET", "/ext/my-extension/api/data")
}
```

## Examples

### Official Extensions

#### Webhooks Extension
Located at `extensions/official/webhooks/`

Features:
- Webhook management and delivery
- HMAC signature verification
- Retry logic with exponential backoff
- Delivery history tracking

#### Analytics Extension
Located at `extensions/community/analytics/`

Features:
- Event tracking
- User analytics
- Performance metrics
- Custom dashboards

### Creating a Custom Extension

1. Create extension directory:
```bash
mkdir extensions/custom/myextension
```

2. Implement extension:
```go
// extensions/custom/myextension/extension.go
package myextension

import (
    "github.com/suppers-ai/solobase/extensions/core"
)

type MyExtension struct {
    // Extension implementation
}

// Implement all required methods...
```

3. Register in main.go:
```go
import "github.com/suppers-ai/solobase/extensions/custom/myextension"

// In main()
myExt := myextension.NewMyExtension()
extensionRegistry.Register(myExt)
```

4. Build and run:
```bash
./compile.sh
./solobase
```

## Best Practices

1. **Error Handling**: Always return meaningful errors
2. **Logging**: Use structured logging with context
3. **Metrics**: Record relevant metrics for monitoring
4. **Testing**: Write comprehensive tests
5. **Documentation**: Document all public APIs
6. **Security**: Validate all inputs and respect permissions
7. **Performance**: Use caching and optimize database queries
8. **Migrations**: Always provide rollback migrations

## Troubleshooting

### Extension Not Loading

1. Check registration in main.go
2. Verify extension implements all required methods
3. Check logs for initialization errors

### Migration Failures

1. Verify SQL syntax
2. Check for schema conflicts
3. Ensure rollback migrations work

### Performance Issues

1. Check metrics for bottlenecks
2. Review database queries
3. Verify resource quotas

## Contributing

1. Fork the repository
2. Create your extension in `extensions/community/`
3. Write tests and documentation
4. Submit pull request

## License

MIT License - See LICENSE file for details