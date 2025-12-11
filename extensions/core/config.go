package core

import (
	"encoding/json"
	"fmt"
	"os"
	"sync"
	"time"

	"gopkg.in/yaml.v3"
)

// ExtensionConfig manages configuration for extensions
type ExtensionConfig struct {
	mu        sync.RWMutex
	Enabled   map[string]bool                   `yaml:"enabled" json:"enabled"`
	Config    map[string]map[string]interface{} `yaml:"config" json:"config"`
	BuildTags []string                          `yaml:"buildTags" json:"buildTags"`
	LoadOrder []string                          `yaml:"loadOrder" json:"loadOrder"`
}

// NewExtensionConfig creates a new extension configuration
func NewExtensionConfig() *ExtensionConfig {
	return &ExtensionConfig{
		Enabled:   make(map[string]bool),
		Config:    make(map[string]map[string]interface{}),
		BuildTags: []string{},
		LoadOrder: []string{},
	}
}

// LoadFromFile loads configuration from a YAML or JSON file
func (c *ExtensionConfig) LoadFromFile(path string) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			// File doesn't exist, use defaults
			return nil
		}
		return fmt.Errorf("failed to read config file: %w", err)
	}

	// Try to unmarshal as YAML first (supports both YAML and JSON)
	if err := yaml.Unmarshal(data, c); err != nil {
		// If YAML fails, try JSON
		if err := json.Unmarshal(data, c); err != nil {
			return fmt.Errorf("failed to parse config file: %w", err)
		}
	}

	return nil
}

// SaveToFile saves configuration to a YAML file
func (c *ExtensionConfig) SaveToFile(path string) error {
	c.mu.RLock()
	defer c.mu.RUnlock()

	data, err := yaml.Marshal(c)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	if err := os.WriteFile(path, data, 0644); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

// IsEnabled checks if an extension is enabled
func (c *ExtensionConfig) IsEnabled(extension string) bool {
	c.mu.RLock()
	defer c.mu.RUnlock()

	enabled, exists := c.Enabled[extension]
	return exists && enabled
}

// SetEnabled sets whether an extension is enabled
func (c *ExtensionConfig) SetEnabled(extension string, enabled bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.Enabled[extension] = enabled
}

// GetExtensionConfig returns configuration for a specific extension
func (c *ExtensionConfig) GetExtensionConfig(extension string) (map[string]interface{}, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	config, exists := c.Config[extension]
	if !exists {
		return nil, false
	}

	// Return a copy to prevent modification
	copy := make(map[string]interface{})
	for k, v := range config {
		copy[k] = v
	}

	return copy, true
}

// SetExtensionConfig sets configuration for a specific extension
func (c *ExtensionConfig) SetExtensionConfig(extension string, config map[string]interface{}) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.Config[extension] = config
}

// GetExtensionConfigValue gets a specific config value for an extension
func (c *ExtensionConfig) GetExtensionConfigValue(extension, key string) (interface{}, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if extConfig, exists := c.Config[extension]; exists {
		value, exists := extConfig[key]
		return value, exists
	}

	return nil, false
}

// SetExtensionConfigValue sets a specific config value for an extension
func (c *ExtensionConfig) SetExtensionConfigValue(extension, key string, value interface{}) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.Config[extension] == nil {
		c.Config[extension] = make(map[string]interface{})
	}

	c.Config[extension][key] = value
}

// ValidateExtensionConfig validates configuration against an extension's schema
func (c *ExtensionConfig) ValidateExtensionConfig(extension Extension) error {
	schema := extension.ConfigSchema()
	if schema == nil {
		// No schema defined, config is valid
		return nil
	}

	config, exists := c.GetExtensionConfig(extension.Metadata().Name)
	if !exists {
		// No config provided, use defaults
		config = make(map[string]interface{})
	}

	// Marshal config to JSON for validation
	configJSON, err := json.Marshal(config)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// Let extension validate its config
	return extension.ValidateConfig(configJSON)
}

// ApplyExtensionConfig applies configuration to an extension
func (c *ExtensionConfig) ApplyExtensionConfig(extension Extension) error {
	config, exists := c.GetExtensionConfig(extension.Metadata().Name)
	if !exists {
		// No config provided, use defaults
		config = make(map[string]interface{})
	}

	// Marshal config to JSON
	configJSON, err := json.Marshal(config)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// Apply config to extension
	return extension.ApplyConfig(configJSON)
}

// GetLoadOrder returns the extension load order
func (c *ExtensionConfig) GetLoadOrder() []string {
	c.mu.RLock()
	defer c.mu.RUnlock()

	// Return a copy
	order := make([]string, len(c.LoadOrder))
	copy(order, c.LoadOrder)

	return order
}

// SetLoadOrder sets the extension load order
func (c *ExtensionConfig) SetLoadOrder(order []string) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.LoadOrder = order
}

// ConfigWatcher watches for configuration changes
type ConfigWatcher struct {
	path     string
	config   *ExtensionConfig
	onChange func(*ExtensionConfig)
	stop     chan bool
	wg       sync.WaitGroup
}

// NewConfigWatcher creates a new configuration watcher
func NewConfigWatcher(path string, onChange func(*ExtensionConfig)) *ConfigWatcher {
	return &ConfigWatcher{
		path:     path,
		config:   NewExtensionConfig(),
		onChange: onChange,
		stop:     make(chan bool),
	}
}

// Start starts watching for configuration changes
func (w *ConfigWatcher) Start() error {
	// Load initial config
	if err := w.config.LoadFromFile(w.path); err != nil {
		return err
	}

	// Trigger initial callback
	if w.onChange != nil {
		w.onChange(w.config)
	}

	// Start watching for changes
	w.wg.Add(1)
	go w.watch()

	return nil
}

// Stop stops watching for configuration changes
func (w *ConfigWatcher) Stop() {
	close(w.stop)
	w.wg.Wait()
}

// watch watches for file changes
func (w *ConfigWatcher) watch() {
	defer w.wg.Done()

	// Simple polling implementation
	// In production, use fsnotify or similar
	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()

	var lastMod time.Time

	for {
		select {
		case <-w.stop:
			return
		case <-ticker.C:
			info, err := os.Stat(w.path)
			if err != nil {
				continue
			}

			if info.ModTime().After(lastMod) {
				lastMod = info.ModTime()

				// Reload config
				newConfig := NewExtensionConfig()
				if err := newConfig.LoadFromFile(w.path); err == nil {
					w.config = newConfig
					if w.onChange != nil {
						w.onChange(w.config)
					}
				}
			}
		}
	}
}

// ConfigValidator validates extension configuration
type ConfigValidator interface {
	ValidateConfig(config interface{}) error
	DefaultConfig() interface{}
	ConfigSchema() interface{}
}
