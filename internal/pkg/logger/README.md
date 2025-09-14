# Logger Package

A comprehensive, extensible logging library for Go applications with support for multiple outputs, structured logging, and database persistence.

## Features

- **Multiple Output Types**: Console, file, database, or multi-output logging
- **Structured Logging**: Support for structured fields and context
- **Database Persistence**: Store logs and request logs in PostgreSQL
- **HTTP Middleware**: Built-in middleware for request/response logging
- **Async Logging**: Buffered async writes for database logger
- **Log Rotation**: Automatic file rotation based on size/age
- **Request Tracking**: Detailed HTTP request/response logging
- **Context Support**: Automatic extraction of trace ID and user ID
- **Query Support**: Query historical logs from database

## Installation

```bash
go get github.com/suppers-ai/logger
```

## Quick Start

### Console Logger

```go
package main

import (
    "context"
    "github.com/suppers-ai/logger"
)

func main() {
    // Create console logger
    log, err := logger.New(logger.Config{
        Level:  logger.LevelInfo,
        Output: "console",
        Format: "text", // or "json"
    })
    if err != nil {
        panic(err)
    }
    defer log.Close()
    
    // Basic logging
    log.Info(context.Background(), "Application started")
    log.Error(context.Background(), "Something went wrong", 
        logger.Err(err),
        logger.String("user", "john"))
}
```

### Database Logger

```go
import (
    "github.com/suppers-ai/logger"
    "github.com/suppers-ai/database"
)

// Create database connection
db, _ := database.New("postgres")
db.Connect(ctx, dbConfig)

// Create database logger
log, err := logger.NewWithDatabase(logger.Config{
    Level:         logger.LevelInfo,
    Output:        "database",
    BufferSize:    1000,
    FlushInterval: 5 * time.Second,
    AsyncMode:     true,
}, db)
if err != nil {
    panic(err)
}
defer log.Close()

// Logs are persisted to database
log.Info(ctx, "User logged in", 
    logger.UserID("user-123"),
    logger.TraceID("trace-456"))
```

### File Logger with Rotation

```go
log, err := logger.New(logger.Config{
    Level:          logger.LevelInfo,
    Output:         "file",
    FilePath:       "/var/log/myapp.log",
    EnableRotation: true,
    MaxSize:        100,  // MB
    MaxAge:         30,   // days
    MaxBackups:     10,   // number of backup files
})
```

### Multi-Output Logger

```go
log, err := logger.NewWithDatabase(logger.Config{
    Level:    logger.LevelInfo,
    Output:   "multi",
    FilePath: "/var/log/myapp.log",
}, db)

// Logs to console, file, and database simultaneously
```

## HTTP Middleware

### Standard net/http

```go
import (
    "net/http"
    "github.com/suppers-ai/logger"
)

func main() {
    log, _ := logger.New(logger.Config{
        Level:  logger.LevelInfo,
        Output: "console",
    })
    
    // Create middleware
    middleware := logger.HTTPMiddleware(log, &logger.MiddlewareConfig{
        LogHeaders:      true,
        LogRequestBody:  true,
        LogResponseBody: true,
        MaxBodySize:     4096,
        SkipPaths:       []string{"/health"},
    })
    
    // Apply to handler
    handler := middleware(yourHandler)
    http.ListenAndServe(":8080", handler)
}
```

## Structured Logging

```go
// Using structured fields
log.Info(ctx, "Order processed",
    logger.String("order_id", "12345"),
    logger.Int("items", 3),
    logger.Float64("total", 99.99),
    logger.Bool("express", true),
    logger.Time("processed_at", time.Now()),
    logger.Duration("processing_time", 150*time.Millisecond))

// With error
if err != nil {
    log.Error(ctx, "Failed to process order",
        logger.Err(err),
        logger.String("order_id", "12345"))
}

// Create logger with persistent fields
userLog := log.With(
    logger.UserID("user-123"),
    logger.String("session", "sess-456"))

userLog.Info(ctx, "Action performed") // Includes user_id and session
```

## Context Integration

```go
// Add values to context
ctx := context.WithValue(context.Background(), "trace_id", "trace-123")
ctx = context.WithValue(ctx, "user_id", "user-456")

// Logger automatically extracts context values
log.Info(ctx, "Operation completed")

// Or use WithContext
ctxLog := log.WithContext(ctx)
ctxLog.Info(context.Background(), "Another operation")
```

## Querying Logs

```go
// Query general logs
logs, err := log.GetLogs(ctx, logger.LogFilter{
    Level:     &logger.LevelError,
    UserID:    &userID,
    StartTime: &startTime,
    EndTime:   &endTime,
    Limit:     100,
    OrderDesc: true,
})

// Query request logs
requests, err := log.GetRequestLogs(ctx, logger.RequestLogFilter{
    Method:      &method,
    PathPrefix:  &pathPrefix,
    StatusCode:  &status,
    MinExecTime: &minTime,
    HasError:    &hasError,
    Limit:       50,
})
```

## Configuration Options

```go
type Config struct {
    Level          Level         // Log level (DEBUG, INFO, WARN, ERROR, FATAL)
    Output         string        // Output type (console, file, database, multi)
    Format         string        // Format (json, text)
    BufferSize     int          // Buffer size for async logging
    FlushInterval  time.Duration // Flush interval for buffered writes
    MaxBatchSize   int          // Max batch size for database inserts
    AsyncMode      bool         // Enable async logging
    IncludeStack   bool         // Include stack traces for errors
    IncludeCaller  bool         // Include caller information
    EnableRotation bool         // Enable file rotation
    MaxSize        int64        // Max file size in MB
    MaxAge         int          // Max file age in days
    MaxBackups     int          // Number of backup files to keep
    FilePath       string       // Path to log file
    Extra          map[string]interface{} // Extra configuration
}
```

## Database Schema

The logger creates tables in the `logger` schema:

```sql
-- General logs
CREATE TABLE logger.logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    fields JSONB,
    user_id TEXT,
    trace_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

-- HTTP request logs
CREATE TABLE logger.request_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    level TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query TEXT,
    status_code INT NOT NULL,
    exec_time_ms BIGINT NOT NULL,
    user_ip TEXT NOT NULL,
    user_agent TEXT,
    user_id TEXT,
    trace_id TEXT,
    error TEXT,
    request_body TEXT,
    response_body TEXT,
    headers TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
```

## Performance Considerations

- Use async mode for database logger in high-throughput applications
- Configure appropriate buffer sizes and flush intervals
- Use log levels to control verbosity
- Consider using file logger with rotation for long-running applications
- Database logger automatically creates indexes for common query patterns

## Best Practices

1. **Use structured logging**: Prefer fields over string concatenation
2. **Set appropriate log levels**: Use DEBUG for development, INFO for production
3. **Include context**: Always pass context for trace ID and user ID extraction
4. **Handle errors**: Check errors when creating loggers
5. **Clean shutdown**: Always call Close() to flush pending logs
6. **Secure sensitive data**: Don't log passwords, tokens, or PII
7. **Use middleware**: Leverage HTTP middleware for consistent request logging

## License

MIT