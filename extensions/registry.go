package extensions

import (
	"fmt"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/extensions/official/analytics"
	"github.com/suppers-ai/solobase/extensions/official/cloudstorage"
	"github.com/suppers-ai/solobase/extensions/official/legalpages"
	// "github.com/suppers-ai/solobase/extensions/official/hugo" // Temporarily disabled - needs API updates
	"github.com/suppers-ai/solobase/extensions/official/products"
	"github.com/suppers-ai/solobase/extensions/official/webhooks"
	"gorm.io/gorm"
)

// RegisterAllExtensions registers all discovered extensions with the registry
func RegisterAllExtensions(registry *core.ExtensionRegistry, db *gorm.DB) error {
	// Register Products extension with database
	productsExt := products.NewProductsExtensionWithDB(db)
	if err := registry.Register(productsExt); err != nil {
		return fmt.Errorf("failed to register products extension: %w", err)
	}
	// Set the database to trigger migrations
	productsExt.SetDatabase(db)

	// Enable Products extension by default
	fmt.Printf("Enabling Products extension...\n")
	if err := registry.Enable("products"); err != nil {
		// Log but don't fail - extension can still work without being enabled
		fmt.Printf("Warning: Failed to enable Products extension: %v\n", err)
	} else {
		fmt.Printf("Products extension enabled successfully\n")
	}

	// Register Hugo extension
	// Hugo extension temporarily disabled - needs API updates
	// if err := registry.Register(hugo.NewHugoExtension()); err != nil {
	// 	return fmt.Errorf("failed to register hugo extension: %w", err)
	// }

	// Register Analytics extension with database
	analyticsExt := analytics.NewAnalyticsExtension()
	// Set the database to enable GORM operations
	analyticsExt.SetDatabase(db)
	if err := registry.Register(analyticsExt); err != nil {
		return fmt.Errorf("failed to register analytics extension: %w", err)
	}

	// Enable Analytics extension by default
	fmt.Printf("Enabling Analytics extension...\n")
	if err := registry.Enable("analytics"); err != nil {
		// Log but don't fail - extension can still work without being enabled
		fmt.Printf("Warning: Failed to enable Analytics extension: %v\n", err)
	} else {
		fmt.Printf("Analytics extension enabled successfully\n")
		// Debug: check if routes were registered
		fmt.Printf("DEBUG: Analytics extension should have registered routes\n")
	}

	// Register Cloud Storage extension with database
	cloudStorageExt := cloudstorage.NewCloudStorageExtensionWithDB(db, nil)
	// Set the database first to trigger migrations before registration
	cloudStorageExt.SetDatabase(db)

	if err := registry.Register(cloudStorageExt); err != nil {
		return fmt.Errorf("failed to register cloud storage extension: %w", err)
	}

	// Enable CloudStorage extension by default for hook functionality
	fmt.Printf("Enabling CloudStorage extension for hooks...\n")
	if err := registry.Enable("cloudstorage"); err != nil {
		// Log but don't fail - extension can still work without being enabled
		fmt.Printf("Warning: Failed to enable CloudStorage extension: %v\n", err)
	} else {
		fmt.Printf("CloudStorage extension enabled successfully\n")
	}

	// Register Webhooks extension
	if err := registry.Register(webhooks.NewWebhooksExtension()); err != nil {
		return fmt.Errorf("failed to register webhooks extension: %w", err)
	}

	// Register Legal Pages extension
	legalPagesExt := legalpages.NewLegalPagesExtension()
	// Set the database first to trigger migrations
	legalPagesExt.SetDatabase(db)

	if err := registry.Register(legalPagesExt); err != nil {
		return fmt.Errorf("failed to register legal pages extension: %w", err)
	}

	// Enable Legal Pages extension by default
	fmt.Printf("Enabling Legal Pages extension...\n")
	if err := registry.Enable("legalpages"); err != nil {
		// Log but don't fail - extension can still work without being enabled
		fmt.Printf("Warning: Failed to enable Legal Pages extension: %v\n", err)
	} else {
		fmt.Printf("Legal Pages extension enabled successfully\n")
	}

	return nil
}

// GetAvailableExtensions returns a list of all available extensions
func GetAvailableExtensions() []string {
	return []string{
		"products",
		"hugo",
		"analytics",
		"cloudstorage",
		"webhooks",
		"legalpages",
	}
}
