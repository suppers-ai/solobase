package extensions

import (
	"context"
	"testing"

	"github.com/suppers-ai/logger"
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
