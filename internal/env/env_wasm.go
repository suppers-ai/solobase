//go:build wasm

package env

// envStore holds environment values set by the host.
// In WASM builds, environment variables are provided by the host
// and must be set before use via SetEnv.
// Using lazy initialization because TinyGo skipping _start may not
// run package-level variable initializations.
var envStore map[string]string

// getStore returns the envStore map, initializing it if needed.
// This handles the case where _start wasn't called.
func getStore() map[string]string {
	if envStore == nil {
		envStore = make(map[string]string)
	}
	return envStore
}

// SetEnv sets an environment variable value for WASM builds.
// This should be called by the WASM host before initializing the app.
func SetEnv(key, value string) {
	getStore()[key] = value
}

// SetEnvMap sets multiple environment variables at once.
func SetEnvMap(vars map[string]string) {
	store := getStore()
	for k, v := range vars {
		store[k] = v
	}
}

// GetEnv returns the value of an environment variable.
// On WASM builds, this reads from the envStore set by the host.
func GetEnv(key string) string {
	return getStore()[key]
}

// GetEnvOrDefault returns the value of an environment variable or a default.
func GetEnvOrDefault(key, defaultValue string) string {
	if value, ok := getStore()[key]; ok && value != "" {
		return value
	}
	return defaultValue
}
