//go:build !wasm

package env

import "os"

// GetEnv returns the value of an environment variable.
// On standard builds, this reads from os.Getenv.
func GetEnv(key string) string {
	return os.Getenv(key)
}

// GetEnvOrDefault returns the value of an environment variable or a default.
func GetEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}
