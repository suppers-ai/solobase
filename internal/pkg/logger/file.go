//go:build !wasm

package logger

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// FileLogger implements Logger interface for file output
type FileLogger struct {
	config      Config
	file        *os.File
	mu          sync.Mutex
	fields      map[string]interface{}
	currentSize int64
	rotateIndex int
}

// NewFile creates a new file logger
func NewFile(config Config) (*FileLogger, error) {
	if config.FilePath == "" {
		config.FilePath = "app.log"
	}

	// Ensure directory exists
	dir := filepath.Dir(config.FilePath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create log directory: %w", err)
	}

	// Open file
	file, err := os.OpenFile(config.FilePath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return nil, fmt.Errorf("failed to open log file: %w", err)
	}

	// Get file size
	stat, err := file.Stat()
	if err != nil {
		file.Close()
		return nil, fmt.Errorf("failed to stat log file: %w", err)
	}

	return &FileLogger{
		config:      config,
		file:        file,
		fields:      make(map[string]interface{}),
		currentSize: stat.Size(),
	}, nil
}

// Debug logs a debug message
func (l *FileLogger) Debug(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelDebug, l.config.Level) {
		return
	}
	l.log(ctx, LevelDebug, message, fields...)
}

// Info logs an info message
func (l *FileLogger) Info(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelInfo, l.config.Level) {
		return
	}
	l.log(ctx, LevelInfo, message, fields...)
}

// Warn logs a warning message
func (l *FileLogger) Warn(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelWarn, l.config.Level) {
		return
	}
	l.log(ctx, LevelWarn, message, fields...)
}

// Error logs an error message
func (l *FileLogger) Error(ctx context.Context, message string, fields ...Field) {
	if !ShouldLog(LevelError, l.config.Level) {
		return
	}
	l.log(ctx, LevelError, message, fields...)
}

// Fatal logs a fatal message and exits
func (l *FileLogger) Fatal(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelFatal, message, fields...)
	l.Close()
	os.Exit(1)
}

// With creates a new logger with additional fields
func (l *FileLogger) With(fields ...Field) Logger {
	newLogger := &FileLogger{
		config:      l.config,
		file:        l.file,
		fields:      make(map[string]interface{}),
		currentSize: l.currentSize,
		rotateIndex: l.rotateIndex,
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
func (l *FileLogger) WithContext(ctx context.Context) Logger {
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
func (l *FileLogger) LogRequest(ctx context.Context, req *RequestLog) error {
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

// GetLogs returns empty list for file logger
func (l *FileLogger) GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error) {
	return []*Log{}, nil
}

// GetRequestLogs returns empty list for file logger
func (l *FileLogger) GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error) {
	return []*RequestLog{}, nil
}

// Flush syncs the file
func (l *FileLogger) Flush() error {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.file.Sync()
}

// Close closes the file
func (l *FileLogger) Close() error {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.file.Close()
}

// log writes the log message
func (l *FileLogger) log(ctx context.Context, level Level, message string, fields ...Field) {
	l.mu.Lock()
	defer l.mu.Unlock()

	// Check rotation
	if l.config.EnableRotation && l.config.MaxSize > 0 {
		if l.currentSize >= l.config.MaxSize*1024*1024 {
			if err := l.rotate(); err != nil {
				fmt.Fprintf(os.Stderr, "Failed to rotate log file: %v\n", err)
			}
		}
	}

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
	var data []byte
	if l.config.Format == "json" {
		entry := map[string]interface{}{
			"level":     level,
			"message":   message,
			"timestamp": apptime.NowTime().UTC().Format(apptime.TimeFormat),
		}

		for k, v := range fieldMap {
			entry[k] = v
		}

		data, _ = json.Marshal(entry)
		data = append(data, '\n')
	} else {
		timestamp := apptime.NowTime().Format("2006-01-02 15:04:05")
		text := fmt.Sprintf("%s [%s] %s", timestamp, level, message)

		if len(fieldMap) > 0 {
			text += " "
			for k, v := range fieldMap {
				text += fmt.Sprintf("%s=%v ", k, v)
			}
		}

		data = []byte(text + "\n")
	}

	// Write to file
	n, err := l.file.Write(data)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to write log: %v\n", err)
		return
	}

	l.currentSize += int64(n)
}

// rotate rotates the log file
func (l *FileLogger) rotate() error {
	// Close current file
	if err := l.file.Close(); err != nil {
		return err
	}

	// Remove oldest backup if needed
	if l.config.MaxBackups > 0 {
		oldestBackup := fmt.Sprintf("%s.%d", l.config.FilePath, l.config.MaxBackups)
		os.Remove(oldestBackup) // Ignore error

		// Shift backups
		for i := l.config.MaxBackups - 1; i > 0; i-- {
			oldName := fmt.Sprintf("%s.%d", l.config.FilePath, i)
			newName := fmt.Sprintf("%s.%d", l.config.FilePath, i+1)
			os.Rename(oldName, newName) // Ignore error
		}
	}

	// Rename current file
	backupName := fmt.Sprintf("%s.1", l.config.FilePath)
	if err := os.Rename(l.config.FilePath, backupName); err != nil {
		return err
	}

	// Open new file
	file, err := os.OpenFile(l.config.FilePath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return err
	}

	l.file = file
	l.currentSize = 0

	return nil
}

// Clean old log files based on MaxAge
func (l *FileLogger) cleanOldFiles() {
	if l.config.MaxAge <= 0 {
		return
	}

	cutoff := apptime.NowTime().AddDate(0, 0, -l.config.MaxAge)

	// Find and remove old backup files
	dir := filepath.Dir(l.config.FilePath)
	base := filepath.Base(l.config.FilePath)

	files, err := os.ReadDir(dir)
	if err != nil {
		return
	}

	for _, file := range files {
		if file.IsDir() {
			continue
		}

		name := file.Name()
		if name == base || !filepath.HasPrefix(name, base+".") {
			continue
		}

		info, err := file.Info()
		if err != nil {
			continue
		}

		if info.ModTime().Before(cutoff) {
			os.Remove(filepath.Join(dir, name))
		}
	}
}
