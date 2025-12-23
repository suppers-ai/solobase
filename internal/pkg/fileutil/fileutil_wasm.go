//go:build wasm

package fileutil

import (
	"errors"
	"fmt"
	"io/fs"
)

var errNotSupported = errors.New("filesystem operations not supported in WASM builds")

// EnsureDir is a no-op in WASM builds
// Filesystem is handled by the WASM runtime (e.g., Spin)
func EnsureDir(path string) error {
	fmt.Printf("WASM fileutil.EnsureDir called for: %s (no-op)\n", path)
	return nil // No-op - runtime handles storage
}

// WriteFile is not supported in WASM builds
func WriteFile(filename string, data []byte, perm fs.FileMode) error {
	return errNotSupported
}

// ReadFile is not supported in WASM builds
func ReadFile(filename string) ([]byte, error) {
	return nil, errNotSupported
}

// FileExists always returns false in WASM builds
func FileExists(path string) bool {
	return false
}
