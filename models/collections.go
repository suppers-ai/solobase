package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// Collection represents a dynamic collection/table
type Collection struct {
	ID          uuid.UUID `gorm:"type:char(36);primary_key" json:"id"`
	Name        string    `gorm:"uniqueIndex;not null;size:255" json:"name"`
	DisplayName string    `gorm:"size:255" json:"display_name,omitempty"`
	Description string    `gorm:"type:text" json:"description,omitempty"`
	Schema      JSON      `gorm:"type:text;not null" json:"schema"`
	Indexes     JSON      `gorm:"type:text" json:"indexes,omitempty"`
	AuthRules   JSON      `gorm:"type:text" json:"auth_rules,omitempty"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`

	// Relationships
	Records []CollectionRecord `gorm:"foreignKey:CollectionID;constraint:OnDelete:CASCADE" json:"-"`
}

// TableName specifies the table name
// Using simple table names that work with both SQLite and PostgreSQL
func (Collection) TableName() string {
	return "collections"
}

// BeforeCreate hook
func (c *Collection) BeforeCreate(tx *gorm.DB) error {
	if c.ID == uuid.Nil {
		c.ID = uuid.New()
	}
	return nil
}

// CollectionRecord represents a record in a collection
type CollectionRecord struct {
	ID           uuid.UUID  `gorm:"type:char(36);primary_key" json:"id"`
	CollectionID uuid.UUID  `gorm:"type:char(36);not null;index" json:"collection_id"`
	Data         JSON       `gorm:"type:text;not null" json:"data"`
	UserID       *uuid.UUID `gorm:"type:char(36);index" json:"user_id,omitempty"`
	CreatedAt    time.Time  `json:"created_at"`
	UpdatedAt    time.Time  `json:"updated_at"`

	// Relationships
	Collection Collection `gorm:"foreignKey:CollectionID" json:"-"`
}

// TableName specifies the table name
func (CollectionRecord) TableName() string {
	return "records"
}

// BeforeCreate hook
func (r *CollectionRecord) BeforeCreate(tx *gorm.DB) error {
	if r.ID == uuid.Nil {
		r.ID = uuid.New()
	}
	return nil
}
