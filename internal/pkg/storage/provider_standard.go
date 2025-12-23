//go:build !wasm

package storage

// NewProvider creates a new storage provider based on the configuration
func NewProvider(cfg Config) (Provider, error) {
	switch cfg.Provider {
	case ProviderLocal:
		return NewLocalProvider(cfg)
	case ProviderS3:
		return NewS3Provider(cfg)
	default:
		return NewLocalProvider(cfg) // Default to local
	}
}
