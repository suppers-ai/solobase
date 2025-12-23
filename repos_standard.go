//go:build !wasm

package solobase

import (
	"database/sql"

	"github.com/suppers-ai/solobase/pkg/adapters/repos"
	"github.com/suppers-ai/solobase/pkg/adapters/repos/sqlite"
)

// NewRepoFactory creates a repository factory for the current platform.
// For standard builds, this creates a SQLite-backed factory.
func NewRepoFactory(sqlDB *sql.DB) repos.RepositoryFactory {
	if sqlDB == nil {
		return nil
	}
	return sqlite.NewFactory(sqlDB)
}
