package solobase

import (
	"database/sql"

	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// Platform defines platform-specific behaviors for the Solobase app.
// This allows different implementations for standard builds vs WASM builds
// without using build tags.
type Platform interface {
	// EnsureDir creates a directory if it doesn't exist.
	// Returns nil on WASM where filesystem operations are handled by the runtime.
	EnsureDir(path string) error

	// SetupShutdownHandler sets up graceful shutdown handling.
	// On standard builds, this listens for OS signals.
	// On WASM, this uses a channel-based approach.
	SetupShutdownHandler(shutdownFunc func())

	// InitializeAdapters registers platform-specific adapters (JWT signer, token generator, etc.)
	// NOTE: Database adapter is set by NewWithOptions, not here.
	InitializeAdapters(config *AdapterConfig)

	// InitIAM creates the IAM service using the repository pattern.
	// The sqlDB is used for migrations, repo is used for all operations.
	InitIAM(sqlDB *sql.DB, repo repos.IAMRepository) (*iam.Service, error)

	// StartLogCleanupScheduler starts periodic log cleanup.
	// This is a no-op on WASM where background goroutines aren't supported.
	StartLogCleanupScheduler(cleanupFunc func())
}

// AdapterConfig contains configuration for initializing adapters
// NOTE: Database configuration is no longer needed here since the host provides the database.
type AdapterConfig struct {
	JWTSecret string
}
