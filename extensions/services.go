package extensions

import (
	"context"
	"database/sql"

	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/extensions/core"
	"gorm.io/gorm"
)

// BasicExtensionServices provides a basic implementation of extension services
type BasicExtensionServices struct {
	db     *gorm.DB
	logger logger.Logger
}

// NewBasicExtensionServices creates basic extension services
func NewBasicExtensionServices(db *gorm.DB, logger logger.Logger) *BasicExtensionServices {
	return &BasicExtensionServices{
		db:     db,
		logger: logger,
	}
}

// ForExtension creates extension-specific services
func (s *BasicExtensionServices) ForExtension(extensionName string) *core.ExtensionServices {
	// Create a basic extension services instance
	// This is a simplified implementation for the initial integration
	return &core.ExtensionServices{}
}

// Database returns a basic database interface
func (s *BasicExtensionServices) Database() core.ExtensionDatabase {
	return &basicExtensionDatabase{db: s.db}
}

// Logger returns the logger
func (s *BasicExtensionServices) Logger() logger.Logger {
	return s.logger
}

// basicExtensionDatabase provides basic database access
type basicExtensionDatabase struct {
	db *gorm.DB
}

func (d *basicExtensionDatabase) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	// Get the underlying sql.DB
	sqlDB, err := d.db.DB()
	if err != nil {
		return nil, err
	}

	// Execute query
	rows, err := sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}

	// Wrap in database.Rows - for now return the sql.Rows directly
	// In production, this would properly implement the database.Rows interface
	return &basicRows{rows: rows}, nil
}

func (d *basicExtensionDatabase) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	// Get the underlying sql.DB
	sqlDB, err := d.db.DB()
	if err != nil {
		return nil, err
	}

	// Execute statement
	result, err := sqlDB.ExecContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}

	// Wrap in database.Result
	return &basicResult{result: result}, nil
}

func (d *basicExtensionDatabase) Transaction(ctx context.Context, fn func(core.ExtensionTx) error) error {
	return d.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		return fn(&basicExtensionTx{db: tx})
	})
}

// basicRows implements database.Rows
type basicRows struct {
	rows *sql.Rows
}

func (r *basicRows) Next() bool {
	return r.rows.Next()
}

func (r *basicRows) Scan(dest ...interface{}) error {
	return r.rows.Scan(dest...)
}

func (r *basicRows) Close() error {
	return r.rows.Close()
}

func (r *basicRows) Err() error {
	return r.rows.Err()
}

func (r *basicRows) Columns() ([]string, error) {
	return r.rows.Columns()
}

// basicResult implements database.Result
type basicResult struct {
	result sql.Result
}

func (r *basicResult) LastInsertId() (int64, error) {
	return r.result.LastInsertId()
}

func (r *basicResult) RowsAffected() (int64, error) {
	return r.result.RowsAffected()
}

// basicExtensionTx implements core.ExtensionTx
type basicExtensionTx struct {
	db *gorm.DB
}

func (t *basicExtensionTx) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	rows, err := t.db.WithContext(ctx).Raw(query, args...).Rows()
	if err != nil {
		return nil, err
	}
	return &basicRows{rows: rows}, nil
}

func (t *basicExtensionTx) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	result := t.db.WithContext(ctx).Exec(query, args...)
	if result.Error != nil {
		return nil, result.Error
	}
	return &basicGormResult{rowsAffected: result.RowsAffected}, nil
}

func (t *basicExtensionTx) Commit() error {
	return t.db.Commit().Error
}

func (t *basicExtensionTx) Rollback() error {
	return t.db.Rollback().Error
}

// basicGormResult implements database.Result for GORM
type basicGormResult struct {
	rowsAffected int64
}

func (r *basicGormResult) LastInsertId() (int64, error) {
	return 0, nil // GORM doesn't directly provide last insert ID
}

func (r *basicGormResult) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}
