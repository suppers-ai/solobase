package logger

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

// LogModel represents an application log entry for database storage
type LogModel struct {
	ID        uuid.UUID      `gorm:"type:char(36);primary_key" json:"id"`
	Level     string         `gorm:"not null;size:20;index" json:"level"`
	Message   string         `gorm:"type:text;not null" json:"message"`
	Fields    datatypes.JSON `gorm:"type:text" json:"fields,omitempty"`
	UserID    *string        `gorm:"size:255;index" json:"user_id,omitempty"`
	TraceID   *string        `gorm:"size:255;index" json:"trace_id,omitempty"`
	CreatedAt time.Time      `gorm:"index" json:"created_at"`
}

// TableName specifies the table name
func (LogModel) TableName() string {
	return "logs"
}

// BeforeCreate hook
func (l *LogModel) BeforeCreate(tx *gorm.DB) error {
	if l.ID == uuid.Nil {
		l.ID = uuid.New()
	}
	return nil
}

// RequestLogModel represents an HTTP request log for database storage
type RequestLogModel struct {
	ID           uuid.UUID `gorm:"type:char(36);primary_key" json:"id"`
	Level        string    `gorm:"not null;size:20" json:"level"`
	Method       string    `gorm:"not null;size:10;index" json:"method"`
	Path         string    `gorm:"not null;index" json:"path"`
	Query        *string   `gorm:"type:text" json:"query,omitempty"`
	StatusCode   int       `gorm:"not null;index" json:"status_code"`
	ExecTimeMs   int64     `gorm:"not null" json:"exec_time_ms"`
	UserIP       string    `gorm:"not null;size:45" json:"user_ip"`
	UserAgent    *string   `gorm:"size:500" json:"user_agent,omitempty"`
	UserID       *string   `gorm:"size:255;index" json:"user_id,omitempty"`
	TraceID      *string   `gorm:"size:255" json:"trace_id,omitempty"`
	Error        *string   `gorm:"type:text" json:"error,omitempty"`
	RequestBody  *string   `gorm:"type:text" json:"request_body,omitempty"`
	ResponseBody *string   `gorm:"type:text" json:"response_body,omitempty"`
	Headers      *string   `gorm:"type:text" json:"headers,omitempty"`
	CreatedAt    time.Time `gorm:"index" json:"created_at"`
}

// TableName specifies the table name
func (RequestLogModel) TableName() string {
	return "request_logs"
}

// BeforeCreate hook
func (r *RequestLogModel) BeforeCreate(tx *gorm.DB) error {
	if r.ID == uuid.Nil {
		r.ID = uuid.New()
	}
	return nil
}
