package database

import (
	"context"
	"database/sql"
)

// Common errors
var (
	ErrNoRows = sql.ErrNoRows
)

// Database defines the universal interface for database operations
// This is a compatibility layer for packages that still use raw SQL
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

	// Get underlying SQL DB (for migration tools and special cases)
	GetDB() *sql.DB
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
