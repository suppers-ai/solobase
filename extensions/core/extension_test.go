package core

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/gorilla/mux"
	"github.com/stretchr/testify/assert"
)

func TestExtensionLifecycle(t *testing.T) {
	// Create test suite
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	// Create mock extension
	mockExt := NewMockExtension("test-ext", "1.0.0")

	// Test registration
	err := suite.Registry.Register(mockExt)
	assert.NoError(t, err)

	// Verify extension is registered
	ext, exists := suite.Registry.Get("test-ext")
	assert.True(t, exists)
	assert.NotNil(t, ext)

	// Test enabling
	err = suite.Registry.Enable("test-ext")
	assert.NoError(t, err)
	assert.True(t, mockExt.IsInitialized())
	assert.True(t, mockExt.IsStarted())

	// Test status
	status, err := suite.Registry.GetStatus("test-ext")
	assert.NoError(t, err)
	assert.Equal(t, "enabled", status.State)

	// Test health check
	health, err := mockExt.Health(context.Background())
	assert.NoError(t, err)
	assert.Equal(t, "healthy", health.Status)

	// Test disabling
	err = suite.Registry.Disable("test-ext")
	assert.NoError(t, err)
	assert.True(t, mockExt.IsStopped())

	// Test unregistering
	err = suite.Registry.Unregister("test-ext")
	assert.NoError(t, err)

	_, exists = suite.Registry.Get("test-ext")
	assert.False(t, exists)
}

func TestExtensionRouting(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	mockExt := NewMockExtension("router-test", "1.0.0")

	// Load extension
	err := suite.LoadExtension(mockExt)
	assert.NoError(t, err)

	// Test route registration
	resp := suite.TestRequest("GET", "/ext/router-test/test", nil)
	assert.Equal(t, http.StatusOK, resp.Code)
	assert.Contains(t, resp.Body.String(), "Mock extension test endpoint")
}

func TestExtensionConfiguration(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	mockExt := NewMockExtension("config-test", "1.0.0")
	suite.Registry.Register(mockExt)

	// Test config schema
	schema := mockExt.ConfigSchema()
	assert.NotEmpty(t, schema)

	// Test config validation
	validConfig := json.RawMessage(`{"enabled": true}`)
	err := mockExt.ValidateConfig(validConfig)
	assert.NoError(t, err)

	// Test config application
	err = mockExt.ApplyConfig(validConfig)
	assert.NoError(t, err)
}

func TestExtensionHooks(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	// Create extension with hooks
	mockExt := &MockExtension{
		name:    "hook-test",
		version: "1.0.0",
		hooks: []HookRegistration{
			{
				Extension: "hook-test",
				Name:      "test-hook",
				Type:      HookPreRequest,
				Priority:  10,
				Handler: func(ctx context.Context, hctx *HookContext) error {
					hctx.Data["test"] = "value"
					return nil
				},
			},
		},
	}

	suite.Registry.Register(mockExt)
	suite.Registry.Enable("hook-test")

	// Execute hooks
	ctx := context.Background()
	data := make(map[string]interface{})
	hookCtx := &HookContext{
		Data: data,
	}
	err := suite.Registry.ExecuteHooks(ctx, HookPreRequest, hookCtx)
	assert.NoError(t, err)
	assert.Equal(t, "value", data["test"])
}

func TestExtensionMiddleware(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	// Create extension with middleware
	mockExt := &MockExtension{
		name:    "middleware-test",
		version: "1.0.0",
		middleware: []MiddlewareRegistration{
			{
				Extension: "middleware-test",
				Name:      "test-middleware",
				Priority:  10,
				Paths:     []string{"/test"},
				Handler: func(next http.Handler) http.Handler {
					return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
						w.Header().Set("X-Test-Middleware", "applied")
						next.ServeHTTP(w, r)
					})
				},
			},
		},
	}

	suite.Registry.Register(mockExt)
	suite.Registry.Enable("middleware-test")

	// Test that middleware is applied
	router := mux.NewRouter()
	suite.Registry.RegisterRoutes(router)

	// Create test handler
	router.HandleFunc("/test", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	// Apply middleware
	handler := suite.Registry.ApplyMiddleware(router)

	// Test request
	req := httptest.NewRequest("GET", "/test", nil)
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	assert.Equal(t, "applied", rec.Header().Get("X-Test-Middleware"))
}

func TestExtensionSecurity(t *testing.T) {
	secMgr := NewSecurityManager()

	// Test permission management
	secMgr.GrantPermission("test-ext", "database.read")
	assert.True(t, secMgr.CheckPermission("test-ext", "database.read"))
	assert.False(t, secMgr.CheckPermission("test-ext", "database.write"))

	secMgr.RevokePermission("test-ext", "database.read")
	assert.False(t, secMgr.CheckPermission("test-ext", "database.read"))

	// Test rate limiting
	secMgr.SetRateLimit("test-ext", 5) // 5 requests per second

	// Should allow first 5 requests
	for i := 0; i < 5; i++ {
		assert.True(t, secMgr.CheckRateLimit("test-ext"))
	}

	// 6th request should be rate limited
	assert.False(t, secMgr.CheckRateLimit("test-ext"))

	// Wait for tokens to refill
	apptime.Sleep(apptime.Second)
	assert.True(t, secMgr.CheckRateLimit("test-ext"))

	// Test resource quotas
	quota := &ResourceQuota{
		MaxMemoryMB:   100,
		MaxGoroutines: 10,
		MaxStorageMB:  50,
	}
	secMgr.SetResourceQuota("test-ext", quota)

	// Should allow within quota
	err := secMgr.CheckResourceQuota("test-ext", "memory", 50)
	assert.NoError(t, err)

	// Should deny exceeding quota
	err = secMgr.CheckResourceQuota("test-ext", "memory", 150)
	assert.Error(t, err)
}

func TestExtensionMetrics(t *testing.T) {
	collector := NewMetricsCollector()

	// Record some metrics
	collector.RecordRequest("test-ext", "GET", "/test", apptime.Second, nil)
	collector.RecordRequest("test-ext", "POST", "/test", 2*apptime.Second, nil)
	collector.RecordRequest("test-ext", "GET", "/test", apptime.Second, fmt.Errorf("error"))

	// Get metrics
	metrics, err := collector.GetMetrics("test-ext")
	assert.NoError(t, err)
	assert.Equal(t, int64(3), metrics.RequestCount)
	assert.Equal(t, int64(1), metrics.ErrorCount)
	assert.Equal(t, 4*apptime.Second, metrics.TotalRequestTime)

	// Test hook metrics
	collector.RecordHook("test-ext", "test-hook", HookPreRequest, 100*apptime.Millisecond, nil)

	metrics, err = collector.GetMetrics("test-ext")
	assert.NoError(t, err)
	assert.Equal(t, int64(1), metrics.HooksExecuted)

	// Test health metrics
	collector.RecordHealth("test-ext", true)

	metrics, err = collector.GetMetrics("test-ext")
	assert.NoError(t, err)
	assert.True(t, metrics.Healthy)

	// Test resource usage
	collector.RecordResourceUsage("test-ext", "memory_mb", 50)
	collector.RecordResourceUsage("test-ext", "goroutines", 5)

	metrics, err = collector.GetMetrics("test-ext")
	assert.NoError(t, err)
	assert.Equal(t, int64(50), metrics.MemoryUsageMB)
	assert.Equal(t, 5, metrics.GoroutineCount)
}

// PanicExtension is an extension that panics during Start
type PanicExtension struct {
	MockExtension
}

func (e *PanicExtension) Start(ctx context.Context) error {
	panic("test panic")
}

func TestExtensionPanicRecovery(t *testing.T) {
	// TODO: Implement panic recovery test once panic handling is fully implemented
	t.Skip("Panic recovery test not yet implemented")
	/*
		suite := NewExtensionTestSuite(t)
		defer suite.Cleanup()

		// Create extension that panics
		panicExt := &PanicExtension{
			MockExtension: MockExtension{
				name:    "panic-test",
				version: "1.0.0",
			},
		}

		suite.Registry.Register(panicExt)

		// Enable should recover from panic
		err := suite.Registry.Enable("panic-test")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "panic recovered")

		// Extension should be disabled after panic
		status, _ := suite.Registry.GetStatus("panic-test")
		assert.Equal(t, "disabled", status.State)
	*/
}

func TestExtensionConcurrency(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	// Register multiple extensions concurrently
	done := make(chan bool, 10)

	for i := 0; i < 10; i++ {
		go func(n int) {
			extName := fmt.Sprintf("concurrent-%d", n)
			ext := NewMockExtension(extName, "1.0.0")
			suite.Registry.Register(ext)
			suite.Registry.Enable(extName)
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}

	// Verify all extensions are registered
	extensions := suite.Registry.List()
	assert.Len(t, extensions, 10)

	// Test concurrent access
	for i := 0; i < 10; i++ {
		go func(n int) {
			extName := fmt.Sprintf("concurrent-%d", n)
			suite.Registry.GetStatus(extName)
			suite.Registry.GetMetrics(extName)
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}
}

// UnhealthyExtension is an extension that reports unhealthy status
type UnhealthyExtension struct {
	MockExtension
}

func (e *UnhealthyExtension) Health(ctx context.Context) (*HealthStatus, error) {
	return &HealthStatus{
		Status:  "unhealthy",
		Message: "Test unhealthy",
		Checks: []HealthCheck{
			{Name: "database", Status: "failed"},
		},
		LastChecked: apptime.NowTime(),
	}, nil
}

func TestExtensionHealthChecks(t *testing.T) {
	suite := NewExtensionTestSuite(t)
	defer suite.Cleanup()

	// Create extension with custom health check
	healthyExt := NewMockExtension("healthy", "1.0.0")
	unhealthyExt := &UnhealthyExtension{
		MockExtension: MockExtension{
			name:        "unhealthy",
			version:     "1.0.0",
			initialized: false,
			started:     false,
		},
	}

	suite.Registry.Register(healthyExt)
	suite.Registry.Register(unhealthyExt)
	suite.Registry.Enable("healthy")
	suite.Registry.Enable("unhealthy")

	// Check health
	AssertExtensionHealthy(t, suite.Registry, "healthy")

	// Unhealthy extension should fail assertion
	status, _ := suite.Registry.GetStatus("unhealthy")
	assert.NotNil(t, status)

	health, _ := unhealthyExt.Health(context.Background())
	assert.Equal(t, "unhealthy", health.Status)
}

// Benchmark tests
func BenchmarkExtensionRegistration(b *testing.B) {
	suite := NewExtensionTestSuite(&testing.T{})
	defer suite.Cleanup()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		ext := NewMockExtension(fmt.Sprintf("bench-%d", i), "1.0.0")
		suite.Registry.Register(ext)
	}
}

func BenchmarkExtensionRouting(b *testing.B) {
	suite := NewExtensionTestSuite(&testing.T{})
	defer suite.Cleanup()

	mockExt := NewMockExtension("bench", "1.0.0")
	suite.LoadExtension(mockExt)

	BenchmarkExtension(b, suite, "GET", "/ext/bench/test")
}

func BenchmarkMetricsCollection(b *testing.B) {
	collector := NewMetricsCollector()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		collector.RecordRequest("bench", "GET", "/test", apptime.Millisecond, nil)
	}
}
