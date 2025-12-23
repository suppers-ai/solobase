package legalpages

import (
	"fmt"
	"math/rand"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

const (
	DocumentTypeTerms   = "terms"
	DocumentTypePrivacy = "privacy"

	// Document status constants
	StatusDraft     = "draft"
	StatusPublished = "published"
	StatusArchived  = "archived"
	StatusReview    = "review"
)

type LegalDocument struct {
	ID           string       `json:"id"`
	DocumentType string       `json:"documentType"`
	Title        string       `json:"title"`
	Content      string       `json:"content"`
	Version      int          `json:"version"`
	Status       string       `json:"status"`
	CreatedAt    apptime.Time `json:"createdAt"`
	UpdatedAt    apptime.Time `json:"updatedAt"`
	CreatedBy    string       `json:"createdBy"`
}

func (LegalDocument) TableName() string {
	return "ext_legalpages_legal_documents"
}

// PrepareForCreate prepares the document for insertion
// Note: Version auto-increment must be handled by the service layer
func (d *LegalDocument) PrepareForCreate() {
	if d.ID == "" {
		d.ID = generateID()
	}
	now := apptime.NowTime()
	if d.CreatedAt.IsZero() {
		d.CreatedAt = now
	}
	d.UpdatedAt = now
	if d.Status == "" {
		d.Status = StatusDraft
	}
	if d.Version == 0 {
		d.Version = 1
	}
}

// generateID generates a unique ID for the document
func generateID() string {
	// Simple ID generation using timestamp and random number
	return fmt.Sprintf("%d-%d", apptime.NowTime().UnixNano(), rand.Intn(10000))
}