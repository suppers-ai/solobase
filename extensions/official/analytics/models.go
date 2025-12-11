package analytics

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

// PageView represents a page view event
type PageView struct {
	ID         string    `gorm:"type:uuid;primaryKey" json:"id"`
	UserID     *string   `gorm:"type:uuid;index" json:"userId,omitempty"`
	SessionID  string    `gorm:"type:varchar(255);index" json:"sessionId"`
	PageURL    string    `gorm:"type:text;not null" json:"pageUrl"`
	Referrer   *string   `gorm:"type:text" json:"referrer,omitempty"`
	UserAgent  *string   `gorm:"type:text" json:"userAgent,omitempty"`
	IPAddress  *string   `gorm:"type:varchar(45)" json:"ipAddress,omitempty"`
	CreatedAt  time.Time `gorm:"index" json:"createdAt"`
}

// TableName sets the table name
func (PageView) TableName() string {
	return "ext_analytics_page_views"
}

// BeforeCreate hook to set UUID
func (p *PageView) BeforeCreate(tx *gorm.DB) error {
	if p.ID == "" {
		p.ID = uuid.New().String()
	}
	return nil
}

// Event represents a custom analytics event
type Event struct {
	ID        string         `gorm:"type:uuid;primaryKey" json:"id"`
	UserID    *string        `gorm:"type:uuid;index" json:"userId,omitempty"`
	EventName string         `gorm:"type:varchar(255);not null;index" json:"eventName"`
	EventData datatypes.JSON `gorm:"type:jsonb" json:"eventData"`
	CreatedAt time.Time      `gorm:"index" json:"createdAt"`
}

// TableName sets the table name
func (Event) TableName() string {
	return "ext_analytics_events"
}

// BeforeCreate hook to set UUID
func (e *Event) BeforeCreate(tx *gorm.DB) error {
	if e.ID == "" {
		e.ID = uuid.New().String()
	}
	return nil
}

// PageViewStats represents aggregated page view statistics
type PageViewStats struct {
	PageURL string `json:"pageUrl"`
	Views   int64  `json:"views"`
}

// DailyStats represents daily statistics
type DailyStats struct {
	Date        time.Time `json:"date"`
	PageViews   int64     `json:"pageViews"`
	UniqueUsers int64     `json:"uniqueUsers"`
	Events      int64     `json:"events"`
}

// AnalyticsStats represents overall analytics statistics
type AnalyticsStats struct {
	TotalViews  int64 `json:"totalViews"`
	UniqueUsers int64 `json:"uniqueUsers"`
	TodayViews  int64 `json:"todayViews"`
	ActiveNow   int64 `json:"activeNow"`
}