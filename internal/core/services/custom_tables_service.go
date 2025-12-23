package services

import (
	"context"
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"strings"

	coremodels "github.com/suppers-ai/solobase/internal/core/models"
	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

// CustomTablesService handles dynamic table creation and management
type CustomTablesService struct {
	db     *sql.DB // Keep for DynamicRepository (CRUD operations on custom tables)
	repo   repos.CustomTablesRepository
	ddl    repos.DDLExecutor
	dbType string // "sqlite" or "postgres" - for SQL generation
}

// NewCustomTablesService creates a new custom tables service
func NewCustomTablesService(db *sql.DB, repo repos.CustomTablesRepository, ddl repos.DDLExecutor, dbType string) *CustomTablesService {
	return &CustomTablesService{
		db:     db,
		repo:   repo,
		ddl:    ddl,
		dbType: dbType,
	}
}

// Reserved table names that cannot be used for custom tables
var reservedTableNames = []string{
	"users", "roles", "permissions", "sessions", "settings",
	"custom_table_definitions", "custom_table_migrations",
	"logs", "audit_logs", "migrations",
}

// isValidTableName validates table name format without using regexp
// Pattern: ^[a-z][a-z0-9_]*$
func isValidTableName(name string) bool {
	if len(name) == 0 {
		return false
	}
	// First character must be lowercase letter
	if name[0] < 'a' || name[0] > 'z' {
		return false
	}
	// Rest can be lowercase letters, digits, or underscores
	for i := 1; i < len(name); i++ {
		c := name[i]
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// ValidateTableName checks if a table name is valid
func (s *CustomTablesService) ValidateTableName(name string) error {
	// Check for empty name
	if name == "" {
		return errors.New("table name cannot be empty")
	}

	// Check length
	if len(name) < 3 || len(name) > 50 {
		return errors.New("table name must be between 3 and 50 characters")
	}

	// Check for valid characters (alphanumeric and underscore only)
	if !isValidTableName(name) {
		return errors.New("table name must start with a letter and contain only lowercase letters, numbers, and underscores")
	}

	// Check for reserved names
	cleanName := models.StripCustomPrefix(name)
	for _, reserved := range reservedTableNames {
		if cleanName == reserved {
			return fmt.Errorf("table name '%s' is reserved", cleanName)
		}
	}

	// Check if table already exists
	tableName := models.EnsureCustomPrefix(name)
	if s.tableExists(tableName) {
		return fmt.Errorf("table '%s' already exists", cleanName)
	}

	return nil
}

// tableExists checks if a table exists in the database
func (s *CustomTablesService) tableExists(tableName string) bool {
	ctx := context.Background()
	exists, _ := s.repo.TableExists(ctx, tableName)
	return exists
}

// CreateTable creates a new custom table based on the definition
func (s *CustomTablesService) CreateTable(definition *models.CustomTableDefinition, userID string) error {
	ctx := context.Background()

	// Validate table name
	if err := s.ValidateTableName(definition.DisplayName); err != nil {
		return err
	}

	// Ensure custom_ prefix
	definition.Name = models.EnsureCustomPrefix(definition.DisplayName)
	definition.CreatedBy = userID
	definition.Status = "active"
	definition.CreatedAt = apptime.NowTime()
	definition.UpdatedAt = definition.CreatedAt

	// Save table definition
	if err := s.repo.CreateDefinition(ctx, definition); err != nil {
		return fmt.Errorf("failed to save table definition: %w", err)
	}

	// Get the created definition to get the ID
	created, err := s.repo.GetDefinitionByName(ctx, definition.Name)
	if err != nil {
		return fmt.Errorf("failed to retrieve created definition: %w", err)
	}
	definition.ID = created.ID

	// Create the actual table
	if err := s.createPhysicalTable(ctx, definition); err != nil {
		// Cleanup: delete the definition if table creation fails
		_ = s.repo.DeleteDefinition(ctx, definition.ID)
		return fmt.Errorf("failed to create table: %w", err)
	}

	// Record migration
	migration := &models.CustomTableMigration{
		TableID:       definition.ID,
		Version:       1,
		MigrationType: "create",
		NewSchema:     s.serializeSchema(definition),
		ExecutedBy:    userID,
		ExecutedAt:    apptime.NowTime(),
		Status:        "completed",
	}
	if err := s.repo.CreateMigration(ctx, migration); err != nil {
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return nil
}

// createPhysicalTable creates the actual database table
func (s *CustomTablesService) createPhysicalTable(ctx context.Context, definition *models.CustomTableDefinition) error {
	// Build SQL for table creation
	var columns []string

	// Add primary key if not explicitly defined
	hasPrimaryKey := false
	for _, field := range definition.Fields {
		if field.IsPrimaryKey {
			hasPrimaryKey = true
			break
		}
	}

	// If no primary key defined, add an auto-increment ID
	if !hasPrimaryKey {
		if s.dbType == "postgres" {
			columns = append(columns, "id BIGSERIAL PRIMARY KEY")
		} else {
			columns = append(columns, "id INTEGER PRIMARY KEY AUTOINCREMENT")
		}
	}

	// Add field columns
	for _, field := range definition.Fields {
		columnDef := s.buildColumnDefinition(field)
		columns = append(columns, columnDef)
	}

	// Add timestamp fields if enabled
	if definition.Options.Timestamps {
		columns = append(columns, "created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
		columns = append(columns, "updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
	}

	// Add soft delete field if enabled
	if definition.Options.SoftDelete {
		columns = append(columns, "deleted_at TIMESTAMP")
	}

	// Build and execute CREATE TABLE statement
	createSQL := fmt.Sprintf("CREATE TABLE %s (\n  %s\n)",
		definition.Name,
		strings.Join(columns, ",\n  "))

	if err := s.ddl.CreateTable(ctx, createSQL); err != nil {
		return err
	}

	// Create indexes
	for _, field := range definition.Fields {
		if field.IsIndexed && !field.IsPrimaryKey && !field.IsUnique {
			indexSQL := fmt.Sprintf("CREATE INDEX idx_%s_%s ON %s(%s)",
				definition.Name, field.Name, definition.Name, field.Name)
			if err := s.ddl.CreateIndex(ctx, indexSQL); err != nil {
				return fmt.Errorf("failed to create index for %s: %w", field.Name, err)
			}
		}
	}

	// Create soft delete index if enabled
	if definition.Options.SoftDelete {
		indexSQL := fmt.Sprintf("CREATE INDEX idx_%s_deleted_at ON %s(deleted_at)", definition.Name, definition.Name)
		if err := s.ddl.CreateIndex(ctx, indexSQL); err != nil {
			return fmt.Errorf("failed to create deleted_at index: %w", err)
		}
	}

	// Create custom indexes
	for _, idx := range definition.Indexes {
		indexType := "INDEX"
		if idx.Unique {
			indexType = "UNIQUE INDEX"
		}

		indexSQL := fmt.Sprintf("CREATE %s %s ON %s(%s)",
			indexType, idx.Name, definition.Name, strings.Join(idx.Columns, ", "))

		if err := s.ddl.CreateIndex(ctx, indexSQL); err != nil {
			return fmt.Errorf("failed to create index %s: %w", idx.Name, err)
		}
	}

	return nil
}

// buildColumnDefinition builds the SQL column definition for a field
func (s *CustomTablesService) buildColumnDefinition(field models.CustomTableField) string {
	var parts []string

	// Column name
	parts = append(parts, field.Name)

	// Column type
	parts = append(parts, field.GetSQLType())

	// Primary key
	if field.IsPrimaryKey {
		if field.AutoIncrement {
			if s.dbType == "postgres" {
				parts[1] = "BIGSERIAL"
			} else {
				parts[1] = "INTEGER"
				parts = append(parts, "AUTOINCREMENT")
			}
		}
		parts = append(parts, "PRIMARY KEY")
	}

	// NOT NULL constraint
	if !field.Nullable && !field.IsPrimaryKey {
		parts = append(parts, "NOT NULL")
	}

	// UNIQUE constraint
	if field.IsUnique {
		parts = append(parts, "UNIQUE")
	}

	// DEFAULT value
	if field.DefaultValue != nil {
		defaultStr := s.formatDefaultValue(field.DefaultValue, field.Type)
		parts = append(parts, fmt.Sprintf("DEFAULT %s", defaultStr))
	}

	// Foreign key constraint
	if field.ForeignKey != nil {
		fkConstraint := fmt.Sprintf("REFERENCES %s(%s)",
			field.ForeignKey.ReferenceTable,
			field.ForeignKey.ReferenceColumn)

		if field.ForeignKey.OnDelete != "" {
			fkConstraint += fmt.Sprintf(" ON DELETE %s", field.ForeignKey.OnDelete)
		}
		if field.ForeignKey.OnUpdate != "" {
			fkConstraint += fmt.Sprintf(" ON UPDATE %s", field.ForeignKey.OnUpdate)
		}

		parts = append(parts, fkConstraint)
	}

	return strings.Join(parts, " ")
}

// formatDefaultValue formats a default value for SQL
func (s *CustomTablesService) formatDefaultValue(value interface{}, fieldType string) string {
	switch fieldType {
	case "string", "text", "varchar":
		return fmt.Sprintf("'%v'", value)
	case "bool", "boolean":
		if v, ok := value.(bool); ok && v {
			return "TRUE"
		}
		return "FALSE"
	case "github.com/suppers-ai/solobase/internal/pkg/apptime", "timestamp":
		if value == "now" || value == "CURRENT_TIMESTAMP" {
			return "CURRENT_TIMESTAMP"
		}
		return fmt.Sprintf("'%v'", value)
	default:
		return fmt.Sprintf("%v", value)
	}
}

// GetTable retrieves a custom table definition by name
func (s *CustomTablesService) GetTable(tableName string) (*models.CustomTableDefinition, error) {
	ctx := context.Background()
	tableName = models.EnsureCustomPrefix(tableName)

	definition, err := s.repo.GetDefinitionByName(ctx, tableName)
	if err != nil {
		if errors.Is(err, repos.ErrNotFound) {
			return nil, fmt.Errorf("table '%s' not found", models.StripCustomPrefix(tableName))
		}
		return nil, err
	}

	// Only return active tables
	if definition.Status != "active" {
		return nil, fmt.Errorf("table '%s' not found", models.StripCustomPrefix(tableName))
	}

	return definition, nil
}

// ListTables retrieves all custom table definitions
func (s *CustomTablesService) ListTables() ([]models.CustomTableDefinition, error) {
	ctx := context.Background()

	defs, err := s.repo.ListActiveDefinitions(ctx)
	if err != nil {
		return nil, err
	}

	// Convert from pointers to values
	definitions := make([]models.CustomTableDefinition, len(defs))
	for i, def := range defs {
		definitions[i] = *def
	}

	return definitions, nil
}

// AlterTable modifies an existing custom table
func (s *CustomTablesService) AlterTable(tableName string, updates *models.CustomTableDefinition, userID string) error {
	ctx := context.Background()

	// Get existing definition
	existing, err := s.GetTable(tableName)
	if err != nil {
		return err
	}

	// Record the old schema
	oldSchema := s.serializeSchema(existing)

	// Apply changes to the physical table
	if err := s.alterPhysicalTable(ctx, existing, updates); err != nil {
		return fmt.Errorf("failed to alter table: %w", err)
	}

	// Update the definition
	existing.Fields = updates.Fields
	existing.Indexes = updates.Indexes
	existing.Options = updates.Options
	existing.Description = updates.Description
	existing.UpdatedAt = apptime.NowTime()

	if err := s.repo.UpdateDefinition(ctx, existing); err != nil {
		return fmt.Errorf("failed to update table definition: %w", err)
	}

	// Record migration
	nextVersion, _ := s.repo.GetNextVersion(ctx, existing.ID)
	migration := &models.CustomTableMigration{
		TableID:       existing.ID,
		Version:       nextVersion,
		MigrationType: "alter",
		OldSchema:     oldSchema,
		NewSchema:     s.serializeSchema(existing),
		ExecutedBy:    userID,
		ExecutedAt:    apptime.NowTime(),
		Status:        "completed",
	}
	if err := s.repo.CreateMigration(ctx, migration); err != nil {
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return nil
}

// alterPhysicalTable applies changes to the physical database table
func (s *CustomTablesService) alterPhysicalTable(ctx context.Context, existing, updates *models.CustomTableDefinition) error {
	// Compare fields and apply changes
	existingFields := make(map[string]models.CustomTableField)
	for _, field := range existing.Fields {
		existingFields[field.Name] = field
	}

	newFields := make(map[string]models.CustomTableField)
	for _, field := range updates.Fields {
		newFields[field.Name] = field
	}

	// Add new columns
	for name, field := range newFields {
		if _, exists := existingFields[name]; !exists {
			columnDef := s.buildColumnDefinition(field)
			alterSQL := fmt.Sprintf("ALTER TABLE %s ADD COLUMN %s", existing.Name, columnDef)
			if err := s.ddl.AlterTable(ctx, alterSQL); err != nil {
				return fmt.Errorf("failed to add column %s: %w", name, err)
			}
		}
	}

	// Drop removed columns (with safety check)
	for name := range existingFields {
		if _, exists := newFields[name]; !exists {
			// Safety: Don't drop system columns
			if name == "id" || name == "created_at" || name == "updated_at" || name == "deleted_at" {
				continue
			}

			alterSQL := fmt.Sprintf("ALTER TABLE %s DROP COLUMN %s", existing.Name, name)
			if err := s.ddl.AlterTable(ctx, alterSQL); err != nil {
				return fmt.Errorf("failed to drop column %s: %w", name, err)
			}
		}
	}

	return nil
}

// DropTable soft-deletes or permanently removes a custom table
func (s *CustomTablesService) DropTable(tableName string, permanent bool, userID string) error {
	ctx := context.Background()

	// Get existing definition
	existing, err := s.GetTable(tableName)
	if err != nil {
		return err
	}

	// Record migration before making changes
	nextVersion, _ := s.repo.GetNextVersion(ctx, existing.ID)
	migration := &models.CustomTableMigration{
		TableID:       existing.ID,
		Version:       nextVersion,
		MigrationType: "drop",
		OldSchema:     s.serializeSchema(existing),
		ExecutedBy:    userID,
		ExecutedAt:    apptime.NowTime(),
		Status:        "completed",
	}
	if err := s.repo.CreateMigration(ctx, migration); err != nil {
		return fmt.Errorf("failed to record migration: %w", err)
	}

	if permanent {
		// Drop the physical table
		if err := s.ddl.DropTable(ctx, existing.Name); err != nil {
			return fmt.Errorf("failed to drop table: %w", err)
		}

		// Delete the definition
		if err := s.repo.DeleteDefinition(ctx, existing.ID); err != nil {
			return fmt.Errorf("failed to delete table definition: %w", err)
		}
	} else {
		// Soft delete - just mark as archived
		existing.Status = "archived"
		existing.UpdatedAt = apptime.NowTime()
		if err := s.repo.UpdateDefinition(ctx, existing); err != nil {
			return fmt.Errorf("failed to archive table: %w", err)
		}
	}

	return nil
}

// GetTableSchema returns the current schema of a custom table
func (s *CustomTablesService) GetTableSchema(tableName string) (map[string]interface{}, error) {
	ctx := context.Background()

	definition, err := s.GetTable(tableName)
	if err != nil {
		return nil, err
	}

	// Get actual column information from database
	columns, err := s.repo.GetTableColumns(ctx, definition.Name)
	if err != nil {
		return nil, err
	}

	// Convert to map format for API response
	columnsMap := make([]map[string]interface{}, len(columns))
	for i, col := range columns {
		colMap := map[string]interface{}{
			"name":       col.Name,
			"type":       col.Type,
			"nullable":   col.Nullable,
			"has_default": col.HasDefault,
			"is_primary": col.IsPrimary,
		}
		if col.Default != nil {
			colMap["default_value"] = *col.Default
		}
		columnsMap[i] = colMap
	}

	// Note: Row count would require a separate query; omitting for now to avoid direct SQL
	schema := map[string]interface{}{
		"name":        definition.DisplayName,
		"table_name":  definition.Name,
		"description": definition.Description,
		"fields":      definition.Fields,
		"indexes":     definition.Indexes,
		"options":     definition.Options,
		"columns":     columnsMap,
		"created_at":  definition.CreatedAt,
		"created_by":  definition.CreatedBy,
	}

	return schema, nil
}

// serializeSchema converts a table definition to JSON for storage
func (s *CustomTablesService) serializeSchema(definition *models.CustomTableDefinition) json.RawMessage {
	data, _ := json.Marshal(definition)
	return json.RawMessage(data)
}

// getNextVersion gets the next migration version for a table
func (s *CustomTablesService) getNextVersion(tableID uint) int {
	ctx := context.Background()
	version, _ := s.repo.GetNextVersion(ctx, tableID)
	return version
}

// ValidateFieldType checks if a field type is valid
func (s *CustomTablesService) ValidateFieldType(fieldType string) bool {
	validTypes := []string{"string", "int", "float", "bool", "github.com/suppers-ai/solobase/internal/pkg/apptime", "date", "json", "jsonb", "text", "uuid"}
	for _, valid := range validTypes {
		if fieldType == valid {
			return true
		}
	}
	return false
}

// GetMigrationHistory retrieves the migration history for a table
func (s *CustomTablesService) GetMigrationHistory(tableName string) ([]models.CustomTableMigration, error) {
	ctx := context.Background()

	definition, err := s.GetTable(tableName)
	if err != nil {
		return nil, err
	}

	migs, err := s.repo.ListMigrationsByTableID(ctx, definition.ID)
	if err != nil {
		return nil, err
	}

	// Convert from pointers to values
	migrations := make([]models.CustomTableMigration, len(migs))
	for i, m := range migs {
		migrations[i] = *m
	}

	return migrations, nil
}

// GetRepository returns a DynamicRepository for CRUD operations on a custom table.
func (s *CustomTablesService) GetRepository(tableName string) (*coremodels.DynamicRepository, error) {
	definition, err := s.GetTable(tableName)
	if err != nil {
		return nil, err
	}

	return coremodels.NewDynamicRepository(s.db, tableName, definition), nil
}

// GetDB returns the underlying database connection
func (s *CustomTablesService) GetDB() *sql.DB {
	return s.db
}

// GetDBType returns the database type
func (s *CustomTablesService) GetDBType() string {
	return s.dbType
}
