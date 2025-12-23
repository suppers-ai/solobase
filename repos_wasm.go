//go:build wasm

package solobase

import (
	"database/sql"

	"github.com/suppers-ai/solobase/pkg/adapters/repos"
	"github.com/suppers-ai/solobase/pkg/adapters/repos/wasm"
)

// NewRepoFactory creates a repository factory for the current platform.
// For WASM builds, this creates a WASM-backed factory that delegates to host functions.
func NewRepoFactory(sqlDB *sql.DB) repos.RepositoryFactory {
	// WASM builds don't use sqlDB - host provides the database
	return wasm.NewFactory()
}
