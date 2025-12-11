package database

import (
	"time"
)

// Config holds database configuration
type Config struct {
	// Database type: "sqlite", "postgres", "mysql"
	Type string `json:"type" yaml:"type"`

	// DSN (Data Source Name) - if provided, overrides other connection settings
	DSN string `json:"dsn" yaml:"dsn"`

	// Connection details (used if DSN is not provided)
	Host     string `json:"host" yaml:"host"`
	Port     int    `json:"port" yaml:"port"`
	Database string `json:"database" yaml:"database"`
	Username string `json:"username" yaml:"username"`
	Password string `json:"password" yaml:"password"`
	SSLMode  string `json:"sslMode" yaml:"sslMode"`

	// Connection pool settings
	MaxOpenConns    int           `json:"maxOpenConns" yaml:"maxOpenConns"`
	MaxIdleConns    int           `json:"maxIdleConns" yaml:"maxIdleConns"`
	ConnMaxLifetime time.Duration `json:"connMaxLifetime" yaml:"connMaxLifetime"`

	// Debug mode enables query logging
	Debug bool `json:"debug" yaml:"debug"`

	// AutoMigrate runs migrations automatically
	AutoMigrate bool `json:"autoMigrate" yaml:"autoMigrate"`

	// MigrateModels is a list of models to auto-migrate
	MigrateModels []interface{} `json:"-" yaml:"-"`
}

// NewConfig creates a new config with defaults
func NewConfig() *Config {
	return &Config{
		Type:            "sqlite",
		DSN:             "",
		Host:            "localhost",
		Port:            5432,
		Database:        "database",
		Username:        "user",
		Password:        "",
		SSLMode:         "disable",
		MaxOpenConns:    100,
		MaxIdleConns:    10,
		ConnMaxLifetime: time.Hour,
		Debug:           false,
		AutoMigrate:     true,
	}
}

// NewSQLiteConfig creates a config for SQLite
func NewSQLiteConfig(path string) *Config {
	if path == "" {
		path = "./.data/database.db"
	}
	return &Config{
		Type:            "sqlite",
		DSN:             "file:" + path,
		MaxOpenConns:    1, // SQLite doesn't support concurrent writes
		MaxIdleConns:    1,
		ConnMaxLifetime: 0, // No connection timeout for SQLite
		Debug:           false,
		AutoMigrate:     true,
	}
}

// NewPostgresConfig creates a config for PostgreSQL
func NewPostgresConfig(host string, port int, database, username, password string) *Config {
	return &Config{
		Type:            "postgres",
		Host:            host,
		Port:            port,
		Database:        database,
		Username:        username,
		Password:        password,
		SSLMode:         "disable",
		MaxOpenConns:    100,
		MaxIdleConns:    10,
		ConnMaxLifetime: time.Hour,
		Debug:           false,
		AutoMigrate:     true,
	}
}

// NewMemoryConfig creates a config for in-memory SQLite (testing)
func NewMemoryConfig() *Config {
	return &Config{
		Type:            "sqlite",
		DSN:             ":memory:",
		MaxOpenConns:    1,
		MaxIdleConns:    1,
		ConnMaxLifetime: 0,
		Debug:           false,
		AutoMigrate:     true,
	}
}

// WithDebug enables debug mode
func (c *Config) WithDebug(debug bool) *Config {
	c.Debug = debug
	return c
}

// WithAutoMigrate sets auto-migration
func (c *Config) WithAutoMigrate(auto bool, models ...interface{}) *Config {
	c.AutoMigrate = auto
	c.MigrateModels = models
	return c
}

// WithDSN sets the DSN
func (c *Config) WithDSN(dsn string) *Config {
	c.DSN = dsn
	return c
}
