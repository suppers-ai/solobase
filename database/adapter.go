// Package database provides an adapter to use the github.com/suppers-ai/database package
package database

import (
	"database/sql"
	"fmt"
	"strings"
	"time"

	pkgdb "github.com/suppers-ai/database"
	"gorm.io/gorm"
)

const (
	DatabaseTypePostgres = "postgres"
	DatabaseTypeSQLite   = "sqlite"
)

// GetDatabaseType returns the normalized database type for display
func GetDatabaseType(dbType string) string {
	switch strings.ToLower(dbType) {
	case "postgres", "postgresql":
		return "PostgreSQL"
	case "sqlite", "sqlite3":
		return "SQLite"
	default:
		return "Unknown"
	}
}

// Config wraps the package database config
type Config struct {
	Type     string
	Host     string
	Port     int
	Database string
	Username string
	Password string
	SSLMode  string
}

// DB wraps the package database DB
type DB struct {
	*gorm.DB
	sqlDB  *sql.DB
	Config Config
}

// New creates a new database connection using the package
func New(cfg Config) (*DB, error) {
	// Convert to package config
	pkgConfig := &pkgdb.Config{
		Type:     cfg.Type,
		Host:     cfg.Host,
		Port:     cfg.Port,
		Database: cfg.Database,
		Username: cfg.Username,
		Password: cfg.Password,
		SSLMode:  cfg.SSLMode,
	}

	// For SQLite, set the DSN properly
	if cfg.Type == "sqlite" {
		if cfg.Database != "" {
			// If Database starts with "file:", use it as-is for DSN
			// Otherwise, add "file:" prefix
			if strings.HasPrefix(cfg.Database, "file:") {
				pkgConfig.DSN = cfg.Database
			} else {
				pkgConfig.DSN = "file:" + cfg.Database
			}
		}
	}

	// Create database connection using package
	pkgDB, err := pkgdb.New(pkgConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to create database: %w", err)
	}

	// Get the underlying sql.DB
	sqlDB, err := pkgDB.DB.DB()
	if err != nil {
		return nil, err
	}

	// Add callbacks for metrics tracking
	pkgDB.DB.Callback().Query().Before("gorm:query").Register("metrics:before_query", beforeQuery)
	pkgDB.DB.Callback().Query().After("gorm:query").Register("metrics:after_query", afterQuery)
	pkgDB.DB.Callback().Create().Before("gorm:create").Register("metrics:before_create", beforeQuery)
	pkgDB.DB.Callback().Create().After("gorm:create").Register("metrics:after_query", afterQuery)
	pkgDB.DB.Callback().Update().Before("gorm:update").Register("metrics:before_update", beforeQuery)
	pkgDB.DB.Callback().Update().After("gorm:update").Register("metrics:after_query", afterQuery)
	pkgDB.DB.Callback().Delete().Before("gorm:delete").Register("metrics:before_delete", beforeQuery)
	pkgDB.DB.Callback().Delete().After("gorm:delete").Register("metrics:after_query", afterQuery)

	return &DB{
		DB:     pkgDB.DB,
		sqlDB:  sqlDB,
		Config: cfg,
	}, nil
}

// Close closes the database connection
func (db *DB) Close() error {
	return db.sqlDB.Close()
}

// Migrate runs auto migrations for models
func (db *DB) Migrate() error {
	// This is handled by main.go with AutoMigrate
	return nil
}

// QueryMetrics tracks database query metrics
var QueryMetrics struct {
	TotalQueries int64
	QueryTime    time.Time
}

// RecordDBQueryFunc is a function that can be set to record database query metrics
var RecordDBQueryFunc func(operation string, duration float64, isError bool)

// beforeQuery is called before each database query
func beforeQuery(db *gorm.DB) {
	db.Set("query_start_time", time.Now())
}

// afterQuery is called after each database query
func afterQuery(db *gorm.DB) {
	if startTime, ok := db.Get("query_start_time"); ok {
		if start, ok := startTime.(time.Time); ok {
			duration := time.Since(start)

			// Track metrics
			QueryMetrics.TotalQueries++
			QueryMetrics.QueryTime = time.Now()

			// Get operation type
			operation := "query"
			if db.Statement != nil && db.Statement.Schema != nil {
				operation = strings.ToLower(db.Statement.Schema.Table)
			}

			// Record in metrics collector
			if RecordDBQueryFunc != nil {
				RecordDBQueryFunc(operation, duration.Seconds(), db.Error != nil)
			}

			// Log slow queries (optional)
			if duration > 100*time.Millisecond {
				fmt.Printf("Slow query (%v): %s\n", duration, db.Statement.SQL.String())
			}
		}
	}
}

// GetQueryCount returns the total number of database queries
func GetQueryCount() int64 {
	return QueryMetrics.TotalQueries
}
