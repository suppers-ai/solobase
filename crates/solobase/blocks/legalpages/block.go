package legalpages

import (
	"context"
	"errors"
	"fmt"
	"strconv"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
)

const BlockName = "legalpages-feature"

const legalDocumentsCollection = "ext_legalpages_legal_documents"

// LegalPagesBlock uses ctx.Services().Database for legal document management.
type LegalPagesBlock struct {
	router *wafer.Router
	config *LegalPagesConfig
}

func NewLegalPagesBlock() *LegalPagesBlock {
	b := &LegalPagesBlock{
		config: &LegalPagesConfig{
			EnableTerms:   true,
			EnablePrivacy: true,
		},
	}
	b.router = wafer.NewRouter()

	// Public routes
	b.router.Retrieve("/ext/legalpages/terms", b.handlePublicTerms)
	b.router.Retrieve("/ext/legalpages/privacy", b.handlePublicPrivacy)
	// Admin API routes
	b.router.Retrieve("/ext/legalpages/api/documents", b.handleGetDocuments)
	b.router.Retrieve("/ext/legalpages/api/documents/{type}", b.handleGetDocument)
	b.router.Create("/ext/legalpages/api/documents/{type}", b.handleSaveDocument)
	b.router.Update("/ext/legalpages/api/documents/{type}", b.handleSaveDocument)
	b.router.Create("/ext/legalpages/api/documents/{type}/publish", b.handlePublishDocument)
	b.router.Retrieve("/ext/legalpages/api/documents/{type}/preview", b.handlePreviewDocument)
	b.router.Retrieve("/ext/legalpages/api/documents/{type}/history", b.handleGetDocumentHistory)
	// Admin UI
	b.router.Retrieve("/ext/legalpages/admin", b.handleAdminUI)

	return b
}

func (b *LegalPagesBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Legal pages management",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
	}
}

func (b *LegalPagesBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *LegalPagesBlock) Lifecycle(ctx wafer.Context, evt wafer.LifecycleEvent) error {
	if evt.Type == wafer.Init {
		svc := ctx.Services()
		if svc == nil {
			return nil
		}
		db := svc.Database
		if db == nil {
			return nil
		}
		return b.seedDefaults(db)
	}
	return nil
}

// --- Public handlers ---

func (b *LegalPagesBlock) handlePublicTerms(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.renderPublicPage(ctx, msg, DocumentTypeTerms, "Terms and Conditions")
}

func (b *LegalPagesBlock) handlePublicPrivacy(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.renderPublicPage(ctx, msg, DocumentTypePrivacy, "Privacy Policy")
}

func (b *LegalPagesBlock) renderPublicPage(ctx wafer.Context, msg *wafer.Message, docType, defaultTitle string) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		html := renderPublicPageHTML(defaultTitle, "", "This document is not yet available.")
		return wafer.Respond(msg, 200, []byte(html), "text/html; charset=utf-8")
	}

	doc, err := b.getPublishedDocument(db, docType)

	title := defaultTitle
	content := ""
	message := ""

	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			message = "This document is not yet available. Please check back later."
		} else {
			message = "An error occurred while loading this document. Please try again later."
		}
	} else {
		title = fmt.Sprintf("%v", doc["title"])
		content = fmt.Sprintf("%v", doc["content"])
	}

	html := renderPublicPageHTML(title, content, message)
	return wafer.Respond(msg, 200, []byte(html), "text/html; charset=utf-8")
}

// --- Admin API handlers ---

func (b *LegalPagesBlock) handleGetDocuments(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Query("type")

	if docType == "" {
		var response []map[string]any
		for _, dt := range []string{DocumentTypeTerms, DocumentTypePrivacy} {
			doc, err := b.getLatestDocument(db, dt)
			docInfo := map[string]any{"type": dt}
			if err == nil {
				docInfo["document"] = doc
			}
			response = append(response, docInfo)
		}
		return wafer.JSONRespond(msg, 200, response)
	}

	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	doc, err := b.getLatestDocument(db, docType)
	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			return wafer.JSONRespond(msg, 200, map[string]any{"type": docType, "document": nil})
		}
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, doc)
}

func (b *LegalPagesBlock) handleGetDocument(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Var("type")
	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	doc, err := b.getLatestDocument(db, docType)
	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			return wafer.Error(msg, 404, "not_found", "Document not found")
		}
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, doc)
}

func (b *LegalPagesBlock) handleSaveDocument(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Var("type")
	userID := msg.UserID()

	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	var body struct {
		Title   string `json:"title"`
		Content string `json:"content"`
	}
	if err := msg.Decode(&body); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	// Get next version number
	nextVersion := 1
	existing, err := b.getLatestDocument(db, docType)
	if err == nil {
		if v, ok := existing["version"]; ok {
			switch vt := v.(type) {
			case float64:
				nextVersion = int(vt) + 1
			case int:
				nextVersion = vt + 1
			case int64:
				nextVersion = int(vt) + 1
			}
		}
	}

	now := apptime.NowTime().Format(apptime.TimeFormat)
	data := map[string]any{
		"document_type": docType,
		"title":         body.Title,
		"content":       body.Content,
		"version":       nextVersion,
		"status":        "published",
		"created_at":    now,
		"updated_at":    now,
		"created_by":    userID,
	}

	data["id"] = uuid.New().String()
	record, err := db.Create(context.Background(), legalDocumentsCollection, data)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	result := record.Data
	result["id"] = record.ID
	return wafer.JSONRespond(msg, 201, result)
}

func (b *LegalPagesBlock) handlePublishDocument(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Var("type")
	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	var body struct {
		Version int `json:"version"`
	}
	if err := msg.Decode(&body); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	// Find the document by type and version
	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "document_type", Operator: database.OpEqual, Value: docType},
			{Field: "version", Operator: database.OpEqual, Value: body.Version},
		},
		Limit: 1,
	})
	if err != nil || len(records.Records) == 0 {
		return wafer.Error(msg, 404, "not_found", "Document version not found")
	}

	record := records.Records[0]
	_, err = db.Update(context.Background(), legalDocumentsCollection, record.ID, map[string]any{
		"status":     "published",
		"updated_at": apptime.NowTime().Format(apptime.TimeFormat),
	})
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, map[string]string{"status": "published"})
}

func (b *LegalPagesBlock) handlePreviewDocument(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Var("type")
	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	versionStr := msg.Query("version")
	var doc map[string]any
	var err error

	if versionStr != "" {
		version, parseErr := strconv.Atoi(versionStr)
		if parseErr != nil {
			return wafer.Error(msg, 400, "bad_request", "Invalid version number")
		}
		doc, err = b.getDocumentByVersion(db, docType, version)
	} else {
		doc, err = b.getLatestDocument(db, docType)
	}

	if err != nil {
		if errors.Is(err, database.ErrNotFound) {
			return wafer.Error(msg, 404, "not_found", "Document not found")
		}
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	title := fmt.Sprintf("%v", doc["title"])
	version := 0
	if v, ok := doc["version"].(float64); ok {
		version = int(v)
	}
	content := fmt.Sprintf("%v", doc["content"])

	html := fmt.Sprintf(`<!DOCTYPE html>
<html><head><title>Preview: %s</title>
<style>body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif;line-height:1.6;color:#333;max-width:800px;margin:0 auto;padding:20px;}.preview-header{background:#f0f0f0;padding:10px;margin-bottom:20px;border-radius:4px;}</style>
</head><body>
<div class="preview-header"><strong>Preview Mode</strong> - Version %d</div>
<h1>%s</h1>%s
</body></html>`, title, version, title, content)

	return wafer.Respond(msg, 200, []byte(html), "text/html; charset=utf-8")
}

func (b *LegalPagesBlock) handleGetDocumentHistory(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	docType := msg.Var("type")
	if !isValidDocumentType(docType) {
		return wafer.Error(msg, 400, "bad_request", "Invalid document type")
	}

	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "document_type", Operator: database.OpEqual, Value: docType},
		},
		Sort:  []database.SortField{{Field: "version", Desc: true}},
		Limit: 100,
	})
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	var docs []map[string]any
	for _, r := range records.Records {
		doc := r.Data
		doc["id"] = r.ID
		docs = append(docs, doc)
	}
	return wafer.JSONRespond(msg, 200, docs)
}

func (b *LegalPagesBlock) handleAdminUI(_ wafer.Context, msg *wafer.Message) wafer.Result {
	return wafer.Respond(msg, 200, []byte(AdminTemplate), "text/html; charset=utf-8")
}

// --- Internal helpers ---

func (b *LegalPagesBlock) getLatestDocument(db database.Service, docType string) (map[string]any, error) {
	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "document_type", Operator: database.OpEqual, Value: docType},
		},
		Sort:  []database.SortField{{Field: "version", Desc: true}},
		Limit: 1,
	})
	if err != nil {
		return nil, err
	}
	if len(records.Records) == 0 {
		return nil, database.ErrNotFound
	}
	doc := records.Records[0].Data
	doc["id"] = records.Records[0].ID
	return doc, nil
}

func (b *LegalPagesBlock) getDocumentByVersion(db database.Service, docType string, version int) (map[string]any, error) {
	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "document_type", Operator: database.OpEqual, Value: docType},
			{Field: "version", Operator: database.OpEqual, Value: version},
		},
		Limit: 1,
	})
	if err != nil {
		return nil, err
	}
	if len(records.Records) == 0 {
		return nil, database.ErrNotFound
	}
	doc := records.Records[0].Data
	doc["id"] = records.Records[0].ID
	return doc, nil
}

func (b *LegalPagesBlock) getPublishedDocument(db database.Service, docType string) (map[string]any, error) {
	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "document_type", Operator: database.OpEqual, Value: docType},
			{Field: "status", Operator: database.OpEqual, Value: "published"},
		},
		Sort:  []database.SortField{{Field: "version", Desc: true}},
		Limit: 1,
	})
	if err != nil {
		return nil, err
	}
	if len(records.Records) == 0 {
		return nil, database.ErrNotFound
	}
	doc := records.Records[0].Data
	doc["id"] = records.Records[0].ID
	return doc, nil
}

func (b *LegalPagesBlock) seedDefaults(db database.Service) error {
	// Check if documents already exist
	records, err := db.List(context.Background(), legalDocumentsCollection, &database.ListOptions{
		Limit: 1,
	})
	if err != nil {
		return nil // If table doesn't exist yet, skip seeding
	}
	if len(records.Records) > 0 {
		return nil // Already seeded
	}

	now := apptime.NowTime().Format(apptime.TimeFormat)
	defaults := []map[string]any{
		{
			"document_type": DocumentTypeTerms,
			"title":         "Terms and Conditions",
			"content":       defaultTermsContent,
			"version":       1,
			"status":        "published",
			"created_at":    now,
			"updated_at":    now,
			"created_by":    "system",
		},
		{
			"document_type": DocumentTypePrivacy,
			"title":         "Privacy Policy",
			"content":       defaultPrivacyContent,
			"version":       1,
			"status":        "published",
			"created_at":    now,
			"updated_at":    now,
			"created_by":    "system",
		},
	}

	for _, data := range defaults {
		data["id"] = uuid.New().String()
		if _, err := db.Create(context.Background(), legalDocumentsCollection, data); err != nil {
			return err
		}
	}
	return nil
}

func isValidDocumentType(docType string) bool {
	return docType == DocumentTypeTerms || docType == DocumentTypePrivacy
}

// Default content for seeding.
const defaultTermsContent = `<h2>1. Acceptance of Terms</h2>
<p>By accessing and using this service, you accept and agree to be bound by the terms and provision of this agreement.</p>
<h2>2. Use License</h2>
<p>Permission is granted to temporarily use the materials on our service for personal, non-commercial transitory viewing only.</p>
<h2>3. Disclaimer</h2>
<p>The materials on our service are provided on an 'as is' basis. We make no warranties, expressed or implied.</p>`

const defaultPrivacyContent = `<h2>1. Information We Collect</h2>
<p>We collect information you provide directly to us, such as when you create an account or communicate with us.</p>
<h2>2. How We Use Your Information</h2>
<p>We use the information we collect to provide, maintain, and improve our services.</p>
<h2>3. Information Sharing</h2>
<p>We do not sell, trade, or otherwise transfer your personal information to third parties without your consent.</p>`
