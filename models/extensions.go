package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// ExtensionMigration tracks extension migrations
type ExtensionMigration struct {
	ID            uuid.UUID `gorm:"type:char(36);primary_key" json:"id"`
	ExtensionName string    `gorm:"not null;size:255;index" json:"extension_name"`
	Version       string    `gorm:"not null;size:50" json:"version"`
	Description   string    `gorm:"type:text" json:"description,omitempty"`
	Checksum      string    `gorm:"size:255" json:"checksum"`
	AppliedAt     time.Time `json:"applied_at"`
}

// TableName specifies the table name
func (ExtensionMigration) TableName() string {
	return "extension_migrations"
}

// BeforeCreate hook
func (e *ExtensionMigration) BeforeCreate(tx *gorm.DB) error {
	if e.ID == uuid.Nil {
		e.ID = uuid.New()
	}
	if e.AppliedAt.IsZero() {
		e.AppliedAt = time.Now()
	}
	return nil
}
