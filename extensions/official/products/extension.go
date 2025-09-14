package products

import (
	"context"
	"encoding/json"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"gorm.io/gorm"
)

// ProductsExtension implements the products and pricing extension
type ProductsExtension struct {
	db        *gorm.DB
	adminAPI  *AdminAPI
	userAPI   *UserAPI
	publicAPI *PublicAPI
}

// NewProductsExtension creates a new products extension instance
func NewProductsExtension() *ProductsExtension {
	return &ProductsExtension{}
}

// NewProductsExtensionWithDB creates a new products extension with database
func NewProductsExtensionWithDB(db *gorm.DB) *ProductsExtension {
	ext := &ProductsExtension{
		db: db,
	}
	if db != nil {
		ext.initializeAPIs()
	}
	return ext
}

// Name returns the extension name
func (e *ProductsExtension) Name() string {
	return "products"
}

// Version returns the extension version
func (e *ProductsExtension) Version() string {
	return "1.0.0"
}

// Description returns the extension description
func (e *ProductsExtension) Description() string {
	return "Complete products and pricing management with dynamic fields and formula-based pricing"
}

// SetDatabase sets the database and initializes APIs
func (e *ProductsExtension) SetDatabase(db *gorm.DB) {
	e.db = db
	e.initializeAPIs()

	// Register models for auto-migration (tables have prefix in TableName methods)
	if err := models.RegisterModels(e.db); err != nil {
		// Log error but don't fail
		return
	}

	// Run seed data if needed
	if err := SeedData(e.db); err != nil {
		// Log error but don't fail
		return
	}
}

// initializeAPIs initializes all API handlers
func (e *ProductsExtension) initializeAPIs() {
	if e.db == nil {
		return
	}

	// Initialize services
	variableService := NewVariableService(e.db)
	groupService := NewGroupService(e.db)
	productService := NewProductService(e.db, variableService)
	pricingService := NewPricingService(e.db, variableService)

	// Initialize APIs with services
	e.adminAPI = NewAdminAPI(e.db, variableService, groupService, productService, pricingService)
	e.userAPI = NewUserAPI(e.db, groupService, productService, pricingService)
	e.publicAPI = NewPublicAPI(e.db, productService)
}

// GetAdminAPI returns the admin API
func (e *ProductsExtension) GetAdminAPI() *AdminAPI {
	return e.adminAPI
}

// GetUserAPI returns the user API
func (e *ProductsExtension) GetUserAPI() *UserAPI {
	return e.userAPI
}

// GetPublicAPI returns the public API
func (e *ProductsExtension) GetPublicAPI() *PublicAPI {
	return e.publicAPI
}

// Metadata returns extension metadata
func (e *ProductsExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "products",
		Version:     "1.0.0",
		Description: "Complete products and pricing management with dynamic fields and formula-based pricing",
		Author:      "Solobase Official",
		License:     "MIT",
		Tags:        []string{"products", "pricing", "ecommerce", "catalog"},
		Homepage:    "https://solobase.dev/extensions/products",
	}
}

// Initialize initializes the extension
func (e *ProductsExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	// For now, we'll handle initialization separately
	// The database is set via SetDatabase method
	return nil
}

// Start starts the extension
func (e *ProductsExtension) Start(ctx context.Context) error {
	return nil
}

// Stop stops the extension
func (e *ProductsExtension) Stop(ctx context.Context) error {
	return nil
}

// Health returns the health status of the extension
func (e *ProductsExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	if e.db == nil {
		return &core.HealthStatus{
			Status:  "unhealthy",
			Message: "Database not initialized",
		}, nil
	}

	// Check if we can query the database
	var count int64
	if err := e.db.WithContext(ctx).Table("variables").Count(&count).Error; err != nil {
		return &core.HealthStatus{
			Status:  "unhealthy",
			Message: "Failed to query database: " + err.Error(),
		}, err
	}

	return &core.HealthStatus{
		Status:  "healthy",
		Message: "Products extension is running",
	}, nil
}

// ApplyConfig applies configuration to the extension
func (e *ProductsExtension) ApplyConfig(config json.RawMessage) error {
	// For now, we don't have any specific configuration to apply
	return nil
}

// ConfigSchema returns the JSON schema for the extension's configuration
func (e *ProductsExtension) ConfigSchema() json.RawMessage {
	// Return a basic schema for now
	schema := `{
		"type": "object",
		"properties": {
			"enabled": {
				"type": "boolean",
				"description": "Enable or disable the products extension"
			}
		}
	}`
	return json.RawMessage(schema)
}

// DatabaseSchema returns the database schema for the extension
func (e *ProductsExtension) DatabaseSchema() string {
	return "products"
}

// RegisterHooks registers the extension's hooks
func (e *ProductsExtension) RegisterHooks() []core.HookRegistration {
	// No hooks to register for now
	return []core.HookRegistration{}
}

// RegisterMiddleware registers the extension's middleware
func (e *ProductsExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	// No middleware to register for now
	return []core.MiddlewareRegistration{}
}

// RegisterRoutes registers the extension's routes
func (e *ProductsExtension) RegisterRoutes(router core.ExtensionRouter) error {
	if e.userAPI == nil {
		// APIs not initialized yet
		return nil
	}

	// For now, just register the basic product list route for testing
	router.HandleFunc("/list", e.userAPI.ListMyProducts)

	return nil
}

// RegisterStaticAssets registers the extension's static assets
func (e *ProductsExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	// No static assets for now
	return []core.StaticAssetRegistration{}
}

// RegisterTemplates registers the extension's templates
func (e *ProductsExtension) RegisterTemplates() []core.TemplateRegistration {
	// No templates for now
	return []core.TemplateRegistration{}
}

// RequiredPermissions returns the permissions required by the extension
func (e *ProductsExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{Name: "products:read", Description: "Read products and pricing data"},
		{Name: "products:write", Description: "Create and modify products"},
		{Name: "products:admin", Description: "Administer product types and variables"},
	}
}

// ValidateConfig validates the extension's configuration
func (e *ProductsExtension) ValidateConfig(config json.RawMessage) error {
	// For now, accept any configuration
	return nil
}
