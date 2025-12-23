// Package interfaces defines the core interfaces for Solobase's pluggable architecture.
// These interfaces enable the same codebase to compile for both standard Go and TinyGo/WASM.
package interfaces

import (
	"context"
	"database/sql"
)

// Common database errors
var (
	ErrNoRows = sql.ErrNoRows
)

// Database defines the universal interface for database operations.
// Implementations:
//   - Standard: PostgreSQL (lib/pq), SQLite (go-sqlite)
//   - WASM: Spin PostgreSQL (spin-pg)
type Database interface {
	// Connection management
	Close() error
	Ping(ctx context.Context) error

	// Transaction management
	BeginTx(ctx context.Context) (Transaction, error)

	// Query operations
	Query(ctx context.Context, query string, args ...interface{}) (Rows, error)
	QueryRow(ctx context.Context, query string, args ...interface{}) Row
	Exec(ctx context.Context, query string, args ...interface{}) (Result, error)

	// Structured query operations (sqlx-like)
	Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error
	Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error
	NamedExec(ctx context.Context, query string, arg interface{}) (Result, error)
	NamedQuery(ctx context.Context, query string, arg interface{}) (Rows, error)

	// Prepared statements
	Prepare(ctx context.Context, query string) (Statement, error)

	// GetDB returns the underlying sql.DB for compatibility.
	// Returns nil in WASM mode.
	GetDB() *sql.DB

	// Database type information
	GetType() string
	IsPostgres() bool
	IsSQLite() bool
}

// Transaction represents a database transaction
type Transaction interface {
	Commit() error
	Rollback() error
	Query(ctx context.Context, query string, args ...interface{}) (Rows, error)
	QueryRow(ctx context.Context, query string, args ...interface{}) Row
	Exec(ctx context.Context, query string, args ...interface{}) (Result, error)

	// Structured query operations in transaction
	Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error
	Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error
	NamedExec(ctx context.Context, query string, arg interface{}) (Result, error)
}

// Statement represents a prepared statement
type Statement interface {
	Query(ctx context.Context, args ...interface{}) (Rows, error)
	QueryRow(ctx context.Context, args ...interface{}) Row
	Exec(ctx context.Context, args ...interface{}) (Result, error)
	Close() error
}

// Rows represents the result of a query
type Rows interface {
	Next() bool
	Scan(dest ...interface{}) error
	Close() error
	Err() error
	Columns() ([]string, error)
}

// Row represents a single row result
type Row interface {
	Scan(dest ...interface{}) error
}

// Result represents the result of an exec operation
type Result interface {
	LastInsertId() (int64, error)
	RowsAffected() (int64, error)
}

// DatabaseConfig contains database configuration
type DatabaseConfig struct {
	Type            string // postgres, sqlite
	DSN             string
	Host            string
	Port            int
	Username        string
	Password        string
	Database        string
	SSLMode         string
	MaxOpenConns    int
	MaxIdleConns    int
	ConnMaxLifetime int // in seconds
}
