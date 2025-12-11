package services

import (
	"log"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/database"
)

type DatabaseService struct {
	db *database.DB
}

func NewDatabaseService(db *database.DB) *DatabaseService {
	log.Printf("DatabaseService initialized with type: %s", db.GetConfig().Type)
	return &DatabaseService{db: db}
}

func (s *DatabaseService) GetTables() ([]interface{}, error) {
	var tables []interface{}

	// Detect database type
	dbType := strings.ToLower(s.db.GetConfig().Type)
	log.Printf("Database type detected: %s", dbType)

	var query string
	if dbType == "postgres" || dbType == "postgresql" {
		// PostgreSQL query
		query = `
			SELECT 
				table_name as name,
				table_schema as schema,
				'table' as type
			FROM information_schema.tables
			WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
			ORDER BY table_schema, table_name
		`

		rows, err := s.db.Raw(query).Rows()
		if err != nil {
			return nil, err
		}
		defer rows.Close()

		for rows.Next() {
			var name, schema, tableType string
			if err := rows.Scan(&name, &schema, &tableType); err != nil {
				continue
			}

			tables = append(tables, map[string]interface{}{
				"name":   name,
				"schema": schema,
				"type":   tableType,
			})
		}
	} else {
		// SQLite query
		query = `
			SELECT 
				name,
				type
			FROM sqlite_master
			WHERE type IN ('table', 'view')
			AND name NOT LIKE 'sqlite_%'
			ORDER BY name
		`

		rows, err := s.db.Raw(query).Rows()
		if err != nil {
			return nil, err
		}
		defer rows.Close()

		for rows.Next() {
			var name, tableType string
			if err := rows.Scan(&name, &tableType); err != nil {
				continue
			}

			// Get row count for each table using safe table reference
			var count int64
			if err := s.db.Table(name).Count(&count).Error; err != nil {
				log.Printf("Error counting rows in table %s: %v", name, err)
				count = 0
			}

			tables = append(tables, map[string]interface{}{
				"name":       name,
				"schema":     "main",
				"type":       tableType,
				"rows_count": count,
			})
		}
	}

	return tables, nil
}

// GetTotalRowCount returns the total number of rows across all user tables
func (s *DatabaseService) GetTotalRowCount() (int64, error) {
	var totalRows int64

	tables, err := s.GetTables()
	if err != nil {
		return 0, err
	}

	for _, table := range tables {
		tableMap, ok := table.(map[string]interface{})
		if !ok {
			continue
		}

		tableName, ok := tableMap["name"].(string)
		if !ok {
			continue
		}

		// Skip system tables
		if strings.HasPrefix(tableName, "pg_") || strings.HasPrefix(tableName, "sqlite_") {
			continue
		}

		var count int64
		if err := s.db.Table(tableName).Count(&count).Error; err != nil {
			log.Printf("Error counting rows in table %s: %v", tableName, err)
			continue
		}

		totalRows += count
	}

	return totalRows, nil
}

func (s *DatabaseService) GetTableColumns(tableName string) ([]interface{}, error) {
	var columns []interface{}

	// Detect database type
	dbType := strings.ToLower(s.db.GetConfig().Type)

	if dbType == "postgres" || dbType == "postgresql" {
		// PostgreSQL query
		query := `
			SELECT 
				column_name as name,
				data_type as type,
				is_nullable = 'YES' as nullable,
				column_default IS NOT NULL as has_default
			FROM information_schema.columns
			WHERE table_name = ?
			ORDER BY ordinal_position
		`

		rows, err := s.db.Raw(query, tableName).Rows()
		if err != nil {
			return nil, err
		}
		defer rows.Close()

		for rows.Next() {
			var name, dataType string
			var nullable, hasDefault bool

			if err := rows.Scan(&name, &dataType, &nullable, &hasDefault); err != nil {
				continue
			}

			columns = append(columns, map[string]interface{}{
				"name":        name,
				"type":        dataType,
				"nullable":    nullable,
				"has_default": hasDefault,
			})
		}
	} else {
		// SQLite query using PRAGMA
		query := "PRAGMA table_info(" + tableName + ")"

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

			if err := rows.Scan(&cid, &name, &dataType, &notNull, &defaultValue, &pk); err != nil {
				continue
			}

			columns = append(columns, map[string]interface{}{
				"name":        name,
				"type":        dataType,
				"nullable":    !notNull,
				"has_default": defaultValue != nil,
				"is_primary":  pk > 0,
			})
		}
	}

	return columns, nil
}

func (s *DatabaseService) GetDatabaseInfo() (string, string) {
	dbType := strings.ToLower(s.db.GetConfig().Type)

	var displayType, version string
	switch dbType {
	case "postgres", "postgresql":
		displayType = "PostgreSQL"
		version = "14.5"
	case "sqlite", "sqlite3":
		displayType = "SQLite"
		version = "3.x"
	default:
		displayType = "Unknown"
		version = "N/A"
	}

	return displayType, version
}

func (s *DatabaseService) ExecuteQuery(query string) (interface{}, error) {
	// WARNING: This is for development/admin use only
	// In production, this should be heavily restricted or disabled

	// Basic SQL injection prevention - check for dangerous patterns
	// This is NOT comprehensive security - just basic protection
	dangerousPatterns := []string{
		"DROP DATABASE",
		"DROP SCHEMA",
		"TRUNCATE",
		"; DROP",
		"; DELETE FROM",
		"; TRUNCATE",
	}

	queryUpper := strings.ToUpper(query)
	for _, pattern := range dangerousPatterns {
		if strings.Contains(queryUpper, pattern) {
			return map[string]interface{}{
				"error": "Query contains potentially dangerous operations",
			}, nil
		}
	}

	// Determine if this is a SELECT query
	isSelect := strings.HasPrefix(strings.TrimSpace(queryUpper), "SELECT")

	if isSelect {
		// Handle SELECT queries
		rows, err := s.db.Raw(query).Rows()
		if err != nil {
			return map[string]interface{}{
				"error": err.Error(),
			}, nil
		}
		defer rows.Close()

		// Get column names
		columns, err := rows.Columns()
		if err != nil {
			return map[string]interface{}{
				"error": err.Error(),
			}, nil
		}

		// Prepare result
		var result [][]interface{}

		for rows.Next() {
			// Create a slice of interface{} to hold column values
			values := make([]interface{}, len(columns))
			valuePtrs := make([]interface{}, len(columns))
			for i := range values {
				valuePtrs[i] = &values[i]
			}

			if err := rows.Scan(valuePtrs...); err != nil {
				continue
			}

			// Convert to proper types
			row := make([]interface{}, len(columns))
			for i, v := range values {
				if v != nil {
					// Handle byte arrays (common in SQLite)
					if b, ok := v.([]byte); ok {
						row[i] = string(b)
					} else {
						row[i] = v
					}
				} else {
					row[i] = nil
				}
			}

			result = append(result, row)
		}

		return map[string]interface{}{
			"columns": columns,
			"rows":    result,
		}, nil
	} else {
		// Handle INSERT/UPDATE/DELETE queries
		result := s.db.DB.Exec(query)
		if result.Error != nil {
			return map[string]interface{}{
				"error": result.Error.Error(),
			}, nil
		}

		return map[string]interface{}{
			"affected_rows": result.RowsAffected,
		}, nil
	}
}
