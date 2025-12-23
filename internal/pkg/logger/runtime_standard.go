//go:build !wasm

package logger

import (
	"fmt"
	"path/filepath"
	"runtime"
)

// getCaller returns the caller information
func getCaller() string {
	_, file, line, ok := runtime.Caller(3)
	if !ok {
		return ""
	}

	// Get just the filename
	file = filepath.Base(file)
	return fmt.Sprintf("%s:%d", file, line)
}

// getCallerInfo returns detailed caller information for logging
func getCallerInfo(skip int) string {
	pc, file, line, ok := runtime.Caller(skip)
	if !ok {
		return ""
	}
	fn := runtime.FuncForPC(pc)
	if fn == nil {
		return fmt.Sprintf("%s:%d", file, line)
	}
	return fmt.Sprintf("%s:%d %s", file, line, fn.Name())
}

// getStackTrace returns the current stack trace
func getStackTrace() string {
	buf := make([]byte, 1024*64)
	n := runtime.Stack(buf, false)
	return string(buf[:n])
}
