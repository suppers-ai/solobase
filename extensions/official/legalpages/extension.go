package legalpages

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/extensions/core"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

type LegalPagesExtension struct {
	sqlDB    *sql.DB
	queries  *db.Queries
	service  *LegalPagesService
	handlers *Handlers
	config   *LegalPagesConfig
}

type LegalPagesConfig struct {
	EnableTerms   bool   `json:"enableTerms"`
	EnablePrivacy bool   `json:"enablePrivacy"`
	CompanyName   string `json:"companyName"`
}

func NewLegalPagesExtension() *LegalPagesExtension {
	return &LegalPagesExtension{
		config: &LegalPagesConfig{
			EnableTerms:   true,
			EnablePrivacy: true,
		},
	}
}

// Metadata returns information about the extension
func (e *LegalPagesExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "legalpages",
		Version:     "1.0.0",
		Description: "Legal pages management for terms and conditions and privacy policy",
		Author:      "Solobase",
		License:     "MIT",
		Tags:        []string{"legal", "compliance", "terms", "privacy"},
	}
}

// Initialize sets up the extension with provided services
func (e *LegalPagesExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	// The database is set separately via SetSQLDatabase method
	if e.sqlDB == nil {
		return nil // Extension can work without database in limited mode
	}

	return nil
}

// SetSQLDatabase sets the SQL database for sqlc queries and initializes services
func (e *LegalPagesExtension) SetSQLDatabase(sqlDB *sql.DB) {
	fmt.Println("LegalPages: SetSQLDatabase called")
	e.sqlDB = sqlDB
	e.queries = db.New(sqlDB)

	if e.sqlDB != nil {
		fmt.Println("LegalPages: Database is not nil, initializing...")

		// Initialize service and handlers
		e.service = NewLegalPagesService(e.sqlDB)
		e.handlers = NewHandlers(e.service)
		fmt.Printf("LegalPages: Handlers initialized: %v\n", e.handlers != nil)

		// Seed initial data
		if err := SeedDataWithSQL(e.sqlDB); err != nil {
			fmt.Printf("LegalPages: Failed to seed data: %v\n", err)
			// Don't fail initialization, just log the error
		}
	} else {
		fmt.Println("LegalPages: Database is nil")
	}
}

// Start begins extension operation
func (e *LegalPagesExtension) Start(ctx context.Context) error {
	// No background processes needed
	return nil
}

// Stop gracefully shuts down the extension
func (e *LegalPagesExtension) Stop(ctx context.Context) error {
	// No cleanup needed
	return nil
}

// Health returns the current health status of the extension
func (e *LegalPagesExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	status := &core.HealthStatus{
		Status:      "healthy",
		Message:     "Legal pages extension is running",
		LastChecked: apptime.NowTime(),
		Checks:      []core.HealthCheck{},
	}

	// Check database connection
	if e.queries != nil {
		_, err := e.queries.CountLegalDocuments(ctx)
		if err != nil {
			status.Status = "unhealthy"
			status.Message = "Database connection failed"
			status.Checks = append(status.Checks, core.HealthCheck{
				Name:    "database",
				Status:  "unhealthy",
				Message: err.Error(),
			})
		} else {
			status.Checks = append(status.Checks, core.HealthCheck{
				Name:    "database",
				Status:  "healthy",
				Message: "Connected",
			})
		}
	}

	return status, nil
}

// RegisterRoutes registers all HTTP routes for the extension
func (e *LegalPagesExtension) RegisterRoutes(router core.ExtensionRouter) error {
	if e.handlers == nil {
		return nil
	}

	// Admin API routes
	router.HandleFunc("/api/documents", e.handlers.HandleGetDocuments)
	router.HandleFunc("/api/documents/{type}", e.handlers.HandleGetDocument)
	router.HandleFunc("/api/documents/{type}", e.handlers.HandleSaveDocument)
	router.HandleFunc("/api/documents/{type}/publish", e.handlers.HandlePublishDocument)
	router.HandleFunc("/api/documents/{type}/preview", e.handlers.HandlePreviewDocument)
	router.HandleFunc("/api/documents/{type}/history", e.handlers.HandleGetDocumentHistory)

	// Public routes (these need to be registered at root level)
	// For now we'll register them under the extension prefix
	if e.config.EnableTerms {
		router.HandleFunc("/terms", e.handlers.HandlePublicTerms)
	}
	if e.config.EnablePrivacy {
		router.HandleFunc("/privacy", e.handlers.HandlePublicPrivacy)
	}

	// Admin UI route
	router.HandleFunc("/admin", e.handleAdminUI)

	return nil
}

// RegisterMiddleware registers any middleware for the extension
func (e *LegalPagesExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	// No custom middleware needed
	return nil
}

// RegisterHooks registers any hooks for the extension
func (e *LegalPagesExtension) RegisterHooks() []core.HookRegistration {
	// No hooks needed
	return nil
}

// RegisterTemplates registers any templates for the extension
func (e *LegalPagesExtension) RegisterTemplates() []core.TemplateRegistration {
	// Templates are served directly via handlers
	return nil
}

// RegisterStaticAssets registers any static assets for the extension
func (e *LegalPagesExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	// No static assets needed
	return nil
}

// ConfigSchema returns the JSON schema for configuration
func (e *LegalPagesExtension) ConfigSchema() json.RawMessage {
	schema := `{
		"type": "object",
		"properties": {
			"enable_terms": {
				"type": "boolean",
				"description": "Enable terms and conditions page",
				"default": true
			},
			"enable_privacy": {
				"type": "boolean",
				"description": "Enable privacy policy page",
				"default": true
			},
			"company_name": {
				"type": "string",
				"description": "Company name to use in legal documents"
			}
		}
	}`
	return json.RawMessage(schema)
}

// ValidateConfig validates the provided configuration
func (e *LegalPagesExtension) ValidateConfig(config json.RawMessage) error {
	var cfg LegalPagesConfig
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}
	return nil
}

// ApplyConfig applies the provided configuration
func (e *LegalPagesExtension) ApplyConfig(config json.RawMessage) error {
	var cfg LegalPagesConfig
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}
	e.config = &cfg
	return nil
}

// DatabaseSchema returns the database schema name
func (e *LegalPagesExtension) DatabaseSchema() string {
	return "ext_legalpages"
}

// RequiredPermissions returns the permissions required by the extension
func (e *LegalPagesExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "legalpages.admin",
			Description: "Manage legal pages content",
			Resource:    "legalpages",
			Actions:     []string{"create", "read", "update", "delete"},
		},
	}
}

// GetHandlers returns the extension's handlers
func (e *LegalPagesExtension) GetHandlers() *Handlers {
	return e.handlers
}

// HandleAdminUI serves the admin interface
func (e *LegalPagesExtension) HandleAdminUI(w http.ResponseWriter, r *http.Request) {
	e.handleAdminUI(w, r)
}

// handleAdminUI serves the admin interface
func (e *LegalPagesExtension) handleAdminUI(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write([]byte(adminTemplate))
}