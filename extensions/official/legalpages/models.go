package legalpages

import (
	"fmt"
	"math/rand"
	"time"
	"gorm.io/gorm"
)

const (
	DocumentTypeTerms   = "terms"
	DocumentTypePrivacy = "privacy"
)

type LegalDocument struct {
	ID           string    `gorm:"primaryKey" json:"id"`
	DocumentType string    `gorm:"not null;index:idx_doc_type_published" json:"document_type"`
	Title        string    `gorm:"not null" json:"title"`
	Content      string    `gorm:"type:text" json:"content"`
	Version      int       `gorm:"not null;default:1" json:"version"`
	IsPublished  bool      `gorm:"default:false;index:idx_doc_type_published" json:"is_published"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
	CreatedBy    string    `gorm:"type:uuid" json:"created_by"`
}

func (LegalDocument) TableName() string {
	return "ext_legalpages_legal_documents"
}

func (d *LegalDocument) BeforeCreate(tx *gorm.DB) error {
	// Generate ID if not set
	if d.ID == "" {
		d.ID = generateID()
	}

	// Auto-increment version for new documents of the same type
	var maxVersion int
	tx.Model(&LegalDocument{}).
		Where("document_type = ?", d.DocumentType).
		Select("COALESCE(MAX(version), 0)").
		Scan(&maxVersion)

	d.Version = maxVersion + 1
	return nil
}

// generateID generates a unique ID for the document
func generateID() string {
	// Simple ID generation using timestamp and random number
	return fmt.Sprintf("%d-%d", time.Now().UnixNano(), rand.Intn(10000))
}

func (d *LegalDocument) GetLatestVersion(db *gorm.DB, docType string) (*LegalDocument, error) {
	var doc LegalDocument
	err := db.Where("document_type = ? AND is_published = ?", docType, true).
		Order("version DESC").
		First(&doc).Error

	if err != nil {
		return nil, err
	}
	return &doc, nil
}

func (d *LegalDocument) IsCurrentVersion(db *gorm.DB) bool {
	var count int64
	db.Model(&LegalDocument{}).
		Where("document_type = ? AND version > ?", d.DocumentType, d.Version).
		Count(&count)
	return count == 0
}

func RegisterModels(db *gorm.DB) error {
	return db.AutoMigrate(&LegalDocument{})
}