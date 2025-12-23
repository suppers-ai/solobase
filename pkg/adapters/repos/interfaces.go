// Package repos provides repository interfaces for data access.
// These interfaces abstract away database-specific implementation details,
// allowing Solobase core to work with any database backend.
package repos

import (
	"context"
	"errors"
)

// Common errors returned by repositories
var (
	// ErrNotFound is returned when a requested entity does not exist
	ErrNotFound = errors.New("not found")

	// ErrDuplicate is returned when trying to create an entity that already exists
	ErrDuplicate = errors.New("duplicate entry")

	// ErrInvalidInput is returned when input validation fails
	ErrInvalidInput = errors.New("invalid input")

	// ErrConstraintViolation is returned when a database constraint is violated
	ErrConstraintViolation = errors.New("constraint violation")

	// ErrTransactionFailed is returned when a transaction fails
	ErrTransactionFailed = errors.New("transaction failed")
)

// Pagination contains common pagination parameters
type Pagination struct {
	Limit  int
	Offset int
}

// PaginatedResult wraps a result with total count for pagination
type PaginatedResult[T any] struct {
	Items []T
	Total int64
}

// Rows represents database rows returned from a query
// Used by ExtensionRepository for backwards compatibility
type Rows interface {
	Next() bool
	Scan(dest ...interface{}) error
	Close() error
	Err() error
	Columns() ([]string, error)
}

// Result represents the result of an exec operation
type Result interface {
	LastInsertId() (int64, error)
	RowsAffected() (int64, error)
}

// UnitOfWork provides transactional access to all repositories
type UnitOfWork interface {
	// Access repositories within transaction
	Users() UserRepository
	Tokens() TokenRepository
	APIKeys() APIKeyRepository
	Settings() SettingsRepository
	Storage() StorageRepository
	Logs() LogsRepository
	IAM() IAMRepository
	CustomTables() CustomTablesRepository

	// Transaction control
	Commit() error
	Rollback() error
}

// RepositoryFactory creates repositories and units of work
type RepositoryFactory interface {
	// Individual repositories (for read-only or single operations)
	Users() UserRepository
	Tokens() TokenRepository
	APIKeys() APIKeyRepository
	Settings() SettingsRepository
	Storage() StorageRepository
	Logs() LogsRepository
	IAM() IAMRepository
	CustomTables() CustomTablesRepository
	DDL() DDLExecutor

	// Transaction support
	BeginTx(ctx context.Context) (UnitOfWork, error)

	// For extensions
	Extension(name string) ExtensionRepository

	// Lifecycle
	Close() error
}
