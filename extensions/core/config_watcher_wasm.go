//go:build wasm

package core

// ConfigWatcher is a no-op in WASM builds
// Configuration changes are not watched in WASM runtime
type ConfigWatcher struct {
	path     string
	config   *ExtensionConfig
	onChange func(*ExtensionConfig)
}

// NewConfigWatcher creates a new configuration watcher (no-op in WASM)
func NewConfigWatcher(path string, onChange func(*ExtensionConfig)) *ConfigWatcher {
	return &ConfigWatcher{
		path:     path,
		config:   NewExtensionConfig(),
		onChange: onChange,
	}
}

// Start loads initial config and triggers callback (no watching in WASM)
func (w *ConfigWatcher) Start() error {
	// Load initial config
	if err := w.config.LoadFromFile(w.path); err != nil {
		return err
	}

	// Trigger initial callback
	if w.onChange != nil {
		w.onChange(w.config)
	}

	// No watching in WASM builds
	return nil
}

// Stop is a no-op in WASM builds
func (w *ConfigWatcher) Stop() {
	// No-op
}
