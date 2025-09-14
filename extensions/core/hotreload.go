package core

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/fsnotify/fsnotify"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
)

// HotReloader manages hot-reload of extension configurations
type HotReloader struct {
	mu              sync.RWMutex
	registry        *ExtensionRegistry
	configPath      string
	watcher         *fsnotify.Watcher
	logger          logger.Logger
	stopCh          chan struct{}
	reloadCallbacks map[string]func(config json.RawMessage) error
}

// NewHotReloader creates a new hot reloader
func NewHotReloader(registry *ExtensionRegistry, configPath string, logger logger.Logger) (*HotReloader, error) {
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, fmt.Errorf("failed to create file watcher: %w", err)
	}

	return &HotReloader{
		registry:        registry,
		configPath:      configPath,
		watcher:         watcher,
		logger:          logger,
		stopCh:          make(chan struct{}),
		reloadCallbacks: make(map[string]func(config json.RawMessage) error),
	}, nil
}

// Start starts the hot reloader
func (hr *HotReloader) Start(ctx context.Context) error {
	// Watch config directory
	configDir := filepath.Dir(hr.configPath)
	if err := hr.watcher.Add(configDir); err != nil {
		return fmt.Errorf("failed to watch config directory: %w", err)
	}

	// Watch extension config files
	extensionConfigDir := filepath.Join(configDir, "extensions")
	if _, err := os.Stat(extensionConfigDir); err == nil {
		if err := hr.watcher.Add(extensionConfigDir); err != nil {
			hr.logger.Warn(ctx, "Failed to watch extension config directory: "+err.Error())
		}
	}

	// Start watching for changes
	go hr.watch(ctx)

	hr.logger.Info(ctx, "Hot reload started, watching: "+configDir)

	return nil
}

// Stop stops the hot reloader
func (hr *HotReloader) Stop(ctx context.Context) error {
	close(hr.stopCh)
	return hr.watcher.Close()
}

// RegisterReloadCallback registers a callback for configuration reload
func (hr *HotReloader) RegisterReloadCallback(extension string, callback func(config json.RawMessage) error) {
	hr.mu.Lock()
	defer hr.mu.Unlock()
	hr.reloadCallbacks[extension] = callback
}

// watch watches for file changes
func (hr *HotReloader) watch(ctx context.Context) {
	debounceTimer := time.NewTimer(0)
	<-debounceTimer.C // Drain initial timer

	var pendingReload bool

	for {
		select {
		case <-hr.stopCh:
			hr.logger.Info(ctx, "Hot reload stopped")
			return

		case event, ok := <-hr.watcher.Events:
			if !ok {
				return
			}

			// Check if it's a config file
			if isConfigFile(event.Name) {
				hr.logger.Info(ctx, fmt.Sprintf("Config file changed: %s (%s)", event.Name, event.Op))

				// Debounce rapid changes
				if !pendingReload {
					pendingReload = true
					debounceTimer.Reset(500 * time.Millisecond)
				}
			}

		case err, ok := <-hr.watcher.Errors:
			if !ok {
				return
			}
			hr.logger.Error(ctx, "File watcher error: "+err.Error())

		case <-debounceTimer.C:
			if pendingReload {
				pendingReload = false
				hr.reloadConfigurations(ctx)
			}
		}
	}
}

// reloadConfigurations reloads all configurations
func (hr *HotReloader) reloadConfigurations(ctx context.Context) {
	hr.logger.Info(ctx, "Reloading configurations...")

	// Reload main config
	if err := hr.reloadMainConfig(ctx); err != nil {
		hr.logger.Error(ctx, "Failed to reload main config: "+err.Error())
	}

	// Reload extension configs
	if err := hr.reloadExtensionConfigs(ctx); err != nil {
		hr.logger.Error(ctx, "Failed to reload extension configs: "+err.Error())
	}

	hr.logger.Info(ctx, "Configuration reload completed")
}

// reloadMainConfig reloads the main configuration
func (hr *HotReloader) reloadMainConfig(ctx context.Context) error {
	data, err := os.ReadFile(hr.configPath)
	if err != nil {
		return fmt.Errorf("failed to read config file: %w", err)
	}

	var config map[string]interface{}
	if err := json.Unmarshal(data, &config); err != nil {
		return fmt.Errorf("failed to parse config: %w", err)
	}

	// Apply to extensions if they have config sections
	if extensions, ok := config["extensions"].(map[string]interface{}); ok {
		for name, extConfig := range extensions {
			if err := hr.applyExtensionConfig(ctx, name, extConfig); err != nil {
				hr.logger.Error(ctx, fmt.Sprintf("Failed to apply config for extension %s: %v", name, err))
			}
		}
	}

	return nil
}

// reloadExtensionConfigs reloads individual extension configs
func (hr *HotReloader) reloadExtensionConfigs(ctx context.Context) error {
	configDir := filepath.Dir(hr.configPath)
	extensionConfigDir := filepath.Join(configDir, "extensions")

	if _, err := os.Stat(extensionConfigDir); os.IsNotExist(err) {
		return nil // No extension config directory
	}

	files, err := os.ReadDir(extensionConfigDir)
	if err != nil {
		return fmt.Errorf("failed to read extension config directory: %w", err)
	}

	for _, file := range files {
		if filepath.Ext(file.Name()) != ".json" {
			continue
		}

		extensionName := strings.TrimSuffix(file.Name(), ".json")
		configPath := filepath.Join(extensionConfigDir, file.Name())

		data, err := os.ReadFile(configPath)
		if err != nil {
			hr.logger.Error(ctx, fmt.Sprintf("Failed to read config for %s: %v", extensionName, err))
			continue
		}

		var config interface{}
		if err := json.Unmarshal(data, &config); err != nil {
			hr.logger.Error(ctx, fmt.Sprintf("Failed to parse config for %s: %v", extensionName, err))
			continue
		}

		if err := hr.applyExtensionConfig(ctx, extensionName, config); err != nil {
			hr.logger.Error(ctx, fmt.Sprintf("Failed to apply config for %s: %v", extensionName, err))
		}
	}

	return nil
}

// applyExtensionConfig applies configuration to an extension
func (hr *HotReloader) applyExtensionConfig(ctx context.Context, extensionName string, config interface{}) error {
	ext, exists := hr.registry.Get(extensionName)
	if !exists {
		return fmt.Errorf("extension not found: %s", extensionName)
	}

	// Marshal config to JSON
	configData, err := json.Marshal(config)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// Validate configuration
	if err := ext.ValidateConfig(configData); err != nil {
		return fmt.Errorf("invalid configuration: %w", err)
	}

	// Apply configuration
	if err := ext.ApplyConfig(configData); err != nil {
		return fmt.Errorf("failed to apply configuration: %w", err)
	}

	// Call reload callback if registered
	hr.mu.RLock()
	callback, hasCallback := hr.reloadCallbacks[extensionName]
	hr.mu.RUnlock()

	if hasCallback {
		if err := callback(configData); err != nil {
			return fmt.Errorf("reload callback failed: %w", err)
		}
	}

	hr.logger.Info(ctx, fmt.Sprintf("Configuration reloaded for extension: %s", extensionName))

	return nil
}

// isConfigFile checks if a file is a configuration file
func isConfigFile(path string) bool {
	ext := filepath.Ext(path)
	return ext == ".json" || ext == ".yaml" || ext == ".yml" || ext == ".toml"
}

// ConfigHistory tracks configuration changes over time
type ConfigHistory struct {
	mu      sync.RWMutex
	history map[string][]ConfigHistoryEntry
	maxSize int
}

// ConfigHistoryEntry represents a configuration change
type ConfigHistoryEntry struct {
	Version   int
	Config    json.RawMessage
	Timestamp time.Time
	Action    string // "update", "rollback", "apply"
	User      string
	Success   bool
	Error     string
}

// NewConfigHistory creates a new config history tracker
func NewConfigHistory(maxSize int) *ConfigHistory {
	return &ConfigHistory{
		history: make(map[string][]ConfigHistoryEntry),
		maxSize: maxSize,
	}
}

// AddEntry adds a history entry
func (ch *ConfigHistory) AddEntry(extension string, entry ConfigHistoryEntry) {
	ch.mu.Lock()
	defer ch.mu.Unlock()

	entries := ch.history[extension]
	entries = append(entries, entry)

	// Trim to max size
	if len(entries) > ch.maxSize {
		entries = entries[len(entries)-ch.maxSize:]
	}

	ch.history[extension] = entries
}

// GetHistory gets configuration history
func (ch *ConfigHistory) GetHistory(extension string, limit int) []ConfigHistoryEntry {
	ch.mu.RLock()
	defer ch.mu.RUnlock()

	entries := ch.history[extension]
	if len(entries) == 0 {
		return []ConfigHistoryEntry{}
	}

	// Return last N entries
	if limit > 0 && limit < len(entries) {
		return entries[len(entries)-limit:]
	}

	// Return copy
	result := make([]ConfigHistoryEntry, len(entries))
	copy(result, entries)
	return result
}
