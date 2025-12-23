//go:build !wasm && !tinygo

package solobase

import (
	"database/sql"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/pkg/adapters"
	"github.com/suppers-ai/solobase/pkg/adapters/auth/jwt"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// StandardPlatform provides platform-specific implementations for standard Go builds
type StandardPlatform struct{}

// DefaultPlatform returns the default platform for the current build
func DefaultPlatform() Platform {
	return &StandardPlatform{}
}

// EnsureDir creates a directory if it doesn't exist
func (p *StandardPlatform) EnsureDir(path string) error {
	return os.MkdirAll(path, 0755)
}

// SetupShutdownHandler sets up graceful shutdown handling
func (p *StandardPlatform) SetupShutdownHandler(shutdownFunc func()) {
	go func() {
		quit := make(chan os.Signal, 1)
		signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
		<-quit
		logger.StdLogPrintf("Shutting down server...")
		shutdownFunc()
	}()
}

// InitializeAdapters registers platform-specific adapters
func (p *StandardPlatform) InitializeAdapters(config *AdapterConfig) {
	// Set up JWT signer
	if config.JWTSecret != "" {
		signer := jwt.New(config.JWTSecret, 24) // 24 hours default expiration
		adapters.SetJWTSigner(signer)
	}
}

// InitIAM creates the IAM service
func (p *StandardPlatform) InitIAM(sqlDB *sql.DB, repo repos.IAMRepository) (*iam.Service, error) {
	return iam.NewService(sqlDB, repo)
}

// StartLogCleanupScheduler starts periodic log cleanup
func (p *StandardPlatform) StartLogCleanupScheduler(cleanupFunc func()) {
	go func() {
		ticker := time.NewTicker(24 * time.Hour)
		defer ticker.Stop()

		for range ticker.C {
			cleanupFunc()
		}
	}()
}
