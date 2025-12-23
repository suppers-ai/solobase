//go:build !wasm

package sqlite

import (
	"database/sql"

	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// UnitOfWork provides transactional access to all repositories
type UnitOfWork struct {
	tx      *sql.Tx
	queries *db.Queries

	// Cached repositories
	usersRepo        repos.UserRepository
	tokensRepo       repos.TokenRepository
	apiKeysRepo      repos.APIKeyRepository
	settingsRepo     repos.SettingsRepository
	storageRepo      repos.StorageRepository
	logsRepo         repos.LogsRepository
	iamRepo          repos.IAMRepository
	customTablesRepo repos.CustomTablesRepository
}

// NewUnitOfWork creates a new unit of work from a transaction
func NewUnitOfWork(tx *sql.Tx) *UnitOfWork {
	return &UnitOfWork{
		tx:      tx,
		queries: db.New(tx),
	}
}

func (u *UnitOfWork) Users() repos.UserRepository {
	if u.usersRepo == nil {
		u.usersRepo = NewUserRepository(nil, u.queries)
	}
	return u.usersRepo
}

func (u *UnitOfWork) Tokens() repos.TokenRepository {
	if u.tokensRepo == nil {
		u.tokensRepo = NewTokenRepository(nil, u.queries)
	}
	return u.tokensRepo
}

func (u *UnitOfWork) APIKeys() repos.APIKeyRepository {
	if u.apiKeysRepo == nil {
		u.apiKeysRepo = NewAPIKeyRepository(nil, u.queries)
	}
	return u.apiKeysRepo
}

func (u *UnitOfWork) Settings() repos.SettingsRepository {
	if u.settingsRepo == nil {
		u.settingsRepo = NewSettingsRepository(nil, u.queries)
	}
	return u.settingsRepo
}

func (u *UnitOfWork) Storage() repos.StorageRepository {
	if u.storageRepo == nil {
		u.storageRepo = NewStorageRepository(nil, u.queries)
	}
	return u.storageRepo
}

func (u *UnitOfWork) Logs() repos.LogsRepository {
	if u.logsRepo == nil {
		u.logsRepo = NewLogsRepository(nil, u.queries)
	}
	return u.logsRepo
}

func (u *UnitOfWork) IAM() repos.IAMRepository {
	if u.iamRepo == nil {
		u.iamRepo = NewIAMRepository(nil, u.queries)
	}
	return u.iamRepo
}

func (u *UnitOfWork) CustomTables() repos.CustomTablesRepository {
	if u.customTablesRepo == nil {
		u.customTablesRepo = NewCustomTablesRepository(nil, u.queries)
	}
	return u.customTablesRepo
}

func (u *UnitOfWork) Commit() error {
	return u.tx.Commit()
}

func (u *UnitOfWork) Rollback() error {
	return u.tx.Rollback()
}

// Ensure UnitOfWork implements repos.UnitOfWork
var _ repos.UnitOfWork = (*UnitOfWork)(nil)
