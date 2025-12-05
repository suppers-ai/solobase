package hugo

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/suppers-ai/solobase/extensions/core"
	"gorm.io/gorm"
)

// HugoExtension provides static site generation with Hugo
type HugoExtension struct {
	services *core.ExtensionServices
	db       *gorm.DB
	enabled  bool
	config   HugoConfig
	service  *HugoService
}

// NewHugoExtension creates a new Hugo extension
func NewHugoExtension() *HugoExtension {
	return &HugoExtension{
		enabled: true,
		config: HugoConfig{
			HugoBinaryPath:  "hugo",
			MaxSitesPerUser: 10,
			MaxSiteSize:     1073741824, // 1GB
			BuildTimeout:    "10m",
			AllowedThemes:   []string{"default", "blog", "portfolio"},
			DefaultTheme:    "default",
			StorageBucket:   "hugo-sites",
		},
	}
}

// Metadata returns extension metadata
func (e *HugoExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "hugo",
		Version:     "1.0.0",
		Description: "Static site generator powered by Hugo. Create, edit, and deploy beautiful static websites with ease. Features include multiple themes, file management, live preview, and one-click deployment.",
		Author:      "Solobase Official",
		License:     "MIT",
		Homepage:    "https://github.com/suppers-ai/solobase",
		Tags:        []string{"hugo", "static-site", "website", "cms", "blog"},
		MinVersion:  "1.0.0",
		MaxVersion:  "2.0.0",
	}
}

// Initialize initializes the extension
func (e *HugoExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	e.services = services
	services.Logger().Info(ctx, "Hugo extension initializing")

	// Initialize database tables if database is available
	if e.db != nil {
		if err := e.db.AutoMigrate(&HugoSite{}); err != nil {
			services.Logger().Error(ctx, fmt.Sprintf("Failed to migrate Hugo tables: %v", err))
			return err
		}
		services.Logger().Info(ctx, "Hugo tables migrated successfully")

		// Initialize the Hugo service
		e.service = NewHugoService(e.db, e.config)
	}

	return nil
}

// Start starts the extension
func (e *HugoExtension) Start(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Hugo extension started")
	}
	return nil
}

// Stop stops the extension
func (e *HugoExtension) Stop(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Hugo extension stopped")
	}
	e.enabled = false
	return nil
}

// Health returns health status
func (e *HugoExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	status := "healthy"
	if !e.enabled {
		status = "stopped"
	}

	// Check if Hugo is installed
	checks := []core.HealthCheck{
		{
			Name:   "database",
			Status: "healthy",
		},
	}

	// Try to check Hugo installation
	hugoInstalled := e.service != nil && e.service.CheckHugoInstalled()
	if hugoInstalled {
		checks = append(checks, core.HealthCheck{
			Name:   "hugo_binary",
			Status: "healthy",
		})
	} else {
		checks = append(checks, core.HealthCheck{
			Name:    "hugo_binary",
			Status:  "warning",
			Message: "Hugo binary not found. Please install Hugo to use this extension.",
		})
	}

	return &core.HealthStatus{
		Status:      status,
		Message:     "Hugo extension is running",
		LastChecked: time.Now(),
		Checks:      checks,
	}, nil
}

// RegisterRoutes registers extension routes
func (e *HugoExtension) RegisterRoutes(router core.ExtensionRouter) error {
	// Site management endpoints
	router.HandleFunc("/sites", e.handleSites)
	router.HandleFunc("/sites/{id}", e.handleSiteDetail)
	router.HandleFunc("/sites/{id}/build", e.handleBuildSite)
	router.HandleFunc("/sites/{id}/files", e.handleListFiles)
	router.HandleFunc("/sites/{id}/files/read", e.handleReadFile)
	router.HandleFunc("/sites/{id}/files/save", e.handleSaveFile)
	router.HandleFunc("/stats", e.handleStats)

	return nil
}

// RegisterMiddleware registers middleware
func (e *HugoExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	return []core.MiddlewareRegistration{}
}

// RegisterHooks registers hooks
func (e *HugoExtension) RegisterHooks() []core.HookRegistration {
	return []core.HookRegistration{}
}

// RegisterTemplates registers templates
func (e *HugoExtension) RegisterTemplates() []core.TemplateRegistration {
	return []core.TemplateRegistration{}
}

// RegisterStaticAssets registers static assets
func (e *HugoExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	return []core.StaticAssetRegistration{}
}

// ConfigSchema returns configuration schema
func (e *HugoExtension) ConfigSchema() json.RawMessage {
	schema := map[string]interface{}{
		"type": "object",
		"properties": map[string]interface{}{
			"hugo_binary_path": map[string]interface{}{
				"type":        "string",
				"description": "Path to Hugo binary",
				"default":     "hugo",
			},
			"max_sites_per_user": map[string]interface{}{
				"type":        "integer",
				"description": "Maximum sites per user",
				"default":     10,
			},
			"max_site_size": map[string]interface{}{
				"type":        "integer",
				"description": "Maximum site size in bytes",
				"default":     1073741824,
			},
			"build_timeout": map[string]interface{}{
				"type":        "string",
				"description": "Build timeout duration",
				"default":     "10m",
			},
		},
	}

	data, _ := json.Marshal(schema)
	return data
}

// ValidateConfig validates configuration
func (e *HugoExtension) ValidateConfig(config json.RawMessage) error {
	var cfg HugoConfig
	return json.Unmarshal(config, &cfg)
}

// ApplyConfig applies configuration
func (e *HugoExtension) ApplyConfig(config json.RawMessage) error {
	var cfg HugoConfig
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}

	e.config = cfg
	if e.service != nil {
		e.service.config = cfg
	}

	return nil
}

// DatabaseSchema returns database schema name
func (e *HugoExtension) DatabaseSchema() string {
	return "ext_hugo"
}

// SetDatabase sets the database instance for the extension
func (e *HugoExtension) SetDatabase(db *gorm.DB) {
	e.db = db
	if e.service == nil {
		e.service = NewHugoService(db, e.config)
	}
}

// RequiredPermissions returns required permissions
func (e *HugoExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "hugo.manage",
			Description: "Manage Hugo sites",
			Resource:    "hugo",
			Actions:     []string{"create", "read", "update", "delete"},
		},
		{
			Name:        "hugo.build",
			Description: "Build Hugo sites",
			Resource:    "hugo",
			Actions:     []string{"execute"},
		},
	}
}

// DashboardPath returns the dashboard path
func (e *HugoExtension) DashboardPath() string {
	return ""
}

// Documentation returns comprehensive documentation
func (e *HugoExtension) Documentation() core.ExtensionDocumentation {
	return core.ExtensionDocumentation{
		Overview: "The Hugo extension enables you to create, manage, and deploy static websites using the Hugo static site generator. Perfect for blogs, portfolios, documentation sites, and marketing pages.",
		DataCollected: []core.DataPoint{
			{
				Name:        "Site Content",
				Type:        "files",
				Description: "Markdown files, templates, and static assets for your Hugo sites",
				Purpose:     "Store and manage website content",
				Retention:   "Until site is deleted",
				Sensitive:   false,
			},
			{
				Name:        "Build History",
				Type:        "metadata",
				Description: "Build timestamps, duration, and status",
				Purpose:     "Track site build history and performance",
				Retention:   "30 days",
				Sensitive:   false,
			},
		},
		Endpoints: []core.EndpointDoc{
			{
				Path:        "/ext/hugo/api/sites",
				Methods:     []string{"GET", "POST"},
				Description: "List all sites or create a new site",
				Auth:        "Required",
			},
			{
				Path:        "/ext/hugo/api/sites/{id}/build",
				Methods:     []string{"POST"},
				Description: "Build and deploy a Hugo site",
				Auth:        "Required",
			},
			{
				Path:        "/ext/hugo/api/sites/{id}/files",
				Methods:     []string{"GET"},
				Description: "List files in a Hugo site",
				Auth:        "Required",
			},
		},
		UsageExamples: []core.UsageExample{
			{
				Title:       "Creating a New Site",
				Description: "Create a new Hugo site with a specific theme",
				Language:    "javascript",
				Code: `fetch('/admin/ext/hugo/sites', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    name: 'My Blog',
    domain: 'myblog.com',
    theme: 'default'
  })
})`,
			},
		},
	}
}

// Handler methods - delegate to handlers.go
func (e *HugoExtension) handleSites(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	switch r.Method {
	case http.MethodGet:
		e.listSites(w, r)
	case http.MethodPost:
		e.createSite(w, r)
	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

func (e *HugoExtension) handleSiteDetail(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	switch r.Method {
	case http.MethodGet:
		e.getSite(w, r)
	case http.MethodDelete:
		e.deleteSite(w, r)
	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

func (e *HugoExtension) handleBuildSite(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	e.buildSite(w, r)
}

func (e *HugoExtension) handleListFiles(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	e.listFiles(w, r)
}

func (e *HugoExtension) handleReadFile(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	e.readFile(w, r)
}

func (e *HugoExtension) handleSaveFile(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	e.saveFile(w, r)
}

func (e *HugoExtension) handleStats(w http.ResponseWriter, r *http.Request) {
	if e.service == nil {
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	e.getStats(w, r)
}
