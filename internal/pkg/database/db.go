package database

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"gorm.io/driver/postgres"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
	"gorm.io/gorm/schema"
)

// DB wraps the GORM database connection with additional functionality
type DB struct {
	*gorm.DB
	config *Config
	dbType string
}

// GetConfig returns the database configuration
func (db *DB) GetConfig() *Config {
	return db.config
}

// New creates a new database connection based on config
func New(cfg *Config) (*DB, error) {
	var dialector gorm.Dialector
	dbType := cfg.Type

	switch dbType {
	case "sqlite":
		dialector = createSQLiteDialector(cfg)
	case "postgres", "postgresql":
		dialector = createPostgresDialector(cfg)
	default:
		return nil, fmt.Errorf("unsupported database type: %s", dbType)
	}

	// Configure GORM
	gormConfig := &gorm.Config{
		NamingStrategy: schema.NamingStrategy{
			TablePrefix:   "",   // No prefix, we'll use full names
			SingularTable: true, // Don't pluralize table names
		},
		NowFunc: func() time.Time {
			return time.Now().UTC()
		},
		QueryFields: true, // Select all fields by default
	}

	// Set logger based on environment
	if cfg.Debug {
		gormConfig.Logger = logger.Default.LogMode(logger.Info)
	} else {
		gormConfig.Logger = logger.Default.LogMode(logger.Silent)
	}

	// Open database connection
	gormDB, err := gorm.Open(dialector, gormConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to database: %w", err)
	}

	// Get underlying SQL database for connection pool settings
	sqlDB, err := gormDB.DB()
	if err != nil {
		return nil, fmt.Errorf("failed to get database instance: %w", err)
	}

	// Configure connection pool
	if cfg.MaxOpenConns > 0 {
		sqlDB.SetMaxOpenConns(cfg.MaxOpenConns)
	} else {
		sqlDB.SetMaxOpenConns(100)
	}

	if cfg.MaxIdleConns > 0 {
		sqlDB.SetMaxIdleConns(cfg.MaxIdleConns)
	} else {
		sqlDB.SetMaxIdleConns(10)
	}

	if cfg.ConnMaxLifetime > 0 {
		sqlDB.SetConnMaxLifetime(cfg.ConnMaxLifetime)
	} else {
		sqlDB.SetConnMaxLifetime(time.Hour)
	}

	// Test connection
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := sqlDB.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	db := &DB{
		DB:     gormDB,
		config: cfg,
		dbType: dbType,
	}

	return db, nil
}

// createSQLiteDialector creates a SQLite dialector
func createSQLiteDialector(cfg *Config) gorm.Dialector {
	dsn := cfg.DSN
	if dsn == "" {
		dsn = "file:./.data/database.db?cache=shared&mode=rwc"
	}

	// Ensure .data directory exists for SQLite
	if !strings.Contains(dsn, ":memory:") {
		if err := ensureDataDirectory(dsn); err != nil {
			log.Printf("Warning: failed to create data directory: %v", err)
		}
	}

	// Add pragma statements for better SQLite performance
	if dsn != ":memory:" && !strings.Contains(dsn, "?") {
		dsn += "?_pragma=foreign_keys(1)&_pragma=journal_mode(WAL)"
	} else if dsn != ":memory:" && !strings.Contains(dsn, "_pragma") {
		dsn += "&_pragma=foreign_keys(1)&_pragma=journal_mode(WAL)"
	}

	return sqlite.Open(dsn)
}

// createPostgresDialector creates a PostgreSQL dialector
func createPostgresDialector(cfg *Config) gorm.Dialector {
	dsn := cfg.DSN
	if dsn == "" {
		// Build DSN from individual components
		sslMode := cfg.SSLMode
		if sslMode == "" {
			sslMode = "disable"
		}
		dsn = fmt.Sprintf("host=%s user=%s password=%s dbname=%s port=%d sslmode=%s",
			cfg.Host, cfg.Username, cfg.Password, cfg.Database, cfg.Port, sslMode)
	}

	return postgres.Open(dsn)
}

// Close closes the database connection
func (db *DB) Close() error {
	sqlDB, err := db.DB.DB()
	if err != nil {
		return err
	}
	return sqlDB.Close()
}

// WithContext returns a new DB instance with context
func (db *DB) WithContext(ctx context.Context) *gorm.DB {
	return db.DB.WithContext(ctx)
}

// Transaction executes a function within a database transaction
func (db *DB) Transaction(fn func(*gorm.DB) error) error {
	return db.DB.Transaction(fn)
}

// IsPostgres returns true if using PostgreSQL
func (db *DB) IsPostgres() bool {
	return db.dbType == "postgres" || db.dbType == "postgresql"
}

// IsSQLite returns true if using SQLite
func (db *DB) IsSQLite() bool {
	return db.dbType == "sqlite"
}

// GetType returns the database type
func (db *DB) GetType() string {
	return db.dbType
}

// GetGORM returns the underlying GORM database
// This allows direct access when needed
func (db *DB) GetGORM() *gorm.DB {
	return db.DB
}

// Migrate runs auto-migration for the given models
func (db *DB) Migrate(models ...interface{}) error {
	return db.AutoMigrate(models...)
}

// Helper function to ensure data directory exists
func ensureDataDirectory(dsn string) error {
	// Extract file path from DSN
	filePath := dsn
	if strings.HasPrefix(filePath, "file:") {
		filePath = strings.TrimPrefix(filePath, "file:")
	}
	if idx := strings.Index(filePath, "?"); idx != -1 {
		filePath = filePath[:idx]
	}

	// Get directory from file path
	dir := filepath.Dir(filePath)
	if dir == "." || dir == "" {
		return nil
	}

	// Create directory if it doesn't exist
	return os.MkdirAll(dir, 0755)
}
