package database

import (
	"context"
	"database/sql"
	"fmt"

	"gorm.io/gorm"
)

// Ensure DB implements Database interface
var _ Database = (*DB)(nil)

// Adapter methods to implement Database interface

// Ping verifies the database connection
func (db *DB) Ping(ctx context.Context) error {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return err
	}
	return sqlDB.PingContext(ctx)
}

// BeginTx starts a new transaction
func (db *DB) BeginTx(ctx context.Context) (Transaction, error) {
	tx := db.DB.WithContext(ctx).Begin()
	if tx.Error != nil {
		return nil, tx.Error
	}
	return &GormTransaction{tx: tx}, nil
}

// Query executes a query that returns rows
func (db *DB) Query(ctx context.Context, query string, args ...interface{}) (Rows, error) {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return nil, err
	}
	rows, err := sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &SqlRows{rows: rows}, nil
}

// QueryRow executes a query that returns a single row
func (db *DB) QueryRow(ctx context.Context, query string, args ...interface{}) Row {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return &SqlRow{err: err}
	}
	row := sqlDB.QueryRowContext(ctx, query, args...)
	return &SqlRow{row: row}
}

// Exec executes a query that doesn't return rows
func (db *DB) Exec(ctx context.Context, query string, args ...interface{}) (Result, error) {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return nil, err
	}
	result, err := sqlDB.ExecContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &SqlResult{result: result}, nil
}

// Get executes a query and scans the result into dest (single row)
func (db *DB) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	err := db.DB.WithContext(ctx).Raw(query, args...).First(dest).Error
	if err == gorm.ErrRecordNotFound {
		return sql.ErrNoRows
	}
	return err
}

// Select executes a query and scans the results into dest (multiple rows)
func (db *DB) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return db.DB.WithContext(ctx).Raw(query, args...).Find(dest).Error
}

// NamedExec executes a named query with named parameters
func (db *DB) NamedExec(ctx context.Context, query string, arg interface{}) (Result, error) {
	result := db.DB.WithContext(ctx).Exec(query, arg)
	if result.Error != nil {
		return nil, result.Error
	}
	return &GormResult{rowsAffected: result.RowsAffected}, nil
}

// NamedQuery executes a named query with named parameters
func (db *DB) NamedQuery(ctx context.Context, query string, arg interface{}) (Rows, error) {
	// For named queries, we'll use GORM's raw query
	sqlDB, err := db.DB.DB()
	if err != nil {
		return nil, err
	}
	// This is a simplified version - real named query support would require parsing
	rows, err := sqlDB.QueryContext(ctx, query, arg)
	if err != nil {
		return nil, err
	}
	return &SqlRows{rows: rows}, nil
}

// Prepare creates a prepared statement
func (db *DB) Prepare(ctx context.Context, query string) (Statement, error) {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return nil, err
	}
	stmt, err := sqlDB.PrepareContext(ctx, query)
	if err != nil {
		return nil, err
	}
	return &SqlStatement{stmt: stmt}, nil
}

// GetDB returns the underlying sql.DB
func (db *DB) GetDB() *sql.DB {
	sqlDB, _ := db.DB.DB()
	return sqlDB
}

// GormTransaction wraps gorm.DB to implement Transaction interface
type GormTransaction struct {
	tx *gorm.DB
}

func (t *GormTransaction) Commit() error {
	return t.tx.Commit().Error
}

func (t *GormTransaction) Rollback() error {
	return t.tx.Rollback().Error
}

func (t *GormTransaction) Query(ctx context.Context, query string, args ...interface{}) (Rows, error) {
	sqlDB, err := t.tx.DB()
	if err != nil {
		return nil, err
	}
	rows, err := sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	return &SqlRows{rows: rows}, nil
}

func (t *GormTransaction) QueryRow(ctx context.Context, query string, args ...interface{}) Row {
	sqlDB, err := t.tx.DB()
	if err != nil {
		return &SqlRow{err: err}
	}
	row := sqlDB.QueryRowContext(ctx, query, args...)
	return &SqlRow{row: row}
}

func (t *GormTransaction) Exec(ctx context.Context, query string, args ...interface{}) (Result, error) {
	result := t.tx.WithContext(ctx).Exec(query, args...)
	if result.Error != nil {
		return nil, result.Error
	}
	return &GormResult{rowsAffected: result.RowsAffected}, nil
}

func (t *GormTransaction) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	err := t.tx.WithContext(ctx).Raw(query, args...).First(dest).Error
	if err == gorm.ErrRecordNotFound {
		return sql.ErrNoRows
	}
	return err
}

func (t *GormTransaction) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return t.tx.WithContext(ctx).Raw(query, args...).Find(dest).Error
}

func (t *GormTransaction) NamedExec(ctx context.Context, query string, arg interface{}) (Result, error) {
	result := t.tx.WithContext(ctx).Exec(query, arg)
	if result.Error != nil {
		return nil, result.Error
	}
	return &GormResult{rowsAffected: result.RowsAffected}, nil
}

// SqlStatement wraps sql.Stmt to implement Statement interface
type SqlStatement struct {
	stmt *sql.Stmt
}

func (s *SqlStatement) Query(ctx context.Context, args ...interface{}) (Rows, error) {
	rows, err := s.stmt.QueryContext(ctx, args...)
	if err != nil {
		return nil, err
	}
	return &SqlRows{rows: rows}, nil
}

func (s *SqlStatement) QueryRow(ctx context.Context, args ...interface{}) Row {
	row := s.stmt.QueryRowContext(ctx, args...)
	return &SqlRow{row: row}
}

func (s *SqlStatement) Exec(ctx context.Context, args ...interface{}) (Result, error) {
	result, err := s.stmt.ExecContext(ctx, args...)
	if err != nil {
		return nil, err
	}
	return &SqlResult{result: result}, nil
}

func (s *SqlStatement) Close() error {
	return s.stmt.Close()
}

// SqlRows wraps sql.Rows to implement Rows interface
type SqlRows struct {
	rows *sql.Rows
}

func (r *SqlRows) Next() bool {
	return r.rows.Next()
}

func (r *SqlRows) Scan(dest ...interface{}) error {
	return r.rows.Scan(dest...)
}

func (r *SqlRows) Close() error {
	return r.rows.Close()
}

func (r *SqlRows) Err() error {
	return r.rows.Err()
}

func (r *SqlRows) Columns() ([]string, error) {
	return r.rows.Columns()
}

// SqlRow wraps sql.Row to implement Row interface
type SqlRow struct {
	row *sql.Row
	err error
}

func (r *SqlRow) Scan(dest ...interface{}) error {
	if r.err != nil {
		return r.err
	}
	if r.row == nil {
		return fmt.Errorf("no row to scan")
	}
	return r.row.Scan(dest...)
}

// SqlResult wraps sql.Result to implement Result interface
type SqlResult struct {
	result sql.Result
}

func (r *SqlResult) LastInsertId() (int64, error) {
	return r.result.LastInsertId()
}

func (r *SqlResult) RowsAffected() (int64, error) {
	return r.result.RowsAffected()
}

// GormResult implements Result for GORM operations
type GormResult struct {
	rowsAffected int64
}

func (r *GormResult) LastInsertId() (int64, error) {
	// GORM doesn't provide LastInsertId in the same way
	return 0, fmt.Errorf("LastInsertId not supported in GORM result")
}

func (r *GormResult) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}
