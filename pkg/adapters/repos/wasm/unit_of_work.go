//go:build wasm

package wasm

import (
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// UnitOfWork provides transactional access to all repositories for WASM
type unitOfWork struct{}

func (u *unitOfWork) Users() repos.UserRepository {
	return &userRepository{}
}

func (u *unitOfWork) Tokens() repos.TokenRepository {
	return &tokenRepository{}
}

func (u *unitOfWork) APIKeys() repos.APIKeyRepository {
	return &apiKeyRepository{}
}

func (u *unitOfWork) Settings() repos.SettingsRepository {
	return &settingsRepository{}
}

func (u *unitOfWork) Storage() repos.StorageRepository {
	return &storageRepository{}
}

func (u *unitOfWork) Logs() repos.LogsRepository {
	return &logsRepository{}
}

func (u *unitOfWork) IAM() repos.IAMRepository {
	return &iamRepository{}
}

func (u *unitOfWork) CustomTables() repos.CustomTablesRepository {
	return &customTablesRepository{}
}

func (u *unitOfWork) Commit() error {
	return ErrNotImplemented
}

func (u *unitOfWork) Rollback() error {
	return ErrNotImplemented
}

// Ensure UnitOfWork implements repos.UnitOfWork
var _ repos.UnitOfWork = (*unitOfWork)(nil)
