//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"
	"sync"

	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// Factory implements repos.RepositoryFactory for SQLite
type Factory struct {
	sqlDB   *sql.DB
	queries *db.Queries

	// Cached repositories (created once)
	mu             sync.RWMutex
	usersRepo      repos.UserRepository
	tokensRepo     repos.TokenRepository
	apiKeysRepo    repos.APIKeyRepository
	settingsRepo   repos.SettingsRepository
	storageRepo    repos.StorageRepository
	logsRepo       repos.LogsRepository
	iamRepo        repos.IAMRepository
	customTablesRepo repos.CustomTablesRepository
	ddlExecutor    repos.DDLExecutor

	// Extension repos cache
	extensionRepos map[string]repos.ExtensionRepository
}

// NewFactory creates a new SQLite repository factory
func NewFactory(sqlDB *sql.DB) *Factory {
	return &Factory{
		sqlDB:          sqlDB,
		queries:        db.New(sqlDB),
		extensionRepos: make(map[string]repos.ExtensionRepository),
	}
}

// Users returns the user repository
func (f *Factory) Users() repos.UserRepository {
	f.mu.RLock()
	if f.usersRepo != nil {
		f.mu.RUnlock()
		return f.usersRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.usersRepo == nil {
		f.usersRepo = NewUserRepository(f.sqlDB, f.queries)
	}
	return f.usersRepo
}

// Tokens returns the token repository
func (f *Factory) Tokens() repos.TokenRepository {
	f.mu.RLock()
	if f.tokensRepo != nil {
		f.mu.RUnlock()
		return f.tokensRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.tokensRepo == nil {
		f.tokensRepo = NewTokenRepository(f.sqlDB, f.queries)
	}
	return f.tokensRepo
}

// APIKeys returns the API key repository
func (f *Factory) APIKeys() repos.APIKeyRepository {
	f.mu.RLock()
	if f.apiKeysRepo != nil {
		f.mu.RUnlock()
		return f.apiKeysRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.apiKeysRepo == nil {
		f.apiKeysRepo = NewAPIKeyRepository(f.sqlDB, f.queries)
	}
	return f.apiKeysRepo
}

// Settings returns the settings repository
func (f *Factory) Settings() repos.SettingsRepository {
	f.mu.RLock()
	if f.settingsRepo != nil {
		f.mu.RUnlock()
		return f.settingsRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.settingsRepo == nil {
		f.settingsRepo = NewSettingsRepository(f.sqlDB, f.queries)
	}
	return f.settingsRepo
}

// Storage returns the storage repository
func (f *Factory) Storage() repos.StorageRepository {
	f.mu.RLock()
	if f.storageRepo != nil {
		f.mu.RUnlock()
		return f.storageRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.storageRepo == nil {
		f.storageRepo = NewStorageRepository(f.sqlDB, f.queries)
	}
	return f.storageRepo
}

// Logs returns the logs repository
func (f *Factory) Logs() repos.LogsRepository {
	f.mu.RLock()
	if f.logsRepo != nil {
		f.mu.RUnlock()
		return f.logsRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.logsRepo == nil {
		f.logsRepo = NewLogsRepository(f.sqlDB, f.queries)
	}
	return f.logsRepo
}

// IAM returns the IAM repository
func (f *Factory) IAM() repos.IAMRepository {
	f.mu.RLock()
	if f.iamRepo != nil {
		f.mu.RUnlock()
		return f.iamRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.iamRepo == nil {
		f.iamRepo = NewIAMRepository(f.sqlDB, f.queries)
	}
	return f.iamRepo
}

// CustomTables returns the custom tables repository
func (f *Factory) CustomTables() repos.CustomTablesRepository {
	f.mu.RLock()
	if f.customTablesRepo != nil {
		f.mu.RUnlock()
		return f.customTablesRepo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.customTablesRepo == nil {
		f.customTablesRepo = NewCustomTablesRepository(f.sqlDB, f.queries)
	}
	return f.customTablesRepo
}

// DDL returns the DDL executor
func (f *Factory) DDL() repos.DDLExecutor {
	f.mu.RLock()
	if f.ddlExecutor != nil {
		f.mu.RUnlock()
		return f.ddlExecutor
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if f.ddlExecutor == nil {
		f.ddlExecutor = NewDDLExecutor(f.sqlDB)
	}
	return f.ddlExecutor
}

// BeginTx starts a new transaction and returns a UnitOfWork
func (f *Factory) BeginTx(ctx context.Context) (repos.UnitOfWork, error) {
	tx, err := f.sqlDB.BeginTx(ctx, nil)
	if err != nil {
		return nil, err
	}
	return NewUnitOfWork(tx), nil
}

// Extension returns an extension repository for the given extension name
func (f *Factory) Extension(name string) repos.ExtensionRepository {
	f.mu.RLock()
	if repo, ok := f.extensionRepos[name]; ok {
		f.mu.RUnlock()
		return repo
	}
	f.mu.RUnlock()

	f.mu.Lock()
	defer f.mu.Unlock()
	if repo, ok := f.extensionRepos[name]; ok {
		return repo
	}
	repo := NewExtensionRepository(f.sqlDB, name)
	f.extensionRepos[name] = repo
	return repo
}

// Close closes the database connection
func (f *Factory) Close() error {
	return f.sqlDB.Close()
}

// Ensure Factory implements RepositoryFactory
var _ repos.RepositoryFactory = (*Factory)(nil)
