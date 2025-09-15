# Design Document

## Overview

This design document outlines the architecture and implementation approach for rewriting the formulapricing-site from Deno/Fresh to Go. The application will be a standalone Go web server that serves static HTML pages with embedded CSS and JavaScript, following the architectural patterns established in the dufflebagbase project.

The design prioritizes pixel-perfect replication of the existing visual design while leveraging Go's strengths in performance, deployment simplicity, and maintainability.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Go Web Server                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Router    │  │  Handlers   │  │    Templates        │  │
│  │ (Gorilla)   │  │             │  │     (templ)         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Static    │  │    Config   │  │      Logger         │  │
│  │   Assets    │  │             │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Web Framework**: Gorilla Mux (consistent with dufflebagbase)
- **Template Engine**: templ (for type-safe HTML generation)
- **Static Assets**: Standard Go http.FileServer
- **Configuration**: Environment variables with godotenv
- **Logging**: Custom logger (following dufflebagbase patterns)
- **Build**: Standard Go build tools

## Components and Interfaces

### 1. Main Application (`main.go`)

**Purpose**: Application entry point and server initialization

**Key Responsibilities**:
- Load configuration from environment variables
- Initialize logger
- Set up router and routes
- Start HTTP server with graceful shutdown
- Serve static assets

**Interface**:
```go
func main()
func setupRoutes() *mux.Router
func setupStaticAssets(router *mux.Router)
```

### 2. Configuration (`config/config.go`)

**Purpose**: Centralized configuration management

**Key Responsibilities**:
- Load environment variables
- Provide default values
- Validate configuration

**Interface**:
```go
type Config struct {
    Port        string
    Environment string
    LogLevel    string
}

func Load() *Config
```

### 3. Handlers (`handlers/`)

**Purpose**: HTTP request handlers for different routes

**Key Components**:
- `home.go`: Homepage handler
- `404.go`: Not found handler

**Interface**:
```go
func HomeHandler() http.HandlerFunc
func NotFoundHandler() http.HandlerFunc
```

### 4. Templates (`templates/`)

**Purpose**: HTML template definitions using templ

**Key Components**:
- `layout.templ`: Base layout template
- `home.templ`: Homepage template
- `404.templ`: 404 error page template

**Interface**:
```go
templ Layout(title string, content templ.Component) 
templ HomePage()
templ NotFoundPage()
```

### 5. Static Assets (`static/`)

**Purpose**: Static file serving (CSS, JS, images)

**Structure**:
```
static/
├── css/
│   └── styles.css
├── js/
│   └── professor-gopher.js
├── images/
│   ├── professor-gopher.png
│   └── wave-background-tile-512-thinner-seamless.svg
└── favicon.ico
```

## Data Models

### Configuration Model

```go
type Config struct {
    Port        string `env:"PORT" default:"8080"`
    Environment string `env:"ENVIRONMENT" default:"development"`
    LogLevel    string `env:"LOG_LEVEL" default:"info"`
}
```

### Page Data Models

```go
type PageData struct {
    Title       string
    Description string
    Keywords    []string
}

type HomePageData struct {
    PageData
    Features    []Feature
    CodeExample string
}

type Feature struct {
    Icon        string
    Title       string
    Description string
}
```

## Error Handling

### HTTP Error Handling

1. **404 Not Found**: Custom 404 page with same styling as original
2. **500 Internal Server Error**: Generic error page with logging
3. **Static Asset Errors**: Proper HTTP status codes and logging

### Error Logging Strategy

```go
type ErrorHandler struct {
    logger Logger
}

func (e *ErrorHandler) HandleError(w http.ResponseWriter, r *http.Request, err error, statusCode int)
func (e *ErrorHandler) LogError(r *http.Request, err error)
```

### Error Recovery

- Graceful degradation for JavaScript functionality
- Fallback styling if CSS fails to load
- Proper error boundaries in templates

## Testing Strategy

### Unit Testing

1. **Handler Testing**:
   - Test HTTP response codes
   - Test response headers
   - Test response body content

2. **Template Testing**:
   - Test template rendering
   - Test data binding
   - Test conditional logic

3. **Configuration Testing**:
   - Test environment variable loading
   - Test default value assignment
   - Test validation logic

### Integration Testing

1. **End-to-End Testing**:
   - Test complete request/response cycle
   - Test static asset serving
   - Test error page rendering

2. **Visual Regression Testing**:
   - Compare rendered pages with original
   - Test responsive design breakpoints
   - Test interactive elements

### Testing Structure

```
tests/
├── unit/
│   ├── handlers_test.go
│   ├── config_test.go
│   └── templates_test.go
├── integration/
│   └── server_test.go
└── fixtures/
    └── test_data.go
```

## Performance Considerations

### Static Asset Optimization

1. **Caching Headers**: Set appropriate cache headers for static assets
2. **Compression**: Enable gzip compression for text assets
3. **Asset Bundling**: Minimize HTTP requests where possible

### Server Performance

1. **Connection Pooling**: Configure appropriate timeouts and limits
2. **Memory Management**: Efficient template caching
3. **Graceful Shutdown**: Proper cleanup on server shutdown

### Client-Side Performance

1. **JavaScript Optimization**: Minimize and optimize eye-tracking JavaScript
2. **CSS Optimization**: Efficient CSS delivery and parsing
3. **Image Optimization**: Proper image formats and sizes

## Security Considerations

### Input Validation

1. **Search Input**: Sanitize search queries
2. **URL Parameters**: Validate route parameters
3. **Headers**: Validate and sanitize HTTP headers

### Security Headers

```go
func securityHeaders(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        w.Header().Set("X-Content-Type-Options", "nosniff")
        w.Header().Set("X-Frame-Options", "DENY")
        w.Header().Set("X-XSS-Protection", "1; mode=block")
        w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
        next.ServeHTTP(w, r)
    })
}
```

### Content Security Policy

Implement CSP headers to prevent XSS attacks while allowing necessary inline styles and scripts for the eye-tracking functionality.

## Deployment Architecture

### Build Process

1. **Binary Compilation**: Single static binary with embedded assets
2. **Asset Embedding**: Use Go embed for static files
3. **Configuration**: Environment-based configuration

### Deployment Options

1. **Standalone Binary**: Direct deployment on servers
2. **Docker Container**: Containerized deployment
3. **Cloud Deployment**: Compatible with existing cloud infrastructure

### Directory Structure

```
go-sites/formulapricing-site/
├── main.go
├── go.mod
├── go.sum
├── config/
│   └── config.go
├── handlers/
│   ├── home.go
│   └── 404.go
├── templates/
│   ├── layout.templ
│   ├── home.templ
│   └── 404.templ
├── static/
│   ├── css/
│   ├── js/
│   └── images/
├── tests/
└── README.md
```

## Migration Strategy

### Phase 1: Core Structure
- Set up Go project structure
- Implement basic routing and handlers
- Create base templates

### Phase 2: Static Assets
- Port CSS styles exactly
- Implement JavaScript eye-tracking
- Set up static asset serving

### Phase 3: Visual Parity
- Ensure pixel-perfect matching
- Test responsive design
- Validate interactive elements

### Phase 4: Testing & Deployment
- Comprehensive testing
- Performance optimization
- Deployment preparation

## Monitoring and Observability

### Logging Strategy

```go
type Logger interface {
    Info(msg string, fields ...Field)
    Error(msg string, err error, fields ...Field)
    Debug(msg string, fields ...Field)
}
```

### Metrics Collection

1. **Request Metrics**: Response times, status codes
2. **Asset Metrics**: Static file serving performance
3. **Error Metrics**: Error rates and types

### Health Checks

```go
func healthCheck(w http.ResponseWriter, r *http.Request) {
    w.Header().Set("Content-Type", "application/json")
    w.WriteHeader(http.StatusOK)
    json.NewEncoder(w).Encode(map[string]string{
        "status": "healthy",
        "version": "1.0.0",
    })
}
```