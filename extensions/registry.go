package extensions

import (
	"database/sql"
	"fmt"

	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/extensions/official/cloudstorage"
	"github.com/suppers-ai/solobase/extensions/official/legalpages"
	"github.com/suppers-ai/solobase/extensions/official/products"
)

// RegisterAllExtensions registers all discovered extensions with the registry
func RegisterAllExtensions(registry *core.ExtensionRegistry, sqlDB *sql.DB) error {
	return RegisterAllExtensionsWithOptions(registry, sqlDB, nil)
}

// RegisterAllExtensionsWithOptions registers all extensions with custom options
func RegisterAllExtensionsWithOptions(registry *core.ExtensionRegistry, sqlDB *sql.DB, productsSeeder interface{}) error {
	// Register Products extension WITHOUT database first
	productsExt := products.NewProductsExtension()

	// Set custom seeder BEFORE setting database
	if productsSeeder != nil {
		if seeder, ok := productsSeeder.(products.Seeder); ok {
			fmt.Printf("Setting custom theme seeder for Products extension\n")
			productsExt.SetSeeder(seeder)
		}
	}

	if err := registry.Register(productsExt); err != nil {
		return fmt.Errorf("failed to register products extension: %w", err)
	}
	// Set the SQL database for sqlc operations
	if sqlDB != nil {
		productsExt.SetSQLDatabase(sqlDB)
	}

	// Enable Products extension by default
	fmt.Printf("Enabling Products extension...\n")
	if err := registry.Enable("products"); err != nil {
		// Log but don't fail - extension can still work without being enabled
		fmt.Printf("Warning: Failed to enable Products extension: %v\n", err)
	} else {
		fmt.Printf("Products extension enabled successfully\n")
	}

	// Register Cloud Storage extension with database
	cloudStorageExtRaw := cloudstorage.NewCloudStorageExtension(nil)
	cloudStorageExt, ok := cloudStorageExtRaw.(*cloudstorage.CloudStorageExtension)
	if !ok {
		return fmt.Errorf("failed to cast cloud storage extension")
	}
	// Set the SQL database for sqlc operations
	if sqlDB != nil {
		cloudStorageExt.SetSQLDatabase(sqlDB)
	}

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

	// Register Legal Pages extension
	legalPagesExt := legalpages.NewLegalPagesExtension()
	// Set the SQL database for sqlc operations
	if sqlDB != nil {
		legalPagesExt.SetSQLDatabase(sqlDB)
	}

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
		"cloudstorage",
		"legalpages",
	}
}
