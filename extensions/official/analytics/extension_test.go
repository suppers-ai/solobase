package analytics

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gorilla/mux"
	"github.com/stretchr/testify/assert"
	"github.com/suppers-ai/solobase/extensions/core"
)

func TestAnalyticsExtension_Metadata(t *testing.T) {
	ext := NewAnalyticsExtension()
	metadata := ext.Metadata()

	assert.Equal(t, "analytics", metadata.Name)
	assert.Equal(t, "1.0.0", metadata.Version)
	assert.Contains(t, metadata.Description, "analytics and tracking")
	assert.Equal(t, "Solobase Official", metadata.Author)
	assert.Equal(t, "MIT", metadata.License)
	assert.Contains(t, metadata.Tags, "analytics")
	assert.Contains(t, metadata.Tags, "tracking")
	assert.Contains(t, metadata.Tags, "dashboard")
}

func TestAnalyticsExtension_DatabaseSchema(t *testing.T) {
	ext := NewAnalyticsExtension()
	schema := ext.DatabaseSchema()
	assert.Equal(t, "ext_analytics", schema)
}

func TestAnalyticsExtension_Migrations(t *testing.T) {
	ext := NewAnalyticsExtension()
	migrations := ext.Migrations()

	assert.Len(t, migrations, 1)
	assert.Equal(t, "001", migrations[0].Version)
	assert.Contains(t, migrations[0].Description, "analytics tables")
	assert.Contains(t, migrations[0].Up, "CREATE SCHEMA IF NOT EXISTS ext_analytics")
	assert.Contains(t, migrations[0].Up, "ext_analytics.page_views")
	assert.Contains(t, migrations[0].Up, "ext_analytics.events")
	assert.Contains(t, migrations[0].Down, "DROP SCHEMA IF EXISTS ext_analytics CASCADE")
}

func TestAnalyticsExtension_RequiredPermissions(t *testing.T) {
	ext := NewAnalyticsExtension()
	permissions := ext.RequiredPermissions()

	assert.Len(t, permissions, 2)

	// Check view permission
	assert.Equal(t, "analytics.view", permissions[0].Name)
	assert.Contains(t, permissions[0].Actions, "read")
	assert.Equal(t, "analytics", permissions[0].Resource)

	// Check admin permission
	assert.Equal(t, "analytics.admin", permissions[1].Name)
	assert.Contains(t, permissions[1].Actions, "read")
	assert.Contains(t, permissions[1].Actions, "write")
	assert.Contains(t, permissions[1].Actions, "delete")
	assert.Equal(t, "analytics", permissions[1].Resource)
}

func TestAnalyticsExtension_ConfigSchema(t *testing.T) {
	ext := NewAnalyticsExtension()
	schemaBytes := ext.ConfigSchema()

	var schema map[string]interface{}
	err := json.Unmarshal(schemaBytes, &schema)
	assert.NoError(t, err)

	assert.Equal(t, "object", schema["type"])
	props := schema["properties"].(map[string]interface{})
	assert.NotNil(t, props["enabled"])
	assert.NotNil(t, props["excludePaths"])
	assert.NotNil(t, props["retentionDays"])
}

func TestAnalyticsExtension_ValidateConfig(t *testing.T) {
	ext := NewAnalyticsExtension()

	tests := []struct {
		name        string
		config      json.RawMessage
		expectError bool
		errorMsg    string
	}{
		{
			name: "valid config",
			config: json.RawMessage(`{
				"enabled": true,
				"retentionDays": 30,
				"excludePaths": ["/api/", "/health"]
			}`),
			expectError: false,
		},
		{
			name: "invalid enabled type",
			config: json.RawMessage(`{
				"enabled": "yes"
			}`),
			expectError: true,
			errorMsg:    "enabled must be a boolean",
		},
		{
			name: "retention days too high",
			config: json.RawMessage(`{
				"retentionDays": 500
			}`),
			expectError: true,
			errorMsg:    "retentionDays must be between 1 and 365",
		},
		{
			name: "retention days too low",
			config: json.RawMessage(`{
				"retentionDays": 0
			}`),
			expectError: true,
			errorMsg:    "retentionDays must be between 1 and 365",
		},
		{
			name:        "invalid JSON",
			config:      json.RawMessage(`not json`),
			expectError: true,
			errorMsg:    "invalid config format",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ext.ValidateConfig(tt.config)
			if tt.expectError {
				assert.Error(t, err)
				if tt.errorMsg != "" {
					assert.Contains(t, err.Error(), tt.errorMsg)
				}
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestAnalyticsExtension_ApplyConfig(t *testing.T) {
	ext := NewAnalyticsExtension()

	// Test disabling
	config := json.RawMessage(`{
		"enabled": false
	}`)

	ext.enabled = true
	err := ext.ApplyConfig(config)
	assert.NoError(t, err)
	assert.False(t, ext.enabled)

	// Test enabling
	config = json.RawMessage(`{
		"enabled": true
	}`)

	err = ext.ApplyConfig(config)
	assert.NoError(t, err)
	assert.True(t, ext.enabled)
}

func TestAnalyticsExtension_Health(t *testing.T) {
	ext := NewAnalyticsExtension()
	ctx := context.Background()

	// Test enabled state
	ext.enabled = true
	health, err := ext.Health(ctx)
	assert.NoError(t, err)
	assert.NotNil(t, health)
	assert.Equal(t, "healthy", health.Status)
	assert.Contains(t, health.Message, "running")
	assert.Len(t, health.Checks, 1)
	assert.Equal(t, "database", health.Checks[0].Name)
	assert.Equal(t, "healthy", health.Checks[0].Status)

	// Test disabled state
	ext.enabled = false
	health, err = ext.Health(ctx)
	assert.NoError(t, err)
	assert.NotNil(t, health)
	assert.Equal(t, "stopped", health.Status)
}

func TestAnalyticsExtension_HandleDashboard(t *testing.T) {
	ext := NewAnalyticsExtension()

	// Create a test router that captures the dashboard handler
	var dashboardHandler http.Handler
	router := &testExtensionRouter{
		handleFunc: func(path string, handler http.HandlerFunc) core.RouteRegistration {
			if path == "/" {
				dashboardHandler = handler
			}
			return core.RouteRegistration{Path: path}
		},
	}

	ext.RegisterRoutes(router)

	// Test the dashboard handler directly
	if dashboardHandler != nil {
		req := httptest.NewRequest("GET", "/", nil)
		w := httptest.NewRecorder()
		dashboardHandler.ServeHTTP(w, req)

		assert.Equal(t, http.StatusOK, w.Code)
		assert.Equal(t, "text/html", w.Header().Get("Content-Type"))
		assert.Contains(t, w.Body.String(), "Analytics Dashboard")
		assert.Contains(t, w.Body.String(), "<div id=\"analytics-content\">")
		assert.Contains(t, w.Body.String(), "Page views and analytics data will appear here")
	} else {
		t.Fatal("Dashboard handler not registered")
	}
}

func TestAnalyticsExtension_RegisterRoutes(t *testing.T) {
	ext := NewAnalyticsExtension()

	mockRouter := &testExtensionRouter{
		routes: make(map[string]http.Handler),
	}

	err := ext.RegisterRoutes(mockRouter)
	assert.NoError(t, err)

	// Check all routes are registered
	routes := mockRouter.GetRoutes()
	assert.NotNil(t, routes["/"])
	assert.NotNil(t, routes["/pageviews"])
	assert.NotNil(t, routes["/track"])
	assert.NotNil(t, routes["/stats"])
}

func TestAnalyticsExtension_RegisterMiddleware(t *testing.T) {
	ext := NewAnalyticsExtension()
	middlewares := ext.RegisterMiddleware()

	assert.Len(t, middlewares, 1)
	assert.Equal(t, "analytics", middlewares[0].Extension)
	assert.Equal(t, "page-tracker", middlewares[0].Name)
	assert.Equal(t, 100, middlewares[0].Priority)
	assert.NotNil(t, middlewares[0].Handler)
}

func TestAnalyticsExtension_RegisterHooks(t *testing.T) {
	ext := NewAnalyticsExtension()
	hooks := ext.RegisterHooks()

	assert.Len(t, hooks, 1)
	assert.Equal(t, "analytics", hooks[0].Extension)
	assert.Equal(t, "post-auth-track", hooks[0].Name)
	assert.Equal(t, core.HookPostAuth, hooks[0].Type)
	assert.Equal(t, 50, hooks[0].Priority)
	assert.NotNil(t, hooks[0].Handler)
}

func TestAnalyticsExtension_RegisterTemplates(t *testing.T) {
	ext := NewAnalyticsExtension()
	templates := ext.RegisterTemplates()
	assert.Empty(t, templates)
}

func TestAnalyticsExtension_RegisterStaticAssets(t *testing.T) {
	ext := NewAnalyticsExtension()
	assets := ext.RegisterStaticAssets()
	assert.Empty(t, assets)
}

func TestAnalyticsExtension_TrackingMiddleware(t *testing.T) {
	ext := NewAnalyticsExtension()
	ext.enabled = true

	// Create test handler
	called := false
	nextHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		called = true
		w.WriteHeader(http.StatusOK)
	})

	middleware := ext.trackingMiddleware(nextHandler)

	req := httptest.NewRequest("GET", "/test", nil)
	w := httptest.NewRecorder()

	middleware.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	assert.True(t, called, "next handler should be called")

	// Test with disabled extension
	ext.enabled = false
	called = false

	req = httptest.NewRequest("GET", "/test2", nil)
	w = httptest.NewRecorder()

	middleware.ServeHTTP(w, req)

	assert.True(t, called, "next handler should be called even when disabled")
}

func TestAnalyticsExtension_Lifecycle(t *testing.T) {
	ext := NewAnalyticsExtension()
	ctx := context.Background()

	// Initially should be enabled
	assert.True(t, ext.enabled)

	// Test Initialize (services would normally be provided)
	err := ext.Initialize(ctx, nil)
	assert.NoError(t, err)

	// Test Start
	err = ext.Start(ctx)
	assert.NoError(t, err)
	assert.True(t, ext.enabled)

	// Test Stop
	err = ext.Stop(ctx)
	assert.NoError(t, err)
	assert.False(t, ext.enabled)
}

// BenchmarkAnalyticsExtension tests performance
func BenchmarkAnalyticsExtension_Metadata(b *testing.B) {
	ext := NewAnalyticsExtension()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ext.Metadata()
	}
}

func BenchmarkAnalyticsExtension_ConfigValidation(b *testing.B) {
	ext := NewAnalyticsExtension()
	config := json.RawMessage(`{
		"enabled": true,
		"retentionDays": 30,
		"excludePaths": ["/api/", "/health"]
	}`)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ext.ValidateConfig(config)
	}
}

func BenchmarkAnalyticsExtension_Health(b *testing.B) {
	ext := NewAnalyticsExtension()
	ctx := context.Background()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = ext.Health(ctx)
	}
}

func BenchmarkAnalyticsExtension_TrackingMiddleware(b *testing.B) {
	ext := NewAnalyticsExtension()
	ext.enabled = true

	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	middleware := ext.trackingMiddleware(handler)
	req := httptest.NewRequest("GET", "/test", nil)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		w := httptest.NewRecorder()
		middleware.ServeHTTP(w, req)
	}
}

// testExtensionRouter is a mock implementation of core.ExtensionRouter for testing
type testExtensionRouter struct {
	handleFunc func(path string, handler http.HandlerFunc) core.RouteRegistration
	routes     map[string]http.Handler
}

func (r *testExtensionRouter) HandleFunc(path string, handler http.HandlerFunc) core.RouteRegistration {
	if r.handleFunc != nil {
		return r.handleFunc(path, handler)
	}
	if r.routes == nil {
		r.routes = make(map[string]http.Handler)
	}
	r.routes[path] = handler
	return core.RouteRegistration{Path: path}
}

func (r *testExtensionRouter) Handle(path string, handler http.Handler) core.RouteRegistration {
	if r.routes == nil {
		r.routes = make(map[string]http.Handler)
	}
	r.routes[path] = handler
	return core.RouteRegistration{Path: path}
}

func (r *testExtensionRouter) PathPrefix(prefix string) core.ExtensionRouter {
	return r
}

func (r *testExtensionRouter) Use(middleware ...mux.MiddlewareFunc) {}

func (r *testExtensionRouter) RequireAuth(handler http.Handler) http.Handler {
	return handler
}

func (r *testExtensionRouter) RequireRole(role string, handler http.Handler) http.Handler {
	return handler
}

func (r *testExtensionRouter) GetRoutes() map[string]http.Handler {
	return r.routes
}
