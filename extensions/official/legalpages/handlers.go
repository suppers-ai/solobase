package legalpages

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/utils"
)

type Handlers struct {
	service *LegalPagesService
}

func NewHandlers(service *LegalPagesService) *Handlers {
	return &Handlers{
		service: service,
	}
}

// renderPublicPageHTML generates the HTML for a public legal page
func renderPublicPageHTML(title, content, message string) string {
	contentSection := ""
	if content != "" {
		contentSection = content
	} else {
		contentSection = fmt.Sprintf(`<div class="not-found"><p>%s</p></div>`, message)
	}

	return fmt.Sprintf(`<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>%s</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
        }
        h1 { color: #2c3e50; }
        a { color: #3498db; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .back-link { margin-bottom: 20px; }
        .content { margin-top: 30px; }
        .not-found {
            text-align: center;
            padding: 50px 20px;
            background: #f8f9fa;
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <div class="back-link">
        <a href="/">← Back to Home</a>
    </div>
    <h1>%s</h1>
    <div class="content">
        %s
    </div>
</body>
</html>`, title, title, contentSection)
}

// Admin API Handlers

func (h *Handlers) HandleGetDocuments(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	docType := r.URL.Query().Get("type")

	if docType == "" {
		// Return both document types
		var response []map[string]interface{}

		for _, dt := range []string{DocumentTypeTerms, DocumentTypePrivacy} {
			doc, err := h.service.GetLatestDocument(dt)
			if err != nil && !errors.Is(err, ErrDocumentNotFound) {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}

			docInfo := map[string]interface{}{
				"type": dt,
			}

			if doc != nil {
				docInfo["document"] = doc
			}

			response = append(response, docInfo)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
		return
	}

	// Get specific document type
	doc, err := h.service.GetLatestDocument(docType)
	if err != nil {
		if errors.Is(err, ErrDocumentNotFound) {
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(map[string]interface{}{
				"type": docType,
				"document": nil,
			})
			return
		}
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(doc)
}

func (h *Handlers) HandleGetDocument(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	vars := mux.Vars(r)
	docType := vars["type"]

	doc, err := h.service.GetLatestDocument(docType)
	if err != nil {
		if errors.Is(err, ErrDocumentNotFound) {
			http.Error(w, "Document not found", http.StatusNotFound)
			return
		}
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(doc)
}

func (h *Handlers) HandleSaveDocument(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	vars := mux.Vars(r)
	docType := vars["type"]

	var req struct {
		Title   string `json:"title"`
		Content string `json:"content"`
	}

	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	// Get user ID from context (this would be set by auth middleware)
	userID := r.Context().Value("user_id")
	if userID == nil {
		userID = ""
	}

	doc, err := h.service.SaveDocument(docType, req.Title, req.Content, userID.(string))
	if err != nil {
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		if errors.Is(err, ErrValidationFailed) {
			http.Error(w, "Validation failed", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Auto-publish the new version
	if err := h.service.PublishDocument(docType, doc.Version); err != nil {
		// Log error but don't fail the request
		fmt.Printf("Failed to auto-publish document: %v\n", err)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(doc)
}

func (h *Handlers) HandlePublishDocument(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	vars := mux.Vars(r)
	docType := vars["type"]

	var req struct {
		Version int `json:"version"`
	}

	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	if err := h.service.PublishDocument(docType, req.Version); err != nil {
		if errors.Is(err, ErrDocumentNotFound) {
			http.Error(w, "Document version not found", http.StatusNotFound)
			return
		}
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "published"})
}

func (h *Handlers) HandlePreviewDocument(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	vars := mux.Vars(r)
	docType := vars["type"]

	versionStr := r.URL.Query().Get("version")
	var doc *LegalDocument
	var err error

	if versionStr != "" {
		version, err := strconv.Atoi(versionStr)
		if err != nil {
			http.Error(w, "Invalid version number", http.StatusBadRequest)
			return
		}
		doc, err = h.service.GetDocumentByVersion(docType, version)
	} else {
		doc, err = h.service.GetLatestDocument(docType)
	}

	if err != nil {
		if errors.Is(err, ErrDocumentNotFound) {
			http.Error(w, "Document not found", http.StatusNotFound)
			return
		}
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Return HTML preview
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	fmt.Fprintf(w, `
		<!DOCTYPE html>
		<html>
		<head>
			<title>Preview: %s</title>
			<style>
				body {
					font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
					line-height: 1.6;
					color: #333;
					max-width: 800px;
					margin: 0 auto;
					padding: 20px;
				}
				.preview-header {
					background: #f0f0f0;
					padding: 10px;
					margin-bottom: 20px;
					border-radius: 4px;
				}
			</style>
		</head>
		<body>
			<div class="preview-header">
				<strong>Preview Mode</strong> - Version %d
			</div>
			<h1>%s</h1>
			%s
		</body>
		</html>
	`, doc.Title, doc.Version, doc.Title, doc.Content)
}

func (h *Handlers) HandleGetDocumentHistory(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	vars := mux.Vars(r)
	docType := vars["type"]

	documents, err := h.service.GetDocumentHistory(docType)
	if err != nil {
		if errors.Is(err, ErrInvalidDocumentType) {
			http.Error(w, "Invalid document type", http.StatusBadRequest)
			return
		}
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(documents)
}

// Public Page Handlers

func (h *Handlers) HandlePublicTerms(w http.ResponseWriter, r *http.Request) {
	h.renderPublicPage(w, DocumentTypeTerms, "Terms and Conditions")
}

func (h *Handlers) HandlePublicPrivacy(w http.ResponseWriter, r *http.Request) {
	h.renderPublicPage(w, DocumentTypePrivacy, "Privacy Policy")
}

func (h *Handlers) renderPublicPage(w http.ResponseWriter, docType, defaultTitle string) {
	doc, err := h.service.GetDocument(docType)

	title := defaultTitle
	content := ""
	message := ""

	if err != nil {
		if errors.Is(err, ErrDocumentNotFound) {
			message = "This document is not yet available. Please check back later."
		} else {
			message = "An error occurred while loading this document. Please try again later."
		}
	} else {
		title = doc.Title
		content = doc.Content
	}

	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write([]byte(renderPublicPageHTML(title, content, message)))
}