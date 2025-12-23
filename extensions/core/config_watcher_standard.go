//go:build !wasm

package core

import (
	"os"
	"sync"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

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
	ticker := apptime.NewTicker(5 * apptime.Second)
	defer ticker.Stop()

	var lastMod apptime.Time

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
