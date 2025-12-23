//go:build wasm

// Package database provides a WASM database adapter that calls host-provided functions.
// The host (Cloudflare Workers, Fermyon Spin, etc.) implements the actual database operations.
package database

import (
	"context"
	"database/sql"
	"encoding/json"
	"unsafe"

	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Host-provided database functions via wasmimport
//
//go:wasmimport env db_query
func hostDBQuery(queryPtr, queryLen, argsPtr, argsLen uint32) uint64

//go:wasmimport env db_exec
func hostDBExec(queryPtr, queryLen, argsPtr, argsLen uint32) uint64

//go:wasmimport env db_begin
func hostDBBegin() uint64

//go:wasmimport env db_commit
func hostDBCommit(txID uint32) uint32

//go:wasmimport env db_rollback
func hostDBRollback(txID uint32) uint32

// HostDB implements interfaces.Database using host-provided functions
type HostDB struct {
	dbType string
}

// NewHostDB creates a new host-provided database connection
func NewHostDB() *HostDB {
	return &HostDB{
		dbType: "sqlite", // Host typically uses SQLite (D1, Turso, etc.)
	}
}

// NewHostDBWithType creates a new host-provided database with specified type
func NewHostDBWithType(dbType string) *HostDB {
	return &HostDB{
		dbType: dbType,
	}
}

// Close closes the database connection
func (db *HostDB) Close() error {
	// Host manages connection lifecycle
	return nil
}

// Ping verifies the database connection
func (db *HostDB) Ping(ctx context.Context) error {
	// Execute a simple query to verify connection
	_, err := db.Exec(ctx, "SELECT 1")
	return err
}

// BeginTx starts a new transaction
func (db *HostDB) BeginTx(ctx context.Context) (interfaces.Transaction, error) {
	result := hostDBBegin()
	if result == 0 {
		return nil, errString("failed to begin transaction")
	}
	txID := uint32(result & 0xFFFFFFFF)
	errCode := uint32(result >> 32)
	if errCode != 0 {
		return nil, errString("transaction error")
	}
	return &hostTransaction{txID: txID, db: db}, nil
}

// Query executes a query that returns rows
func (db *HostDB) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	argsJSON, _ := json.Marshal(args)

	queryBytes := []byte(query)
	result := hostDBQuery(
		uint32(uintptr(unsafe.Pointer(&queryBytes[0]))),
		uint32(len(queryBytes)),
		uint32(uintptr(unsafe.Pointer(&argsJSON[0]))),
		uint32(len(argsJSON)),
	)

	if result == 0 {
		return nil, errString("query failed")
	}

	ptr := uint32(result >> 32)
	length := uint32(result & 0xFFFFFFFF)
	if ptr == 0 {
		return nil, errString("query returned no data")
	}

	data := readBytes(ptr, length)
	var queryResult queryResultJSON
	if err := json.Unmarshal(data, &queryResult); err != nil {
		return nil, err
	}

	if queryResult.Error != "" {
		return nil, errString(queryResult.Error)
	}

	return &hostRows{
		columns: queryResult.Columns,
		rows:    queryResult.Rows,
		index:   -1,
	}, nil
}

// QueryRow executes a query that returns a single row
func (db *HostDB) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	rows, err := db.Query(ctx, query, args...)
	if err != nil {
		return &hostRow{err: err}
	}
	if !rows.Next() {
		rows.Close()
		return &hostRow{err: sql.ErrNoRows}
	}
	hr := rows.(*hostRows)
	return &hostRow{values: hr.rows[hr.index]}
}

// Exec executes a query that doesn't return rows
func (db *HostDB) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	argsJSON, _ := json.Marshal(args)

	queryBytes := []byte(query)
	var argsPtr uint32
	if len(argsJSON) > 0 {
		argsPtr = uint32(uintptr(unsafe.Pointer(&argsJSON[0])))
	}

	result := hostDBExec(
		uint32(uintptr(unsafe.Pointer(&queryBytes[0]))),
		uint32(len(queryBytes)),
		argsPtr,
		uint32(len(argsJSON)),
	)

	ptr := uint32(result >> 32)
	length := uint32(result & 0xFFFFFFFF)

	if ptr == 0 && length == 0 {
		// Success with no additional data
		return &hostResult{rowsAffected: 0}, nil
	}

	if ptr != 0 {
		data := readBytes(ptr, length)
		var execResult execResultJSON
		if err := json.Unmarshal(data, &execResult); err != nil {
			return nil, err
		}
		if execResult.Error != "" {
			return nil, errString(execResult.Error)
		}
		return &hostResult{
			lastInsertID: execResult.LastInsertID,
			rowsAffected: execResult.RowsAffected,
		}, nil
	}

	return &hostResult{}, nil
}

// Get executes a query and scans the first row into dest
func (db *HostDB) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	row := db.QueryRow(ctx, query, args...)
	return row.Scan(dest)
}

// Select executes a query and scans all rows into dest
func (db *HostDB) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	// For WASM, we use a simplified approach
	// The caller should use Query and iterate manually
	return errString("Select not implemented in WASM - use Query instead")
}

// NamedExec executes a named query
func (db *HostDB) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return nil, errString("NamedExec not implemented in WASM")
}

// NamedQuery executes a named query that returns rows
func (db *HostDB) NamedQuery(ctx context.Context, query string, arg interface{}) (interfaces.Rows, error) {
	return nil, errString("NamedQuery not implemented in WASM")
}

// Prepare creates a prepared statement
func (db *HostDB) Prepare(ctx context.Context, query string) (interfaces.Statement, error) {
	return &hostStatement{db: db, query: query}, nil
}

// GetDB returns nil - no sql.DB in WASM mode
func (db *HostDB) GetDB() *sql.DB {
	return nil
}

// GetType returns the database type
func (db *HostDB) GetType() string {
	return db.dbType
}

// IsPostgres returns true if this is a PostgreSQL database
func (db *HostDB) IsPostgres() bool {
	return db.dbType == "postgres"
}

// IsSQLite returns true if this is a SQLite database
func (db *HostDB) IsSQLite() bool {
	return db.dbType == "sqlite"
}

// hostTransaction implements interfaces.Transaction
type hostTransaction struct {
	txID uint32
	db   *HostDB
}

func (tx *hostTransaction) Commit() error {
	result := hostDBCommit(tx.txID)
	if result != 0 {
		return errString("commit failed")
	}
	return nil
}

func (tx *hostTransaction) Rollback() error {
	result := hostDBRollback(tx.txID)
	if result != 0 {
		return errString("rollback failed")
	}
	return nil
}

func (tx *hostTransaction) Query(ctx context.Context, query string, args ...interface{}) (interfaces.Rows, error) {
	return tx.db.Query(ctx, query, args...)
}

func (tx *hostTransaction) QueryRow(ctx context.Context, query string, args ...interface{}) interfaces.Row {
	return tx.db.QueryRow(ctx, query, args...)
}

func (tx *hostTransaction) Exec(ctx context.Context, query string, args ...interface{}) (interfaces.Result, error) {
	return tx.db.Exec(ctx, query, args...)
}

func (tx *hostTransaction) Get(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return tx.db.Get(ctx, dest, query, args...)
}

func (tx *hostTransaction) Select(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	return tx.db.Select(ctx, dest, query, args...)
}

func (tx *hostTransaction) NamedExec(ctx context.Context, query string, arg interface{}) (interfaces.Result, error) {
	return tx.db.NamedExec(ctx, query, arg)
}

// hostStatement implements interfaces.Statement
type hostStatement struct {
	db    *HostDB
	query string
}

func (s *hostStatement) Query(ctx context.Context, args ...interface{}) (interfaces.Rows, error) {
	return s.db.Query(ctx, s.query, args...)
}

func (s *hostStatement) QueryRow(ctx context.Context, args ...interface{}) interfaces.Row {
	return s.db.QueryRow(ctx, s.query, args...)
}

func (s *hostStatement) Exec(ctx context.Context, args ...interface{}) (interfaces.Result, error) {
	return s.db.Exec(ctx, s.query, args...)
}

func (s *hostStatement) Close() error {
	return nil
}

// hostRows implements interfaces.Rows
type hostRows struct {
	columns []string
	rows    [][]interface{}
	index   int
	err     error
}

func (r *hostRows) Next() bool {
	r.index++
	return r.index < len(r.rows)
}

func (r *hostRows) Scan(dest ...interface{}) error {
	if r.index < 0 || r.index >= len(r.rows) {
		return errString("no row to scan")
	}
	row := r.rows[r.index]
	for i, d := range dest {
		if i >= len(row) {
			break
		}
		if err := scanValue(d, row[i]); err != nil {
			return err
		}
	}
	return nil
}

func (r *hostRows) Close() error {
	return nil
}

func (r *hostRows) Err() error {
	return r.err
}

func (r *hostRows) Columns() ([]string, error) {
	return r.columns, nil
}

// hostRow implements interfaces.Row
type hostRow struct {
	values []interface{}
	err    error
}

func (r *hostRow) Scan(dest ...interface{}) error {
	if r.err != nil {
		return r.err
	}
	for i, d := range dest {
		if i >= len(r.values) {
			break
		}
		if err := scanValue(d, r.values[i]); err != nil {
			return err
		}
	}
	return nil
}

// hostResult implements interfaces.Result
type hostResult struct {
	lastInsertID int64
	rowsAffected int64
}

func (r *hostResult) LastInsertId() (int64, error) {
	return r.lastInsertID, nil
}

func (r *hostResult) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}

// JSON structures for host communication
type queryResultJSON struct {
	Columns []string        `json:"columns"`
	Rows    [][]interface{} `json:"rows"`
	Error   string          `json:"error,omitempty"`
}

type execResultJSON struct {
	LastInsertID int64  `json:"lastInsertId"`
	RowsAffected int64  `json:"rowsAffected"`
	Error        string `json:"error,omitempty"`
}

// Helper functions

func scanValue(dest interface{}, src interface{}) error {
	if src == nil {
		return nil
	}

	switch d := dest.(type) {
	case *string:
		switch v := src.(type) {
		case string:
			*d = v
		case float64:
			*d = formatFloat(v)
		default:
			data, _ := json.Marshal(src)
			*d = string(data)
		}
	case *int:
		if v, ok := src.(float64); ok {
			*d = int(v)
		}
	case *int64:
		if v, ok := src.(float64); ok {
			*d = int64(v)
		}
	case *float64:
		if v, ok := src.(float64); ok {
			*d = v
		}
	case *bool:
		switch v := src.(type) {
		case bool:
			*d = v
		case float64:
			*d = v != 0
		}
	case *interface{}:
		*d = src
	case *sql.NullString:
		if src == nil {
			d.Valid = false
		} else {
			d.Valid = true
			switch v := src.(type) {
			case string:
				d.String = v
			default:
				data, _ := json.Marshal(src)
				d.String = string(data)
			}
		}
	}
	return nil
}

func formatFloat(f float64) string {
	// Simple float to string conversion
	if f == float64(int64(f)) {
		return formatInt(int64(f))
	}
	// For simplicity, just use JSON encoding
	data, _ := json.Marshal(f)
	return string(data)
}

func formatInt(i int64) string {
	if i == 0 {
		return "0"
	}
	negative := i < 0
	if negative {
		i = -i
	}
	var buf [20]byte
	pos := len(buf)
	for i > 0 {
		pos--
		buf[pos] = byte('0' + i%10)
		i /= 10
	}
	if negative {
		pos--
		buf[pos] = '-'
	}
	return string(buf[pos:])
}

func readBytes(ptr, length uint32) []byte {
	if ptr == 0 || length == 0 {
		return nil
	}
	return unsafe.Slice((*byte)(unsafe.Pointer(uintptr(ptr))), length)
}

// errString is a simple error type
type errString string

func (e errString) Error() string { return string(e) }
