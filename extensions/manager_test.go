package extensions

import (
	"context"
	"testing"

	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

func TestExtensionManager(t *testing.T) {
	// Create in-memory SQLite database for testing
	db, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
	if err != nil {
		t.Fatalf("Failed to create test database: %v", err)
	}

	// Create test logger
	testLogger := &testLogger{}

	// Create extension manager
	manager, err := NewExtensionManager(db, testLogger)
	if err != nil {
		t.Fatalf("Failed to create extension manager: %v", err)
	}

	// Test initialization
	ctx := context.Background()
	if err := manager.Initialize(ctx); err != nil {
		t.Fatalf("Failed to initialize extension manager: %v", err)
	}

	// Test getting registry
	registry := manager.GetRegistry()
	if registry == nil {
		t.Fatal("Registry should not be nil")
	}

	// Test shutdown
	if err := manager.Shutdown(ctx); err != nil {
		t.Fatalf("Failed to shutdown extension manager: %v", err)
	}
}

// testLogger implements a simple logger for testing
type testLogger struct{}

func (l *testLogger) Debug(ctx context.Context, msg string, fields ...logger.Field) {}
func (l *testLogger) Info(ctx context.Context, msg string, fields ...logger.Field)  {}
func (l *testLogger) Warn(ctx context.Context, msg string, fields ...logger.Field)  {}
func (l *testLogger) Error(ctx context.Context, msg string, fields ...logger.Field) {}
func (l *testLogger) Fatal(ctx context.Context, msg string, fields ...logger.Field) {}
func (l *testLogger) With(fields ...logger.Field) logger.Logger { return l }
func (l *testLogger) WithContext(ctx context.Context) logger.Logger { return l }
func (l *testLogger) LogRequest(ctx context.Context, req *logger.RequestLog) error { return nil }
func (l *testLogger) GetLogs(ctx context.Context, filter logger.LogFilter) ([]*logger.Log, error) {
	return nil, nil
}
func (l *testLogger) GetRequestLogs(ctx context.Context, filter logger.RequestLogFilter) ([]*logger.RequestLog, error) {
	return nil, nil
}
func (l *testLogger) Flush() error { return nil }
func (l *testLogger) Close() error { return nil }
