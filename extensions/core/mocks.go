package core

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/auth"
	"github.com/suppers-ai/database"
	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/core/services"
)

// MockExtensionServices provides mock services for testing
func NewMockExtensionServices(db database.Database, logger logger.Logger) *ExtensionServices {
	return &ExtensionServices{
		db:          db,
		auth:        &auth.Service{},
		logger:      logger,
		storage:     &services.EnhancedStorageService{},
		config:      &config.Config{},
		stats:       &services.StatsService{},
	}
}

// MockExtension is a mock extension for testing
type MockExtension struct {
	name        string
	version     string
	initialized bool
	started     bool
	stopped     bool
	routes      []string
	hooks       []HookRegistration
	middleware  []MiddlewareRegistration
	mu          sync.Mutex
}

// NewMockExtension creates a new mock extension
func NewMockExtension(name, version string) *MockExtension {
	return &MockExtension{
		name:    name,
		version: version,
		routes:  []string{},
	}
}

func (e *MockExtension) Metadata() ExtensionMetadata {
	return ExtensionMetadata{
		Name:        e.name,
		Version:     e.version,
		Description: "Mock extension for testing",
		Author:      "Test",
		License:     "MIT",
	}
}

func (e *MockExtension) Initialize(ctx context.Context, services *ExtensionServices) error {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.initialized = true
	return nil
}

func (e *MockExtension) Start(ctx context.Context) error {
	e.mu.Lock()
	defer e.mu.Unlock()
	if !e.initialized {
		return fmt.Errorf("extension not initialized")
	}
	e.started = true
	return nil
}

func (e *MockExtension) Stop(ctx context.Context) error {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.stopped = true
	return nil
}

func (e *MockExtension) Health(ctx context.Context) (*HealthStatus, error) {
	e.mu.Lock()
	defer e.mu.Unlock()

	status := "healthy"
	if !e.started {
		status = "stopped"
	}

	return &HealthStatus{
		Status:  status,
		Message: "Mock extension health",
	}, nil
}

func (e *MockExtension) RegisterRoutes(router ExtensionRouter) error {
	e.routes = append(e.routes, "/test")
	router.HandleFunc("/test", e.testHandler)
	return nil
}

func (e *MockExtension) RegisterMiddleware() []MiddlewareRegistration {
	return e.middleware
}

func (e *MockExtension) RegisterHooks() []HookRegistration {
	return e.hooks
}

func (e *MockExtension) RegisterTemplates() []TemplateRegistration {
	return []TemplateRegistration{}
}

func (e *MockExtension) RegisterStaticAssets() []StaticAssetRegistration {
	return []StaticAssetRegistration{}
}

func (e *MockExtension) ConfigSchema() json.RawMessage {
	return json.RawMessage(`{"type": "object"}`)
}

func (e *MockExtension) ValidateConfig(config json.RawMessage) error {
	return nil
}

func (e *MockExtension) ApplyConfig(config json.RawMessage) error {
	return nil
}

func (e *MockExtension) DatabaseSchema() string {
	return "ext_mock"
}

func (e *MockExtension) Migrations() []Migration {
	return []Migration{}
}

func (e *MockExtension) RequiredPermissions() []Permission {
	return []Permission{}
}

func (e *MockExtension) testHandler(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Mock extension test endpoint"))
}

// IsInitialized returns whether the extension was initialized
func (e *MockExtension) IsInitialized() bool {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.initialized
}

// IsStarted returns whether the extension was started
func (e *MockExtension) IsStarted() bool {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.started
}

// IsStopped returns whether the extension was stopped
func (e *MockExtension) IsStopped() bool {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.stopped
}

// MockRouter implements ExtensionRouter for testing
type MockRouter struct {
	routes        map[string]http.Handler
	extensionName string
	middlewares   []mux.MiddlewareFunc
}

// NewMockRouter creates a new mock router for testing
func NewMockRouter(extensionName string) *MockRouter {
	return &MockRouter{
		routes:        make(map[string]http.Handler),
		extensionName: extensionName,
		middlewares:   []mux.MiddlewareFunc{},
	}
}

func (r *MockRouter) HandleFunc(pattern string, handler http.HandlerFunc) RouteRegistration {
	r.routes[pattern] = handler
	return RouteRegistration{
		Extension: r.extensionName,
		Path:      pattern,
		Handler:   handler,
		Methods:   []string{"GET", "POST"},
	}
}

func (r *MockRouter) Handle(pattern string, handler http.Handler) RouteRegistration {
	r.routes[pattern] = handler
	return RouteRegistration{
		Extension: r.extensionName,
		Path:      pattern,
		Handler:   handler,
		Methods:   []string{"GET", "POST"},
	}
}

func (r *MockRouter) PathPrefix(prefix string) ExtensionRouter {
	// Return self for chaining in tests
	return r
}

func (r *MockRouter) Use(middleware ...mux.MiddlewareFunc) {
	r.middlewares = append(r.middlewares, middleware...)
}

func (r *MockRouter) RequireAuth(handler http.Handler) http.Handler {
	// Wrap handler with mock auth check
	return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		// In tests, just pass through
		handler.ServeHTTP(w, req)
	})
}

func (r *MockRouter) RequireRole(role string, handler http.Handler) http.Handler {
	// Wrap handler with mock role check
	return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		// In tests, just pass through
		handler.ServeHTTP(w, req)
	})
}

// GetRoutes returns the registered routes for testing
func (r *MockRouter) GetRoutes() map[string]http.Handler {
	return r.routes
}

// MockDatabase is a simple in-memory mock database
type MockDatabase struct {
	data    map[string]interface{}
	mu      sync.RWMutex
	queries []string
}

// NewMockDatabase creates a new mock database
func NewMockDatabase() *MockDatabase {
	return &MockDatabase{
		data:    make(map[string]interface{}),
		queries: []string{},
	}
}

func (db *MockDatabase) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	db.mu.Lock()
	defer db.mu.Unlock()
	db.queries = append(db.queries, query)
	return &MockRows{}, nil
}

func (db *MockDatabase) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	db.mu.Lock()
	defer db.mu.Unlock()
	db.queries = append(db.queries, query)
	return &MockResult{}, nil
}

func (db *MockDatabase) Transaction(ctx context.Context, fn func(ExtensionTx) error) error {
	tx := &MockTransaction{db: db}
	return fn(tx)
}

// GetQueries returns all executed queries
func (db *MockDatabase) GetQueries() []string {
	db.mu.RLock()
	defer db.mu.RUnlock()
	result := make([]string, len(db.queries))
	copy(result, db.queries)
	return result
}

// ClearQueries clears the query history
func (db *MockDatabase) ClearQueries() {
	db.mu.Lock()
	defer db.mu.Unlock()
	db.queries = []string{}
}

// MockTransaction implements ExtensionTx
type MockTransaction struct {
	db *MockDatabase
}

func (t *MockTransaction) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	return t.db.Query(ctx, query, args...)
}

func (t *MockTransaction) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	return t.db.Exec(ctx, query, args...)
}

func (t *MockTransaction) Commit() error {
	return nil
}

func (t *MockTransaction) Rollback() error {
	return nil
}

// MockRows implements database.Rows
type MockRows struct {
	data [][]interface{}
	idx  int
}

func (r *MockRows) Next() bool {
	r.idx++
	return r.idx < len(r.data)
}

func (r *MockRows) Scan(dest ...interface{}) error {
	if r.idx >= len(r.data) {
		return fmt.Errorf("no more rows")
	}
	// Simple mock implementation
	return nil
}

func (r *MockRows) Close() error {
	return nil
}
func (r *MockRows) Err() error {
	return nil
}
func (r *MockRows) Columns() ([]string, error) {
	return []string{}, nil
}

// MockResult implements database.Result
type MockResult struct{}

func (r *MockResult) LastInsertId() (int64, error) {
	return 1, nil
}

func (r *MockResult) RowsAffected() (int64, error) {
	return 1, nil
}
