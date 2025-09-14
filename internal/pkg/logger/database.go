package logger

import (
	"context"
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"time"

	"github.com/suppers-ai/solobase/internal/pkg/database"
)

// DatabaseLogger implements Logger interface with database storage
type DatabaseLogger struct {
	config  Config
	db      database.Database
	level   Level
	fields  map[string]interface{}
	buffer  chan interface{}
	wg      sync.WaitGroup
	closing chan bool
	closed  bool
	mu      sync.RWMutex
}

// NewDatabase creates a new database logger
func NewDatabase(config Config, db interface{}) (*DatabaseLogger, error) {
	dbConn, ok := db.(database.Database)
	if !ok {
		return nil, ErrDatabaseRequired
	}

	logger := &DatabaseLogger{
		config:  config,
		db:      dbConn,
		level:   config.Level,
		fields:  make(map[string]interface{}),
		buffer:  make(chan interface{}, config.BufferSize),
		closing: make(chan bool),
	}

	if config.BufferSize <= 0 {
		config.BufferSize = 1000
	}

	if config.FlushInterval <= 0 {
		config.FlushInterval = 5 * time.Second
	}

	if config.MaxBatchSize <= 0 {
		config.MaxBatchSize = 100
	}

	// Run migrations
	if err := logger.runMigrations(context.Background()); err != nil {
		return nil, fmt.Errorf("%w: %v", ErrMigrationFailed, err)
	}

	// Start background worker if async mode
	if config.AsyncMode {
		logger.wg.Add(1)
		go logger.worker()
	}

	return logger, nil
}

// Debug logs a debug message
func (l *DatabaseLogger) Debug(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelDebug, message, fields...)
}

// Info logs an info message
func (l *DatabaseLogger) Info(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelInfo, message, fields...)
}

// Warn logs a warning message
func (l *DatabaseLogger) Warn(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelWarn, message, fields...)
}

// Error logs an error message
func (l *DatabaseLogger) Error(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelError, message, fields...)
}

// Fatal logs a fatal message
func (l *DatabaseLogger) Fatal(ctx context.Context, message string, fields ...Field) {
	l.log(ctx, LevelFatal, message, fields...)
	l.Flush()
}

// With creates a new logger with additional fields
func (l *DatabaseLogger) With(fields ...Field) Logger {
	l.mu.RLock()
	defer l.mu.RUnlock()

	newFields := make(map[string]interface{})
	for k, v := range l.fields {
		newFields[k] = v
	}

	for _, field := range fields {
		newFields[field.Key] = field.Value
	}

	return &DatabaseLogger{
		config:  l.config,
		db:      l.db,
		level:   l.level,
		fields:  newFields,
		buffer:  l.buffer,
		closing: l.closing,
		wg:      l.wg,
	}
}

// WithContext creates a new logger with context
func (l *DatabaseLogger) WithContext(ctx context.Context) Logger {
	// Extract common context values
	fields := []Field{}

	if traceID := ctx.Value("trace_id"); traceID != nil {
		fields = append(fields, TraceID(fmt.Sprintf("%v", traceID)))
	}

	if userID := ctx.Value("user_id"); userID != nil {
		fields = append(fields, UserID(fmt.Sprintf("%v", userID)))
	}

	return l.With(fields...)
}

// LogRequest logs an HTTP request
func (l *DatabaseLogger) LogRequest(ctx context.Context, req *RequestLog) error {
	if !ShouldLog(req.Level, l.level) {
		return nil
	}

	if l.config.AsyncMode {
		select {
		case l.buffer <- req:
			return nil
		default:
			// Buffer full, write directly
			return l.insertRequestLog(ctx, req)
		}
	}

	return l.insertRequestLog(ctx, req)
}

// GetLogs queries general logs
func (l *DatabaseLogger) GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error) {
	query := `
		SELECT id, level, message, fields, user_id, trace_id, created_at
		FROM logger.logs
		WHERE 1=1
	`

	args := []interface{}{}
	argCount := 0

	if filter.Level != nil {
		argCount++
		query += fmt.Sprintf(" AND level = $%d", argCount)
		args = append(args, *filter.Level)
	}

	if filter.UserID != nil {
		argCount++
		query += fmt.Sprintf(" AND user_id = $%d", argCount)
		args = append(args, *filter.UserID)
	}

	if filter.TraceID != nil {
		argCount++
		query += fmt.Sprintf(" AND trace_id = $%d", argCount)
		args = append(args, *filter.TraceID)
	}

	if filter.StartTime != nil {
		argCount++
		query += fmt.Sprintf(" AND created_at >= $%d", argCount)
		args = append(args, *filter.StartTime)
	}

	if filter.EndTime != nil {
		argCount++
		query += fmt.Sprintf(" AND created_at <= $%d", argCount)
		args = append(args, *filter.EndTime)
	}

	// Order by
	orderBy := "created_at"
	if filter.OrderBy != "" {
		orderBy = filter.OrderBy
	}

	if filter.OrderDesc {
		query += fmt.Sprintf(" ORDER BY %s DESC", orderBy)
	} else {
		query += fmt.Sprintf(" ORDER BY %s ASC", orderBy)
	}

	// Limit and offset
	if filter.Limit > 0 {
		argCount++
		query += fmt.Sprintf(" LIMIT $%d", argCount)
		args = append(args, filter.Limit)
	}

	if filter.Offset > 0 {
		argCount++
		query += fmt.Sprintf(" OFFSET $%d", argCount)
		args = append(args, filter.Offset)
	}

	var logs []*Log
	err := l.db.Select(ctx, &logs, query, args...)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrQueryFailed, err)
	}

	return logs, nil
}

// GetRequestLogs queries request logs
func (l *DatabaseLogger) GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error) {
	query := `
		SELECT id, level, method, path, query, status_code, exec_time_ms,
		       user_ip, user_agent, user_id, trace_id, error,
		       request_body, response_body, headers, created_at
		FROM logger.request_logs
		WHERE 1=1
	`

	args := []interface{}{}
	argCount := 0

	if filter.Method != nil {
		argCount++
		query += fmt.Sprintf(" AND method = $%d", argCount)
		args = append(args, *filter.Method)
	}

	if filter.Path != nil {
		argCount++
		query += fmt.Sprintf(" AND path = $%d", argCount)
		args = append(args, *filter.Path)
	}

	if filter.PathPrefix != nil {
		argCount++
		query += fmt.Sprintf(" AND path LIKE $%d", argCount)
		args = append(args, *filter.PathPrefix+"%")
	}

	if filter.StatusCode != nil {
		argCount++
		query += fmt.Sprintf(" AND status_code = $%d", argCount)
		args = append(args, *filter.StatusCode)
	}

	if filter.MinExecTime != nil {
		argCount++
		query += fmt.Sprintf(" AND exec_time_ms >= $%d", argCount)
		args = append(args, *filter.MinExecTime)
	}

	if filter.MaxExecTime != nil {
		argCount++
		query += fmt.Sprintf(" AND exec_time_ms <= $%d", argCount)
		args = append(args, *filter.MaxExecTime)
	}

	if filter.UserID != nil {
		argCount++
		query += fmt.Sprintf(" AND user_id = $%d", argCount)
		args = append(args, *filter.UserID)
	}

	if filter.UserIP != nil {
		argCount++
		query += fmt.Sprintf(" AND user_ip = $%d", argCount)
		args = append(args, *filter.UserIP)
	}

	if filter.TraceID != nil {
		argCount++
		query += fmt.Sprintf(" AND trace_id = $%d", argCount)
		args = append(args, *filter.TraceID)
	}

	if filter.HasError != nil {
		if *filter.HasError {
			query += " AND error IS NOT NULL"
		} else {
			query += " AND error IS NULL"
		}
	}

	if filter.StartTime != nil {
		argCount++
		query += fmt.Sprintf(" AND created_at >= $%d", argCount)
		args = append(args, *filter.StartTime)
	}

	if filter.EndTime != nil {
		argCount++
		query += fmt.Sprintf(" AND created_at <= $%d", argCount)
		args = append(args, *filter.EndTime)
	}

	// Order by
	orderBy := "created_at"
	if filter.OrderBy != "" {
		orderBy = filter.OrderBy
	}

	if filter.OrderDesc {
		query += fmt.Sprintf(" ORDER BY %s DESC", orderBy)
	} else {
		query += fmt.Sprintf(" ORDER BY %s ASC", orderBy)
	}

	// Limit and offset
	if filter.Limit > 0 {
		argCount++
		query += fmt.Sprintf(" LIMIT $%d", argCount)
		args = append(args, filter.Limit)
	}

	if filter.Offset > 0 {
		argCount++
		query += fmt.Sprintf(" OFFSET $%d", argCount)
		args = append(args, filter.Offset)
	}

	var logs []*RequestLog
	err := l.db.Select(ctx, &logs, query, args...)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrQueryFailed, err)
	}

	return logs, nil
}

// Flush flushes any buffered logs
func (l *DatabaseLogger) Flush() error {
	if !l.config.AsyncMode {
		return nil
	}

	// Process all items in buffer
	for {
		select {
		case item := <-l.buffer:
			l.processItem(context.Background(), item)
		default:
			return nil
		}
	}
}

// Close closes the logger
func (l *DatabaseLogger) Close() error {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.closed {
		return nil
	}

	l.closed = true

	if l.config.AsyncMode {
		close(l.closing)
		l.wg.Wait()
	}

	return l.Flush()
}

// Internal methods

func (l *DatabaseLogger) log(ctx context.Context, level Level, message string, fields ...Field) {
	if !ShouldLog(level, l.level) {
		return
	}

	log := &Log{
		Level:     level,
		Message:   message,
		Fields:    l.mergeFields(fields...),
		Timestamp: time.Now(),
	}

	// Extract special fields
	if userID, ok := log.Fields["user_id"].(string); ok {
		log.UserID = &userID
		delete(log.Fields, "user_id")
	}

	if traceID, ok := log.Fields["trace_id"].(string); ok {
		log.TraceID = &traceID
		delete(log.Fields, "trace_id")
	}

	// Add caller info if configured
	if l.config.IncludeCaller {
		if pc, file, line, ok := runtime.Caller(2); ok {
			fn := runtime.FuncForPC(pc)
			log.Fields["caller"] = fmt.Sprintf("%s:%d %s", file, line, fn.Name())
		}
	}

	// Add stack trace for error and fatal levels
	if l.config.IncludeStack && (level == LevelError || level == LevelFatal) {
		log.Fields["stack"] = getStackTrace()
	}

	if l.config.AsyncMode {
		select {
		case l.buffer <- log:
			return
		default:
			// Buffer full, write directly
			l.insertLog(ctx, log)
		}
	} else {
		l.insertLog(ctx, log)
	}
}

func (l *DatabaseLogger) mergeFields(fields ...Field) map[string]interface{} {
	result := make(map[string]interface{})

	// Copy existing fields
	l.mu.RLock()
	for k, v := range l.fields {
		result[k] = v
	}
	l.mu.RUnlock()

	// Add new fields
	for _, field := range fields {
		result[field.Key] = field.Value
	}

	return result
}

func (l *DatabaseLogger) insertLog(ctx context.Context, log *Log) error {
	fieldsJSON, err := json.Marshal(log.Fields)
	if err != nil {
		return err
	}

	query := `
		INSERT INTO logger.logs (level, message, fields, user_id, trace_id, created_at)
		VALUES ($1, $2, $3, $4, $5, $6)
	`

	_, err = l.db.Exec(ctx, query, log.Level, log.Message, fieldsJSON, log.UserID, log.TraceID, log.Timestamp)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrInsertFailed, err)
	}

	return nil
}

func (l *DatabaseLogger) insertRequestLog(ctx context.Context, req *RequestLog) error {
	query := `
		INSERT INTO logger.request_logs (
			level, method, path, query, status_code, exec_time_ms,
			user_ip, user_agent, user_id, trace_id, error,
			request_body, response_body, headers, created_at
		) VALUES (
			$1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
		)
	`

	_, err := l.db.Exec(ctx, query,
		req.Level, req.Method, req.Path, req.Query, req.StatusCode, req.ExecTimeMs,
		req.UserIP, req.UserAgent, req.UserID, req.TraceID, req.Error,
		req.RequestBody, req.ResponseBody, req.Headers, req.CreatedAt,
	)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrInsertFailed, err)
	}

	return nil
}

func (l *DatabaseLogger) worker() {
	defer l.wg.Done()

	ticker := time.NewTicker(l.config.FlushInterval)
	defer ticker.Stop()

	batch := make([]interface{}, 0, l.config.MaxBatchSize)

	for {
		select {
		case <-l.closing:
			// Process remaining items
			for item := range l.buffer {
				l.processItem(context.Background(), item)
			}
			return

		case item := <-l.buffer:
			batch = append(batch, item)

			if len(batch) >= l.config.MaxBatchSize {
				l.processBatch(context.Background(), batch)
				batch = batch[:0]
			}

		case <-ticker.C:
			if len(batch) > 0 {
				l.processBatch(context.Background(), batch)
				batch = batch[:0]
			}
		}
	}
}

func (l *DatabaseLogger) processBatch(ctx context.Context, batch []interface{}) {
	for _, item := range batch {
		l.processItem(ctx, item)
	}
}

func (l *DatabaseLogger) processItem(ctx context.Context, item interface{}) {
	switch v := item.(type) {
	case *Log:
		l.insertLog(ctx, v)
	case *RequestLog:
		l.insertRequestLog(ctx, v)
	}
}

func (l *DatabaseLogger) runMigrations(ctx context.Context) error {
	// Create schema
	schemaQuery := `CREATE SCHEMA IF NOT EXISTS logger`
	if _, err := l.db.Exec(ctx, schemaQuery); err != nil {
		return err
	}

	// Create logs table
	logsTableQuery := `
		CREATE TABLE IF NOT EXISTS logger.logs (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			level TEXT NOT NULL DEFAULT 'INFO',
			message TEXT NOT NULL,
			fields JSONB,
			user_id TEXT,
			trace_id TEXT,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT timezone('utc'::text, now()) NOT NULL
		)
	`
	if _, err := l.db.Exec(ctx, logsTableQuery); err != nil {
		return err
	}

	// Create request_logs table
	requestLogsTableQuery := `
		CREATE TABLE IF NOT EXISTS logger.request_logs (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			level TEXT NOT NULL DEFAULT 'INFO',
			method TEXT NOT NULL,
			path TEXT NOT NULL,
			query TEXT,
			status_code INTEGER,
			exec_time_ms INTEGER,
			user_ip TEXT,
			user_agent TEXT,
			user_id TEXT,
			trace_id TEXT,
			error TEXT,
			request_body TEXT,
			response_body TEXT,
			headers TEXT,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT timezone('utc'::text, now()) NOT NULL
		)
	`
	if _, err := l.db.Exec(ctx, requestLogsTableQuery); err != nil {
		return err
	}

	// Create indexes
	indexes := []string{
		`CREATE INDEX IF NOT EXISTS idx_logger_logs_level ON logger.logs(level)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_logs_user_id ON logger.logs(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_logs_trace_id ON logger.logs(trace_id)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_logs_created_at ON logger.logs(created_at DESC)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_method ON logger.request_logs(method)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_path ON logger.request_logs(path)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_status_code ON logger.request_logs(status_code)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_user_id ON logger.request_logs(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_user_ip ON logger.request_logs(user_ip)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_trace_id ON logger.request_logs(trace_id)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_created_at ON logger.request_logs(created_at DESC)`,
		`CREATE INDEX IF NOT EXISTS idx_logger_request_logs_exec_time ON logger.request_logs(exec_time_ms)`,
	}

	for _, index := range indexes {
		if _, err := l.db.Exec(ctx, index); err != nil {
			return err
		}
	}

	return nil
}

func getStackTrace() string {
	buf := make([]byte, 1024*64)
	n := runtime.Stack(buf, false)
	return string(buf[:n])
}
