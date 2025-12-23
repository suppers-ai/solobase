package utils

import (
	"fmt"
	"strings"
)

// SQL identifier validation
var (
	// Reserved PostgreSQL keywords that shouldn't be used as identifiers
	reservedKeywords = map[string]bool{
		"select": true, "insert": true, "update": true, "delete": true,
		"from": true, "where": true, "join": true, "union": true,
		"drop": true, "create": true, "alter": true, "table": true,
		"database": true, "schema": true, "index": true, "view": true,
		"user": true, "role": true, "grant": true, "revoke": true,
	}

	// Allowed schemas for queries (whitelist approach)
	allowedSchemas = map[string]bool{
		"public":      true,
		"auth":        true,
		"storage":     true,
		"collections": true,
		"logger":      true,
	}
)

// isValidSQLIdentifier validates identifier format without regexp
// Pattern: ^[a-zA-Z_][a-zA-Z0-9_]{0,62}$
func isValidSQLIdentifier(identifier string) bool {
	if len(identifier) == 0 {
		return false
	}
	// First character must be letter or underscore
	c := identifier[0]
	if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_') {
		return false
	}
	// Rest can be letters, digits, or underscores
	for i := 1; i < len(identifier); i++ {
		c := identifier[i]
		if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// ValidateSQLIdentifier validates a SQL identifier (table/column/schema name)
func ValidateSQLIdentifier(identifier string) error {
	if identifier == "" {
		return fmt.Errorf("identifier cannot be empty")
	}

	// Check length
	if len(identifier) > 63 {
		return fmt.Errorf("identifier too long (max 63 characters)")
	}

	// Check pattern (without regexp for TinyGo compatibility)
	if !isValidSQLIdentifier(identifier) {
		return fmt.Errorf("invalid identifier format: must start with letter or underscore, contain only letters, numbers, and underscores")
	}

	// Check reserved keywords
	if reservedKeywords[strings.ToLower(identifier)] {
		return fmt.Errorf("identifier is a reserved SQL keyword: %s", identifier)
	}

	return nil
}

// ValidateSchemaName validates and checks if a schema is allowed
func ValidateSchemaName(schema string) error {
	if err := ValidateSQLIdentifier(schema); err != nil {
		return fmt.Errorf("invalid schema name: %w", err)
	}

	if !allowedSchemas[schema] {
		return fmt.Errorf("schema not allowed: %s", schema)
	}

	return nil
}

// ValidateTableName validates a table name
func ValidateTableName(table string) error {
	if err := ValidateSQLIdentifier(table); err != nil {
		return fmt.Errorf("invalid table name: %w", err)
	}
	return nil
}

// SafeSchemaTable returns a safely quoted schema.table identifier
func SafeSchemaTable(schema, table string) (string, error) {
	if err := ValidateSchemaName(schema); err != nil {
		return "", err
	}

	if err := ValidateTableName(table); err != nil {
		return "", err
	}

	// Use PostgreSQL identifier quoting with double quotes
	quotedSchema := strings.ReplaceAll(schema, `"`, `""`)
	quotedTable := strings.ReplaceAll(table, `"`, `""`)
	return fmt.Sprintf(`"%s"."%s"`, quotedSchema, quotedTable), nil
}

// QuoteIdentifier safely quotes a SQL identifier
func QuoteIdentifier(identifier string) (string, error) {
	if err := ValidateSQLIdentifier(identifier); err != nil {
		return "", err
	}

	// Use PostgreSQL double-quote escaping
	escaped := strings.ReplaceAll(identifier, `"`, `""`)
	return fmt.Sprintf(`"%s"`, escaped), nil
}

// BuildSafeQuery builds a safe query with validated identifiers
func BuildSafeQuery(queryTemplate string, schema, table string) (string, error) {
	safeTable, err := SafeSchemaTable(schema, table)
	if err != nil {
		return "", err
	}

	return strings.Replace(queryTemplate, "$TABLE", safeTable, -1), nil
}

// ValidateColumns validates a list of column names
func ValidateColumns(columns []string) error {
	if len(columns) == 0 {
		return fmt.Errorf("no columns provided")
	}

	for _, col := range columns {
		if err := ValidateSQLIdentifier(col); err != nil {
			return fmt.Errorf("invalid column '%s': %w", col, err)
		}
	}

	return nil
}

// QuoteColumns safely quotes a list of column names
func QuoteColumns(columns []string) ([]string, error) {
	if err := ValidateColumns(columns); err != nil {
		return nil, err
	}

	quoted := make([]string, len(columns))
	for i, col := range columns {
		quotedCol, err := QuoteIdentifier(col)
		if err != nil {
			return nil, err
		}
		quoted[i] = quotedCol
	}

	return quoted, nil
}
