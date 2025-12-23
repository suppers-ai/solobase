package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/data/models"
)

// ColumnInfo describes a database column
type ColumnInfo struct {
	Name       string
	Type       string
	Nullable   bool
	HasDefault bool
	Default    *string
	IsPrimary  bool
}

// CustomTablesRepository provides custom table definition operations
type CustomTablesRepository interface {
	// Definitions
	CreateDefinition(ctx context.Context, def *models.CustomTableDefinition) error
	GetDefinition(ctx context.Context, id uint) (*models.CustomTableDefinition, error)
	GetDefinitionByName(ctx context.Context, name string) (*models.CustomTableDefinition, error)
	ListDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error)
	ListActiveDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error)
	UpdateDefinition(ctx context.Context, def *models.CustomTableDefinition) error
	DeleteDefinition(ctx context.Context, id uint) error

	// Migrations
	CreateMigration(ctx context.Context, migration *models.CustomTableMigration) error
	GetMigration(ctx context.Context, id uint) (*models.CustomTableMigration, error)
	ListMigrationsByTableID(ctx context.Context, tableID uint) ([]*models.CustomTableMigration, error)
	GetLatestMigration(ctx context.Context, tableID uint) (*models.CustomTableMigration, error)
	UpdateMigrationStatus(ctx context.Context, id uint, status, errorMessage string) error
	RollbackMigration(ctx context.Context, id uint) error

	// Schema introspection
	TableExists(ctx context.Context, tableName string) (bool, error)
	GetTableColumns(ctx context.Context, tableName string) ([]ColumnInfo, error)
	GetNextVersion(ctx context.Context, tableID uint) (int, error)
}

// DDLExecutor handles DDL operations (separated for safety)
// These operations directly modify the database schema
type DDLExecutor interface {
	// CreateTable executes a CREATE TABLE statement
	CreateTable(ctx context.Context, sql string) error

	// AlterTable executes an ALTER TABLE statement
	AlterTable(ctx context.Context, sql string) error

	// DropTable drops a table by name
	DropTable(ctx context.Context, tableName string) error

	// CreateIndex executes a CREATE INDEX statement
	CreateIndex(ctx context.Context, sql string) error

	// DropIndex drops an index by name
	DropIndex(ctx context.Context, indexName string) error

	// ExecDDL executes an arbitrary DDL statement (use with caution)
	ExecDDL(ctx context.Context, sql string) error
}
