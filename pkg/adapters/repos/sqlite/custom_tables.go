//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type customTablesRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewCustomTablesRepository creates a new SQLite custom tables repository
func NewCustomTablesRepository(sqlDB *sql.DB, queries *db.Queries) repos.CustomTablesRepository {
	return &customTablesRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *customTablesRepository) CreateDefinition(ctx context.Context, def *models.CustomTableDefinition) error {
	now := apptime.NowString()
	fieldsJSON, _ := json.Marshal(def.Fields)
	indexesJSON, _ := json.Marshal(def.Indexes)
	optionsJSON, _ := json.Marshal(def.Options)

	_, err := r.queries.CreateCustomTableDefinition(ctx, db.CreateCustomTableDefinitionParams{
		Name:        def.Name,
		DisplayName: strPtr(def.DisplayName),
		Description: strPtr(def.Description),
		Fields:      fieldsJSON,
		Indexes:     indexesJSON,
		Options:     optionsJSON,
		CreatedBy:   strPtr(def.CreatedBy),
		Status:      strPtr(def.Status),
		CreatedAt:   now,
		UpdatedAt:   now,
	})
	return err
}

func (r *customTablesRepository) GetDefinition(ctx context.Context, id uint) (*models.CustomTableDefinition, error) {
	dbDef, err := r.queries.GetCustomTableDefinitionByID(ctx, int64(id))
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBCustomTableDefToModel(dbDef), nil
}

func (r *customTablesRepository) GetDefinitionByName(ctx context.Context, name string) (*models.CustomTableDefinition, error) {
	dbDef, err := r.queries.GetCustomTableDefinitionByName(ctx, name)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBCustomTableDefToModel(dbDef), nil
}

func (r *customTablesRepository) ListDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error) {
	dbDefs, err := r.queries.ListCustomTableDefinitions(ctx)
	if err != nil {
		return nil, err
	}
	defs := make([]*models.CustomTableDefinition, len(dbDefs))
	for i, d := range dbDefs {
		defs[i] = convertDBCustomTableDefToModel(d)
	}
	return defs, nil
}

func (r *customTablesRepository) ListActiveDefinitions(ctx context.Context) ([]*models.CustomTableDefinition, error) {
	dbDefs, err := r.queries.ListActiveCustomTableDefinitions(ctx)
	if err != nil {
		return nil, err
	}
	defs := make([]*models.CustomTableDefinition, len(dbDefs))
	for i, d := range dbDefs {
		defs[i] = convertDBCustomTableDefToModel(d)
	}
	return defs, nil
}

func (r *customTablesRepository) UpdateDefinition(ctx context.Context, def *models.CustomTableDefinition) error {
	now := apptime.NowString()
	fieldsJSON, _ := json.Marshal(def.Fields)
	indexesJSON, _ := json.Marshal(def.Indexes)
	optionsJSON, _ := json.Marshal(def.Options)

	return r.queries.UpdateCustomTableDefinition(ctx, db.UpdateCustomTableDefinitionParams{
		ID:          int64(def.ID),
		DisplayName: strPtr(def.DisplayName),
		Description: strPtr(def.Description),
		Fields:      fieldsJSON,
		Indexes:     indexesJSON,
		Options:     optionsJSON,
		Status:      strPtr(def.Status),
		UpdatedAt:   now,
	})
}

func (r *customTablesRepository) DeleteDefinition(ctx context.Context, id uint) error {
	return r.queries.DeleteCustomTableDefinition(ctx, int64(id))
}

func (r *customTablesRepository) CreateMigration(ctx context.Context, migration *models.CustomTableMigration) error {
	oldSchema, _ := json.Marshal(migration.OldSchema)
	newSchema, _ := json.Marshal(migration.NewSchema)
	tableID := int64(migration.TableID)
	version := int64(migration.Version)

	_, err := r.queries.CreateCustomTableMigration(ctx, db.CreateCustomTableMigrationParams{
		TableID:       &tableID,
		Version:       &version,
		MigrationType: strPtr(migration.MigrationType),
		OldSchema:     oldSchema,
		NewSchema:     newSchema,
		ExecutedBy:    strPtr(migration.ExecutedBy),
		ExecutedAt:    migration.ExecutedAt,
		Status:        strPtr(migration.Status),
	})
	return err
}

func (r *customTablesRepository) GetMigration(ctx context.Context, id uint) (*models.CustomTableMigration, error) {
	dbMig, err := r.queries.GetCustomTableMigrationByID(ctx, int64(id))
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBCustomTableMigToModel(dbMig), nil
}

func (r *customTablesRepository) ListMigrationsByTableID(ctx context.Context, tableID uint) ([]*models.CustomTableMigration, error) {
	tid := int64(tableID)
	dbMigs, err := r.queries.ListCustomTableMigrationsByTableID(ctx, &tid)
	if err != nil {
		return nil, err
	}
	migs := make([]*models.CustomTableMigration, len(dbMigs))
	for i, m := range dbMigs {
		migs[i] = convertDBCustomTableMigToModel(m)
	}
	return migs, nil
}

func (r *customTablesRepository) GetLatestMigration(ctx context.Context, tableID uint) (*models.CustomTableMigration, error) {
	tid := int64(tableID)
	dbMig, err := r.queries.GetLatestCustomTableMigration(ctx, &tid)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBCustomTableMigToModel(dbMig), nil
}

func (r *customTablesRepository) UpdateMigrationStatus(ctx context.Context, id uint, status, errorMessage string) error {
	return r.queries.UpdateCustomTableMigrationStatus(ctx, db.UpdateCustomTableMigrationStatusParams{
		ID:           int64(id),
		Status:       &status,
		ErrorMessage: strPtr(errorMessage),
	})
}

func (r *customTablesRepository) RollbackMigration(ctx context.Context, id uint) error {
	return r.queries.RollbackCustomTableMigration(ctx, db.RollbackCustomTableMigrationParams{
		ID:         int64(id),
		RollbackAt: apptime.NewNullTimeNow(),
	})
}

func (r *customTablesRepository) TableExists(ctx context.Context, tableName string) (bool, error) {
	var exists int
	err := r.sqlDB.QueryRowContext(ctx,
		"SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
		tableName,
	).Scan(&exists)
	if err == sql.ErrNoRows {
		return false, nil
	}
	if err != nil {
		return false, err
	}
	return true, nil
}

func (r *customTablesRepository) GetTableColumns(ctx context.Context, tableName string) ([]repos.ColumnInfo, error) {
	rows, err := r.sqlDB.QueryContext(ctx, fmt.Sprintf("PRAGMA table_info(%s)", tableName))
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var columns []repos.ColumnInfo
	for rows.Next() {
		var cid int
		var name, colType string
		var notNull, pk int
		var dfltValue sql.NullString
		if err := rows.Scan(&cid, &name, &colType, &notNull, &dfltValue, &pk); err != nil {
			return nil, err
		}
		col := repos.ColumnInfo{
			Name:      name,
			Type:      colType,
			Nullable:  notNull == 0,
			IsPrimary: pk == 1,
		}
		if dfltValue.Valid {
			col.HasDefault = true
			col.Default = &dfltValue.String
		}
		columns = append(columns, col)
	}
	return columns, nil
}

func (r *customTablesRepository) GetNextVersion(ctx context.Context, tableID uint) (int, error) {
	var maxVersion sql.NullInt64
	err := r.sqlDB.QueryRowContext(ctx,
		"SELECT MAX(version) FROM custom_table_migrations WHERE table_id = ?",
		tableID,
	).Scan(&maxVersion)
	if err != nil {
		return 1, nil
	}
	if !maxVersion.Valid {
		return 1, nil
	}
	return int(maxVersion.Int64) + 1, nil
}

// DDL Executor

type ddlExecutor struct {
	sqlDB *sql.DB
}

// NewDDLExecutor creates a new SQLite DDL executor
func NewDDLExecutor(sqlDB *sql.DB) repos.DDLExecutor {
	return &ddlExecutor{sqlDB: sqlDB}
}

func (e *ddlExecutor) CreateTable(ctx context.Context, sql string) error {
	_, err := e.sqlDB.ExecContext(ctx, sql)
	return err
}

func (e *ddlExecutor) AlterTable(ctx context.Context, sql string) error {
	_, err := e.sqlDB.ExecContext(ctx, sql)
	return err
}

func (e *ddlExecutor) DropTable(ctx context.Context, tableName string) error {
	_, err := e.sqlDB.ExecContext(ctx, fmt.Sprintf("DROP TABLE IF EXISTS %s", tableName))
	return err
}

func (e *ddlExecutor) CreateIndex(ctx context.Context, sql string) error {
	_, err := e.sqlDB.ExecContext(ctx, sql)
	return err
}

func (e *ddlExecutor) DropIndex(ctx context.Context, indexName string) error {
	_, err := e.sqlDB.ExecContext(ctx, fmt.Sprintf("DROP INDEX IF EXISTS %s", indexName))
	return err
}

func (e *ddlExecutor) ExecDDL(ctx context.Context, sql string) error {
	_, err := e.sqlDB.ExecContext(ctx, sql)
	return err
}

// Conversion helpers

func convertDBCustomTableDefToModel(dbDef db.CustomTableDefinition) *models.CustomTableDefinition {
	var fields []models.CustomTableField
	var indexes []models.CustomTableIndex
	var options models.CustomTableOptions

	json.Unmarshal(dbDef.Fields, &fields)
	json.Unmarshal(dbDef.Indexes, &indexes)
	json.Unmarshal(dbDef.Options, &options)

	var displayName, description, createdBy, status string
	if dbDef.DisplayName != nil {
		displayName = *dbDef.DisplayName
	}
	if dbDef.Description != nil {
		description = *dbDef.Description
	}
	if dbDef.CreatedBy != nil {
		createdBy = *dbDef.CreatedBy
	}
	if dbDef.Status != nil {
		status = *dbDef.Status
	}

	return &models.CustomTableDefinition{
		ID:          uint(dbDef.ID),
		Name:        dbDef.Name,
		DisplayName: displayName,
		Description: description,
		Fields:      fields,
		Indexes:     indexes,
		Options:     options,
		CreatedBy:   createdBy,
		Status:      status,
		CreatedAt:   apptime.MustParse(dbDef.CreatedAt),
		UpdatedAt:   apptime.MustParse(dbDef.UpdatedAt),
	}
}

func convertDBCustomTableMigToModel(dbMig db.CustomTableMigration) *models.CustomTableMigration {
	var migrationType, executedBy, status, errorMessage string
	var tableID uint
	var version int

	if dbMig.TableID != nil {
		tableID = uint(*dbMig.TableID)
	}
	if dbMig.Version != nil {
		version = int(*dbMig.Version)
	}
	if dbMig.MigrationType != nil {
		migrationType = *dbMig.MigrationType
	}
	if dbMig.ExecutedBy != nil {
		executedBy = *dbMig.ExecutedBy
	}
	if dbMig.Status != nil {
		status = *dbMig.Status
	}
	if dbMig.ErrorMessage != nil {
		errorMessage = *dbMig.ErrorMessage
	}

	return &models.CustomTableMigration{
		ID:            uint(dbMig.ID),
		TableID:       tableID,
		Version:       version,
		MigrationType: migrationType,
		OldSchema:     dbMig.OldSchema,
		NewSchema:     dbMig.NewSchema,
		ExecutedBy:    executedBy,
		ExecutedAt:    dbMig.ExecutedAt,
		RollbackAt:    dbMig.RollbackAt,
		Status:        status,
		ErrorMessage:  errorMessage,
	}
}

// Ensure implementations
var _ repos.CustomTablesRepository = (*customTablesRepository)(nil)
var _ repos.DDLExecutor = (*ddlExecutor)(nil)
