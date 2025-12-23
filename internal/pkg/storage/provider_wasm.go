//go:build wasm

package storage

// NewProvider creates a new storage provider based on the configuration
// In WASM builds, storage is not available - using NoopProvider
// TODO: Implement storage via wasi:http when wasmCloud adds storage capability
func NewProvider(cfg Config) (Provider, error) {
	// Storage not available in WASM - use noop provider
	// Local filesystem is not available in WASM
	// S3 provider uses AWS SDK which is not WASM-compatible
	return NewNoopProvider(), nil
}
