package logger

import (
	"context"
	"time"
)

// Logger is the main interface for logging
type Logger interface {
	// Log methods
	Debug(ctx context.Context, message string, fields ...Field)
	Info(ctx context.Context, message string, fields ...Field)
	Warn(ctx context.Context, message string, fields ...Field)
	Error(ctx context.Context, message string, fields ...Field)
	Fatal(ctx context.Context, message string, fields ...Field)

	// Structured logging
	With(fields ...Field) Logger
	WithContext(ctx context.Context) Logger

	// HTTP request logging
	LogRequest(ctx context.Context, req *RequestLog) error

	// Query methods (for database loggers)
	GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error)
	GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error)

	// Flush any buffered logs
	Flush() error

	// Close the logger
	Close() error
}

// Level represents the log level
type Level string

const (
	LevelDebug Level = "DEBUG"
	LevelInfo  Level = "INFO"
	LevelWarn  Level = "WARN"
	LevelError Level = "ERROR"
	LevelFatal Level = "FATAL"
)

// Log represents a general log entry
type Log struct {
	ID        string                 `json:"id" db:"id"`
	Level     Level                  `json:"level" db:"level"`
	Message   string                 `json:"message" db:"message"`
	Fields    map[string]interface{} `json:"fields" db:"fields"`
	UserID    *string                `json:"user_id,omitempty" db:"user_id"`
	TraceID   *string                `json:"trace_id,omitempty" db:"trace_id"`
	Timestamp time.Time              `json:"timestamp" db:"created_at"`
}

// RequestLog represents an HTTP request log entry
type RequestLog struct {
	ID           string    `json:"id" db:"id"`
	Level        Level     `json:"level" db:"level"`
	Method       string    `json:"method" db:"method"`
	Path         string    `json:"path" db:"path"`
	Query        string    `json:"query,omitempty" db:"query"`
	StatusCode   int       `json:"status_code" db:"status_code"`
	ExecTimeMs   int64     `json:"exec_time_ms" db:"exec_time_ms"`
	UserIP       string    `json:"user_ip" db:"user_ip"`
	UserAgent    string    `json:"user_agent,omitempty" db:"user_agent"`
	UserID       *string   `json:"user_id,omitempty" db:"user_id"`
	TraceID      *string   `json:"trace_id,omitempty" db:"trace_id"`
	Error        *string   `json:"error,omitempty" db:"error"`
	RequestBody  *string   `json:"request_body,omitempty" db:"request_body"`
	ResponseBody *string   `json:"response_body,omitempty" db:"response_body"`
	Headers      *string   `json:"headers,omitempty" db:"headers"`
	CreatedAt    time.Time `json:"created_at" db:"created_at"`
}

// Field represents a structured logging field
type Field struct {
	Key   string
	Value interface{}
}

// LogFilter for querying logs
type LogFilter struct {
	Level     *Level     `json:"level,omitempty"`
	UserID    *string    `json:"user_id,omitempty"`
	TraceID   *string    `json:"trace_id,omitempty"`
	StartTime *time.Time `json:"start_time,omitempty"`
	EndTime   *time.Time `json:"end_time,omitempty"`
	Limit     int        `json:"limit,omitempty"`
	Offset    int        `json:"offset,omitempty"`
	OrderBy   string     `json:"order_by,omitempty"`
	OrderDesc bool       `json:"order_desc,omitempty"`
}

// RequestLogFilter for querying request logs
type RequestLogFilter struct {
	Method      *string    `json:"method,omitempty"`
	Path        *string    `json:"path,omitempty"`
	PathPrefix  *string    `json:"path_prefix,omitempty"`
	StatusCode  *int       `json:"status_code,omitempty"`
	MinExecTime *int64     `json:"min_exec_time,omitempty"`
	MaxExecTime *int64     `json:"max_exec_time,omitempty"`
	UserID      *string    `json:"user_id,omitempty"`
	UserIP      *string    `json:"user_ip,omitempty"`
	TraceID     *string    `json:"trace_id,omitempty"`
	HasError    *bool      `json:"has_error,omitempty"`
	StartTime   *time.Time `json:"start_time,omitempty"`
	EndTime     *time.Time `json:"end_time,omitempty"`
	Limit       int        `json:"limit,omitempty"`
	Offset      int        `json:"offset,omitempty"`
	OrderBy     string     `json:"order_by,omitempty"`
	OrderDesc   bool       `json:"order_desc,omitempty"`
}

// Config holds logger configuration
type Config struct {
	Level          Level                  `json:"level"`
	Output         string                 `json:"output"` // console, database, file, multi
	Format         string                 `json:"format"` // json, text
	BufferSize     int                    `json:"buffer_size"`
	FlushInterval  time.Duration          `json:"flush_interval"`
	MaxBatchSize   int                    `json:"max_batch_size"`
	AsyncMode      bool                   `json:"async_mode"`
	IncludeStack   bool                   `json:"include_stack"`
	IncludeCaller  bool                   `json:"include_caller"`
	EnableRotation bool                   `json:"enable_rotation"`
	MaxSize        int64                  `json:"max_size"`    // MB for file rotation
	MaxAge         int                    `json:"max_age"`     // days for file rotation
	MaxBackups     int                    `json:"max_backups"` // number of backup files
	FilePath       string                 `json:"file_path"`
	Extra          map[string]interface{} `json:"extra"`
}

// New creates a new logger instance based on the output type
func New(config Config) (Logger, error) {
	switch config.Output {
	case "console":
		return NewConsole(config)
	case "database":
		return nil, ErrDatabaseRequired
	case "file":
		return NewFile(config)
	case "multi":
		return nil, ErrNotImplemented
	default:
		return NewConsole(config)
	}
}

// NewWithDatabase creates a logger with database support
func NewWithDatabase(config Config, db interface{}) (Logger, error) {
	switch config.Output {
	case "database":
		return NewDatabase(config, db)
	case "multi":
		// Can combine database with other loggers
		return NewMulti(config, db)
	default:
		return New(config)
	}
}

// Helper functions for creating fields

// String creates a string field
func String(key string, value string) Field {
	return Field{Key: key, Value: value}
}

// Int creates an int field
func Int(key string, value int) Field {
	return Field{Key: key, Value: value}
}

// Int64 creates an int64 field
func Int64(key string, value int64) Field {
	return Field{Key: key, Value: value}
}

// Float64 creates a float64 field
func Float64(key string, value float64) Field {
	return Field{Key: key, Value: value}
}

// Bool creates a bool field
func Bool(key string, value bool) Field {
	return Field{Key: key, Value: value}
}

// Time creates a time field
func Time(key string, value time.Time) Field {
	return Field{Key: key, Value: value}
}

// Duration creates a duration field
func Duration(key string, value time.Duration) Field {
	return Field{Key: key, Value: value.String()}
}

// Err creates an error field
func Err(err error) Field {
	if err == nil {
		return Field{Key: "error", Value: nil}
	}
	return Field{Key: "error", Value: err.Error()}
}

// Any creates a field with any value
func Any(key string, value interface{}) Field {
	return Field{Key: key, Value: value}
}

// TraceID creates a trace ID field
func TraceID(id string) Field {
	return Field{Key: "trace_id", Value: id}
}

// UserID creates a user ID field
func UserID(id string) Field {
	return Field{Key: "user_id", Value: id}
}

// Stack creates a stack trace field
func Stack() Field {
	return Field{Key: "stack", Value: getStackTrace()}
}

// ParseLevel parses a string into a Level
func ParseLevel(s string) (Level, error) {
	switch s {
	case "DEBUG", "debug":
		return LevelDebug, nil
	case "INFO", "info":
		return LevelInfo, nil
	case "WARN", "warn", "WARNING", "warning":
		return LevelWarn, nil
	case "ERROR", "error":
		return LevelError, nil
	case "FATAL", "fatal":
		return LevelFatal, nil
	default:
		return LevelInfo, ErrInvalidLevel
	}
}

// ShouldLog returns true if the message level should be logged
func ShouldLog(messageLevel, configLevel Level) bool {
	return levelValue(messageLevel) >= levelValue(configLevel)
}

func levelValue(level Level) int {
	switch level {
	case LevelDebug:
		return 0
	case LevelInfo:
		return 1
	case LevelWarn:
		return 2
	case LevelError:
		return 3
	case LevelFatal:
		return 4
	default:
		return 1
	}
}
