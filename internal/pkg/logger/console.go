package logger

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sync"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// ConsoleLogger implements Logger interface for console output
type ConsoleLogger struct {
	config Config
	writer io.Writer
	mu     sync.Mutex
	fields map[string]interface{}
}

// NewConsole creates a new console logger
func NewConsole(config Config) (*ConsoleLogger, error) {
	writer := os.Stdout
	if config.Output == "stderr" {
		writer = os.Stderr
	}

	return &ConsoleLogger{
		config: config,
		writer: writer,
		fields: make(map[string]interface{}),
	}, nil
}

// Debug logs a debug message
func (l *ConsoleLogger) Debug(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelDebug, l.config.Level) {
		return
	}
	l.log(ctx, LevelDebug, message, fields...)
}

// Info logs an info message
func (l *ConsoleLogger) Info(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelInfo, l.config.Level) {
		return
	}
	l.log(ctx, LevelInfo, message, fields...)
}

// Warn logs a warning message
func (l *ConsoleLogger) Warn(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelWarn, l.config.Level) {
		return
	}
	l.log(ctx, LevelWarn, message, fields...)
}

// Error logs an error message
func (l *ConsoleLogger) Error(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelError, l.config.Level) {
		return
	}
	l.log(ctx, LevelError, message, fields...)
}

// Fatal logs a fatal message and exits
func (l *ConsoleLogger) Fatal(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelFatal, message, fields...)
	os.Exit(1)
}

// With creates a new logger with additional fields
func (l *ConsoleLogger) With(fields ...Field) Logger {
	newLogger := &ConsoleLogger{
		config: l.config,
		writer: l.writer,
		fields: make(map[string]interface{}),
	}

	// Copy existing fields
	for k, v := range l.fields {
		newLogger.fields[k] = v
	}

	// Add new fields
	for _, field := range fields {
		newLogger.fields[field.Key] = field.Value
	}

	return newLogger
}

// WithContext creates a new logger with context
func (l *ConsoleLogger) WithContext(ctx context.Context) Logger {
	// Extract common context values
	fields := []Field{}

	if traceID := ctx.Value("trace_id"); traceID != nil {
		fields = append(fields, TraceID(traceID.(string)))
	}

	if userID := ctx.Value("user_id"); userID != nil {
		fields = append(fields, UserID(userID.(string)))
	}

	return l.With(fields...)
}

// LogRequest logs an HTTP request
func (l *ConsoleLogger) LogRequest(ctx context.Context, req *RequestLog) error {
	fields := []Field{
		String("method", req.Method),
		String("path", req.Path),
		Int("status_code", req.StatusCode),
		Int64("exec_time_ms", req.ExecTimeMs),
		String("user_ip", req.UserIP),
	}

	if req.UserAgent != "" {
		fields = append(fields, String("user_agent", req.UserAgent))
	}

	if req.UserID != nil {
		fields = append(fields, String("user_id", *req.UserID))
	}

	if req.TraceID != nil {
		fields = append(fields, String("trace_id", *req.TraceID))
	}

	if req.Error != nil {
		fields = append(fields, String("error", *req.Error))
	}

	level := LevelInfo
	if req.StatusCode >= 400 && req.StatusCode < 500 {
		level = LevelWarn
	} else if req.StatusCode >= 500 {
		level = LevelError
	}

	message := fmt.Sprintf("%s %s %d %dms", req.Method, req.Path, req.StatusCode, req.ExecTimeMs)
	l.log(ctx, level, message, fields...)

	return nil
}

// GetLogs returns empty list for console logger
func (l *ConsoleLogger) GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error) {
	return []*Log{}, nil
}

// GetRequestLogs returns empty list for console logger
func (l *ConsoleLogger) GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error) {
	return []*RequestLog{}, nil
}

// Flush does nothing for console logger
func (l *ConsoleLogger) Flush() error {
	return nil
}

// Close does nothing for console logger
func (l *ConsoleLogger) Close() error {
	return nil
}

// log writes the log message
func (l *ConsoleLogger) log(ctx context.Context, level Level, message string, fields ...Field) {
	l.mu.Lock()
	defer l.mu.Unlock()

	// Build field map
	fieldMap := make(map[string]interface{})
	for k, v := range l.fields {
		fieldMap[k] = v
	}
	for _, field := range fields {
		fieldMap[field.Key] = field.Value
	}

	// Extract context values
	if traceID := ctx.Value("trace_id"); traceID != nil {
		fieldMap["trace_id"] = traceID
	}
	if userID := ctx.Value("user_id"); userID != nil {
		fieldMap["user_id"] = userID
	}

	// Include caller info
	if l.config.IncludeCaller {
		if caller := getCaller(); caller != "" {
			fieldMap["caller"] = caller
		}
	}

	// Include stack trace for errors
	if l.config.IncludeStack && level >= LevelError {
		fieldMap["stack"] = getStackTrace()
	}

	// Format output
	if l.config.Format == "json" {
		l.writeJSON(level, message, fieldMap)
	} else {
		l.writeText(level, message, fieldMap)
	}
}

// writeJSON writes JSON formatted log
func (l *ConsoleLogger) writeJSON(level Level, message string, fields map[string]interface{}) {
	entry := map[string]interface{}{
		"level":     level,
		"message":   message,
		"timestamp": apptime.NowTime().UTC().Format(apptime.TimeFormat),
	}

	for k, v := range fields {
		entry[k] = v
	}

	data, _ := json.Marshal(entry)
	fmt.Fprintf(l.writer, "%s\n", data)
}

// writeText writes text formatted log
func (l *ConsoleLogger) writeText(level Level, message string, fields map[string]interface{}) {
	// Color codes for levels
	var levelStr string
	switch level {
	case LevelDebug:
		levelStr = "\033[36mDEBUG\033[0m" // Cyan
	case LevelInfo:
		levelStr = "\033[32mINFO\033[0m" // Green
	case LevelWarn:
		levelStr = "\033[33mWARN\033[0m" // Yellow
	case LevelError:
		levelStr = "\033[31mERROR\033[0m" // Red
	case LevelFatal:
		levelStr = "\033[35mFATAL\033[0m" // Magenta
	default:
		levelStr = string(level)
	}

	timestamp := apptime.NowTime().Format("2006-01-02 15:04:05")
	fmt.Fprintf(l.writer, "%s [%s] %s", timestamp, levelStr, message)

	// Add fields
	if len(fields) > 0 {
		fmt.Fprint(l.writer, " ")
		for k, v := range fields {
			fmt.Fprintf(l.writer, "%s=%v ", k, v)
		}
	}

	fmt.Fprintln(l.writer)
}
