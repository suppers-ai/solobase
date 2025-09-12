package logger

import (
	"context"
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

// contextKey is a type for context keys
type contextKey string

const (
	// TraceIDKey is the context key for trace ID
	TraceIDKey contextKey = "trace_id"

	// UserIDKey is the context key for user ID
	UserIDKey contextKey = "user_id"

	// LoggerKey is the context key for logger instance
	LoggerKey contextKey = "logger"
)

// Default logger instance
var defaultLogger Logger

// SetDefault sets the default logger
func SetDefault(logger Logger) {
	defaultLogger = logger
}

// GetDefault gets the default logger
func GetDefault() Logger {
	if defaultLogger == nil {
		// Create a basic console logger if none set
		defaultLogger, _ = NewConsole(Config{
			Level:  LevelInfo,
			Format: "text",
		})
	}
	return defaultLogger
}

// Helper functions that use the default logger

// Debug logs a debug message using the default logger
func Debug(message string, fields ...Field) {
	GetDefault().Debug(context.Background(), message, fields...)
}

// Info logs an info message using the default logger
func Info(message string, fields ...Field) {
	GetDefault().Info(context.Background(), message, fields...)
}

// Warn logs a warning message using the default logger
func Warn(message string, fields ...Field) {
	GetDefault().Warn(context.Background(), message, fields...)
}

// Error logs an error message using the default logger
func Error(message string, fields ...Field) {
	GetDefault().Error(context.Background(), message, fields...)
}

// Fatal logs a fatal message using the default logger and exits
func Fatal(message string, fields ...Field) {
	GetDefault().Fatal(context.Background(), message, fields...)
}
