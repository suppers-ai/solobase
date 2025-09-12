package core

import (
	"context"
	"encoding/json"
	"net/http"
	"time"
)

// Extension is the primary interface that all extensions must implement
type Extension interface {
	// Metadata returns information about the extension
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

// ExtensionMetadata contains information about an extension
type ExtensionMetadata struct {
	Name         string   `json:"name"`
	Version      string   `json:"version"`
	Description  string   `json:"description"`
	Author       string   `json:"author"`
	License      string   `json:"license"`
	Homepage     string   `json:"homepage,omitempty"`
	Dependencies []string `json:"dependencies,omitempty"`
	Tags         []string `json:"tags,omitempty"`
	MinVersion   string   `json:"min_solobase_version,omitempty"`
	MaxVersion   string   `json:"max_solobase_version,omitempty"`
}

// HealthStatus represents the health of an extension
type HealthStatus struct {
	Status      string        `json:"status"` // "healthy", "degraded", "unhealthy"
	Message     string        `json:"message,omitempty"`
	Checks      []HealthCheck `json:"checks,omitempty"`
	LastChecked time.Time     `json:"last_checked"`
}

// HealthCheck represents a single health check
type HealthCheck struct {
	Name    string `json:"name"`
	Status  string `json:"status"`
	Message string `json:"message,omitempty"`
}

// ExtensionStatus represents the runtime status of an extension
type ExtensionStatus struct {
	Name      string             `json:"name"`
	Version   string             `json:"version"`
	State     string             `json:"state"` // "enabled", "disabled", "error"
	Enabled   bool               `json:"enabled"`
	Loaded    bool               `json:"loaded"`
	Health    *HealthStatus      `json:"health,omitempty"`
	Error     string             `json:"error,omitempty"`
	EnabledAt *time.Time         `json:"enabled_at,omitempty"`
	LoadedAt  time.Time          `json:"loaded_at,omitempty"`
	Resources ExtensionResources `json:"resources"`
	Endpoints []EndpointInfo     `json:"endpoints"`
	Metrics   ExtensionMetrics   `json:"metrics"`
}

// ExtensionResources tracks resources registered by an extension
type ExtensionResources struct {
	Routes     int `json:"routes"`
	Middleware int `json:"middleware"`
	Hooks      int `json:"hooks"`
	Templates  int `json:"templates"`
	Assets     int `json:"static_assets"`
	Migrations int `json:"migrations"`
}

// EndpointInfo describes an API endpoint registered by an extension
type EndpointInfo struct {
	Path        string   `json:"path"`
	Methods     []string `json:"methods"`
	Description string   `json:"description,omitempty"`
	Protected   bool     `json:"protected"`
	Roles       []string `json:"roles,omitempty"`
}

// ExtensionMetrics tracks performance metrics for an extension
type ExtensionMetrics struct {
	RequestCount        int64         `json:"request_count"`
	ErrorCount          int64         `json:"error_count"`
	AverageLatency      time.Duration `json:"average_latency"`
	P95Latency          time.Duration `json:"p95_latency"`
	P99Latency          time.Duration `json:"p99_latency"`
	MemoryUsage         int64         `json:"memory_bytes"`
	MemoryUsageMB       int64         `json:"memory_mb"`
	GoroutineCount      int           `json:"goroutine_count"`
	DatabaseQueries     int64         `json:"database_queries"`
	DatabaseConnections int           `json:"database_connections"`
	CacheHitRate        float64       `json:"cache_hit_rate"`
	TotalRequestTime    time.Duration `json:"total_request_time"`
	LastActive          time.Time     `json:"last_active"`
	HooksExecuted       int64         `json:"hooks_executed"`
	HookErrors          int64         `json:"hook_errors"`
	Healthy             bool          `json:"healthy"`
	LastHealthCheck     time.Time     `json:"last_health_check"`
	LastError           string        `json:"last_error"`
	LastErrorTime       time.Time     `json:"last_error_time"`
	StartTime           time.Time     `json:"start_time"`
}

// Migration represents a database migration
type Migration struct {
	Version     string `json:"version"`
	Description string `json:"description"`
	Up          string `json:"up"`
	Down        string `json:"down"`
	Extension   string `json:"extension"`
}

// Permission represents a permission required by an extension
type Permission struct {
	Name        string   `json:"name"`
	Description string   `json:"description"`
	Resource    string   `json:"resource"`
	Actions     []string `json:"actions"`
}

// MiddlewareRegistration represents a middleware registered by an extension
type MiddlewareRegistration struct {
	Extension string
	Name      string
	Priority  int
	Handler   func(http.Handler) http.Handler
	Paths     []string // Optional: specific paths to apply to
}

// HookRegistration represents a hook registered by an extension
type HookRegistration struct {
	Extension string
	Name      string
	Type      HookType
	Priority  int
	Handler   HookFunc
	Paths     []string // Optional: specific paths to apply to
}

// HookType defines the type of hook
type HookType string

const (
	HookPreRequest     HookType = "pre_request"
	HookPostRequest    HookType = "post_request"
	HookPreResponse    HookType = "pre_response"
	HookPostResponse   HookType = "post_response"
	HookError          HookType = "error"
	HookAuthentication HookType = "authentication"
	HookAuthorization  HookType = "authorization"
	HookPreAuth        HookType = "pre_auth"
	HookPostAuth       HookType = "post_auth"
	HookPreDatabase    HookType = "pre_database"
	HookPostDatabase   HookType = "post_database"

	// Storage-specific hooks
	HookBeforeUpload   HookType = "before_upload"
	HookAfterUpload    HookType = "after_upload"
	HookBeforeDownload HookType = "before_download"
	HookAfterDownload  HookType = "after_download"

	// User lifecycle hooks
	HookPostLogin  HookType = "post_login"
	HookPostSignup HookType = "post_signup"
)

// HookContext provides context for hook execution
type HookContext struct {
	Request   *http.Request
	Response  http.ResponseWriter
	Data      map[string]interface{}
	Services  *ExtensionServices
	Extension string
}

// HookFunc is the function signature for hooks
type HookFunc func(ctx context.Context, hookCtx *HookContext) error

// TemplateRegistration represents a template registered by an extension
type TemplateRegistration struct {
	Extension string
	Name      string
	Path      string
	Content   []byte
}

// StaticAssetRegistration represents a static asset registered by an extension
type StaticAssetRegistration struct {
	Extension   string
	Path        string
	ContentType string
	Content     []byte
}

// RouteRegistration represents a route registered by an extension
type RouteRegistration struct {
	Extension string
	Path      string
	Methods   []string
	Handler   http.Handler
	Protected bool
	Roles     []string
}

// ExtensionError represents an error from an extension
type ExtensionError struct {
	Extension string                 `json:"extension"`
	Type      string                 `json:"type"`
	Message   string                 `json:"message"`
	Context   map[string]interface{} `json:"context,omitempty"`
	Timestamp time.Time              `json:"timestamp"`
}

func (e *ExtensionError) Error() string {
	return e.Message
}

// ExtensionErrorHandler handles extension errors
type ExtensionErrorHandler func(err *ExtensionError)

// ExtensionPanicHandler handles extension panics
type ExtensionPanicHandler func(extension string, recovered interface{})
