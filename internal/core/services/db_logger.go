package services

import (
	"context"
	"encoding/json"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/database"
	"gorm.io/datatypes"
)

// DBLogger implements the logger.Logger interface and writes to database
type DBLogger struct {
	db       *database.DB
	logChan  chan *logger.LogModel
	reqChan  chan *logger.RequestLogModel
	shutdown chan bool
}

// NewDBLogger creates a new database logger with buffered async writing
func NewDBLogger(db *database.DB) *DBLogger {
	l := &DBLogger{
		db:       db,
		logChan:  make(chan *logger.LogModel, 100),        // Buffer up to 100 logs
		reqChan:  make(chan *logger.RequestLogModel, 100), // Buffer up to 100 request logs
		shutdown: make(chan bool),
	}

	// Start background workers for batch writing
	go l.logWriter()
	go l.requestLogWriter()

	return l
}

// logWriter processes logs in batches for better performance
func (l *DBLogger) logWriter() {
	ticker := time.NewTicker(1 * time.Second) // Batch write every second
	batch := make([]*logger.LogModel, 0, 10)

	for {
		select {
		case log := <-l.logChan:
			batch = append(batch, log)
			// Write immediately if batch is full
			if len(batch) >= 10 {
				if len(batch) > 0 {
					l.db.CreateInBatches(batch, len(batch))
					batch = batch[:0] // Clear batch
				}
			}
		case <-ticker.C:
			// Write any pending logs
			if len(batch) > 0 {
				l.db.CreateInBatches(batch, len(batch))
				batch = batch[:0] // Clear batch
			}
		case <-l.shutdown:
			// Write any remaining logs before shutdown
			if len(batch) > 0 {
				l.db.CreateInBatches(batch, len(batch))
			}
			return
		}
	}
}

// requestLogWriter processes request logs in batches
func (l *DBLogger) requestLogWriter() {
	ticker := time.NewTicker(1 * time.Second) // Batch write every second
	batch := make([]*logger.RequestLogModel, 0, 10)

	for {
		select {
		case log := <-l.reqChan:
			batch = append(batch, log)
			// Write immediately if batch is full
			if len(batch) >= 10 {
				if len(batch) > 0 {
					l.db.CreateInBatches(batch, len(batch))
					batch = batch[:0] // Clear batch
				}
			}
		case <-ticker.C:
			// Write any pending logs
			if len(batch) > 0 {
				l.db.CreateInBatches(batch, len(batch))
				batch = batch[:0] // Clear batch
			}
		case <-l.shutdown:
			// Write any remaining logs before shutdown
			if len(batch) > 0 {
				l.db.CreateInBatches(batch, len(batch))
			}
			return
		}
	}
}

// Log implements the generic log method
func (l *DBLogger) Log(ctx context.Context, level logger.Level, msg string, fields ...logger.Field) {
	// Convert fields to JSON
	fieldsMap := make(map[string]interface{})
	for _, field := range fields {
		fieldsMap[field.Key] = field.Value
	}

	fieldsJSON, _ := json.Marshal(fieldsMap)

	// Extract user ID and trace ID from fields if present
	var userID, traceID *string
	if uid, ok := fieldsMap["user_id"].(string); ok {
		userID = &uid
	}
	if tid, ok := fieldsMap["trace_id"].(string); ok {
		traceID = &tid
	}

	// Convert level to string
	levelStr := "info"
	switch level {
	case logger.LevelDebug:
		levelStr = "debug"
	case logger.LevelInfo:
		levelStr = "info"
	case logger.LevelWarn:
		levelStr = "warn"
	case logger.LevelError:
		levelStr = "error"
	case logger.LevelFatal:
		levelStr = "fatal"
	}

	// Create log entry
	logEntry := &logger.LogModel{
		ID:        uuid.New(),
		Level:     levelStr,
		Message:   msg,
		Fields:    datatypes.JSON(fieldsJSON),
		UserID:    userID,
		TraceID:   traceID,
		CreatedAt: time.Now(),
	}

	// Send to channel for async batch processing (non-blocking)
	select {
	case l.logChan <- logEntry:
		// Successfully sent
	default:
		// Channel is full, drop the log to avoid blocking
		// In production, you might want to handle this differently
	}
}

// Debug logs a debug message
func (l *DBLogger) Debug(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelDebug, msg, fields...)
}

// Info logs an info message
func (l *DBLogger) Info(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelInfo, msg, fields...)
}

// Warn logs a warning message
func (l *DBLogger) Warn(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelWarn, msg, fields...)
}

// Error logs an error message
func (l *DBLogger) Error(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelError, msg, fields...)
}

// Fatal logs a fatal message (but doesn't exit)
func (l *DBLogger) Fatal(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelFatal, msg, fields...)
}

// With creates a new logger with additional fields
func (l *DBLogger) With(fields ...logger.Field) logger.Logger {
	// For simplicity, return the same logger
	// In a real implementation, you might want to store fields
	return l
}

// Close closes the logger and flushes any pending logs
func (l *DBLogger) Close() error {
	// Signal shutdown to both workers
	close(l.shutdown)

	// Give workers time to flush pending logs
	time.Sleep(100 * time.Millisecond)

	return nil
}

// Flush flushes any buffered logs (no-op for database logger)
func (l *DBLogger) Flush() error {
	return nil
}

// WithContext creates a new logger with context
func (l *DBLogger) WithContext(ctx context.Context) logger.Logger {
	return l
}

// LogRequest logs an HTTP request
func (l *DBLogger) LogRequest(ctx context.Context, req *logger.RequestLog) error {
	requestLog := &logger.RequestLogModel{
		ID:         uuid.New(),
		Level:      string(req.Level),
		Method:     req.Method,
		Path:       req.Path,
		StatusCode: req.StatusCode,
		ExecTimeMs: req.ExecTimeMs,
		UserIP:     req.UserIP,
		UserAgent:  &req.UserAgent,
		UserID:     req.UserID,
		Error:      req.Error,
		CreatedAt:  req.CreatedAt,
	}

	return l.db.Create(requestLog).Error
}

// GetLogs retrieves logs based on filter
func (l *DBLogger) GetLogs(ctx context.Context, filter logger.LogFilter) ([]*logger.Log, error) {
	// Not implemented for now - return empty
	return []*logger.Log{}, nil
}

// GetRequestLogs retrieves request logs based on filter
func (l *DBLogger) GetRequestLogs(ctx context.Context, filter logger.RequestLogFilter) ([]*logger.RequestLog, error) {
	// Not implemented for now - return empty
	return []*logger.RequestLog{}, nil
}

// LogHTTPRequest logs an HTTP request to the request_logs table
func (l *DBLogger) LogHTTPRequest(ctx context.Context, method, path string, statusCode int, duration time.Duration, userIP, userAgent string, userID *string, err error) {
	var errorStr *string
	if err != nil {
		errMsg := err.Error()
		errorStr = &errMsg
	}

	// Determine level based on status code
	level := "info"
	if statusCode >= 500 {
		level = "error"
	} else if statusCode >= 400 {
		level = "warning"
	}

	requestLog := &logger.RequestLogModel{
		ID:         uuid.New(),
		Level:      level,
		Method:     method,
		Path:       path,
		StatusCode: statusCode,
		ExecTimeMs: duration.Milliseconds(),
		UserIP:     userIP,
		UserAgent:  &userAgent,
		UserID:     userID,
		Error:      errorStr,
		CreatedAt:  time.Now(),
	}

	// Send to channel for async batch processing (non-blocking)
	select {
	case l.reqChan <- requestLog:
		// Successfully sent
	default:
		// Channel is full, drop the log to avoid blocking
	}
}

// HTTPLoggingMiddleware creates a middleware that logs HTTP requests to database
func HTTPLoggingMiddleware(dbLogger *DBLogger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()

			// Create a response writer wrapper to capture status code
			wrapped := &responseWriter{
				ResponseWriter: w,
				statusCode:     http.StatusOK,
			}

			// Get user ID from context if available
			var userID *string
			if uid, ok := r.Context().Value("user_id").(string); ok {
				userID = &uid
			}

			// Process request
			next.ServeHTTP(wrapped, r)

			// Calculate duration
			duration := time.Since(start)

			// Log the request to database
			dbLogger.LogHTTPRequest(
				r.Context(),
				r.Method,
				r.URL.Path,
				wrapped.statusCode,
				duration,
				r.RemoteAddr,
				r.UserAgent(),
				userID,
				nil,
			)
		})
	}
}

type responseWriter struct {
	http.ResponseWriter
	statusCode int
	written    bool
}

func (rw *responseWriter) WriteHeader(code int) {
	if !rw.written {
		rw.statusCode = code
		rw.ResponseWriter.WriteHeader(code)
		rw.written = true
	}
}

func (rw *responseWriter) Write(b []byte) (int, error) {
	if !rw.written {
		rw.WriteHeader(http.StatusOK)
	}
	return rw.ResponseWriter.Write(b)
}
