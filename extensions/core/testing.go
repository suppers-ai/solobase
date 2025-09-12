package core

import (
	"context"
	"database/sql"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gorilla/mux"
	_ "github.com/mattn/go-sqlite3"
	"github.com/suppers-ai/database"
	"github.com/suppers-ai/logger"
)

// ExtensionTestSuite provides testing utilities for extensions
type ExtensionTestSuite struct {
	Registry *ExtensionRegistry
	Services *ExtensionServices
	Router   *mux.Router
	DB       database.Database
	Logger   logger.Logger
	Server   *httptest.Server
}

// NewExtensionTestSuite creates a new test suite
func NewExtensionTestSuite(t *testing.T) *ExtensionTestSuite {
	// Create test logger
	testLogger, _ := logger.New(logger.Config{
		Level:  logger.LevelDebug,
		Output: "console",
		Format: "text",
	})

	// Create in-memory SQLite database
	sqlDB, err := sql.Open("sqlite3", ":memory:")
	if err != nil {
		t.Fatalf("Failed to create test database: %v", err)
	}

	// Create test database wrapper
	testDB := &testDatabase{db: sqlDB}

	// Create mock services
	mockServices := NewMockExtensionServices(testDB, testLogger)

	// Create registry
	registry := NewExtensionRegistry(testLogger, mockServices)

	// Create router
	router := mux.NewRouter()

	// Create test server
	server := httptest.NewServer(router)

	return &ExtensionTestSuite{
		Registry: registry,
		Services: mockServices,
		Router:   router,
		DB:       testDB,
		Logger:   testLogger,
		Server:   server,
	}
}

// LoadExtension loads an extension in the test environment
func (ts *ExtensionTestSuite) LoadExtension(ext Extension) error {
	if err := ts.Registry.Register(ext); err != nil {
		return fmt.Errorf("failed to register extension: %w", err)
	}

	if err := ts.Registry.Enable(ext.Metadata().Name); err != nil {
		return fmt.Errorf("failed to enable extension: %w", err)
	}

	// Register routes with the test router
	ts.Registry.RegisterRoutes(ts.Router)

	return nil
}

// TestRequest makes a test HTTP request
func (ts *ExtensionTestSuite) TestRequest(method, path string, body []byte) *httptest.ResponseRecorder {
	req := httptest.NewRequest(method, path, nil)
	if body != nil {
		req.Body = http.NoBody
		req.ContentLength = int64(len(body))
	}

	recorder := httptest.NewRecorder()
	ts.Router.ServeHTTP(recorder, req)

	return recorder
}

// Cleanup cleans up test resources
func (ts *ExtensionTestSuite) Cleanup() {
	if ts.Server != nil {
		ts.Server.Close()
	}
	if ts.DB != nil {
		ts.DB.Close()
	}
}

// testDatabase implements database.Database for testing
type testDatabase struct {
	db *sql.DB
}

func (d *testDatabase) Connect(ctx context.Context, config database.Config) error {
	return nil
}

func (d *testDatabase) Close() error {
	return d.db.Close()
}

func (d *testDatabase) Ping(ctx context.Context) error {
	return d.db.PingContext(ctx)
}

func (d *testDatabase) BeginTx(ctx context.Context) (database.Transaction, error) {
	tx, err := d.db.BeginTx(ctx, nil)
	if err != nil {
		return nil, err
	}
	return &testTransaction{tx: tx}, nil
}

func (d *testDatabase) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	rows, err := d.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &testRows{rows: rows}, nil
}

func (d *testDatabase) QueryRow(ctx context.Context, query string, args ...interface{}) database.Row {
	return d.db.QueryRowContext(ctx, query, args...)
}

func (d *testDatabase) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	return d.db.ExecContext(ctx, query, args...)
}

func (d *testDatabase) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("not implemented")
}

func (d *testDatabase) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("not implemented")
}

func (d *testDatabase) NamedExec(ctx context.Context, query string, arg interface{}) (database.Result, error) {
	return nil, fmt.Errorf("not implemented")
}

func (d *testDatabase) NamedQuery(ctx context.Context, query string, arg interface{}) (database.Rows, error) {
	return nil, fmt.Errorf("not implemented")
}

func (d *testDatabase) Prepare(ctx context.Context, query string) (database.Statement, error) {
	return nil, fmt.Errorf("not implemented")
}

func (d *testDatabase) GetDB() *sql.DB {
	return d.db
}

// testTransaction implements database.Transaction
type testTransaction struct {
	tx *sql.Tx
}

func (t *testTransaction) Commit() error {
	return t.tx.Commit()
}

func (t *testTransaction) Rollback() error {
	return t.tx.Rollback()
}

func (t *testTransaction) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	rows, err := t.tx.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &testRows{rows: rows}, nil
}

func (t *testTransaction) QueryRow(ctx context.Context, query string, args ...interface{}) database.Row {
	return t.tx.QueryRowContext(ctx, query, args...)
}

func (t *testTransaction) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	return t.tx.ExecContext(ctx, query, args...)
}

func (t *testTransaction) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("not implemented")
}

func (t *testTransaction) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("not implemented")
}

func (t *testTransaction) NamedExec(ctx context.Context, query string, arg interface{}) (database.Result, error) {
	return nil, fmt.Errorf("not implemented")
}

// testRows implements database.Rows
type testRows struct {
	rows *sql.Rows
}

func (r *testRows) Next() bool {
	return r.rows.Next()
}

func (r *testRows) Scan(dest ...interface{}) error {
	return r.rows.Scan(dest...)
}

func (r *testRows) Close() error {
	return r.rows.Close()
}

func (r *testRows) Err() error {
	return r.rows.Err()
}

func (r *testRows) Columns() ([]string, error) {
	return r.rows.Columns()
}

// AssertExtensionHealthy asserts that an extension is healthy
func AssertExtensionHealthy(t *testing.T, registry *ExtensionRegistry, name string) {
	status, err := registry.GetStatus(name)
	if err != nil {
		t.Errorf("Failed to get extension status: %v", err)
		return
	}

	if status.Health.Status != "healthy" {
		t.Errorf("Extension %s is not healthy: %s", name, status.Health.Status)
	}
}

// AssertRouteRegistered asserts that a route is registered
func AssertRouteRegistered(t *testing.T, router *mux.Router, path string) {
	req := httptest.NewRequest("GET", path, nil)
	recorder := httptest.NewRecorder()
	router.ServeHTTP(recorder, req)

	if recorder.Code == http.StatusNotFound {
		t.Errorf("Route %s not registered", path)
	}
}

// BenchmarkExtension benchmarks an extension endpoint
func BenchmarkExtension(b *testing.B, suite *ExtensionTestSuite, method, path string) {
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = suite.TestRequest(method, path, nil)
	}
}
