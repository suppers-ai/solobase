package logs

import (
	"context"
	"encoding/json"
	"time"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/logger"
	"github.com/suppers-ai/solobase/core/uuid"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

// DBLogger implements the logger.Logger interface and writes to database.
// It also handles WAFFLE message logging via LogMessage.
type DBLogger struct {
	db       database.Service
	logChan  chan map[string]any
	msgChan  chan map[string]any
	shutdown chan bool
}

// NewDBLogger creates a new database logger with buffered async writing.
func NewDBLogger(db database.Service) *DBLogger {
	l := &DBLogger{
		db:       db,
		logChan:  make(chan map[string]any, 100),
		msgChan:  make(chan map[string]any, 100),
		shutdown: make(chan bool),
	}

	go l.logWriter()
	go l.messageLogWriter()

	return l
}

// logWriter processes logs in batches for better performance.
func (l *DBLogger) logWriter() {
	ticker := apptime.NewTicker(1 * apptime.Second)
	batch := make([]map[string]any, 0, 10)

	for {
		select {
		case log := <-l.logChan:
			batch = append(batch, log)
			if len(batch) >= 10 {
				l.insertLogs(batch)
				batch = batch[:0]
			}
		case <-ticker.C:
			if len(batch) > 0 {
				l.insertLogs(batch)
				batch = batch[:0]
			}
		case <-l.shutdown:
			if len(batch) > 0 {
				l.insertLogs(batch)
			}
			return
		}
	}
}

func (l *DBLogger) insertLogs(logs []map[string]any) {
	if len(logs) == 0 {
		return
	}
	for _, logData := range logs {
		_, _ = l.db.Create(context.Background(), "sys_logs", logData)
	}
}

// messageLogWriter processes WAFFLE message logs in batches.
func (l *DBLogger) messageLogWriter() {
	ticker := apptime.NewTicker(1 * apptime.Second)
	batch := make([]map[string]any, 0, 10)

	for {
		select {
		case log := <-l.msgChan:
			batch = append(batch, log)
			if len(batch) >= 10 {
				l.insertMessageLogs(batch)
				batch = batch[:0]
			}
		case <-ticker.C:
			if len(batch) > 0 {
				l.insertMessageLogs(batch)
				batch = batch[:0]
			}
		case <-l.shutdown:
			if len(batch) > 0 {
				l.insertMessageLogs(batch)
			}
			return
		}
	}
}

func (l *DBLogger) insertMessageLogs(logs []map[string]any) {
	if len(logs) == 0 {
		return
	}
	for _, entryData := range logs {
		_, _ = l.db.Create(context.Background(), "sys_message_logs", entryData)
	}
}

// LogMessage logs a WAFFLE message execution to the message_logs table.
func (l *DBLogger) LogMessage(obsCtx waffle.ObservabilityContext, result waffle.Result, duration time.Duration) {
	// Extract useful meta keys for snapshot
	var metaSnapshot string
	if obsCtx.Message != nil {
		meta := make(map[string]string)
		for _, key := range []string{"req.action", "req.resource", "req.client.ip", "auth.user_id"} {
			if v := obsCtx.Message.GetMeta(key); v != "" {
				meta[key] = v
			}
		}
		if len(meta) > 0 {
			if data, err := json.Marshal(meta); err == nil {
				metaSnapshot = string(data)
			}
		}
	}

	// Extract error message
	var errStr string
	if result.Error != nil {
		errStr = result.Error.Message
	}

	// Extract user ID from meta
	var userID string
	if obsCtx.Message != nil {
		userID = obsCtx.Message.GetMeta("auth.user_id")
	}

	entry := map[string]any{
		"id":            uuid.New(),
		"chain_id":      obsCtx.ChainID,
		"block_name":    obsCtx.BlockName,
		"message_kind":  obsCtx.Message.Kind,
		"action":        result.Action.String(),
		"duration_ms":   duration.Milliseconds(),
		"trace_id":      obsCtx.TraceID,
		"error":         errStr,
		"user_id":       userID,
		"meta_snapshot":  metaSnapshot,
		"created_at":    apptime.NowTime().Format(apptime.TimeFormat),
	}

	select {
	case l.msgChan <- entry:
	default:
		// Channel full, drop to avoid blocking
	}
}

// --- logger.Logger interface implementation ---

func (l *DBLogger) Log(ctx context.Context, level logger.Level, msg string, fields ...logger.Field) {
	fieldsMap := make(map[string]interface{})
	for _, field := range fields {
		fieldsMap[field.Key] = field.Value
	}
	fieldsJSON, _ := json.Marshal(fieldsMap)

	var userID, traceID string
	if uid, ok := fieldsMap["user_id"].(string); ok {
		userID = uid
	}
	if tid, ok := fieldsMap["trace_id"].(string); ok {
		traceID = tid
	}

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

	logEntry := map[string]any{
		"id":         uuid.New(),
		"level":      levelStr,
		"message":    msg,
		"fields":     string(fieldsJSON),
		"user_id":    userID,
		"trace_id":   traceID,
		"created_at": apptime.NowTime().Format(apptime.TimeFormat),
	}

	select {
	case l.logChan <- logEntry:
	default:
	}
}

func (l *DBLogger) Debug(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelDebug, msg, fields...)
}

func (l *DBLogger) Info(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelInfo, msg, fields...)
}

func (l *DBLogger) Warn(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelWarn, msg, fields...)
}

func (l *DBLogger) Error(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelError, msg, fields...)
}

func (l *DBLogger) Fatal(ctx context.Context, msg string, fields ...logger.Field) {
	l.Log(ctx, logger.LevelFatal, msg, fields...)
}

func (l *DBLogger) With(fields ...logger.Field) logger.Logger {
	return l
}

func (l *DBLogger) Close() error {
	close(l.shutdown)
	apptime.Sleep(100 * apptime.Millisecond)
	return nil
}

func (l *DBLogger) Flush() error {
	return nil
}

func (l *DBLogger) WithContext(ctx context.Context) logger.Logger {
	return l
}

func (l *DBLogger) LogRequest(ctx context.Context, req *logger.RequestLog) error {
	return nil // No longer used — message logging via hooks
}

func (l *DBLogger) GetLogs(ctx context.Context, filter logger.LogFilter) ([]*logger.Log, error) {
	return []*logger.Log{}, nil
}

func (l *DBLogger) GetRequestLogs(ctx context.Context, filter logger.RequestLogFilter) ([]*logger.RequestLog, error) {
	return []*logger.RequestLog{}, nil
}
