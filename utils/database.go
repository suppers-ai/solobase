package utils

import (
	"context"
	"database/sql"
	"fmt"

	"github.com/suppers-ai/solobase/internal/pkg/database"
)

// DatabaseHelper provides common database operations
type DatabaseHelper struct {
	db database.Database
}

// NewDatabaseHelper creates a new database helper
func NewDatabaseHelper(db database.Database) *DatabaseHelper {
	return &DatabaseHelper{db: db}
}

// QueryWithCount executes a query and count query in parallel
func (d *DatabaseHelper) QueryWithCount(ctx context.Context, query, countQuery string, args ...interface{}) (database.Rows, int, error) {
	// Execute count query
	var total int
	err := d.db.Get(ctx, &total, countQuery, args...)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to count: %w", err)
	}

	// Execute main query
	rows, err := d.db.Query(ctx, query, args...)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to query: %w", err)
	}

	return rows, total, nil
}

// TransactionFunc is a function that runs within a transaction
type TransactionFunc func(tx database.Transaction) error

// WithTransaction executes a function within a database transaction
func (d *DatabaseHelper) WithTransaction(ctx context.Context, fn TransactionFunc) error {
	tx, err := d.db.BeginTx(ctx)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	if err := fn(tx); err != nil {
		return err
	}

	return tx.Commit()
}

// Exists checks if a record exists
func (d *DatabaseHelper) Exists(ctx context.Context, query string, args ...interface{}) (bool, error) {
	var exists bool
	err := d.db.Get(ctx, &exists, query, args...)
	return exists, err
}

// GetOne retrieves a single record or returns an error if not found
func (d *DatabaseHelper) GetOne(ctx context.Context, dest interface{}, query string, args ...interface{}) error {
	err := d.db.Get(ctx, dest, query, args...)
	if err == sql.ErrNoRows {
		return fmt.Errorf("record not found")
	}
	return err
}

// BuildPaginationQuery adds limit and offset to a query
func BuildPaginationQuery(baseQuery string, limit, offset int) string {
	if limit > 0 {
		baseQuery += fmt.Sprintf(" LIMIT %d", limit)
	}
	if offset > 0 {
		baseQuery += fmt.Sprintf(" OFFSET %d", offset)
	}
	return baseQuery
}

// BuildCountQuery converts a SELECT query to a COUNT query
func BuildCountQuery(selectQuery string) string {
	// Simple implementation - in production, use a proper SQL parser
	return "SELECT COUNT(*) FROM (" + selectQuery + ") AS count_query"
}
