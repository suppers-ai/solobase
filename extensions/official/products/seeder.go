package products

import (
	"fmt"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"gorm.io/gorm"
)

// Seeder interface allows custom seeding implementations
type Seeder interface {
	// SeedVariables seeds custom variables
	SeedVariables(db *gorm.DB) ([]models.Variable, error)

	// SeedGroupTemplates seeds custom group templates
	SeedGroupTemplates(db *gorm.DB) ([]models.GroupTemplate, error)

	// SeedProductTemplates seeds custom product templates
	SeedProductTemplates(db *gorm.DB) ([]models.ProductTemplate, error)

	// SeedPricingTemplates seeds custom pricing templates
	SeedPricingTemplates(db *gorm.DB) ([]models.PricingTemplate, error)

	// ShouldSeed returns whether seeding should occur
	// Can be used to check if data already exists
	ShouldSeed(db *gorm.DB) bool
}

// DefaultSeeder implements the default seeding behavior
type DefaultSeeder struct{}

// ShouldSeed checks if seeding should occur
func (d *DefaultSeeder) ShouldSeed(db *gorm.DB) bool {
	var count int64
	db.Model(&models.Variable{}).Count(&count)
	return count == 0 // Only seed if no data exists
}

// SeedVariables returns the default variables
func (d *DefaultSeeder) SeedVariables(db *gorm.DB) ([]models.Variable, error) {
	return DefaultVariables(), nil
}

// SeedGroupTemplates returns the default group templates
func (d *DefaultSeeder) SeedGroupTemplates(db *gorm.DB) ([]models.GroupTemplate, error) {
	return DefaultGroupTemplates(), nil
}

// SeedProductTemplates returns the default product templates
func (d *DefaultSeeder) SeedProductTemplates(db *gorm.DB) ([]models.ProductTemplate, error) {
	return DefaultProductTemplates(), nil
}

// SeedPricingTemplates returns the default pricing templates
func (d *DefaultSeeder) SeedPricingTemplates(db *gorm.DB) ([]models.PricingTemplate, error) {
	return DefaultPricingTemplates(), nil
}

// CustomSeeder allows for partial customization while using defaults
type CustomSeeder struct {
	DefaultSeeder

	// Optional custom implementations
	CustomVariables        func(db *gorm.DB) ([]models.Variable, error)
	CustomGroupTemplates   func(db *gorm.DB) ([]models.GroupTemplate, error)
	CustomProductTemplates func(db *gorm.DB) ([]models.ProductTemplate, error)
	CustomPricingTemplates func(db *gorm.DB) ([]models.PricingTemplate, error)
	CustomShouldSeed      func(db *gorm.DB) bool
}

// SeedVariables returns custom or default variables
func (c *CustomSeeder) SeedVariables(db *gorm.DB) ([]models.Variable, error) {
	if c.CustomVariables != nil {
		return c.CustomVariables(db)
	}
	return c.DefaultSeeder.SeedVariables(db)
}

// SeedGroupTemplates returns custom or default group templates
func (c *CustomSeeder) SeedGroupTemplates(db *gorm.DB) ([]models.GroupTemplate, error) {
	if c.CustomGroupTemplates != nil {
		return c.CustomGroupTemplates(db)
	}
	return c.DefaultSeeder.SeedGroupTemplates(db)
}

// SeedProductTemplates returns custom or default product templates
func (c *CustomSeeder) SeedProductTemplates(db *gorm.DB) ([]models.ProductTemplate, error) {
	if c.CustomProductTemplates != nil {
		return c.CustomProductTemplates(db)
	}
	return c.DefaultSeeder.SeedProductTemplates(db)
}

// SeedPricingTemplates returns custom or default pricing templates
func (c *CustomSeeder) SeedPricingTemplates(db *gorm.DB) ([]models.PricingTemplate, error) {
	if c.CustomPricingTemplates != nil {
		return c.CustomPricingTemplates(db)
	}
	return c.DefaultSeeder.SeedPricingTemplates(db)
}

// ShouldSeed checks if seeding should occur
func (c *CustomSeeder) ShouldSeed(db *gorm.DB) bool {
	if c.CustomShouldSeed != nil {
		return c.CustomShouldSeed(db)
	}
	return c.DefaultSeeder.ShouldSeed(db)
}

// SeedWithSeeder seeds the database using the provided seeder
func SeedWithSeeder(db *gorm.DB, seeder Seeder) error {
	// Check if we should seed
	if !seeder.ShouldSeed(db) {
		return nil
	}

	// Seed variables
	variables, err := seeder.SeedVariables(db)
	if err != nil {
		return err
	}
	for _, v := range variables {
		if err := db.Create(&v).Error; err != nil {
			return err
		}
	}

	// Seed group templates
	groupTemplates, err := seeder.SeedGroupTemplates(db)
	if err != nil {
		return err
	}
	for _, gt := range groupTemplates {
		if err := db.Create(&gt).Error; err != nil {
			return err
		}
	}

	// Seed product templates
	productTemplates, err := seeder.SeedProductTemplates(db)
	if err != nil {
		return err
	}
	fmt.Printf("SeedWithSeeder: Seeding %d product templates\n", len(productTemplates))
	for _, pt := range productTemplates {
		fmt.Printf("SeedWithSeeder: Creating product template: %s\n", pt.Name)
		if err := db.Create(&pt).Error; err != nil {
			fmt.Printf("SeedWithSeeder: Error creating product template %s: %v\n", pt.Name, err)
			return err
		}
		fmt.Printf("SeedWithSeeder: Successfully created product template: %s\n", pt.Name)
	}

	// Seed pricing templates
	pricingTemplates, err := seeder.SeedPricingTemplates(db)
	if err != nil {
		return err
	}
	for _, pt := range pricingTemplates {
		if err := db.Create(&pt).Error; err != nil {
			return err
		}
	}

	return nil
}