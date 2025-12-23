//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type customTablesRepository struct{}

func (r *customTablesRepository) CreateDefinition(ctx context.Context, def *models.CustomTableDefinition) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) GetDefinition(ctx context.Context, id uint) (*models.CustomTableDefinition, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) GetDefinitionByName(ctx context.Context, name string) (*models.CustomTableDefinition, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) ListDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) ListActiveDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) UpdateDefinition(ctx context.Context, def *models.CustomTableDefinition) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) DeleteDefinition(ctx context.Context, id uint) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) CreateMigration(ctx context.Context, migration *models.CustomTableMigration) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) GetMigration(ctx context.Context, id uint) (*models.CustomTableMigration, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) ListMigrationsByTableID(ctx context.Context, tableID uint) ([]*models.CustomTableMigration, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) GetLatestMigration(ctx context.Context, tableID uint) (*models.CustomTableMigration, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) UpdateMigrationStatus(ctx context.Context, id uint, status, errorMessage string) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) RollbackMigration(ctx context.Context, id uint) error {
	return ErrNotImplemented
}

func (r *customTablesRepository) TableExists(ctx context.Context, tableName string) (bool, error) {
	return false, ErrNotImplemented
}

func (r *customTablesRepository) GetTableColumns(ctx context.Context, tableName string) ([]repos.ColumnInfo, error) {
	return nil, ErrNotImplemented
}

func (r *customTablesRepository) GetNextVersion(ctx context.Context, tableID uint) (int, error) {
	return 0, ErrNotImplemented
}

// DDL Executor

type ddlExecutor struct{}

func (e *ddlExecutor) CreateTable(ctx context.Context, sql string) error {
	return ErrNotImplemented
}

func (e *ddlExecutor) AlterTable(ctx context.Context, sql string) error {
	return ErrNotImplemented
}

func (e *ddlExecutor) DropTable(ctx context.Context, tableName string) error {
	return ErrNotImplemented
}

func (e *ddlExecutor) CreateIndex(ctx context.Context, sql string) error {
	return ErrNotImplemented
}

func (e *ddlExecutor) DropIndex(ctx context.Context, indexName string) error {
	return ErrNotImplemented
}

func (e *ddlExecutor) ExecDDL(ctx context.Context, sql string) error {
	return ErrNotImplemented
}

// Ensure implementations
var _ repos.CustomTablesRepository = (*customTablesRepository)(nil)
var _ repos.DDLExecutor = (*ddlExecutor)(nil)
