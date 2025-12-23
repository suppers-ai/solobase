package logger

import (
	"encoding/json"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

// LogModel represents an application log entry for database storage
type LogModel struct {
	ID        uuid.UUID       `json:"id"`
	Level     string          `json:"level"`
	Message   string          `json:"message"`
	Fields    json.RawMessage `json:"fields,omitempty"`
	UserID    *string         `json:"userId,omitempty"`
	TraceID   *string         `json:"traceId,omitempty"`
	CreatedAt apptime.Time    `json:"createdAt"`
}

// TableName specifies the table name
func (LogModel) TableName() string {
	return "sys_logs"
}

// PrepareForCreate prepares the log model for database insertion
// Prepares model for database insert
func (l *LogModel) PrepareForCreate() {
	if l.ID == uuid.Nil {
		l.ID = uuid.New()
	}
	if l.CreatedAt.IsZero() {
		l.CreatedAt = apptime.NowTime()
	}
}

// RequestLogModel represents an HTTP request log for database storage
type RequestLogModel struct {
	ID           uuid.UUID    `json:"id"`
	Level        string       `json:"level"`
	Method       string       `json:"method"`
	Path         string       `json:"path"`
	Query        *string      `json:"query,omitempty"`
	StatusCode   int          `json:"statusCode"`
	ExecTimeMs   int64        `json:"execTimeMs"`
	UserIP       string       `json:"userIp"`
	UserAgent    *string      `json:"userAgent,omitempty"`
	UserID       *string      `json:"userId,omitempty"`
	TraceID      *string      `json:"traceId,omitempty"`
	Error        *string      `json:"error,omitempty"`
	RequestBody  *string      `json:"requestBody,omitempty"`
	ResponseBody *string      `json:"responseBody,omitempty"`
	Headers      *string      `json:"headers,omitempty"`
	CreatedAt    apptime.Time `json:"createdAt"`
}

// TableName specifies the table name
func (RequestLogModel) TableName() string {
	return "sys_request_logs"
}

// PrepareForCreate prepares the request log model for database insertion
// Prepares model for database insert
func (r *RequestLogModel) PrepareForCreate() {
	if r.ID == uuid.Nil {
		r.ID = uuid.New()
	}
	if r.CreatedAt.IsZero() {
		r.CreatedAt = apptime.NowTime()
	}
}
