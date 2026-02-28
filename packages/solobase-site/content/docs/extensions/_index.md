---
title: "Extensions"
description: "Extend Solobase with powerful compile-time extensions"
weight: 60
---

# Extensions

Solobase provides a powerful, compile-time extension system that allows developers to extend the platform's functionality while maintaining security and isolation. Extensions are compiled into your application, ensuring optimal performance while maintaining isolation through schema separation and security boundaries.

## Overview

The extension system enables you to:
- Add custom API endpoints and functionality
- Extend the admin dashboard with new interfaces
- Hook into existing functionality
- Manage database migrations per extension
- Implement custom middleware
- Create reusable components

## Quick Start

### Using Official Extensions

```go
package main

import (
    "github.com/suppers-ai/solobase"
    "github.com/suppers-ai/solobase/extensions/official/webhooks"
    "github.com/suppers-ai/solobase/extensions/official/analytics"
)

func main() {
    app := solobase.New()

    // Register official extensions
    app.RegisterExtension(webhooks.New())
    app.RegisterExtension(analytics.New())

    app.Initialize()
    app.Start()
}
```

## Creating Your Own Extension

### Basic Extension Structure

Create a new extension by implementing the Extension interface:

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

func New() *MyExtension {
    return &MyExtension{
        enabled: true,
    }
}

// Metadata returns information about the extension
func (e *MyExtension) Metadata() core.ExtensionMetadata {
    return core.ExtensionMetadata{
        Name:        "my-extension",
        Version:     "1.0.0",
        Description: "My custom extension",
        Author:      "Your Name",
        License:     "MIT",
    }
}

// Initialize sets up the extension with core services
func (e *MyExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
    e.services = services
    services.Logger().Info(ctx, "Extension initializing")
    return nil
}

// Start begins the extension's operation
func (e *MyExtension) Start(ctx context.Context) error {
    e.services.Logger().Info(ctx, "Extension started")
    return nil
}

// Stop gracefully shuts down the extension
func (e *MyExtension) Stop(ctx context.Context) error {
    e.enabled = false
    return nil
}

// Health returns the extension's health status
func (e *MyExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
    return &core.HealthStatus{
        Status:  "healthy",
        Message: "Extension is running",
    }, nil
}
```

### Adding Routes

Extensions can register their own API routes:

```go
func (e *MyExtension) RegisterRoutes(router core.ExtensionRouter) error {
    // Register API endpoints
    router.HandleFunc("/api/my-extension/data", e.handleData).Methods("GET")
    router.HandleFunc("/api/my-extension/process", e.handleProcess).Methods("POST")

    // Register admin dashboard routes
    router.HandleFunc("/admin/my-extension", e.handleDashboard).Methods("GET")

    return nil
}

func (e *MyExtension) handleData(w http.ResponseWriter, r *http.Request) {
    // Handle data request
    data := map[string]interface{}{
        "message": "Hello from extension",
        "enabled": e.enabled,
    }
    json.NewEncoder(w).Encode(data)
}

func (e *MyExtension) handleProcess(w http.ResponseWriter, r *http.Request) {
    // Process incoming data
    var input map[string]interface{}
    if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }

    // Process the input...
    result := processData(input)

    json.NewEncoder(w).Encode(result)
}
```

### Database Migrations

Extensions can manage their own database schema and migrations:

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

                CREATE TABLE ext_myextension.settings (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    key VARCHAR(255) UNIQUE NOT NULL,
                    value JSONB,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE ext_myextension.events (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    type VARCHAR(100) NOT NULL,
                    payload JSONB,
                    processed BOOLEAN DEFAULT FALSE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX idx_events_type ON ext_myextension.events(type);
                CREATE INDEX idx_events_processed ON ext_myextension.events(processed);
            `,
            Down: `DROP SCHEMA IF EXISTS ext_myextension CASCADE;`,
        },
        {
            Version:     "002",
            Description: "Add user tracking",
            Extension:   "my-extension",
            Up: `
                ALTER TABLE ext_myextension.events
                ADD COLUMN user_id UUID,
                ADD COLUMN metadata JSONB;
            `,
            Down: `
                ALTER TABLE ext_myextension.events
                DROP COLUMN user_id,
                DROP COLUMN metadata;
            `,
        },
    }
}
```

### Using Hooks

Extensions can hook into various system events:

```go
func (e *MyExtension) RegisterHooks() []core.HookRegistration {
    return []core.HookRegistration{
        {
            Extension: "my-extension",
            Name:      "before-user-create",
            Type:      core.HookPreRequest,
            Priority:  10,
            Handler:   e.beforeUserCreate,
        },
        {
            Extension: "my-extension",
            Name:      "after-file-upload",
            Type:      core.HookPostRequest,
            Priority:  20,
            Handler:   e.afterFileUpload,
        },
        {
            Extension: "my-extension",
            Name:      "on-auth",
            Type:      core.HookAuthentication,
            Priority:  5,
            Handler:   e.onAuthentication,
        },
    }
}

func (e *MyExtension) beforeUserCreate(ctx context.Context, hctx *core.HookContext) error {
    // Validate or modify user data before creation
    if userData, ok := hctx.Data["user"].(map[string]interface{}); ok {
        // Add custom validation
        if email, ok := userData["email"].(string); ok {
            if !isValidEmail(email) {
                return fmt.Errorf("invalid email format")
            }
        }

        // Add default metadata
        userData["metadata"] = map[string]interface{}{
            "source": "my-extension",
            "timestamp": time.Now(),
        }
    }
    return nil
}

func (e *MyExtension) afterFileUpload(ctx context.Context, hctx *core.HookContext) error {
    // Process uploaded files
    if file, ok := hctx.Data["file"].(map[string]interface{}); ok {
        // Log file upload
        e.services.Logger().Info(ctx, "File uploaded",
            "filename", file["name"],
            "size", file["size"])

        // Trigger processing if needed
        go e.processUploadedFile(ctx, file)
    }
    return nil
}
```

### Configuration Management

Extensions can have their own configuration:

```go
func (e *MyExtension) ConfigSchema() json.RawMessage {
    schema := map[string]interface{}{
        "type": "object",
        "properties": map[string]interface{}{
            "enabled": map[string]interface{}{
                "type":        "boolean",
                "description": "Enable or disable the extension",
                "default":     true,
            },
            "apiKey": map[string]interface{}{
                "type":        "string",
                "description": "API key for external service integration",
            },
            "webhookUrl": map[string]interface{}{
                "type":        "string",
                "format":      "uri",
                "description": "Webhook URL for notifications",
            },
            "maxRetries": map[string]interface{}{
                "type":        "integer",
                "minimum":     1,
                "maximum":     10,
                "default":     3,
                "description": "Maximum number of retry attempts",
            },
        },
        "required": []string{"enabled"},
    }

    data, _ := json.Marshal(schema)
    return data
}

func (e *MyExtension) ValidateConfig(config json.RawMessage) error {
    var cfg map[string]interface{}
    if err := json.Unmarshal(config, &cfg); err != nil {
        return fmt.Errorf("invalid config format: %w", err)
    }

    // Custom validation logic
    if apiKey, ok := cfg["apiKey"].(string); ok {
        if len(apiKey) < 32 {
            return fmt.Errorf("API key must be at least 32 characters")
        }
    }

    return nil
}

func (e *MyExtension) ApplyConfig(config json.RawMessage) error {
    var cfg struct {
        Enabled     bool   `json:"enabled"`
        APIKey      string `json:"apiKey"`
        WebhookURL  string `json:"webhookUrl"`
        MaxRetries  int    `json:"maxRetries"`
    }

    if err := json.Unmarshal(config, &cfg); err != nil {
        return err
    }

    e.enabled = cfg.Enabled
    // Apply other configuration...

    return nil
}
```

## Extension Services

Extensions have access to a comprehensive set of services:

### Database Service

```go
// Schema-isolated database access
db := e.services.DB()

// All queries are automatically scoped to the extension's schema
rows, err := db.Query(ctx, "SELECT * FROM settings WHERE key = $1", "my-key")

// Transaction support
tx, err := db.Begin(ctx)
defer tx.Rollback(ctx)

_, err = tx.Exec(ctx, "INSERT INTO events (type, payload) VALUES ($1, $2)",
    "user.created", payload)

err = tx.Commit(ctx)
```

### Authentication Service

```go
auth := e.services.Auth()

// Verify user permissions
hasPermission := auth.HasPermission(ctx, userID, "myext.manage")

// Get current user
user, err := auth.GetCurrentUser(ctx, r)

// Create API tokens
token, err := auth.CreateToken(ctx, userID, []string{"read", "write"})
```

### Storage Service

```go
storage := e.services.Storage()

// Upload file
fileID, err := storage.Upload(ctx, "path/to/file.pdf", fileData)

// Download file
data, err := storage.Download(ctx, fileID)

// Delete file
err = storage.Delete(ctx, fileID)

// List files
files, err := storage.List(ctx, "path/to/")
```

### Logger Service

```go
logger := e.services.Logger()

// Structured logging
logger.Info(ctx, "Processing request",
    "user_id", userID,
    "action", "create",
    "resource", "document")

logger.Error(ctx, "Failed to process",
    "error", err,
    "retry_count", retries)
```

## Security

### Permission System

Extensions must declare required permissions:

```go
func (e *MyExtension) RequiredPermissions() []core.Permission {
    return []core.Permission{
        {
            Name:        "myext.read",
            Description: "Read extension data",
            Resource:    "myext_data",
            Actions:     []string{"read"},
        },
        {
            Name:        "myext.manage",
            Description: "Manage extension settings",
            Resource:    "myext_settings",
            Actions:     []string{"create", "read", "update", "delete"},
        },
    }
}
```

### Resource Quotas

Extensions are subject to configurable resource quotas:

- Maximum memory usage
- Maximum goroutines
- Maximum database connections
- Maximum storage space
- Request rate limiting

### Schema Isolation

Each extension operates in its own database schema, preventing unauthorized access to other data:

```go
// This query is automatically scoped to ext_myextension schema
db.Query(ctx, "SELECT * FROM settings")
// Executes: SELECT * FROM ext_myextension.settings
```

## Middleware

Extensions can add custom middleware:

```go
func (e *MyExtension) RegisterMiddleware() []core.MiddlewareRegistration {
    return []core.MiddlewareRegistration{
        {
            Name:     "rate-limiter",
            Priority: 10,
            Handler:  e.rateLimitMiddleware,
        },
        {
            Name:     "auth-validator",
            Priority: 20,
            Handler:  e.authValidatorMiddleware,
        },
    }
}

func (e *MyExtension) rateLimitMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        // Implement rate limiting logic
        if !e.checkRateLimit(r) {
            http.Error(w, "Rate limit exceeded", http.StatusTooManyRequests)
            return
        }
        next.ServeHTTP(w, r)
    })
}
```

## Testing Extensions

### Unit Testing

```go
func TestMyExtension(t *testing.T) {
    // Create test suite
    suite := core.NewExtensionTestSuite(t)
    defer suite.Cleanup()

    // Create and register extension
    ext := New()
    err := suite.Registry.Register(ext)
    assert.NoError(t, err)

    // Test initialization
    err = suite.Registry.Enable("my-extension")
    assert.NoError(t, err)

    // Test API endpoint
    resp := suite.TestRequest("GET", "/api/my-extension/data", nil)
    assert.Equal(t, http.StatusOK, resp.Code)

    var data map[string]interface{}
    err = json.Unmarshal(resp.Body.Bytes(), &data)
    assert.NoError(t, err)
    assert.Equal(t, "Hello from extension", data["message"])
}
```

### Integration Testing

```go
func TestExtensionIntegration(t *testing.T) {
    suite := core.NewExtensionTestSuite(t)
    defer suite.Cleanup()

    ext := New()
    suite.LoadExtension(ext)

    // Test with real database
    ctx := context.Background()
    db := suite.Services.DB()

    // Insert test data
    _, err := db.Exec(ctx,
        "INSERT INTO settings (key, value) VALUES ($1, $2)",
        "test-key", map[string]interface{}{"data": "test"})
    assert.NoError(t, err)

    // Query data
    var value map[string]interface{}
    err = db.QueryRow(ctx,
        "SELECT value FROM settings WHERE key = $1",
        "test-key").Scan(&value)
    assert.NoError(t, err)
    assert.Equal(t, "test", value["data"])
}
```

## Official Extensions

### Webhooks Extension

Provides webhook management and delivery:

```go
import "github.com/suppers-ai/solobase/extensions/official/webhooks"

app.RegisterExtension(webhooks.New())
```

Features:
- Webhook endpoint management
- Automatic retry with exponential backoff
- HMAC signature verification
- Delivery history and logs

### Analytics Extension

Tracks and analyzes application usage:

```go
import "github.com/suppers-ai/solobase/extensions/official/analytics"

app.RegisterExtension(analytics.New())
```

Features:
- Event tracking
- User behavior analytics
- Performance metrics
- Custom dashboards

### Storage Extension

Advanced file storage capabilities:

```go
import "github.com/suppers-ai/solobase/extensions/official/storage"

app.RegisterExtension(storage.New())
```

Features:
- Multi-provider support (S3, GCS, Azure)
- Image optimization
- File versioning
- Access control

## Best Practices

### 1. Error Handling

Always return meaningful errors:

```go
if err != nil {
    return fmt.Errorf("failed to process data: %w", err)
}
```

### 2. Logging

Use structured logging for better debugging:

```go
logger.Info(ctx, "Processing request",
    "extension", "my-extension",
    "action", action,
    "user_id", userID,
    "duration", time.Since(start))
```

### 3. Database Queries

Use parameterized queries to prevent SQL injection:

```go
// Good
db.Query(ctx, "SELECT * FROM users WHERE id = $1", userID)

// Bad - never do this!
db.Query(ctx, fmt.Sprintf("SELECT * FROM users WHERE id = %s", userID))
```

### 4. Resource Cleanup

Always clean up resources:

```go
func (e *MyExtension) Stop(ctx context.Context) error {
    // Close connections
    if e.conn != nil {
        e.conn.Close()
    }

    // Cancel background tasks
    if e.cancel != nil {
        e.cancel()
    }

    return nil
}
```

### 5. Testing

Write comprehensive tests:

```go
// Test normal operation
func TestExtensionNormalOperation(t *testing.T) { ... }

// Test error cases
func TestExtensionErrorHandling(t *testing.T) { ... }

// Test configuration
func TestExtensionConfiguration(t *testing.T) { ... }

// Benchmark performance
func BenchmarkExtension(b *testing.B) { ... }
```

## Deployment

### Development

```bash
# Run with extensions in development
go run main.go
```

### Production

```bash
# Build with extensions
go build -o myapp main.go

# Run in production
./myapp
```

### Docker

```dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY . .
RUN go build -o myapp main.go

FROM alpine:latest
COPY --from=builder /app/myapp /myapp
ENTRYPOINT ["/myapp"]
```

## Troubleshooting

### Extension Not Loading

Check:
1. Extension is registered in main.go
2. All required methods are implemented
3. Check logs for initialization errors

### Migration Failures

```go
// Check migration status
status, err := e.services.DB().GetMigrationStatus(ctx)

// Rollback if needed
err = e.services.DB().RollbackMigration(ctx, "001")
```

### Performance Issues

1. Check metrics and logs
2. Review database queries for N+1 problems
3. Verify resource quotas aren't being exceeded

## Next Steps

- Browse [Official Extensions](/docs/extensions/official/)
- Explore [Community Extensions](https://github.com/suppers-ai/solobase-extensions)
- Read the [Extension API Reference](/docs/api/extensions/)
- Join our [Discord Community](https://discord.gg/jKqMcbrVzm) for help