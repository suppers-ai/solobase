package repos

import (
	"context"
)

// ExtensionQueryOptions configures extension queries
type ExtensionQueryOptions struct {
	Where   map[string]interface{}
	OrderBy string
	Pagination
}

// ExtensionRepository provides extension-specific database access
// Extensions use typed operations with table name parameters
type ExtensionRepository interface {
	// Typed operations (encouraged for new extensions)
	Get(ctx context.Context, table string, id string) (map[string]interface{}, error)
	List(ctx context.Context, table string, opts ExtensionQueryOptions) ([]map[string]interface{}, error)
	Count(ctx context.Context, table string, where map[string]interface{}) (int64, error)
	Insert(ctx context.Context, table string, data map[string]interface{}) (string, error)
	Update(ctx context.Context, table string, id string, data map[string]interface{}) error
	Delete(ctx context.Context, table string, id string) error

	// Batch operations
	InsertMany(ctx context.Context, table string, data []map[string]interface{}) ([]string, error)
	UpdateMany(ctx context.Context, table string, where map[string]interface{}, data map[string]interface{}) (int64, error)
	DeleteMany(ctx context.Context, table string, where map[string]interface{}) (int64, error)

	// Raw query support (for complex queries)
	// NOTE: Use with caution - prefer typed operations when possible
	Query(ctx context.Context, query string, args ...interface{}) (Rows, error)
	Exec(ctx context.Context, query string, args ...interface{}) (Result, error)

	// Transaction support
	Transaction(ctx context.Context, fn func(ExtensionRepository) error) error

	// Table prefix for this extension
	TablePrefix() string
}
