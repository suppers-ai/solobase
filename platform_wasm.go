//go:build wasm

package solobase

import (
	"database/sql"

	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/pkg/adapters"
	"github.com/suppers-ai/solobase/pkg/adapters/auth/jwt"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// WASMPlatform provides platform-specific implementations for WASM builds
type WASMPlatform struct{}

// DefaultPlatform returns the default platform for WASM builds
func DefaultPlatform() Platform {
	return &WASMPlatform{}
}

// EnsureDir is a no-op in WASM since filesystem is handled by the runtime
func (p *WASMPlatform) EnsureDir(path string) error {
	return nil
}

// SetupShutdownHandler is a no-op in WASM
func (p *WASMPlatform) SetupShutdownHandler(shutdownFunc func()) {
	// WASM doesn't support signals - the host manages lifecycle
}

// InitializeAdapters registers platform-specific adapters
func (p *WASMPlatform) InitializeAdapters(config *AdapterConfig) {
	// Set up JWT signer
	if config.JWTSecret != "" {
		signer := jwt.New(config.JWTSecret, 24)
		adapters.SetJWTSigner(signer)
	}
}

// InitIAM creates the IAM service
// In WASM, sqlDB is typically nil since the database is provided by the host
func (p *WASMPlatform) InitIAM(sqlDB *sql.DB, repo repos.IAMRepository) (*iam.Service, error) {
	return iam.NewService(sqlDB, repo)
}

// StartLogCleanupScheduler is a no-op in WASM
func (p *WASMPlatform) StartLogCleanupScheduler(cleanupFunc func()) {
	// No background tasks in WASM
}
