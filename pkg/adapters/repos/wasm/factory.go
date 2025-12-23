//go:build wasm

package wasm

import (
	"context"
	"database/sql"
	"errors"

	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// ErrNotImplemented is returned when a WASM repository method needs host implementation
var ErrNotImplemented = errors.New("not implemented: requires host function")

// Factory implements RepositoryFactory for WASM builds
// All operations delegate to host-provided functions
type Factory struct{}

// NewFactory creates a new WASM repository factory
func NewFactory() *Factory {
	return &Factory{}
}

func (f *Factory) Users() repos.UserRepository {
	return &userRepository{}
}

func (f *Factory) Tokens() repos.TokenRepository {
	return &tokenRepository{}
}

func (f *Factory) APIKeys() repos.APIKeyRepository {
	return &apiKeyRepository{}
}

func (f *Factory) Settings() repos.SettingsRepository {
	return &settingsRepository{}
}

func (f *Factory) Storage() repos.StorageRepository {
	return &storageRepository{}
}

func (f *Factory) Logs() repos.LogsRepository {
	return &logsRepository{}
}

func (f *Factory) IAM() repos.IAMRepository {
	return &iamRepository{}
}

func (f *Factory) CustomTables() repos.CustomTablesRepository {
	return &customTablesRepository{}
}

func (f *Factory) DDL() repos.DDLExecutor {
	return &ddlExecutor{}
}

func (f *Factory) Extension(name string) repos.ExtensionRepository {
	return &extensionRepository{name: name}
}

func (f *Factory) BeginTx(ctx context.Context) (repos.UnitOfWork, error) {
	return &unitOfWork{}, nil
}

func (f *Factory) DB() *sql.DB {
	return nil
}

func (f *Factory) Close() error {
	return nil
}

// Ensure Factory implements RepositoryFactory
var _ repos.RepositoryFactory = (*Factory)(nil)
