package extensions

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/extensions/core"
	"gorm.io/gorm"
)

// ExtensionManager manages the extension system lifecycle
type ExtensionManager struct {
	registry *core.ExtensionRegistry
	services *core.ExtensionServices
	config   *ExtensionConfig
	logger   logger.Logger
	db       *gorm.DB
}

// ExtensionConfig holds the configuration for all extensions
type ExtensionConfig struct {
	Extensions map[string]ExtensionSettings `json:"extensions"`
}

// ExtensionSettings holds settings for a single extension
type ExtensionSettings struct {
	Enabled bool                   `json:"enabled"`
	Config  map[string]interface{} `json:"config"`
}

// NewExtensionManager creates a new extension manager
func NewExtensionManager(db *gorm.DB, logger logger.Logger) (*ExtensionManager, error) {
	// Create extension services
	// For now, we pass nil for services we don't have
	services := core.NewExtensionServices(
		nil, // database.Database - we'll pass nil for now
		nil, // auth.Service
		logger,
		nil, // storage
		nil, // config
		nil, // stats
		nil, // iam
	)

	// Create extension registry
	registry := core.NewExtensionRegistry(logger, services)

	// Load configuration
	config, err := loadExtensionConfig()
	if err != nil {
		return nil, fmt.Errorf("failed to load extension config: %w", err)
	}

	return &ExtensionManager{
		registry: registry,
		services: services,
		config:   config,
		logger:   logger,
		db:       db,
	}, nil
}

// Initialize initializes the extension system
func (m *ExtensionManager) Initialize(ctx context.Context) error {
	m.logger.Info(ctx, "Initializing extension system")

	// Register all discovered extensions
	if err := m.registerExtensions(); err != nil {
		return fmt.Errorf("failed to register extensions: %w", err)
	}

	// Enable configured extensions
	for name, settings := range m.config.Extensions {
		if settings.Enabled {
			if err := m.enableExtension(ctx, name, settings.Config); err != nil {
				m.logger.Error(ctx, "Failed to enable extension",
					logger.String("extension", name),
					logger.Err(err))
				// Continue with other extensions
			}
		}
	}

	m.logger.Info(ctx, "Extension system initialized")
	return nil
}

// RegisterRoutes registers extension routes with the main router
func (m *ExtensionManager) RegisterRoutes(router *mux.Router) {
	// Create extension subrouter
	extRouter := router.PathPrefix("/ext").Subrouter()

	// Register extension routes
	m.registry.RegisterRoutes(extRouter)

	m.logger.Info(context.Background(), "Extension routes registered")
}

// ApplyMiddleware applies extension middleware to the router
func (m *ExtensionManager) ApplyMiddleware(handler http.Handler) http.Handler {
	return m.registry.ApplyMiddleware(handler)
}

// Shutdown gracefully shuts down all extensions
func (m *ExtensionManager) Shutdown(ctx context.Context) error {
	m.logger.Info(ctx, "Shutting down extension system")

	// Get all enabled extensions
	extensions := m.registry.GetAll()

	// Stop all extensions
	for _, ext := range extensions {
		metadata := ext.Metadata()
		if err := ext.Stop(ctx); err != nil {
			m.logger.Error(ctx, "Failed to stop extension",
				logger.String("extension", metadata.Name),
				logger.Err(err))
		}
	}

	m.logger.Info(ctx, "Extension system shutdown complete")
	return nil
}

// GetRegistry returns the extension registry
func (m *ExtensionManager) GetRegistry() *core.ExtensionRegistry {
	return m.registry
}

// GetExtension returns a specific extension by name
func (m *ExtensionManager) GetExtension(name string) (core.Extension, bool) {
	return m.registry.Get(name)
}

// SaveExtensionState saves the enabled/disabled state of an extension
func (m *ExtensionManager) SaveExtensionState(name string, enabled bool) {
	if m.config.Extensions == nil {
		m.config.Extensions = make(map[string]ExtensionSettings)
	}

	settings, exists := m.config.Extensions[name]
	if !exists {
		settings = ExtensionSettings{
			Config: make(map[string]interface{}),
		}
	}
	settings.Enabled = enabled
	m.config.Extensions[name] = settings

	// Save to file (optional, for persistence)
	m.saveConfig()
}

// enableExtension enables and configures an extension
func (m *ExtensionManager) enableExtension(ctx context.Context, name string, config map[string]interface{}) error {
	// Get the extension
	ext, exists := m.registry.Get(name)
	if !exists {
		return fmt.Errorf("extension %s not found", name)
	}

	// Apply configuration if provided
	if len(config) > 0 {
		configJSON, err := json.Marshal(config)
		if err != nil {
			return fmt.Errorf("failed to marshal config for %s: %w", name, err)
		}

		if err := ext.ValidateConfig(configJSON); err != nil {
			return fmt.Errorf("invalid config for %s: %w", name, err)
		}

		if err := ext.ApplyConfig(configJSON); err != nil {
			return fmt.Errorf("failed to apply config for %s: %w", name, err)
		}
	}

	// Enable the extension
	if err := m.registry.Enable(name); err != nil {
		return fmt.Errorf("failed to enable %s: %w", name, err)
	}

	m.logger.Info(ctx, "Extension enabled",
		logger.String("extension", name))

	return nil
}

// saveConfig saves the extension configuration to file
func (m *ExtensionManager) saveConfig() error {
	configPath := filepath.Join("extensions", "config.json")

	// Ensure directory exists
	if err := os.MkdirAll(filepath.Dir(configPath), 0755); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	// Marshal config to JSON
	data, err := json.MarshalIndent(m.config, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// Write to file
	if err := os.WriteFile(configPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

// loadExtensionConfig loads the extension configuration from file
func loadExtensionConfig() (*ExtensionConfig, error) {
	configPath := filepath.Join("extensions", "config.json")

	// Check if config file exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		// Return default config if file doesn't exist
		return &ExtensionConfig{
			Extensions: make(map[string]ExtensionSettings),
		}, nil
	}

	// Read config file
	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	// Parse config
	var config ExtensionConfig
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %w", err)
	}

	return &config, nil
}

// registerExtensions registers all available extensions
func (m *ExtensionManager) registerExtensions() error {
	m.logger.Info(context.Background(), "Registering extensions")

	// Register all available extensions with database
	// Use the manual registration for now since we need to pass database
	if err := RegisterAllExtensions(m.registry, m.db); err != nil {
		return fmt.Errorf("failed to register extensions: %w", err)
	}

	m.logger.Info(context.Background(), "Extension registration complete")
	return nil
}
