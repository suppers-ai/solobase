//go:build !wasm

package fileutil

import (
	"io/fs"
	"os"
)

// EnsureDir creates a directory and all its parents if they don't exist
func EnsureDir(path string) error {
	return os.MkdirAll(path, 0755)
}

// WriteFile writes data to a file, creating it if it doesn't exist
func WriteFile(filename string, data []byte, perm fs.FileMode) error {
	return os.WriteFile(filename, data, perm)
}

// ReadFile reads a file and returns its contents
func ReadFile(filename string) ([]byte, error) {
	return os.ReadFile(filename)
}

// FileExists checks if a file or directory exists
func FileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}
