package core

import (
	"context"
	"fmt"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/suppers-ai/database"
	"github.com/suppers-ai/logger"
)

// MigrationRunner handles database migrations for extensions
type MigrationRunner struct {
	db         database.Database
	logger     logger.Logger
	dryRun     bool
	migrations map[string][]Migration // Cache of registered migrations
}

// NewMigrationRunner creates a new migration runner
func NewMigrationRunner(db database.Database, logger logger.Logger) *MigrationRunner {
	return &MigrationRunner{
		db:         db,
		logger:     logger,
		dryRun:     false,
		migrations: make(map[string][]Migration),
	}
}

// InitializeMigrationTable creates the migration tracking table
func (r *MigrationRunner) InitializeMigrationTable(ctx context.Context) error {
	query := `
		CREATE TABLE IF NOT EXISTS ext_migrations (
			id SERIAL PRIMARY KEY,
			extension VARCHAR(255) NOT NULL,
			version VARCHAR(255) NOT NULL,
			description TEXT,
			applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			checksum VARCHAR(64),
			UNIQUE(extension, version)
		)
	`

	_, err := r.db.Exec(ctx, query)
	if err != nil {
		return fmt.Errorf("failed to create migrations table: %w", err)
	}

	return nil
}

// RunMigrations runs all pending migrations for an extension
func (r *MigrationRunner) RunMigrations(ctx context.Context, extension string, migrations []Migration) error {
	// Create extension schema if it doesn't exist
	schemaName := fmt.Sprintf("ext_%s", strings.ToLower(extension))
	if err := r.createSchema(ctx, schemaName); err != nil {
		return fmt.Errorf("failed to create schema: %w", err)
	}

	// Get applied migrations
	applied, err := r.getAppliedMigrations(ctx, extension)
	if err != nil {
		return fmt.Errorf("failed to get applied migrations: %w", err)
	}

	// Sort migrations by version
	sort.Slice(migrations, func(i, j int) bool {
		return migrations[i].Version < migrations[j].Version
	})

	// Run pending migrations
	for _, migration := range migrations {
		if _, exists := applied[migration.Version]; exists {
			r.logger.Debug(ctx, fmt.Sprintf("Migration %s already applied for %s", migration.Version, extension))
			continue
		}

		r.logger.Info(ctx, fmt.Sprintf("Running migration %s for %s: %s", migration.Version, extension, migration.Description))

		if err := r.runMigration(ctx, extension, migration); err != nil {
			return fmt.Errorf("failed to run migration %s: %w", migration.Version, err)
		}
	}

	return nil
}

// RollbackMigrationByVersion rolls back a specific migration by version
func (r *MigrationRunner) RollbackMigration(ctx context.Context, extension string, version string) error {
	// Find the migration
	var migration *Migration
	if migrations, exists := r.migrations[extension]; exists {
		for _, m := range migrations {
			if m.Version == version {
				migration = &m
				break
			}
		}
	}

	if migration == nil {
		return fmt.Errorf("migration %s not found for extension %s", version, extension)
	}

	return r.rollbackMigration(ctx, extension, *migration)
}

// rollbackMigration rolls back a specific migration
func (r *MigrationRunner) rollbackMigration(ctx context.Context, extension string, migration Migration) error {
	// Start transaction
	tx, err := r.db.BeginTx(ctx)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}

	// Execute rollback SQL
	if migration.Down != "" {
		// Set search path to extension schema
		schemaName := fmt.Sprintf("ext_%s", strings.ToLower(extension))
		setPathQuery := fmt.Sprintf("SET search_path TO %s, public", schemaName)
		if _, err := tx.Exec(ctx, setPathQuery); err != nil {
			tx.Rollback()
			return fmt.Errorf("failed to set search path: %w", err)
		}

		if _, err := tx.Exec(ctx, migration.Down); err != nil {
			tx.Rollback()
			return fmt.Errorf("failed to execute rollback: %w", err)
		}
	}

	// Remove migration record
	deleteQuery := `
		DELETE FROM ext_migrations
		WHERE extension = $1 AND version = $2
	`
	if _, err := tx.Exec(ctx, deleteQuery, extension, migration.Version); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to delete migration record: %w", err)
	}

	// Commit transaction
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit rollback: %w", err)
	}

	r.logger.Info(ctx, fmt.Sprintf("Rolled back migration %s for %s", migration.Version, extension))
	return nil
}

// GetMigrationStatuses returns the status of all migrations for an extension
func (r *MigrationRunner) GetMigrationStatuses(ctx context.Context, extension string) ([]MigrationStatus, error) {
	query := `
		SELECT version, description, applied_at, checksum
		FROM ext_migrations
		WHERE extension = $1
		ORDER BY applied_at DESC
	`

	rows, err := r.db.Query(ctx, query, extension)
	if err != nil {
		return nil, fmt.Errorf("failed to query migrations: %w", err)
	}
	defer rows.Close()

	var statuses []MigrationStatus
	for rows.Next() {
		var status MigrationStatus
		err := rows.Scan(&status.Version, &status.Description, &status.AppliedAt, &status.Checksum)
		if err != nil {
			return nil, fmt.Errorf("failed to scan migration status: %w", err)
		}
		status.Extension = extension
		status.Applied = true
		statuses = append(statuses, status)
	}

	return statuses, nil
}

// runMigration executes a single migration
func (r *MigrationRunner) runMigration(ctx context.Context, extension string, migration Migration) error {
	// Start transaction
	tx, err := r.db.BeginTx(ctx)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}

	// Set search path to extension schema
	schemaName := fmt.Sprintf("ext_%s", strings.ToLower(extension))
	setPathQuery := fmt.Sprintf("SET search_path TO %s, public", schemaName)
	if _, err := tx.Exec(ctx, setPathQuery); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to set search path: %w", err)
	}

	// Execute migration SQL
	if _, err := tx.Exec(ctx, migration.Up); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to execute migration: %w", err)
	}

	// Record migration
	insertQuery := `
		INSERT INTO ext_migrations (extension, version, description, checksum)
		VALUES ($1, $2, $3, $4)
	`
	checksum := r.calculateChecksum(migration.Up)
	if _, err := tx.Exec(ctx, insertQuery, extension, migration.Version, migration.Description, checksum); err != nil {
		tx.Rollback()
		return fmt.Errorf("failed to record migration: %w", err)
	}

	// Commit transaction
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit migration: %w", err)
	}

	return nil
}

// createSchema creates the extension schema if it doesn't exist
func (r *MigrationRunner) createSchema(ctx context.Context, schemaName string) error {
	// Validate schema name to prevent SQL injection
	if err := validateExtensionSchemaName(schemaName); err != nil {
		return fmt.Errorf("invalid schema name: %w", err)
	}

	// Quote schema name properly
	quotedSchema := strings.ReplaceAll(schemaName, `"`, `""`)
	query := fmt.Sprintf("CREATE SCHEMA IF NOT EXISTS \"%s\"", quotedSchema)
	_, err := r.db.Exec(ctx, query)
	return err
}

// getAppliedMigrations returns a map of applied migration versions
func (r *MigrationRunner) getAppliedMigrations(ctx context.Context, extension string) (map[string]bool, error) {
	query := `
		SELECT version FROM ext_migrations
		WHERE extension = $1
	`

	rows, err := r.db.Query(ctx, query, extension)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	applied := make(map[string]bool)
	for rows.Next() {
		var version string
		if err := rows.Scan(&version); err != nil {
			return nil, err
		}
		applied[version] = true
	}

	return applied, nil
}

// calculateChecksum calculates a simple checksum for migration SQL
func (r *MigrationRunner) calculateChecksum(sql string) string {
	// Simple checksum implementation - in production use crypto hash
	sum := 0
	for _, ch := range sql {
		sum += int(ch)
	}
	return fmt.Sprintf("%x", sum)
}

// MigrationStatus represents the status of a migration
type MigrationStatus struct {
	Extension   string    `json:"extension"`
	Version     string    `json:"version"`
	Description string    `json:"description"`
	Applied     bool      `json:"applied"`
	AppliedAt   time.Time `json:"applied_at"`
	Checksum    string    `json:"checksum"`
}

// GetMigrationStatus returns a summary status for an extension's migrations
func (r *MigrationRunner) GetMigrationStatus(extension string) *ExtensionMigrationStatus {
	applied := []Migration{}
	pending := []Migration{}

	// Check cached migrations for this extension
	if migrations, exists := r.migrations[extension]; exists {
		// Get applied versions
		ctx := context.Background()
		appliedMap, _ := r.getAppliedMigrations(ctx, extension)

		for _, m := range migrations {
			if appliedMap[m.Version] {
				applied = append(applied, m)
			} else {
				pending = append(pending, m)
			}
		}
	}

	return &ExtensionMigrationStatus{
		Extension:         extension,
		AppliedMigrations: applied,
		PendingMigrations: pending,
		LastMigrationTime: time.Now(), // Would need to query for actual time
	}
}

// ExtensionMigrationStatus represents the migration status for an extension
type ExtensionMigrationStatus struct {
	Extension         string      `json:"extension"`
	AppliedMigrations []Migration `json:"applied_migrations"`
	PendingMigrations []Migration `json:"pending_migrations"`
	LastMigrationTime time.Time   `json:"last_migration_time"`
}

// RegisterMigrations registers migrations for an extension
func (r *MigrationRunner) RegisterMigrations(extension string, migrations []Migration) {
	r.migrations[extension] = migrations
}

// CleanupExtensionData removes all data for an extension
func (r *MigrationRunner) CleanupExtensionData(ctx context.Context, extension string) error {
	// Drop extension schema
	schemaName := fmt.Sprintf("ext_%s", strings.ToLower(extension))

	// Validate schema name to prevent SQL injection
	if err := validateExtensionSchemaName(schemaName); err != nil {
		return fmt.Errorf("invalid schema name: %w", err)
	}

	// Quote schema name properly
	quotedSchema := strings.ReplaceAll(schemaName, `"`, `""`)
	dropQuery := fmt.Sprintf("DROP SCHEMA IF EXISTS \"%s\" CASCADE", quotedSchema)

	if _, err := r.db.Exec(ctx, dropQuery); err != nil {
		return fmt.Errorf("failed to drop schema: %w", err)
	}

	// Remove migration records
	deleteQuery := `DELETE FROM ext_migrations WHERE extension = $1`
	if _, err := r.db.Exec(ctx, deleteQuery, extension); err != nil {
		return fmt.Errorf("failed to delete migration records: %w", err)
	}

	r.logger.Info(ctx, fmt.Sprintf("Cleaned up all data for extension %s", extension))
	return nil
}

// validateExtensionSchemaName validates that an extension schema name is safe
func validateExtensionSchemaName(schemaName string) error {
	// Extension schemas must start with "ext_" and contain only valid characters
	if !strings.HasPrefix(schemaName, "ext_") {
		return fmt.Errorf("extension schema must start with 'ext_'")
	}

	// Check length
	if len(schemaName) > 63 {
		return fmt.Errorf("schema name too long (max 63 characters)")
	}

	// Check pattern - only allow alphanumeric and underscore
	validPattern := regexp.MustCompile(`^ext_[a-z0-9_]+$`)
	if !validPattern.MatchString(schemaName) {
		return fmt.Errorf("invalid schema name format: must be ext_ followed by lowercase letters, numbers, and underscores")
	}

	return nil
}
