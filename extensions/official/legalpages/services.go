package legalpages

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

var (
	ErrDocumentNotFound    = errors.New("document not found")
	ErrInvalidDocumentType = errors.New("invalid document type")
	ErrValidationFailed    = errors.New("content validation failed")
)

type LegalPagesService struct {
	queries   *db.Queries
	sqlDB     *sql.DB
	sanitizer htmlSanitizer
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

	ctx := context.Background()
	dbDoc, err := s.queries.GetPublishedDocumentByType(ctx, docType)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return dbDocToLegalDocument(dbDoc), nil
}

func (s *LegalPagesService) GetLatestDocument(docType string) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	ctx := context.Background()
	dbDoc, err := s.queries.GetLatestDocumentByType(ctx, docType)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return dbDocToLegalDocument(dbDoc), nil
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

	ctx := context.Background()

	// Get the next version number
	maxVersion, err := s.queries.GetMaxVersionByType(ctx, docType)
	if err != nil {
		return nil, fmt.Errorf("failed to get max version: %w", err)
	}
	nextVersion := maxVersion + 1

	// Prepare the document
	doc := &LegalDocument{
		DocumentType: docType,
		Title:        title,
		Content:      sanitizedContent,
		Version:      int(nextVersion),
		CreatedBy:    userID,
	}
	doc.PrepareForCreate()

	// Create the document
	status := doc.Status
	dbDoc, err := s.queries.CreateLegalDocument(ctx, db.CreateLegalDocumentParams{
		ID:           doc.ID,
		DocumentType: doc.DocumentType,
		Title:        doc.Title,
		Content:      &doc.Content,
		Version:      int64(doc.Version),
		Status:       &status,
		CreatedBy:    &doc.CreatedBy,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to save document: %w", err)
	}

	return dbDocToLegalDocument(dbDoc), nil
}

func (s *LegalPagesService) PublishDocument(docType string, version int) error {
	if err := s.validateDocumentType(docType); err != nil {
		return err
	}

	ctx := context.Background()

	// Start a transaction
	tx, err := s.sqlDB.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	qtx := s.queries.WithTx(tx)

	// Archive all previously published versions
	if err := qtx.ArchivePublishedDocumentsByType(ctx, docType); err != nil {
		return err
	}

	// Publish the specified version
	published := StatusPublished
	if err := qtx.UpdateDocumentStatusByTypeAndVersion(ctx, db.UpdateDocumentStatusByTypeAndVersionParams{
		Status:       &published,
		DocumentType: docType,
		Version:      int64(version),
	}); err != nil {
		return err
	}

	// Verify the document was updated
	_, err = qtx.GetDocumentByTypeAndVersion(ctx, db.GetDocumentByTypeAndVersionParams{
		DocumentType: docType,
		Version:      int64(version),
	})
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrDocumentNotFound
		}
		return err
	}

	return tx.Commit()
}

func (s *LegalPagesService) GetDocumentHistory(docType string) ([]*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	ctx := context.Background()
	dbDocs, err := s.queries.ListLegalDocumentsByType(ctx, docType)
	if err != nil {
		return nil, err
	}

	documents := make([]*LegalDocument, len(dbDocs))
	for i, dbDoc := range dbDocs {
		documents[i] = dbDocToLegalDocument(dbDoc)
	}

	return documents, nil
}

func (s *LegalPagesService) GetDocumentByVersion(docType string, version int) (*LegalDocument, error) {
	if err := s.validateDocumentType(docType); err != nil {
		return nil, err
	}

	ctx := context.Background()
	dbDoc, err := s.queries.GetDocumentByTypeAndVersion(ctx, db.GetDocumentByTypeAndVersionParams{
		DocumentType: docType,
		Version:      int64(version),
	})
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}

	return dbDocToLegalDocument(dbDoc), nil
}

// SetDocumentStatus updates the status of a specific document version
func (s *LegalPagesService) SetDocumentStatus(docType string, version int, status string) error {
	if err := s.validateDocumentType(docType); err != nil {
		return err
	}

	// Validate status
	if status != StatusDraft && status != StatusPublished &&
		status != StatusArchived && status != StatusReview {
		return fmt.Errorf("invalid status: %s", status)
	}

	// If setting to published, archive other published docs
	if status == StatusPublished {
		return s.PublishDocument(docType, version)
	}

	ctx := context.Background()

	// First check if document exists
	_, err := s.queries.GetDocumentByTypeAndVersion(ctx, db.GetDocumentByTypeAndVersionParams{
		DocumentType: docType,
		Version:      int64(version),
	})
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrDocumentNotFound
		}
		return err
	}

	// Update the document status
	return s.queries.UpdateDocumentStatusByTypeAndVersion(ctx, db.UpdateDocumentStatusByTypeAndVersionParams{
		Status:       &status,
		DocumentType: docType,
		Version:      int64(version),
	})
}

// dbDocToLegalDocument converts a sqlc generated document to our model
func dbDocToLegalDocument(dbDoc db.ExtLegalpagesLegalDocument) *LegalDocument {
	doc := &LegalDocument{
		ID:           dbDoc.ID,
		DocumentType: dbDoc.DocumentType,
		Title:        dbDoc.Title,
		Version:      int(dbDoc.Version),
		CreatedAt:    apptime.NewTime(apptime.MustParse(dbDoc.CreatedAt)),
		UpdatedAt:    apptime.NewTime(apptime.MustParse(dbDoc.UpdatedAt)),
	}
	if dbDoc.Content != nil {
		doc.Content = *dbDoc.Content
	}
	if dbDoc.Status != nil {
		doc.Status = *dbDoc.Status
	}
	if dbDoc.CreatedBy != nil {
		doc.CreatedBy = *dbDoc.CreatedBy
	}
	return doc
}
