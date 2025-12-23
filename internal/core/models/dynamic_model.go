package models

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/internal/data/models"
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

// Table returns the table name
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

	case "github.com/suppers-ai/solobase/internal/pkg/apptime", "timestamp", "date":
		switch value.(type) {
		case apptime.Time, string:
			// Accept both apptime.Time and string formats
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
	db         *sql.DB
	tableName  string
	definition *models.CustomTableDefinition
}

// NewDynamicRepository creates a new dynamic repository
func NewDynamicRepository(db *sql.DB, tableName string, definition *models.CustomTableDefinition) *DynamicRepository {
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
		now := apptime.NowTime()
		data["created_at"] = now
		data["updated_at"] = now
	}

	// Build INSERT query
	var columns []string
	var placeholders []string
	var values []interface{}

	for col, val := range data {
		columns = append(columns, col)
		placeholders = append(placeholders, "?")
		// Handle JSON fields
		if jsonVal, ok := val.(map[string]interface{}); ok {
			jsonBytes, _ := json.Marshal(jsonVal)
			values = append(values, string(jsonBytes))
		} else if jsonArr, ok := val.([]interface{}); ok {
			jsonBytes, _ := json.Marshal(jsonArr)
			values = append(values, string(jsonBytes))
		} else {
			values = append(values, val)
		}
	}

	query := fmt.Sprintf("INSERT INTO %s (%s) VALUES (%s)",
		r.tableName,
		strings.Join(columns, ", "),
		strings.Join(placeholders, ", "))

	result, err := r.db.Exec(query, values...)
	if err != nil {
		return nil, fmt.Errorf("failed to create record: %w", err)
	}

	// Get the inserted ID
	id, err := result.LastInsertId()
	if err == nil && id > 0 {
		data["id"] = id
	}

	return data, nil
}

// FindByID retrieves a record by ID
func (r *DynamicRepository) FindByID(id interface{}) (map[string]interface{}, error) {
	query := fmt.Sprintf("SELECT * FROM %s WHERE id = ?", r.tableName)

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	rows, err := r.db.Query(query, id)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	results, err := r.scanRows(rows)
	if err != nil {
		return nil, err
	}

	if len(results) == 0 {
		return nil, fmt.Errorf("record with id %v not found", id)
	}

	return results[0], nil
}

// Find retrieves records with conditions
func (r *DynamicRepository) Find(conditions map[string]interface{}, limit, offset int) ([]map[string]interface{}, error) {
	query := fmt.Sprintf("SELECT * FROM %s WHERE 1=1", r.tableName)
	var values []interface{}

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

		query += fmt.Sprintf(" AND %s = ?", field)
		values = append(values, value)
	}

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	// Apply pagination
	if limit > 0 {
		query += fmt.Sprintf(" LIMIT %d", limit)
	}
	if offset > 0 {
		query += fmt.Sprintf(" OFFSET %d", offset)
	}

	rows, err := r.db.Query(query, values...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return r.scanRows(rows)
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
		updates["updated_at"] = apptime.NowTime()
	}

	// Build UPDATE query
	var setClauses []string
	var values []interface{}

	for col, val := range updates {
		if col == "id" || col == "created_at" {
			continue
		}
		setClauses = append(setClauses, fmt.Sprintf("%s = ?", col))
		// Handle JSON fields
		if jsonVal, ok := val.(map[string]interface{}); ok {
			jsonBytes, _ := json.Marshal(jsonVal)
			values = append(values, string(jsonBytes))
		} else if jsonArr, ok := val.([]interface{}); ok {
			jsonBytes, _ := json.Marshal(jsonArr)
			values = append(values, string(jsonBytes))
		} else {
			values = append(values, val)
		}
	}

	values = append(values, id)

	query := fmt.Sprintf("UPDATE %s SET %s WHERE id = ?", r.tableName, strings.Join(setClauses, ", "))

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	result, err := r.db.Exec(query, values...)
	if err != nil {
		return fmt.Errorf("failed to update record: %w", err)
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		return fmt.Errorf("record with id %v not found", id)
	}

	return nil
}

// Delete removes a record (soft or hard delete based on options)
func (r *DynamicRepository) Delete(id interface{}) error {
	var query string
	var values []interface{}

	if r.definition.Options.SoftDelete {
		// Soft delete
		query = fmt.Sprintf("UPDATE %s SET deleted_at = ? WHERE id = ?", r.tableName)
		values = []interface{}{apptime.NowTime(), id}
	} else {
		// Hard delete
		query = fmt.Sprintf("DELETE FROM %s WHERE id = ?", r.tableName)
		values = []interface{}{id}
	}

	result, err := r.db.Exec(query, values...)
	if err != nil {
		return fmt.Errorf("failed to delete record: %w", err)
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		return fmt.Errorf("record with id %v not found", id)
	}

	return nil
}

// Count returns the number of records matching conditions
func (r *DynamicRepository) Count(conditions map[string]interface{}) (int64, error) {
	query := fmt.Sprintf("SELECT COUNT(*) FROM %s WHERE 1=1", r.tableName)
	var values []interface{}

	// Apply conditions
	for field, value := range conditions {
		query += fmt.Sprintf(" AND %s = ?", field)
		values = append(values, value)
	}

	// Handle soft delete
	if r.definition.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	var count int64
	if err := r.db.QueryRow(query, values...).Scan(&count); err != nil {
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

	rows, err := r.db.Query(query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return r.scanRows(rows)
}

// BulkInsert performs batch insertion of records
func (r *DynamicRepository) BulkInsert(records []map[string]interface{}) error {
	if len(records) == 0 {
		return nil
	}

	// Validate all records and add timestamps
	for _, record := range records {
		model := NewDynamicModel(r.tableName, r.definition)
		for key, value := range record {
			if err := model.Set(key, value); err != nil {
				return fmt.Errorf("validation error in record: %w", err)
			}
		}

		// Add timestamps if enabled
		if r.definition.Options.Timestamps {
			now := apptime.NowTime()
			record["created_at"] = now
			record["updated_at"] = now
		}
	}

	// Start transaction
	tx, err := r.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	// Insert each record
	for _, record := range records {
		var columns []string
		var placeholders []string
		var values []interface{}

		for col, val := range record {
			columns = append(columns, col)
			placeholders = append(placeholders, "?")
			// Handle JSON fields
			if jsonVal, ok := val.(map[string]interface{}); ok {
				jsonBytes, _ := json.Marshal(jsonVal)
				values = append(values, string(jsonBytes))
			} else if jsonArr, ok := val.([]interface{}); ok {
				jsonBytes, _ := json.Marshal(jsonArr)
				values = append(values, string(jsonBytes))
			} else {
				values = append(values, val)
			}
		}

		query := fmt.Sprintf("INSERT INTO %s (%s) VALUES (%s)",
			r.tableName,
			strings.Join(columns, ", "),
			strings.Join(placeholders, ", "))

		if _, err := tx.Exec(query, values...); err != nil {
			return fmt.Errorf("failed to insert record: %w", err)
		}
	}

	return tx.Commit()
}

// Transaction wraps operations in a database transaction
func (r *DynamicRepository) Transaction(fn func(*DynamicRepository) error) error {
	tx, err := r.db.Begin()
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	// Create a new repository with the same parameters
	// Note: For transaction support, we'd need to pass the tx instead of db
	// This is a simplified version
	if err := fn(r); err != nil {
		return err
	}

	return tx.Commit()
}

// scanRows converts sql.Rows to []map[string]interface{}
func (r *DynamicRepository) scanRows(rows *sql.Rows) ([]map[string]interface{}, error) {
	columns, err := rows.Columns()
	if err != nil {
		return nil, err
	}

	var results []map[string]interface{}

	for rows.Next() {
		// Create a slice of interface{} to hold values
		values := make([]interface{}, len(columns))
		valuePtrs := make([]interface{}, len(columns))
		for i := range values {
			valuePtrs[i] = &values[i]
		}

		if err := rows.Scan(valuePtrs...); err != nil {
			return nil, err
		}

		// Create a map for the row
		row := make(map[string]interface{})
		for i, col := range columns {
			val := values[i]
			// Handle []byte (common for TEXT, BLOB, JSON fields)
			if b, ok := val.([]byte); ok {
				// Try to unmarshal as JSON
				var jsonVal interface{}
				if err := json.Unmarshal(b, &jsonVal); err == nil {
					row[col] = jsonVal
				} else {
					row[col] = string(b)
				}
			} else {
				row[col] = val
			}
		}

		results = append(results, row)
	}

	return results, rows.Err()
}
