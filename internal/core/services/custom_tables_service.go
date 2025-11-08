package services

import (
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
	"strings"

	"github.com/suppers-ai/solobase/internal/data/models"
	"gorm.io/gorm"
)

// CustomTablesService handles dynamic table creation and management
type CustomTablesService struct {
	db *gorm.DB
}

// NewCustomTablesService creates a new custom tables service
func NewCustomTablesService(db *gorm.DB) *CustomTablesService {
	return &CustomTablesService{db: db}
}

// Reserved table names that cannot be used for custom tables
var reservedTableNames = []string{
	"users", "roles", "permissions", "sessions", "settings",
	"custom_table_definitions", "custom_table_migrations",
	"logs", "audit_logs", "migrations",
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
	validName := regexp.MustCompile(`^[a-z][a-z0-9_]*$`)
	if !validName.MatchString(name) {
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
	if s.db.Migrator().HasTable(tableName) {
		return fmt.Errorf("table '%s' already exists", cleanName)
	}

	return nil
}

// CreateTable creates a new custom table based on the definition
func (s *CustomTablesService) CreateTable(definition *models.CustomTableDefinition, userID string) error {
	// Validate table name
	if err := s.ValidateTableName(definition.DisplayName); err != nil {
		return err
	}

	// Ensure custom_ prefix
	definition.Name = models.EnsureCustomPrefix(definition.DisplayName)
	definition.CreatedBy = userID
	definition.Status = "active"

	// Start transaction
	tx := s.db.Begin()
	defer func() {
		if r := recover(); r != nil {
			tx.Rollback()
		}
	}()

	// Save table definition
	if err := tx.Create(definition).Error; err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to save table definition: %w", err)
	}

	// Create the actual table
	if err := s.createPhysicalTable(tx, definition); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to create table: %w", err)
	}

	// Record migration
	migration := &models.CustomTableMigration{
		TableID:       definition.ID,
		Version:       1,
		MigrationType: "create",
		NewSchema:     s.serializeSchema(definition),
		ExecutedBy:    userID,
		Status:        "completed",
	}

	if err := tx.Create(migration).Error; err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return tx.Commit().Error
}

// createPhysicalTable creates the actual database table
func (s *CustomTablesService) createPhysicalTable(tx *gorm.DB, definition *models.CustomTableDefinition) error {
	// Build SQL for table creation
	var columns []string
	var indexes []string

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
		columns = append(columns, "id BIGSERIAL PRIMARY KEY")
	}

	// Add field columns
	for _, field := range definition.Fields {
		columnDef := s.buildColumnDefinition(field)
		columns = append(columns, columnDef)

		// Create index if needed
		if field.IsIndexed && !field.IsPrimaryKey && !field.IsUnique {
			indexName := fmt.Sprintf("idx_%s_%s", definition.Name, field.Name)
			indexes = append(indexes, indexName)
		}
	}

	// Add timestamp fields if enabled
	if definition.Options.Timestamps {
		columns = append(columns, "created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
		columns = append(columns, "updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
	}

	// Add soft delete field if enabled
	if definition.Options.SoftDelete {
		columns = append(columns, "deleted_at TIMESTAMP")
		indexes = append(indexes, fmt.Sprintf("idx_%s_deleted_at", definition.Name))
	}

	// Build and execute CREATE TABLE statement
	createSQL := fmt.Sprintf("CREATE TABLE %s (\n  %s\n)",
		definition.Name,
		strings.Join(columns, ",\n  "))

	if err := tx.Exec(createSQL).Error; err != nil {
		return err
	}

	// Create indexes
	for i, field := range definition.Fields {
		if field.IsIndexed && !field.IsPrimaryKey && !field.IsUnique {
			indexSQL := fmt.Sprintf("CREATE INDEX idx_%s_%s ON %s(%s)",
				definition.Name, field.Name, definition.Name, field.Name)
			if err := tx.Exec(indexSQL).Error; err != nil {
				return fmt.Errorf("failed to create index %d: %w", i, err)
			}
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

		if err := tx.Exec(indexSQL).Error; err != nil {
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
			// Use SERIAL for auto-increment in PostgreSQL
			parts[1] = "BIGSERIAL"
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
	case "time", "timestamp":
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
	tableName = models.EnsureCustomPrefix(tableName)

	var definition models.CustomTableDefinition
	if err := s.db.Where("name = ? AND status = ?", tableName, "active").First(&definition).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("table '%s' not found", models.StripCustomPrefix(tableName))
		}
		return nil, err
	}

	return &definition, nil
}

// ListTables retrieves all custom table definitions
func (s *CustomTablesService) ListTables() ([]models.CustomTableDefinition, error) {
	var definitions []models.CustomTableDefinition
	if err := s.db.Where("status = ?", "active").Find(&definitions).Error; err != nil {
		return nil, err
	}
	return definitions, nil
}

// AlterTable modifies an existing custom table
func (s *CustomTablesService) AlterTable(tableName string, updates *models.CustomTableDefinition, userID string) error {
	// Get existing definition
	existing, err := s.GetTable(tableName)
	if err != nil {
		return err
	}

	// Start transaction
	tx := s.db.Begin()
	defer func() {
		if r := recover(); r != nil {
			tx.Rollback()
		}
	}()

	// Record the old schema
	oldSchema := s.serializeSchema(existing)

	// Apply changes to the physical table
	if err := s.alterPhysicalTable(tx, existing, updates); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to alter table: %w", err)
	}

	// Update the definition
	existing.Fields = updates.Fields
	existing.Indexes = updates.Indexes
	existing.Options = updates.Options
	existing.Description = updates.Description

	if err := tx.Save(existing).Error; err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to update table definition: %w", err)
	}

	// Record migration
	migration := &models.CustomTableMigration{
		TableID:       existing.ID,
		Version:       s.getNextVersion(existing.ID),
		MigrationType: "alter",
		OldSchema:     oldSchema,
		NewSchema:     s.serializeSchema(existing),
		ExecutedBy:    userID,
		Status:        "completed",
	}

	if err := tx.Create(migration).Error; err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return tx.Commit().Error
}

// alterPhysicalTable applies changes to the physical database table
func (s *CustomTablesService) alterPhysicalTable(tx *gorm.DB, existing, updates *models.CustomTableDefinition) error {
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
			if err := tx.Exec(alterSQL).Error; err != nil {
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
			if err := tx.Exec(alterSQL).Error; err != nil {
				return fmt.Errorf("failed to drop column %s: %w", name, err)
			}
		}
	}

	// TODO: Handle column modifications (type changes, constraints, etc.)
	// This is complex and database-specific, so keeping it simple for now

	return nil
}

// DropTable soft-deletes or permanently removes a custom table
func (s *CustomTablesService) DropTable(tableName string, permanent bool, userID string) error {
	// Get existing definition
	existing, err := s.GetTable(tableName)
	if err != nil {
		return err
	}

	// Start transaction
	tx := s.db.Begin()
	defer func() {
		if r := recover(); r != nil {
			tx.Rollback()
		}
	}()

	if permanent {
		// Drop the physical table
		dropSQL := fmt.Sprintf("DROP TABLE IF EXISTS %s CASCADE", existing.Name)
		if err := tx.Exec(dropSQL).Error; err != nil {
			tx.Rollback()
			return fmt.Errorf("failed to drop table: %w", err)
		}

		// Delete the definition
		if err := tx.Delete(existing).Error; err != nil {
			tx.Rollback()
			return fmt.Errorf("failed to delete table definition: %w", err)
		}
	} else {
		// Soft delete - just mark as archived
		existing.Status = "archived"
		if err := tx.Save(existing).Error; err != nil {
			tx.Rollback()
			return fmt.Errorf("failed to archive table: %w", err)
		}
	}

	// Record migration
	migration := &models.CustomTableMigration{
		TableID:       existing.ID,
		Version:       s.getNextVersion(existing.ID),
		MigrationType: "drop",
		OldSchema:     s.serializeSchema(existing),
		ExecutedBy:    userID,
		Status:        "completed",
	}

	if err := tx.Create(migration).Error; err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return tx.Commit().Error
}

// GetTableSchema returns the current schema of a custom table
func (s *CustomTablesService) GetTableSchema(tableName string) (map[string]interface{}, error) {
	definition, err := s.GetTable(tableName)
	if err != nil {
		return nil, err
	}

	// Get actual column information from database
	columns, err := s.getPhysicalColumns(definition.Name)
	if err != nil {
		return nil, err
	}

	// Get row count
	var count int64
	s.db.Table(definition.Name).Count(&count)

	schema := map[string]interface{}{
		"name":         definition.DisplayName,
		"table_name":   definition.Name,
		"description":  definition.Description,
		"fields":       definition.Fields,
		"indexes":      definition.Indexes,
		"options":      definition.Options,
		"columns":      columns,
		"row_count":    count,
		"created_at":   definition.CreatedAt,
		"created_by":   definition.CreatedBy,
	}

	return schema, nil
}

// getPhysicalColumns retrieves actual column information from the database
func (s *CustomTablesService) getPhysicalColumns(tableName string) ([]map[string]interface{}, error) {
	var columns []map[string]interface{}

	// Use database-specific query to get column information
	dbType := s.db.Dialector.Name()

	if dbType == "postgres" {
		query := `
			SELECT
				column_name as name,
				data_type as type,
				is_nullable = 'YES' as nullable,
				column_default IS NOT NULL as has_default,
				column_default as default_value
			FROM information_schema.columns
			WHERE table_name = ?
			ORDER BY ordinal_position
		`
		s.db.Raw(query, tableName).Scan(&columns)
	} else {
		// SQLite
		query := fmt.Sprintf("PRAGMA table_info(%s)", tableName)
		rows, err := s.db.Raw(query).Rows()
		if err != nil {
			return nil, err
		}
		defer rows.Close()

		for rows.Next() {
			var cid int
			var name, dataType string
			var notNull bool
			var defaultValue interface{}
			var pk int

			rows.Scan(&cid, &name, &dataType, &notNull, &defaultValue, &pk)

			columns = append(columns, map[string]interface{}{
				"name":         name,
				"type":         dataType,
				"nullable":     !notNull,
				"has_default":  defaultValue != nil,
				"default_value": defaultValue,
				"is_primary":   pk > 0,
			})
		}
	}

	return columns, nil
}

// serializeSchema converts a table definition to JSON for storage
func (s *CustomTablesService) serializeSchema(definition *models.CustomTableDefinition) json.RawMessage {
	data, _ := json.Marshal(definition)
	return json.RawMessage(data)
}

// getNextVersion gets the next migration version for a table
func (s *CustomTablesService) getNextVersion(tableID uint) int {
	var maxVersion int
	s.db.Model(&models.CustomTableMigration{}).
		Where("table_id = ?", tableID).
		Select("COALESCE(MAX(version), 0)").
		Scan(&maxVersion)
	return maxVersion + 1
}

// ValidateFieldType checks if a field type is valid
func (s *CustomTablesService) ValidateFieldType(fieldType string) bool {
	validTypes := []string{"string", "int", "float", "bool", "time", "date", "json", "jsonb", "text", "uuid"}
	for _, valid := range validTypes {
		if fieldType == valid {
			return true
		}
	}
	return false
}

// GetMigrationHistory retrieves the migration history for a table
func (s *CustomTablesService) GetMigrationHistory(tableName string) ([]models.CustomTableMigration, error) {
	definition, err := s.GetTable(tableName)
	if err != nil {
		return nil, err
	}

	var migrations []models.CustomTableMigration
	if err := s.db.Where("table_id = ?", definition.ID).
		Order("version DESC").
		Find(&migrations).Error; err != nil {
		return nil, err
	}

	return migrations, nil
}