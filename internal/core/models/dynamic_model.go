package models

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/suppers-ai/solobase/internal/data/models"
	"gorm.io/gorm"
)

// DynamicModel represents a runtime model for custom tables
type DynamicModel struct {
	TableName  string
	Definition *models.CustomTableDefinition
	Data       map[string]interface{}
}

// NewDynamicModel creates a new dynamic model instance
func NewDynamicModel(tableName string, definition *models.CustomTableDefinition) *DynamicModel {
	return &DynamicModel{
		TableName:  models.EnsureCustomPrefix(tableName),
		Definition: definition,
		Data:       make(map[string]interface{}),
	}
}

// TableName implements the gorm.Tabler interface
func (m *DynamicModel) Table() string {
	return m.TableName
}

// Set sets a field value
func (m *DynamicModel) Set(field string, value interface{}) error {
	// Check if field exists in definition
	found := false
	for _, f := range m.Definition.Fields {
		if f.Name == field {
			found = true
			// Validate type
			if err := m.validateFieldType(f, value); err != nil {
				return err
			}
			break
		}
	}

	if !found && field != "id" && field != "created_at" && field != "updated_at" && field != "deleted_at" {
		return fmt.Errorf("field '%s' not found in table definition", field)
	}

	m.Data[field] = value
	return nil
}

// Get retrieves a field value
func (m *DynamicModel) Get(field string) (interface{}, error) {
	value, exists := m.Data[field]
	if !exists {
		return nil, fmt.Errorf("field '%s' not found", field)
	}
	return value, nil
}

// validateFieldType validates that a value matches the expected field type
func (m *DynamicModel) validateFieldType(field models.CustomTableField, value interface{}) error {
	if value == nil {
		if !field.Nullable && field.DefaultValue == nil {
			return fmt.Errorf("field '%s' cannot be null", field.Name)
		}
		return nil
	}

	switch field.Type {
	case "string", "text", "varchar":
		if _, ok := value.(string); !ok {
			return fmt.Errorf("field '%s' expects string, got %T", field.Name, value)
		}
		// Check length constraints
		if field.Validation.MaxLength != nil {
			if len(value.(string)) > *field.Validation.MaxLength {
				return fmt.Errorf("field '%s' exceeds maximum length of %d", field.Name, *field.Validation.MaxLength)
			}
		}

	case "int":
		switch v := value.(type) {
		case int, int32, int64, float64:
			// Check range constraints
			if field.Validation.MinValue != nil || field.Validation.MaxValue != nil {
				var numVal float64
				switch n := v.(type) {
				case int:
					numVal = float64(n)
				case int32:
					numVal = float64(n)
				case int64:
					numVal = float64(n)
				case float64:
					numVal = n
				}

				if field.Validation.MinValue != nil && numVal < *field.Validation.MinValue {
					return fmt.Errorf("field '%s' below minimum value of %f", field.Name, *field.Validation.MinValue)
				}
				if field.Validation.MaxValue != nil && numVal > *field.Validation.MaxValue {
					return fmt.Errorf("field '%s' exceeds maximum value of %f", field.Name, *field.Validation.MaxValue)
				}
			}
		default:
			return fmt.Errorf("field '%s' expects integer, got %T", field.Name, value)
		}

	case "float", "decimal":
		switch v := value.(type) {
		case float32, float64, int:
			// Check range constraints
			var numVal float64
			switch n := v.(type) {
			case float32:
				numVal = float64(n)
			case float64:
				numVal = n
			case int:
				numVal = float64(n)
			}

			if field.Validation.MinValue != nil && numVal < *field.Validation.MinValue {
				return fmt.Errorf("field '%s' below minimum value of %f", field.Name, *field.Validation.MinValue)
			}
			if field.Validation.MaxValue != nil && numVal > *field.Validation.MaxValue {
				return fmt.Errorf("field '%s' exceeds maximum value of %f", field.Name, *field.Validation.MaxValue)
			}
		default:
			return fmt.Errorf("field '%s' expects float, got %T", field.Name, value)
		}

	case "bool", "boolean":
		if _, ok := value.(bool); !ok {
			return fmt.Errorf("field '%s' expects boolean, got %T", field.Name, value)
		}

	case "time", "timestamp", "date":
		switch value.(type) {
		case time.Time, string:
			// Accept both time.Time and string formats
		default:
			return fmt.Errorf("field '%s' expects time/date, got %T", field.Name, value)
		}

	case "json", "jsonb":
		// Accept any type that can be marshaled to JSON
		if _, err := json.Marshal(value); err != nil {
			return fmt.Errorf("field '%s' value cannot be marshaled to JSON: %w", field.Name, err)
		}

	case "uuid":
		if str, ok := value.(string); ok {
			// Basic UUID validation
			if len(str) != 36 {
				return fmt.Errorf("field '%s' invalid UUID format", field.Name)
			}
		} else {
			return fmt.Errorf("field '%s' expects UUID string, got %T", field.Name, value)
		}
	}

	// Check enum constraints
	if len(field.Validation.EnumValues) > 0 {
		strVal := fmt.Sprintf("%v", value)
		found := false
		for _, enum := range field.Validation.EnumValues {
			if strVal == enum {
				found = true
				break
			}
		}
		if !found {
			return fmt.Errorf("field '%s' value must be one of: %v", field.Name, field.Validation.EnumValues)
		}
	}

	return nil
}

// DynamicRepository provides CRUD operations for dynamic models
type DynamicRepository struct {
	db         *gorm.DB
	tableName  string
	definition *models.CustomTableDefinition
}

// NewDynamicRepository creates a new dynamic repository
func NewDynamicRepository(db *gorm.DB, tableName string, definition *models.CustomTableDefinition) *DynamicRepository {
	return &DynamicRepository{
		db:         db,
		tableName:  models.EnsureCustomPrefix(tableName),
		definition: definition,
	}
}

// Create inserts a new record
func (r *DynamicRepository) Create(data map[string]interface{}) (map[string]interface{}, error) {
	// Validate all fields
	model := NewDynamicModel(r.tableName, r.definition)
	for key, value := range data {
		if err := model.Set(key, value); err != nil {
			return nil, err
		}
	}

	// Add timestamps if enabled
	if r.definition.Options.Timestamps {
		now := time.Now()
		data["created_at"] = now
		data["updated_at"] = now
	}

	// Insert into database
	if err := r.db.Table(r.tableName).Create(&data).Error; err != nil {
		return nil, fmt.Errorf("failed to create record: %w", err)
	}

	return data, nil
}

// FindByID retrieves a record by ID
func (r *DynamicRepository) FindByID(id interface{}) (map[string]interface{}, error) {
	var result map[string]interface{}

	query := r.db.Table(r.tableName).Where("id = ?", id)

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query = query.Where("deleted_at IS NULL")
	}

	if err := query.First(&result).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, fmt.Errorf("record with id %v not found", id)
		}
		return nil, err
	}

	return result, nil
}

// Find retrieves records with conditions
func (r *DynamicRepository) Find(conditions map[string]interface{}, limit, offset int) ([]map[string]interface{}, error) {
	var results []map[string]interface{}

	query := r.db.Table(r.tableName)

	// Apply conditions
	for field, value := range conditions {
		// Validate field exists
		fieldExists := false
		for _, f := range r.definition.Fields {
			if f.Name == field {
				fieldExists = true
				break
			}
		}
		if !fieldExists && field != "id" {
			return nil, fmt.Errorf("field '%s' does not exist", field)
		}

		query = query.Where(fmt.Sprintf("%s = ?", field), value)
	}

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query = query.Where("deleted_at IS NULL")
	}

	// Apply pagination
	if limit > 0 {
		query = query.Limit(limit)
	}
	if offset > 0 {
		query = query.Offset(offset)
	}

	if err := query.Find(&results).Error; err != nil {
		return nil, err
	}

	return results, nil
}

// Update modifies an existing record
func (r *DynamicRepository) Update(id interface{}, updates map[string]interface{}) error {
	// Validate fields
	model := NewDynamicModel(r.tableName, r.definition)
	for key, value := range updates {
		if key == "id" || key == "created_at" {
			continue // Skip system fields
		}
		if err := model.Set(key, value); err != nil {
			return err
		}
	}

	// Add updated timestamp
	if r.definition.Options.Timestamps {
		updates["updated_at"] = time.Now()
	}

	// Update in database
	query := r.db.Table(r.tableName).Where("id = ?", id)

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query = query.Where("deleted_at IS NULL")
	}

	result := query.Updates(updates)
	if result.Error != nil {
		return fmt.Errorf("failed to update record: %w", result.Error)
	}

	if result.RowsAffected == 0 {
		return fmt.Errorf("record with id %v not found", id)
	}

	return nil
}

// Delete removes a record (soft or hard delete based on options)
func (r *DynamicRepository) Delete(id interface{}) error {
	query := r.db.Table(r.tableName).Where("id = ?", id)

	if r.definition.Options.SoftDelete {
		// Soft delete
		updates := map[string]interface{}{
			"deleted_at": time.Now(),
		}
		result := query.Updates(updates)
		if result.Error != nil {
			return fmt.Errorf("failed to delete record: %w", result.Error)
		}
		if result.RowsAffected == 0 {
			return fmt.Errorf("record with id %v not found", id)
		}
	} else {
		// Hard delete
		result := query.Delete(nil)
		if result.Error != nil {
			return fmt.Errorf("failed to delete record: %w", result.Error)
		}
		if result.RowsAffected == 0 {
			return fmt.Errorf("record with id %v not found", id)
		}
	}

	return nil
}

// Count returns the number of records matching conditions
func (r *DynamicRepository) Count(conditions map[string]interface{}) (int64, error) {
	var count int64

	query := r.db.Table(r.tableName)

	// Apply conditions
	for field, value := range conditions {
		query = query.Where(fmt.Sprintf("%s = ?", field), value)
	}

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query = query.Where("deleted_at IS NULL")
	}

	if err := query.Count(&count).Error; err != nil {
		return 0, err
	}

	return count, nil
}

// ExecuteRawQuery allows executing raw SQL queries (with caution)
func (r *DynamicRepository) ExecuteRawQuery(query string, args ...interface{}) ([]map[string]interface{}, error) {
	// Ensure query is SELECT only (for safety)
	queryUpper := strings.ToUpper(strings.TrimSpace(query))
	if !strings.HasPrefix(queryUpper, "SELECT") {
		return nil, errors.New("only SELECT queries are allowed")
	}

	// Ensure the query targets the correct table
	if !strings.Contains(query, r.tableName) {
		return nil, fmt.Errorf("query must target table %s", r.tableName)
	}

	var results []map[string]interface{}
	if err := r.db.Raw(query, args...).Scan(&results).Error; err != nil {
		return nil, err
	}

	return results, nil
}

// BulkInsert performs batch insertion of records
func (r *DynamicRepository) BulkInsert(records []map[string]interface{}) error {
	if len(records) == 0 {
		return nil
	}

	// Validate all records
	for _, record := range records {
		model := NewDynamicModel(r.tableName, r.definition)
		for key, value := range record {
			if err := model.Set(key, value); err != nil {
				return fmt.Errorf("validation error in record: %w", err)
			}
		}

		// Add timestamps if enabled
		if r.definition.Options.Timestamps {
			now := time.Now()
			record["created_at"] = now
			record["updated_at"] = now
		}
	}

	// Perform bulk insert
	return r.db.Table(r.tableName).CreateInBatches(records, 100).Error
}

// Transaction wraps operations in a database transaction
func (r *DynamicRepository) Transaction(fn func(*DynamicRepository) error) error {
	return r.db.Transaction(func(tx *gorm.DB) error {
		// Create a new repository with the transaction
		txRepo := &DynamicRepository{
			db:         tx,
			tableName:  r.tableName,
			definition: r.definition,
		}
		return fn(txRepo)
	})
}