package web

import (
	"fmt"
	"os"
	"path/filepath"

	wafer "github.com/wafer-run/wafer-go"
)

const BlockName = "web-feature"

// WebConfig configures the web block.
type WebConfig struct {
	Dir             string // Required. Root directory with built site (e.g., "./sites/marketing/dist")
	Prefix          string // URL prefix to strip (e.g., "/site"). Default: ""
	SPAMode         bool   // Fallback to index.html for unknown paths. Default: false
	IndexFile       string // Directory index filename. Default: "index.html"
	CacheMaxAge     int    // Cache max-age for normal assets (seconds). Default: 3600
	ImmutableMaxAge int    // Cache max-age for hashed assets (seconds). Default: 31536000
}

// WebBlock serves static files from a configured directory.
type WebBlock struct {
	config  WebConfig
	absRoot string // resolved absolute path to Dir, set during Init
}

// NewWebBlock creates a new web block with the given config, applying defaults.
func NewWebBlock(cfg WebConfig) *WebBlock {
	if cfg.IndexFile == "" {
		cfg.IndexFile = "index.html"
	}
	if cfg.CacheMaxAge == 0 {
		cfg.CacheMaxAge = 3600
	}
	if cfg.ImmutableMaxAge == 0 {
		cfg.ImmutableMaxAge = 31536000
	}
	return &WebBlock{config: cfg}
}

func (b *WebBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Static website serving",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton, wafer.PerNode},
	}
}

func (b *WebBlock) Lifecycle(_ wafer.Context, evt wafer.LifecycleEvent) error {
	if evt.Type == wafer.Init {
		abs, err := filepath.Abs(b.config.Dir)
		if err != nil {
			return fmt.Errorf("web block: resolve dir %q: %w", b.config.Dir, err)
		}
		info, err := os.Stat(abs)
		if err != nil {
			return fmt.Errorf("web block: stat dir %q: %w", abs, err)
		}
		if !info.IsDir() {
			return fmt.Errorf("web block: %q is not a directory", abs)
		}
		b.absRoot = abs
	}
	return nil
}

func (b *WebBlock) Handle(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if msg.Action() != "retrieve" {
		return wafer.Error(msg, 405, "method_not_allowed", "only GET requests are supported")
	}

	reqPath := msg.Path()
	if b.config.Prefix != "" {
		reqPath = stripPrefix(reqPath, b.config.Prefix)
	}

	return b.serveFile(msg, reqPath)
}

// stripPrefix removes the prefix from the path.
func stripPrefix(path, prefix string) string {
	if len(path) >= len(prefix) && path[:len(prefix)] == prefix {
		path = path[len(prefix):]
	}
	if path == "" {
		path = "/"
	}
	return path
}
