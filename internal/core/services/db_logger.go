package services

import (
	"context"
	"database/sql"
	"encoding/json"
	"net/http"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// DBLogger implements the logger.Logger interface and writes to database
type DBLogger struct {
	db       *sql.DB
	logChan  chan *logger.LogModel
	reqChan  chan *logger.RequestLogModel
	shutdown chan bool
}

// NewDBLogger creates a new database logger with buffered async writing
func NewDBLogger(db *sql.DB) *DBLogger {
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
	ticker := apptime.NewTicker(1 * apptime.Second) // Batch write every second
	batch := make([]*logger.LogModel, 0, 10)

	for {
		select {
		case log := <-l.logChan:
			batch = append(batch, log)
			// Write immediately if batch is full
			if len(batch) >= 10 {
				if len(batch) > 0 {
					l.insertLogs(batch)
					batch = batch[:0] // Clear batch
				}
			}
		case <-ticker.C:
			// Write any pending logs
			if len(batch) > 0 {
				l.insertLogs(batch)
				batch = batch[:0] // Clear batch
			}
		case <-l.shutdown:
			// Write any remaining logs before shutdown
			if len(batch) > 0 {
				l.insertLogs(batch)
			}
			return
		}
	}
}

// insertLogs inserts a batch of logs into the database
func (l *DBLogger) insertLogs(logs []*logger.LogModel) {
	for _, log := range logs {
		var userID, traceID interface{} = nil, nil
		if log.UserID != nil {
			userID = *log.UserID
		}
		if log.TraceID != nil {
			traceID = *log.TraceID
		}
		_, _ = l.db.Exec(
			`INSERT INTO sys_logs (id, level, message, fields, user_id, trace_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)`,
			log.ID.String(), log.Level, log.Message, string(log.Fields), userID, traceID, log.CreatedAt,
		)
	}
}

// requestLogWriter processes request logs in batches
func (l *DBLogger) requestLogWriter() {
	ticker := apptime.NewTicker(1 * apptime.Second) // Batch write every second
	batch := make([]*logger.RequestLogModel, 0, 10)

	for {
		select {
		case log := <-l.reqChan:
			batch = append(batch, log)
			// Write immediately if batch is full
			if len(batch) >= 10 {
				if len(batch) > 0 {
					l.insertRequestLogs(batch)
					batch = batch[:0] // Clear batch
				}
			}
		case <-ticker.C:
			// Write any pending logs
			if len(batch) > 0 {
				l.insertRequestLogs(batch)
				batch = batch[:0] // Clear batch
			}
		case <-l.shutdown:
			// Write any remaining logs before shutdown
			if len(batch) > 0 {
				l.insertRequestLogs(batch)
			}
			return
		}
	}
}

// insertRequestLogs inserts a batch of request logs into the database
func (l *DBLogger) insertRequestLogs(logs []*logger.RequestLogModel) {
	for _, log := range logs {
		var userAgent, userID, errorStr interface{} = nil, nil, nil
		if log.UserAgent != nil {
			userAgent = *log.UserAgent
		}
		if log.UserID != nil {
			userID = *log.UserID
		}
		if log.Error != nil {
			errorStr = *log.Error
		}
		_, _ = l.db.Exec(
			`INSERT INTO sys_request_logs (id, level, method, path, status_code, exec_time_ms, user_ip, user_agent, user_id, error, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			log.ID.String(), log.Level, log.Method, log.Path, log.StatusCode, log.ExecTimeMs, log.UserIP, userAgent, userID, errorStr, log.CreatedAt,
		)
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
		Fields:    json.RawMessage(fieldsJSON),
		UserID:    userID,
		TraceID:   traceID,
		CreatedAt: apptime.NowTime(),
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
	apptime.Sleep(100 * apptime.Millisecond)

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
		CreatedAt:  apptime.NewTime(req.CreatedAt),
	}

	var userAgent, userID, errorStr interface{} = nil, nil, nil
	if requestLog.UserAgent != nil {
		userAgent = *requestLog.UserAgent
	}
	if requestLog.UserID != nil {
		userID = *requestLog.UserID
	}
	if requestLog.Error != nil {
		errorStr = *requestLog.Error
	}

	_, err := l.db.Exec(
		`INSERT INTO sys_request_logs (id, level, method, path, status_code, exec_time_ms, user_ip, user_agent, user_id, error, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		requestLog.ID.String(), requestLog.Level, requestLog.Method, requestLog.Path, requestLog.StatusCode, requestLog.ExecTimeMs, requestLog.UserIP, userAgent, userID, errorStr, requestLog.CreatedAt,
	)
	return err
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
func (l *DBLogger) LogHTTPRequest(ctx context.Context, method, path string, statusCode int, duration apptime.Duration, userIP, userAgent string, userID *string, err error) {
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
		CreatedAt:  apptime.NowTime(),
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
			start := apptime.NowTime()

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
			duration := apptime.Since(start)

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
