package interfaces

import "context"

// Logger defines the interface for logging operations.
// Implementations should be TinyGo compatible.
type Logger interface {
	// Standard log levels
	Debug(msg string, fields ...Field)
	Info(msg string, fields ...Field)
	Warn(msg string, fields ...Field)
	Error(msg string, fields ...Field)

	// Context-aware logging
	DebugContext(ctx context.Context, msg string, fields ...Field)
	InfoContext(ctx context.Context, msg string, fields ...Field)
	WarnContext(ctx context.Context, msg string, fields ...Field)
	ErrorContext(ctx context.Context, msg string, fields ...Field)

	// With returns a logger with additional fields
	With(fields ...Field) Logger

	// WithContext returns a logger with context values
	WithContext(ctx context.Context) Logger
}

// Field represents a log field
type Field struct {
	Key   string
	Value interface{}
}

// Helper functions to create fields
func String(key, value string) Field {
	return Field{Key: key, Value: value}
}

func Int(key string, value int) Field {
	return Field{Key: key, Value: value}
}

func Int64(key string, value int64) Field {
	return Field{Key: key, Value: value}
}

func Float64(key string, value float64) Field {
	return Field{Key: key, Value: value}
}

func Bool(key string, value bool) Field {
	return Field{Key: key, Value: value}
}

func Err(err error) Field {
	if err == nil {
		return Field{Key: "error", Value: nil}
	}
	return Field{Key: "error", Value: err.Error()}
}

func Any(key string, value interface{}) Field {
	return Field{Key: key, Value: value}
}

// LogLevel represents log levels
type LogLevel int

const (
	LevelDebug LogLevel = iota
	LevelInfo
	LevelWarn
	LevelError
)

func (l LogLevel) String() string {
	switch l {
	case LevelDebug:
		return "debug"
	case LevelInfo:
		return "info"
	case LevelWarn:
		return "warn"
	case LevelError:
		return "error"
	default:
		return "unknown"
	}
}

// LoggerConfig contains logger configuration
type LoggerConfig struct {
	Level  LogLevel
	Output string // console, json, file
	Format string // text, json
	File   string // for file output
}
