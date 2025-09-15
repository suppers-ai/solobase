package legalpages

import (
	"errors"
	"fmt"
	"github.com/microcosm-cc/bluemonday"
	"gorm.io/gorm"
)

var (
	ErrDocumentNotFound     = errors.New("document not found")
	ErrInvalidDocumentType  = errors.New("invalid document type")
	ErrValidationFailed     = errors.New("content validation failed")
)

type LegalPagesService struct {
	db         *gorm.DB
	sanitizer  *bluemonday.Policy
}

func NewLegalPagesService(db *gorm.DB) *LegalPagesService {
	// Create HTML sanitizer with allowed tags
	p := bluemonday.UGCPolicy()
	p.AllowElements("p", "br", "strong", "em", "ul", "ol", "li", "h1", "h2", "h3", "h4", "h5", "h6")
	p.AllowAttrs("href").OnElements("a")

	return &LegalPagesService{
		db:        db,
		sanitizer: p,
	}
}

func (s *LegalPagesService) validateDocumentType(docType string) error {
	if docType != DocumentTypeTerms && docType != DocumentTypePrivacy {
		return ErrInvalidDocumentType
	}
	return nil
}

func (s *LegalPagesService) GetDocument(docType string) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	var doc LegalDocument
	err := s.db.Where("document_type = ? AND is_published = ?", docType, true).
		Order("version DESC").
		First(&doc).Error

	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return &doc, nil
}

func (s *LegalPagesService) GetLatestDocument(docType string) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	var doc LegalDocument
	err := s.db.Where("document_type = ?", docType).
		Order("version DESC").
		First(&doc).Error

	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return &doc, nil
}

func (s *LegalPagesService) SaveDocument(docType, title, content string, userID string) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	if title == "" {
		return nil, ErrValidationFailed
	}

	// Sanitize HTML content
	sanitizedContent := s.sanitizer.Sanitize(content)

	// Create new document version
	doc := &LegalDocument{
		DocumentType: docType,
		Title:        title,
		Content:      sanitizedContent,
		CreatedBy:    userID,
	}

	// The BeforeCreate hook will handle version incrementing
	if err := s.db.Create(doc).Error; err != nil {
		return nil, fmt.Errorf("failed to save document: %w", err)
	}

	return doc, nil
}

func (s *LegalPagesService) PublishDocument(docType string, version int) error {
	if err := s.validateDocumentType(docType); err != nil {
		return err
	}

	// Start a transaction
	return s.db.Transaction(func(tx *gorm.DB) error {
		// Unpublish all other versions
		if err := tx.Model(&LegalDocument{}).
			Where("document_type = ? AND is_published = ?", docType, true).
			Update("is_published", false).Error; err != nil {
			return err
		}

		// Publish the specified version
		result := tx.Model(&LegalDocument{}).
			Where("document_type = ? AND version = ?", docType, version).
			Update("is_published", true)

		if result.Error != nil {
			return result.Error
		}

		if result.RowsAffected == 0 {
			return ErrDocumentNotFound
		}

		return nil
	})
}

func (s *LegalPagesService) GetDocumentHistory(docType string) ([]*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	var documents []*LegalDocument
	err := s.db.Where("document_type = ?", docType).
		Order("version DESC").
		Find(&documents).Error

	if err != nil {
		return nil, err
	}

	return documents, nil
}

func (s *LegalPagesService) GetDocumentByVersion(docType string, version int) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	var doc LegalDocument
	err := s.db.Where("document_type = ? AND version = ?", docType, version).
		First(&doc).Error

	if err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return &doc, nil
}