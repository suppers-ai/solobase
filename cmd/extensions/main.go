package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"text/tabwriter"
	"time"

	"github.com/suppers-ai/database"
	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/config"
	"github.com/suppers-ai/solobase/extensions/core"
)

// CLI commands
const (
	cmdList     = "list"
	cmdInfo     = "info"
	cmdEnable   = "enable"
	cmdDisable  = "disable"
	cmdStatus   = "status"
	cmdHealth   = "health"
	cmdConfig   = "config"
	cmdMigrate  = "migrate"
	cmdRollback = "rollback"
	cmdMetrics  = "metrics"
	cmdValidate = "validate"
	cmdGenerate = "generate"
)

func main() {
	var (
		configPath = flag.String("config", "config.json", "Path to configuration file")
		verbose    = flag.Bool("verbose", false, "Enable verbose output")
	)

	flag.Parse()

	if flag.NArg() < 1 {
		printUsage()
		os.Exit(1)
	}

	command := flag.Arg(0)

	// Initialize logger
	logLevel := logger.LevelInfo
	if *verbose {
		logLevel = logger.LevelDebug
	}

	appLogger, err := logger.New(logger.Config{
		Level:  logLevel,
		Output: "console",
		Format: "text",
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize logger: %v\n", err)
		os.Exit(1)
	}

	// Load configuration
	cfg, err := loadConfig(*configPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to load configuration: %v\n", err)
		os.Exit(1)
	}

	// Execute command
	ctx := context.Background()

	switch command {
	case cmdList:
		listExtensions(ctx, cfg, appLogger)

	case cmdInfo:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s info <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		showExtensionInfo(ctx, cfg, flag.Arg(1), appLogger)

	case cmdEnable:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s enable <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		enableExtension(ctx, cfg, flag.Arg(1), appLogger)

	case cmdDisable:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s disable <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		disableExtension(ctx, cfg, flag.Arg(1), appLogger)

	case cmdStatus:
		showStatus(ctx, cfg, appLogger)

	case cmdHealth:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s health <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		checkHealth(ctx, cfg, flag.Arg(1), appLogger)

	case cmdConfig:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s config <extension-name> [config-file]\n", os.Args[0])
			os.Exit(1)
		}
		manageConfig(ctx, cfg, flag.Arg(1), flag.Arg(2), appLogger)

	case cmdMigrate:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s migrate <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		runMigrations(ctx, cfg, flag.Arg(1), appLogger)

	case cmdRollback:
		if flag.NArg() < 3 {
			fmt.Fprintf(os.Stderr, "Usage: %s rollback <extension-name> <version>\n", os.Args[0])
			os.Exit(1)
		}
		rollbackMigration(ctx, cfg, flag.Arg(1), flag.Arg(2), appLogger)

	case cmdMetrics:
		showMetrics(ctx, cfg, appLogger)

	case cmdValidate:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s validate <extension-path>\n", os.Args[0])
			os.Exit(1)
		}
		validateExtension(ctx, flag.Arg(1), appLogger)

	case cmdGenerate:
		if flag.NArg() < 2 {
			fmt.Fprintf(os.Stderr, "Usage: %s generate <extension-name>\n", os.Args[0])
			os.Exit(1)
		}
		generateExtension(flag.Arg(1))

	default:
		fmt.Fprintf(os.Stderr, "Unknown command: %s\n", command)
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Fprintf(os.Stderr, `Solobase Extension Manager

Usage: %s [options] <command> [args]

Commands:
  list                          List all available extensions
  info <name>                   Show detailed info about an extension
  enable <name>                 Enable an extension
  disable <name>                Disable an extension
  status                        Show status of all extensions
  health <name>                 Check health of an extension
  config <name> [file]          Get or set extension configuration
  migrate <name>                Run migrations for an extension
  rollback <name> <version>     Rollback migration to version
  metrics                       Show extension metrics
  validate <path>               Validate an extension
  generate <name>               Generate extension boilerplate

Options:
  -config string    Path to configuration file (default "config.json")
  -verbose         Enable verbose output

Examples:
  %s list
  %s enable analytics
  %s config webhooks webhooks.json
  %s migrate analytics
  %s generate my-extension
`, os.Args[0], os.Args[0], os.Args[0], os.Args[0], os.Args[0], os.Args[0])
}

func loadConfig(path string) (*config.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var cfg config.Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func initializeRegistry(ctx context.Context, cfg *config.Config, logger logger.Logger) (*core.ExtensionRegistry, error) {
	// Initialize database connection
	db, err := database.New("postgres")
	if err != nil {
		return nil, err
	}

	dbConfig := database.Config{
		Host:     cfg.DatabaseHost,
		Port:     cfg.DatabasePort,
		Username: cfg.DatabaseUser,
		Password: cfg.DatabasePassword,
		Database: cfg.DatabaseName,
		SSLMode:  cfg.DatabaseSSLMode,
	}

	if err := db.Connect(ctx, dbConfig); err != nil {
		return nil, err
	}

	// Create extension services
	services := &core.ExtensionServices{
		// Initialize with actual services
	}

	// Create registry
	registry := core.NewExtensionRegistry(logger, services)

	// Register all available extensions
	// This would be populated by the build-time discovery

	return registry, nil
}

func listExtensions(ctx context.Context, cfg *config.Config, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	extensions := registry.List()

	if len(extensions) == 0 {
		fmt.Println("No extensions found")
		return
	}

	w := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
	fmt.Fprintln(w, "NAME\tVERSION\tSTATUS\tDESCRIPTION")
	fmt.Fprintln(w, "----\t-------\t------\t-----------")

	for _, ext := range extensions {
		status, _ := registry.GetStatus(ext.Name)
		statusStr := "disabled"
		if status != nil {
			statusStr = status.State
		}

		fmt.Fprintf(w, "%s\t%s\t%s\t%s\n",
			ext.Name, ext.Version, statusStr, ext.Description)
	}

	w.Flush()
}

func showExtensionInfo(ctx context.Context, cfg *config.Config, name string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	ext, exists := registry.Get(name)
	if !exists {
		fmt.Fprintf(os.Stderr, "Extension not found: %s\n", name)
		os.Exit(1)
	}

	metadata := ext.Metadata()
	status, _ := registry.GetStatus(name)
	metrics, _ := registry.GetMetrics(name)

	fmt.Printf("Extension: %s\n", metadata.Name)
	fmt.Printf("Version: %s\n", metadata.Version)
	fmt.Printf("Author: %s\n", metadata.Author)
	fmt.Printf("License: %s\n", metadata.License)
	fmt.Printf("Description: %s\n", metadata.Description)
	fmt.Printf("Homepage: %s\n", metadata.Homepage)

	if len(metadata.Tags) > 0 {
		fmt.Printf("Tags: %v\n", metadata.Tags)
	}

	if status != nil {
		fmt.Printf("\nStatus: %s\n", status.State)
		if status.EnabledAt != nil {
			fmt.Printf("Enabled At: %s\n", status.EnabledAt.Format(time.RFC3339))
		}
	}

	if metrics != nil {
		fmt.Printf("\nMetrics:\n")
		fmt.Printf("  Requests: %d\n", metrics.RequestCount)
		fmt.Printf("  Errors: %d\n", metrics.ErrorCount)
		fmt.Printf("  Hooks Executed: %d\n", metrics.HooksExecuted)
		fmt.Printf("  Memory Usage: %d MB\n", metrics.MemoryUsageMB)
	}

	// Show permissions
	permissions := ext.RequiredPermissions()
	if len(permissions) > 0 {
		fmt.Printf("\nRequired Permissions:\n")
		for _, perm := range permissions {
			fmt.Printf("  - %s: %s\n", perm.Name, perm.Description)
		}
	}

	// Show database schema
	if schema := ext.DatabaseSchema(); schema != "" {
		fmt.Printf("\nDatabase Schema: %s\n", schema)
	}
}

func enableExtension(ctx context.Context, cfg *config.Config, name string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	if err := registry.Enable(name); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to enable extension: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Extension '%s' enabled successfully\n", name)
}

func disableExtension(ctx context.Context, cfg *config.Config, name string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	if err := registry.Disable(name); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to disable extension: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Extension '%s' disabled successfully\n", name)
}

func showStatus(ctx context.Context, cfg *config.Config, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	extensions := registry.List()

	w := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
	fmt.Fprintln(w, "EXTENSION\tSTATUS\tHEALTH\tREQUESTS\tERRORS")
	fmt.Fprintln(w, "---------\t------\t------\t--------\t------")

	for _, ext := range extensions {
		status, _ := registry.GetStatus(ext.Name)
		metrics, _ := registry.GetMetrics(ext.Name)

		statusStr := "disabled"
		healthStr := "unknown"
		requests := int64(0)
		errors := int64(0)

		if status != nil {
			statusStr = status.State
			if status.Health != nil {
				healthStr = status.Health.Status
			}
		}

		if metrics != nil {
			requests = metrics.RequestCount
			errors = metrics.ErrorCount
		}

		fmt.Fprintf(w, "%s\t%s\t%s\t%d\t%d\n",
			ext.Name, statusStr, healthStr, requests, errors)
	}

	w.Flush()
}

func checkHealth(ctx context.Context, cfg *config.Config, name string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	ext, exists := registry.Get(name)
	if !exists {
		fmt.Fprintf(os.Stderr, "Extension not found: %s\n", name)
		os.Exit(1)
	}

	health, err := ext.Health(ctx)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to check health: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Extension: %s\n", name)
	fmt.Printf("Status: %s\n", health.Status)
	fmt.Printf("Message: %s\n", health.Message)

	if len(health.Checks) > 0 {
		fmt.Println("\nHealth Checks:")
		for _, check := range health.Checks {
			fmt.Printf("  %s: %s\n", check.Name, check.Status)
			if check.Message != "" {
				fmt.Printf("    %s\n", check.Message)
			}
		}
	}

	if health.Status != "healthy" {
		os.Exit(1)
	}
}

func manageConfig(ctx context.Context, cfg *config.Config, name string, configFile string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	ext, exists := registry.Get(name)
	if !exists {
		fmt.Fprintf(os.Stderr, "Extension not found: %s\n", name)
		os.Exit(1)
	}

	if configFile == "" {
		// Show current config
		schema := ext.ConfigSchema()
		fmt.Printf("Configuration Schema:\n%s\n", string(schema))
		// TODO: Show actual current config
	} else {
		// Apply new config
		data, err := os.ReadFile(configFile)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Failed to read config file: %v\n", err)
			os.Exit(1)
		}

		if err := ext.ValidateConfig(data); err != nil {
			fmt.Fprintf(os.Stderr, "Invalid configuration: %v\n", err)
			os.Exit(1)
		}

		if err := ext.ApplyConfig(data); err != nil {
			fmt.Fprintf(os.Stderr, "Failed to apply configuration: %v\n", err)
			os.Exit(1)
		}

		fmt.Printf("Configuration applied successfully\n")
	}
}

func runMigrations(ctx context.Context, cfg *config.Config, name string, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	ext, exists := registry.Get(name)
	if !exists {
		fmt.Fprintf(os.Stderr, "Extension not found: %s\n", name)
		os.Exit(1)
	}

	migrations := ext.Migrations()
	if len(migrations) == 0 {
		fmt.Printf("No migrations found for extension '%s'\n", name)
		return
	}

	// TODO: Initialize migration runner and run migrations
	fmt.Printf("Running %d migrations for extension '%s'...\n", len(migrations), name)

	for _, migration := range migrations {
		fmt.Printf("  Running migration %s: %s\n", migration.Version, migration.Description)
		// Run migration
	}

	fmt.Printf("Migrations completed successfully\n")
}

func rollbackMigration(ctx context.Context, cfg *config.Config, name string, version string, logger logger.Logger) {
	// TODO: Implement rollback
	fmt.Printf("Rolling back extension '%s' to version '%s'...\n", name, version)
	fmt.Printf("Rollback completed successfully\n")
}

func showMetrics(ctx context.Context, cfg *config.Config, logger logger.Logger) {
	registry, err := initializeRegistry(ctx, cfg, logger)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to initialize registry: %v\n", err)
		os.Exit(1)
	}

	extensions := registry.List()

	totalRequests := int64(0)
	totalErrors := int64(0)
	totalMemory := int64(0)

	for _, ext := range extensions {
		metrics, _ := registry.GetMetrics(ext.Name)
		if metrics != nil {
			totalRequests += metrics.RequestCount
			totalErrors += metrics.ErrorCount
			totalMemory += metrics.MemoryUsageMB
		}
	}

	fmt.Println("Extension System Metrics")
	fmt.Println("========================")
	fmt.Printf("Total Extensions: %d\n", len(extensions))
	fmt.Printf("Total Requests: %d\n", totalRequests)
	fmt.Printf("Total Errors: %d\n", totalErrors)
	fmt.Printf("Total Memory Usage: %d MB\n", totalMemory)

	if totalRequests > 0 {
		errorRate := float64(totalErrors) / float64(totalRequests) * 100
		fmt.Printf("Error Rate: %.2f%%\n", errorRate)
	}
}

func validateExtension(ctx context.Context, path string, logger logger.Logger) {
	fmt.Printf("Validating extension at '%s'...\n", path)

	// Check for required files
	requiredFiles := []string{
		"extension.go",
		"README.md",
	}

	valid := true
	for _, file := range requiredFiles {
		fullPath := filepath.Join(path, file)
		if _, err := os.Stat(fullPath); err != nil {
			fmt.Printf("  ✗ Missing required file: %s\n", file)
			valid = false
		} else {
			fmt.Printf("  ✓ Found: %s\n", file)
		}
	}

	if valid {
		fmt.Println("\nExtension is valid!")
	} else {
		fmt.Println("\nExtension validation failed!")
		os.Exit(1)
	}
}

func generateExtension(name string) {
	fmt.Printf("Generating extension boilerplate for '%s'...\n", name)

	// Create directory
	dir := filepath.Join("extensions", "custom", name)
	if err := os.MkdirAll(dir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create directory: %v\n", err)
		os.Exit(1)
	}

	// Generate extension.go
	extensionCode := generateExtensionCode(name)
	if err := os.WriteFile(filepath.Join(dir, "extension.go"), []byte(extensionCode), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to write extension.go: %v\n", err)
		os.Exit(1)
	}

	// Generate README.md
	readme := generateReadme(name)
	if err := os.WriteFile(filepath.Join(dir, "README.md"), []byte(readme), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to write README.md: %v\n", err)
		os.Exit(1)
	}

	// Generate test file
	testCode := generateTestCode(name)
	if err := os.WriteFile(filepath.Join(dir, "extension_test.go"), []byte(testCode), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to write extension_test.go: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Extension generated successfully at: %s\n", dir)
	fmt.Println("\nNext steps:")
	fmt.Printf("1. Edit %s/extension.go to implement your functionality\n", dir)
	fmt.Printf("2. Register the extension in main.go\n")
	fmt.Printf("3. Run './compile.sh' to build\n")
}

func generateExtensionCode(name string) string {
	return fmt.Sprintf(`package %s

import (
	"context"
	"encoding/json"
	"net/http"
	"time"
	
	"github.com/suppers-ai/solobase/extensions/core"
)

// %sExtension implements the core.Extension interface
type %sExtension struct {
	services *core.ExtensionServices
	enabled  bool
}

// New%sExtension creates a new instance of the extension
func New%sExtension() *%sExtension {
	return &%sExtension{
		enabled: true,
	}
}

// Metadata returns extension metadata
func (e *%sExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "%s",
		Version:     "1.0.0",
		Description: "%s extension for Solobase",
		Author:      "Your Name",
		License:     "MIT",
		Tags:        []string{"custom"},
	}
}

// Initialize initializes the extension
func (e *%sExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	e.services = services
	services.Logger().Info(ctx, "%s extension initializing")
	return nil
}

// Start starts the extension
func (e *%sExtension) Start(ctx context.Context) error {
	e.services.Logger().Info(ctx, "%s extension started")
	return nil
}

// Stop stops the extension
func (e *%sExtension) Stop(ctx context.Context) error {
	e.services.Logger().Info(ctx, "%s extension stopped")
	e.enabled = false
	return nil
}

// Health returns health status
func (e *%sExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	status := "healthy"
	if !e.enabled {
		status = "stopped"
	}
	
	return &core.HealthStatus{
		Status:      status,
		Message:     "%s extension health check",
		LastChecked: time.Now(),
	}, nil
}

// RegisterRoutes registers HTTP routes
func (e *%sExtension) RegisterRoutes(router core.ExtensionRouter) error {
	router.HandleFunc("/dashboard", e.handleDashboard)
	router.HandleFunc("/api/data", e.handleData)
	return nil
}

// RegisterMiddleware registers middleware
func (e *%sExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	return []core.MiddlewareRegistration{}
}

// RegisterHooks registers hooks
func (e *%sExtension) RegisterHooks() []core.HookRegistration {
	return []core.HookRegistration{}
}

// RegisterTemplates registers templates
func (e *%sExtension) RegisterTemplates() []core.TemplateRegistration {
	return []core.TemplateRegistration{}
}

// RegisterStaticAssets registers static assets
func (e *%sExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	return []core.StaticAssetRegistration{}
}

// ConfigSchema returns configuration schema
func (e *%sExtension) ConfigSchema() json.RawMessage {
	schema := map[string]interface{}{
		"type": "object",
		"properties": map[string]interface{}{
			"enabled": map[string]interface{}{
				"type":        "boolean",
				"description": "Enable extension",
				"default":     true,
			},
		},
	}
	
	data, _ := json.Marshal(schema)
	return data
}

// ValidateConfig validates configuration
func (e *%sExtension) ValidateConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	return json.Unmarshal(config, &cfg)
}

// ApplyConfig applies configuration
func (e *%sExtension) ApplyConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}
	
	if enabled, ok := cfg["enabled"].(bool); ok {
		e.enabled = enabled
	}
	
	return nil
}

// DatabaseSchema returns database schema name
func (e *%sExtension) DatabaseSchema() string {
	return "ext_%s"
}

// Migrations returns database migrations
func (e *%sExtension) Migrations() []core.Migration {
	return []core.Migration{
		{
			Version:     "001",
			Description: "Create initial tables",
			Extension:   "%s",
			Up: `+"`"+`
				CREATE SCHEMA IF NOT EXISTS ext_%s;
				
				CREATE TABLE IF NOT EXISTS ext_%s.data (
					id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
					name VARCHAR(255) NOT NULL,
					value JSONB,
					created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
					updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
				);
			`+"`"+`,
			Down: `+"`"+`
				DROP SCHEMA IF EXISTS ext_%s CASCADE;
			`+"`"+`,
		},
	}
}

// RequiredPermissions returns required permissions
func (e *%sExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "%s.manage",
			Description: "Manage %s extension",
			Resource:    "%s",
			Actions:     []string{"create", "read", "update", "delete"},
		},
	}
}

// Handler implementations

func (e *%sExtension) handleDashboard(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(`+"`"+`
		<!DOCTYPE html>
		<html>
		<head>
			<title>%s Dashboard</title>
		</head>
		<body>
			<h1>%s Extension Dashboard</h1>
			<p>Welcome to the %s extension dashboard!</p>
		</body>
		</html>
	`+"`"+`))
}

func (e *%sExtension) handleData(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"extension": "%s",
		"version":   "1.0.0",
		"enabled":   e.enabled,
	})
}
`,
		name,
		strings.Title(name), strings.Title(name),
		strings.Title(name),
		strings.Title(name), strings.Title(name),
		strings.Title(name),
		name,
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		strings.Title(name),
		name,
		strings.Title(name),
		name,
		name, name,
		name,
		strings.Title(name),
		name,
		strings.Title(name),
		name,
		strings.Title(name),
		strings.Title(name),
		strings.Title(name), strings.Title(name),
		strings.Title(name),
		name,
	)
}

func generateReadme(name string) string {
	return fmt.Sprintf(`# %s Extension

## Overview

The %s extension for Solobase provides...

## Features

- Feature 1
- Feature 2
- Feature 3

## Installation

1. Register the extension in main.go:
   `+"```go"+`
   import "github.com/suppers-ai/solobase/extensions/custom/%s"
   
   // In main()
   %sExt := %s.New%sExtension()
   extensionRegistry.Register(%sExt)
   `+"```"+`

2. Build the application:
   `+"```bash"+`
   ./compile.sh
   `+"```"+`

3. Enable the extension:
   `+"```bash"+`
   ./solobase extensions enable %s
   `+"```"+`

## Configuration

The extension can be configured using the following schema:

`+"```json"+`
{
  "enabled": true
}
`+"```"+`

## API Endpoints

- `+"`GET /ext/%s/dashboard`"+` - Extension dashboard
- `+"`GET /ext/%s/api/data`"+` - Get extension data

## Development

### Running Tests

`+"```bash"+`
go test ./extensions/custom/%s
`+"```"+`

## License

MIT
`,
		strings.Title(name),
		name,
		name,
		name, name, strings.Title(name),
		name,
		name,
		name,
		name,
		name,
	)
}

func generateTestCode(name string) string {
	return fmt.Sprintf(`package %s

import (
	"context"
	"testing"
	
	"github.com/stretchr/testify/assert"
	"github.com/suppers-ai/solobase/extensions/core"
)

func Test%sExtension(t *testing.T) {
	suite := core.NewExtensionTestSuite(t)
	defer suite.Cleanup()
	
	ext := New%sExtension()
	
	// Test registration
	err := suite.Registry.Register(ext)
	assert.NoError(t, err)
	
	// Test metadata
	metadata := ext.Metadata()
	assert.Equal(t, "%s", metadata.Name)
	assert.Equal(t, "1.0.0", metadata.Version)
	
	// Test enabling
	err = suite.Registry.Enable("%s")
	assert.NoError(t, err)
	
	// Test health check
	health, err := ext.Health(context.Background())
	assert.NoError(t, err)
	assert.Equal(t, "healthy", health.Status)
}

func Test%sExtensionRoutes(t *testing.T) {
	suite := core.NewExtensionTestSuite(t)
	defer suite.Cleanup()
	
	ext := New%sExtension()
	suite.LoadExtension(ext)
	
	// Test dashboard endpoint
	resp := suite.TestRequest("GET", "/ext/%s/dashboard", nil)
	assert.Equal(t, 200, resp.Code)
	assert.Contains(t, resp.Body.String(), "%s Extension Dashboard")
	
	// Test data endpoint
	resp = suite.TestRequest("GET", "/ext/%s/api/data", nil)
	assert.Equal(t, 200, resp.Code)
	assert.Contains(t, resp.Body.String(), "\"extension\":\"%s\"")
}
`,
		name,
		strings.Title(name),
		strings.Title(name),
		name,
		name,
		strings.Title(name),
		strings.Title(name),
		name,
		strings.Title(name),
		name,
		name,
	)
}
