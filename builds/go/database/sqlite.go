// Package database provides database implementations for standard Go builds.
package database

import (
	"context"
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	_ "github.com/glebarez/go-sqlite" // Pure Go SQLite driver
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// SQLiteDB implements interfaces.Database for SQLite using modernc.org/sqlite
type SQLiteDB struct {
	db *sql.DB
}

// NewSQLite creates a new SQLite database connection.
// The path should be the database file path (e.g., ".data/solobase.db").
// Use ":memory:" for an in-memory database.
func NewSQLite(path string) (*SQLiteDB, error) {
	// Ensure parent directory exists
	if path != ":memory:" && !strings.HasPrefix(path, "file::memory:") {
		dir := filepath.Dir(path)
		if dir != "" && dir != "." {
			if err := os.MkdirAll(dir, 0755); err != nil {
				return nil, fmt.Errorf("failed to create database directory: %w", err)
			}
		}
	}

	// Build DSN
	dsn := path
	if !strings.HasPrefix(dsn, "file:") && dsn != ":memory:" {
		dsn = "file:" + dsn
	}

	// Add WAL mode for better concurrent read performance
	if !strings.Contains(dsn, "?") {
		dsn += "?_pragma=journal_mode(WAL)&_pragma=busy_timeout(5000)"
	}

	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to open SQLite database: %w", err)
	}

	// Set connection pool settings for SQLite
	db.SetMaxOpenConns(1) // SQLite doesn't handle concurrent writes well
	db.SetMaxIdleConns(1)

	// Verify connection
	if err := db.Ping(); err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to ping SQLite database: %w", err)
	}

	return &SQLiteDB{db: db}, nil
}

// NewInMemory creates a new in-memory SQLite database for testing.
func NewInMemory() (*SQLiteDB, error) {
	return NewSQLite(":memory:")
}

// Close closes the database connection.
func (s *SQLiteDB) Close() error {
	return s.db.Close()
}

// Ping verifies the database connection is alive.
func (s *SQLiteDB) Ping(ctx context.Context) error {
	return s.db.PingContext(ctx)
}

// BeginTx starts a new transaction.
func (s *SQLiteDB) BeginTx(ctx context.Context) (interfaces.Transaction, error) {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return nil, err
	}
	return &sqliteTx{tx: tx}, nil
}

// Query executes a query and returns rows.
func (s *SQLiteDB) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &sqliteRows{rows: rows}, nil
}

// QueryRow executes a query and returns a single row.
func (s *SQLiteDB) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	return s.db.QueryRowContext(ctx, query, args...)
}

// Exec executes a query without returning rows.
func (s *SQLiteDB) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	return s.db.ExecContext(ctx, query, args...)
}

// Get executes a query and scans the result into dest (single row).
func (s *SQLiteDB) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	// Simple implementation - just use QueryRow and Scan
	// For more complex struct scanning, a library like sqlx would be needed
	return fmt.Errorf("Get is not implemented - use QueryRow and Scan instead")
}

// Select executes a query and scans all results into dest (multiple rows).
func (s *SQLiteDB) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	// Simple implementation - for struct scanning, a library like sqlx would be needed
	return fmt.Errorf("Select is not implemented - use Query and Scan instead")
}

// NamedExec executes a named query.
func (s *SQLiteDB) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return nil, fmt.Errorf("NamedExec is not implemented")
}

// NamedQuery executes a named query and returns rows.
func (s *SQLiteDB) NamedQuery(ctx context.Context, query string, arg interface{}) (interfaces.Rows, error) {
	return nil, fmt.Errorf("NamedQuery is not implemented")
}

// Prepare creates a prepared statement.
func (s *SQLiteDB) Prepare(ctx context.Context, query string) (interfaces.Statement, error) {
	stmt, err := s.db.PrepareContext(ctx, query)
	if err != nil {
		return nil, err
	}
	return &sqliteStmt{stmt: stmt}, nil
}

// GetDB returns the underlying *sql.DB for compatibility.
func (s *SQLiteDB) GetDB() *sql.DB {
	return s.db
}

// GetType returns the database type.
func (s *SQLiteDB) GetType() string {
	return "sqlite"
}

// IsPostgres returns false for SQLite.
func (s *SQLiteDB) IsPostgres() bool {
	return false
}

// IsSQLite returns true for SQLite.
func (s *SQLiteDB) IsSQLite() bool {
	return true
}

// sqliteTx implements interfaces.Transaction
type sqliteTx struct {
	tx *sql.Tx
}

func (t *sqliteTx) Commit() error {
	return t.tx.Commit()
}

func (t *sqliteTx) Rollback() error {
	return t.tx.Rollback()
}

func (t *sqliteTx) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	rows, err := t.tx.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &sqliteRows{rows: rows}, nil
}

func (t *sqliteTx) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	return t.tx.QueryRowContext(ctx, query, args...)
}

func (t *sqliteTx) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	return t.tx.ExecContext(ctx, query, args...)
}

func (t *sqliteTx) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Get is not implemented in transaction")
}

func (t *sqliteTx) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Select is not implemented in transaction")
}

func (t *sqliteTx) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return nil, fmt.Errorf("NamedExec is not implemented in transaction")
}

// sqliteRows implements interfaces.Rows
type sqliteRows struct {
	rows *sql.Rows
}

func (r *sqliteRows) Next() bool {
	return r.rows.Next()
}

func (r *sqliteRows) Scan(dest ...interface{}) error {
	return r.rows.Scan(dest...)
}

func (r *sqliteRows) Close() error {
	return r.rows.Close()
}

func (r *sqliteRows) Err() error {
	return r.rows.Err()
}

func (r *sqliteRows) Columns() ([]string, error) {
	return r.rows.Columns()
}

// sqliteStmt implements interfaces.Statement
type sqliteStmt struct {
	stmt *sql.Stmt
}

func (s *sqliteStmt) Query(ctx context.Context, args ...interface{}) (interfaces.Rows, error) {
	rows, err := s.stmt.QueryContext(ctx, args...)
	if err != nil {
		return nil, err
	}
	return &sqliteRows{rows: rows}, nil
}

func (s *sqliteStmt) QueryRow(ctx context.Context, args ...interface{}) interfaces.Row {
	return s.stmt.QueryRowContext(ctx, args...)
}

func (s *sqliteStmt) Exec(ctx context.Context, args ...interface{}) (interfaces.Result, error) {
	return s.stmt.ExecContext(ctx, args...)
}

func (s *sqliteStmt) Close() error {
	return s.stmt.Close()
}
