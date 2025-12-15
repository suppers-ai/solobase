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

// GormRepository provides generic CRUD operations for GORM models
// Usage: repo := NewGormRepository[User](db)
type GormRepository[T any] struct {
	db GormDB
}

// GormDB interface for dependency injection (allows mocking)
type GormDB interface {
	Create(value interface{}) GormDB
	First(dest interface{}, conds ...interface{}) GormDB
	Find(dest interface{}, conds ...interface{}) GormDB
	Model(value interface{}) GormDB
	Where(query interface{}, args ...interface{}) GormDB
	Updates(values interface{}) GormDB
	Delete(value interface{}, conds ...interface{}) GormDB
	Offset(offset int) GormDB
	Limit(limit int) GormDB
	Order(value interface{}) GormDB
	Count(count *int64) GormDB
	Error() error
}

// NewGormRepository creates a new generic repository
func NewGormRepository[T any](db GormDB) *GormRepository[T] {
	return &GormRepository[T]{db: db}
}

// Create creates a new record
func (r *GormRepository[T]) Create(entity *T) error {
	return r.db.Create(entity).Error()
}

// GetByID retrieves a record by ID
func (r *GormRepository[T]) GetByID(id interface{}) (*T, error) {
	var entity T
	if err := r.db.First(&entity, id).Error(); err != nil {
		return nil, err
	}
	return &entity, nil
}

// GetByField retrieves a record by a specific field
func (r *GormRepository[T]) GetByField(field string, value interface{}) (*T, error) {
	var entity T
	if err := r.db.Where(field+" = ?", value).First(&entity).Error(); err != nil {
		return nil, err
	}
	return &entity, nil
}

// List retrieves all records
func (r *GormRepository[T]) List() ([]T, error) {
	var entities []T
	if err := r.db.Find(&entities).Error(); err != nil {
		return nil, err
	}
	return entities, nil
}

// ListPaginated retrieves records with pagination
func (r *GormRepository[T]) ListPaginated(page, pageSize int) ([]T, int64, error) {
	var entities []T
	var total int64

	// Get total count
	if err := r.db.Model(new(T)).Count(&total).Error(); err != nil {
		return nil, 0, err
	}

	// Get paginated results
	offset := (page - 1) * pageSize
	if err := r.db.Offset(offset).Limit(pageSize).Find(&entities).Error(); err != nil {
		return nil, 0, err
	}

	return entities, total, nil
}

// Update updates a record by ID
func (r *GormRepository[T]) Update(id interface{}, updates interface{}) error {
	return r.db.Model(new(T)).Where("id = ?", id).Updates(updates).Error()
}

// Delete deletes a record by ID
func (r *GormRepository[T]) Delete(id interface{}) error {
	return r.db.Delete(new(T), id).Error()
}

// ListByField retrieves records filtered by a field
func (r *GormRepository[T]) ListByField(field string, value interface{}) ([]T, error) {
	var entities []T
	if err := r.db.Where(field+" = ?", value).Find(&entities).Error(); err != nil {
		return nil, err
	}
	return entities, nil
}

// ListByFieldPaginated retrieves records filtered by a field with pagination
func (r *GormRepository[T]) ListByFieldPaginated(field string, value interface{}, page, pageSize int) ([]T, int64, error) {
	var entities []T
	var total int64

	// Get total count with filter
	if err := r.db.Model(new(T)).Where(field+" = ?", value).Count(&total).Error(); err != nil {
		return nil, 0, err
	}

	// Get paginated results with filter
	offset := (page - 1) * pageSize
	if err := r.db.Where(field+" = ?", value).Offset(offset).Limit(pageSize).Find(&entities).Error(); err != nil {
		return nil, 0, err
	}

	return entities, total, nil
}
