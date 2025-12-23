package repos

// This file contains factory function declarations.
// Actual implementations are provided in build-tag specific files:
// - sqlite/factory.go for standard builds
// - wasm/factory.go for WASM builds

// DatabaseType represents the type of database being used
type DatabaseType string

const (
	DatabaseTypeSQLite   DatabaseType = "sqlite"
	DatabaseTypePostgres DatabaseType = "postgres"
	DatabaseTypeWASM     DatabaseType = "wasm"
)

// FactoryConfig contains configuration for creating a RepositoryFactory
type FactoryConfig struct {
	// DatabaseType specifies which database implementation to use
	DatabaseType DatabaseType

	// For SQLite/Postgres: connection string or path
	ConnectionString string

	// For WASM: no additional config needed (uses host functions)
}
