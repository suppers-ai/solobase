//go:build wasm

package logger

// getCaller returns empty string in WASI builds
// Caller information is not available in TinyGo/WASI
func getCaller() string {
	return ""
}

// getCallerInfo returns empty string in WASI builds
func getCallerInfo(skip int) string {
	return ""
}

// getStackTrace returns empty string in WASI builds
// Stack traces are not available in TinyGo/WASI
func getStackTrace() string {
	return ""
}
