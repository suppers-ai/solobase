package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

// CustomTableDefinition stores metadata about admin-created tables
type CustomTableDefinition struct {
	ID          uint                 `gorm:"primaryKey" json:"id"`
	Name        string               `gorm:"uniqueIndex;not null" json:"name"`        // Actual table name with custom_ prefix
	DisplayName string               `json:"display_name"`                             // User-friendly name without prefix
	Description string               `json:"description"`
	Fields      []CustomTableField   `gorm:"type:jsonb;serializer:json" json:"fields"` // Column definitions
	Indexes     []CustomTableIndex   `gorm:"type:jsonb;serializer:json" json:"indexes"` // Index definitions
	Options     CustomTableOptions   `gorm:"type:jsonb;serializer:json" json:"options"` // Table options
	CreatedBy   string               `json:"created_by"`                              // User ID who created the table
	Status      string               `gorm:"default:'active'" json:"status"`          // active, disabled, archived
	CreatedAt   time.Time            `json:"created_at"`
	UpdatedAt   time.Time            `json:"updated_at"`
}

// TableName specifies the table name
func (CustomTableDefinition) TableName() string {
	return "custom_table_definitions"
}

// CustomTableField defines a column in a custom table
type CustomTableField struct {
	Name          string      `json:"name"`           // Column name
	Type          string      `json:"type"`           // GORM data type: string, int, float, bool, time, json
	Size          int         `json:"size,omitempty"` // For varchar(n)
	Nullable      bool        `json:"nullable"`
	DefaultValue  interface{} `json:"default_value,omitempty"`
	IsPrimaryKey  bool        `json:"is_primary_key"`
	IsUnique      bool        `json:"is_unique"`
	IsIndexed     bool        `json:"is_indexed"`
	AutoIncrement bool        `json:"auto_increment"`
	Description   string      `json:"description,omitempty"`

	// Foreign key support
	ForeignKey    *ForeignKeyDef `json:"foreign_key,omitempty"`

	// Validation rules
	Validation    FieldValidation `json:"validation,omitempty"`
}

// ForeignKeyDef defines a foreign key relationship
type ForeignKeyDef struct {
	ReferenceTable  string `json:"reference_table"`  // Table to reference (with custom_ prefix)
	ReferenceColumn string `json:"reference_column"` // Column in reference table
	OnDelete        string `json:"on_delete"`        // CASCADE, SET NULL, RESTRICT
	OnUpdate        string `json:"on_update"`        // CASCADE, SET NULL, RESTRICT
}

// FieldValidation defines validation rules for a field
type FieldValidation struct {
	MinLength   *int     `json:"min_length,omitempty"`
	MaxLength   *int     `json:"max_length,omitempty"`
	MinValue    *float64 `json:"min_value,omitempty"`
	MaxValue    *float64 `json:"max_value,omitempty"`
	Pattern     string   `json:"pattern,omitempty"`     // Regex pattern
	EnumValues  []string `json:"enum_values,omitempty"` // Allowed values
	Required    bool     `json:"required"`
}

// CustomTableIndex defines an index on a custom table
type CustomTableIndex struct {
	Name    string   `json:"name"`
	Columns []string `json:"columns"`
	Unique  bool     `json:"unique"`
	Type    string   `json:"type,omitempty"` // btree, hash, gin, gist (PostgreSQL specific)
}

// CustomTableOptions defines additional table options
type CustomTableOptions struct {
	SoftDelete     bool `json:"soft_delete"`      // Add deleted_at field
	Timestamps     bool `json:"timestamps"`       // Add created_at, updated_at
	Versioning     bool `json:"versioning"`       // Add version field for optimistic locking
	Auditing       bool `json:"auditing"`         // Track changes in audit log
	CacheEnabled   bool `json:"cache_enabled"`    // Enable query caching
	MaxRows        int  `json:"max_rows,omitempty"` // Maximum allowed rows
}

// CustomTableMigration tracks schema changes to custom tables
type CustomTableMigration struct {
	ID            uint                  `gorm:"primaryKey" json:"id"`
	TableID       uint                  `gorm:"index" json:"table_id"`
	Version       int                   `json:"version"`
	MigrationType string                `json:"migration_type"` // create, alter, drop
	OldSchema     json.RawMessage       `gorm:"type:jsonb" json:"old_schema,omitempty"`
	NewSchema     json.RawMessage       `gorm:"type:jsonb" json:"new_schema"`
	ExecutedBy    string                `json:"executed_by"`
	ExecutedAt    time.Time             `json:"executed_at"`
	RollbackAt    *time.Time            `json:"rollback_at,omitempty"`
	Status        string                `json:"status"` // pending, completed, failed, rolled_back
	ErrorMessage  string                `json:"error_message,omitempty"`
}

// TableName specifies the table name
func (CustomTableMigration) TableName() string {
	return "custom_table_migrations"
}

// CustomTableData represents a generic row in a custom table
// This is used for dynamic CRUD operations
type CustomTableData map[string]interface{}

// Value implements the driver.Valuer interface for database storage
func (c CustomTableData) Value() (driver.Value, error) {
	if c == nil {
		return nil, nil
	}
	return json.Marshal(c)
}

// Scan implements the sql.Scanner interface for database retrieval
func (c *CustomTableData) Scan(value interface{}) error {
	if value == nil {
		*c = make(CustomTableData)
		return nil
	}
	bytes, ok := value.([]byte)
	if !ok {
		return nil
	}
	return json.Unmarshal(bytes, c)
}

// MapGORMType converts our simplified types to GORM/SQL types
func MapGORMType(fieldType string, size int) string {
	switch fieldType {
	case "string":
		if size > 0 && size <= 255 {
			return "varchar"
		}
		return "text"
	case "int":
		return "bigint"
	case "float":
		return "decimal"
	case "bool":
		return "boolean"
	case "time":
		return "timestamp"
	case "date":
		return "date"
	case "json", "jsonb":
		return "jsonb"
	case "uuid":
		return "uuid"
	case "text":
		return "text"
	default:
		return "text"
	}
}

// GetSQLType returns the SQL type definition for a field
func (f CustomTableField) GetSQLType() string {
	baseType := MapGORMType(f.Type, f.Size)

	// Add size specification for varchar
	if baseType == "varchar" && f.Size > 0 {
		baseType = fmt.Sprintf("varchar(%d)", f.Size)
	}

	// Add precision for decimal
	if baseType == "decimal" {
		baseType = "decimal(10,2)" // Default precision
	}

	return baseType
}

// Helper function to ensure custom_ prefix
func EnsureCustomPrefix(name string) string {
	if !strings.HasPrefix(name, "custom_") {
		return "custom_" + name
	}
	return name
}

// Helper function to remove custom_ prefix for display
func StripCustomPrefix(name string) string {
	return strings.TrimPrefix(name, "custom_")
}