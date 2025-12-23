//go:build wasm

// Package wasm provides a database adapter that uses WIT-imported database interface.
// The host runtime provides the actual database implementation.
package wasm

import (
	"context"
	"database/sql"
	"fmt"

	"go.bytecodealliance.org/cm"

	"github.com/suppers-ai/solobase/builds/wasm/gen/solobase/core/database"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Adapter implements interfaces.Database using WIT database imports
type Adapter struct{}

// Ensure Adapter implements interfaces.Database
var _ interfaces.Database = (*Adapter)(nil)

// New creates a new WASM database adapter
func New() *Adapter {
	return &Adapter{}
}

// Close is a no-op for WASM (host manages connection)
func (a *Adapter) Close() error {
	return nil
}

// Ping verifies the database connection
func (a *Adapter) Ping(ctx context.Context) error {
	// Execute a simple query to check connection
	result := database.Query("SELECT 1", cm.ToList([]database.ColumnValue{}))
	if result.IsErr() {
		err := result.Err()
		return fmt.Errorf("ping failed: %s: %s", err.Code, err.Message)
	}
	return nil
}

// BeginTx starts a new transaction
func (a *Adapter) BeginTx(ctx context.Context) (interfaces.Transaction, error) {
	result := database.BeginTransaction()
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("begin transaction failed: %s: %s", err.Code, err.Message)
	}
	return &transaction{}, nil
}

// Query executes a query that returns rows
func (a *Adapter) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	params := argsToColumnValues(args)
	result := database.Query(query, cm.ToList(params))
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("query failed: %s: %s", err.Code, err.Message)
	}
	qr := result.OK()
	return &rows{result: qr, index: -1}, nil
}

// QueryRow executes a query that returns a single row
func (a *Adapter) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	r, err := a.Query(ctx, query, args...)
	return &row{rows: r, err: err}
}

// Exec executes a query that doesn't return rows
func (a *Adapter) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	params := argsToColumnValues(args)
	result := database.Execute(query, cm.ToList(params))
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("exec failed: %s: %s", err.Code, err.Message)
	}
	rowsAffected := *result.OK()
	return &execResult{rowsAffected: int64(rowsAffected)}, nil
}

// Get executes a query and scans the result into dest (single row)
func (a *Adapter) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Get() requires manual scanning in WASM")
}

// Select executes a query and scans the results into dest (multiple rows)
func (a *Adapter) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Select() requires manual scanning in WASM")
}

// NamedExec executes a named query with named parameters
func (a *Adapter) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return nil, fmt.Errorf("NamedExec() not supported in WASM")
}

// NamedQuery executes a named query with named parameters
func (a *Adapter) NamedQuery(ctx context.Context, query string, arg interface{}) (interfaces.Rows, error) {
	return nil, fmt.Errorf("NamedQuery() not supported in WASM")
}

// Prepare creates a prepared statement
func (a *Adapter) Prepare(ctx context.Context, query string) (interfaces.Statement, error) {
	return nil, fmt.Errorf("Prepare() not supported in WASM")
}

// GetDB returns nil (no underlying sql.DB in WASM)
func (a *Adapter) GetDB() *sql.DB {
	return nil
}

// GetType returns the database type
func (a *Adapter) GetType() string {
	return "wasm"
}

// IsPostgres returns false (we don't know the underlying type)
func (a *Adapter) IsPostgres() bool {
	return false // Could be either, host decides
}

// IsSQLite returns false (we don't know the underlying type)
func (a *Adapter) IsSQLite() bool {
	return false // Could be either, host decides
}

// transaction implements interfaces.Transaction
type transaction struct{}

func (t *transaction) Commit() error {
	result := database.Commit()
	if result.IsErr() {
		err := result.Err()
		return fmt.Errorf("commit failed: %s: %s", err.Code, err.Message)
	}
	return nil
}

func (t *transaction) Rollback() error {
	result := database.Rollback()
	if result.IsErr() {
		err := result.Err()
		return fmt.Errorf("rollback failed: %s: %s", err.Code, err.Message)
	}
	return nil
}

func (t *transaction) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	params := argsToColumnValues(args)
	result := database.Query(query, cm.ToList(params))
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("query failed: %s: %s", err.Code, err.Message)
	}
	qr := result.OK()
	return &rows{result: qr, index: -1}, nil
}

func (t *transaction) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	r, err := t.Query(ctx, query, args...)
	return &row{rows: r, err: err}
}

func (t *transaction) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	params := argsToColumnValues(args)
	result := database.Execute(query, cm.ToList(params))
	if result.IsErr() {
		err := result.Err()
		return nil, fmt.Errorf("exec failed: %s: %s", err.Code, err.Message)
	}
	rowsAffected := *result.OK()
	return &execResult{rowsAffected: int64(rowsAffected)}, nil
}

func (t *transaction) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Get() not supported in WASM transactions")
}

func (t *transaction) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return fmt.Errorf("Select() not supported in WASM transactions")
}

func (t *transaction) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return nil, fmt.Errorf("NamedExec() not supported in WASM transactions")
}

// rows implements interfaces.Rows
type rows struct {
	result  *database.QueryResult
	index   int
	current database.Row
}

func (r *rows) Next() bool {
	r.index++
	rowsList := r.result.Rows.Slice()
	if r.index >= len(rowsList) {
		return false
	}
	r.current = rowsList[r.index]
	return true
}

func (r *rows) Scan(dest ...interface{}) error {
	cols := r.current.Slice()
	if len(dest) != len(cols) {
		return fmt.Errorf("scan: expected %d columns, got %d", len(cols), len(dest))
	}
	for i, col := range cols {
		if err := scanColumnValue(&col, dest[i]); err != nil {
			return fmt.Errorf("scan column %d: %w", i, err)
		}
	}
	return nil
}

func (r *rows) Close() error {
	return nil // No resources to release in WASM
}

func (r *rows) Err() error {
	return nil
}

func (r *rows) Columns() ([]string, error) {
	return r.result.Columns.Slice(), nil
}

// row implements interfaces.Row
type row struct {
	rows interfaces.Rows
	err  error
}

func (r *row) Scan(dest ...interface{}) error {
	if r.err != nil {
		return r.err
	}
	if r.rows == nil {
		return sql.ErrNoRows
	}
	if !r.rows.Next() {
		return sql.ErrNoRows
	}
	return r.rows.Scan(dest...)
}

// execResult implements interfaces.Result
type execResult struct {
	rowsAffected int64
}

func (r *execResult) LastInsertId() (int64, error) {
	return 0, fmt.Errorf("LastInsertId not supported in WASM")
}

func (r *execResult) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}

// argsToColumnValues converts Go args to WIT ColumnValue
func argsToColumnValues(args []interface{}) []database.ColumnValue {
	result := make([]database.ColumnValue, len(args))
	for i, arg := range args {
		result[i] = toColumnValue(arg)
	}
	return result
}

func toColumnValue(v interface{}) database.ColumnValue {
	if v == nil {
		return database.ColumnValueColNull()
	}
	switch val := v.(type) {
	case bool:
		return database.ColumnValueColBool(val)
	case int:
		return database.ColumnValueColI64(int64(val))
	case int32:
		return database.ColumnValueColI32(val)
	case int64:
		return database.ColumnValueColI64(val)
	case float64:
		return database.ColumnValueColF64(val)
	case string:
		return database.ColumnValueColText(val)
	case []byte:
		return database.ColumnValueColBlob(cm.ToList(val))
	default:
		// Convert to string as fallback
		return database.ColumnValueColText(fmt.Sprintf("%v", v))
	}
}

func scanColumnValue(col *database.ColumnValue, dest interface{}) error {
	if col.ColNull() {
		// Handle null - set pointer types to nil
		return nil
	}

	switch d := dest.(type) {
	case *bool:
		if v := col.ColBool(); v != nil {
			*d = *v
			return nil
		}
	case *int:
		if v := col.ColI64(); v != nil {
			*d = int(*v)
			return nil
		}
		if v := col.ColI32(); v != nil {
			*d = int(*v)
			return nil
		}
	case *int32:
		if v := col.ColI32(); v != nil {
			*d = *v
			return nil
		}
	case *int64:
		if v := col.ColI64(); v != nil {
			*d = *v
			return nil
		}
	case *float64:
		if v := col.ColF64(); v != nil {
			*d = *v
			return nil
		}
	case *string:
		if v := col.ColText(); v != nil {
			*d = *v
			return nil
		}
	case *[]byte:
		if v := col.ColBlob(); v != nil {
			*d = v.Slice()
			return nil
		}
	}
	return fmt.Errorf("cannot scan column value to %T", dest)
}
