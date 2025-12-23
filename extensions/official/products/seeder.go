package products

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/extensions/official/products/models"
)

// Seeder interface allows custom seeding implementations
type Seeder interface {
	// SeedVariables seeds custom variables
	SeedVariables(db *sql.DB) ([]models.Variable, error)

	// SeedGroupTemplates seeds custom group templates
	SeedGroupTemplates(db *sql.DB) ([]models.GroupTemplate, error)

	// SeedProductTemplates seeds custom product templates
	SeedProductTemplates(db *sql.DB) ([]models.ProductTemplate, error)

	// SeedPricingTemplates seeds custom pricing templates
	SeedPricingTemplates(db *sql.DB) ([]models.PricingTemplate, error)

	// ShouldSeed returns whether seeding should occur
	// Can be used to check if data already exists
	ShouldSeed(db *sql.DB) bool
}

// DefaultSeeder implements the default seeding behavior
type DefaultSeeder struct{}

// ShouldSeed checks if seeding should occur
func (d *DefaultSeeder) ShouldSeed(db *sql.DB) bool {
	var count int64
	db.QueryRow("SELECT COUNT(*) FROM ext_products_variables").Scan(&count)
	return count == 0 // Only seed if no data exists
}

// SeedVariables returns the default variables
func (d *DefaultSeeder) SeedVariables(db *sql.DB) ([]models.Variable, error) {
	return DefaultVariables(), nil
}

// SeedGroupTemplates returns the default group templates
func (d *DefaultSeeder) SeedGroupTemplates(db *sql.DB) ([]models.GroupTemplate, error) {
	return DefaultGroupTemplates(), nil
}

// SeedProductTemplates returns the default product templates
func (d *DefaultSeeder) SeedProductTemplates(db *sql.DB) ([]models.ProductTemplate, error) {
	return DefaultProductTemplates(), nil
}

// SeedPricingTemplates returns the default pricing templates
func (d *DefaultSeeder) SeedPricingTemplates(db *sql.DB) ([]models.PricingTemplate, error) {
	return DefaultPricingTemplates(), nil
}

// CustomSeeder allows for partial customization while using defaults
type CustomSeeder struct {
	DefaultSeeder

	// Optional custom implementations
	CustomVariables        func(db *sql.DB) ([]models.Variable, error)
	CustomGroupTemplates   func(db *sql.DB) ([]models.GroupTemplate, error)
	CustomProductTemplates func(db *sql.DB) ([]models.ProductTemplate, error)
	CustomPricingTemplates func(db *sql.DB) ([]models.PricingTemplate, error)
	CustomShouldSeed       func(db *sql.DB) bool
}

// SeedVariables returns custom or default variables
func (c *CustomSeeder) SeedVariables(db *sql.DB) ([]models.Variable, error) {
	if c.CustomVariables != nil {
		return c.CustomVariables(db)
	}
	return c.DefaultSeeder.SeedVariables(db)
}

// SeedGroupTemplates returns custom or default group templates
func (c *CustomSeeder) SeedGroupTemplates(db *sql.DB) ([]models.GroupTemplate, error) {
	if c.CustomGroupTemplates != nil {
		return c.CustomGroupTemplates(db)
	}
	return c.DefaultSeeder.SeedGroupTemplates(db)
}

// SeedProductTemplates returns custom or default product templates
func (c *CustomSeeder) SeedProductTemplates(db *sql.DB) ([]models.ProductTemplate, error) {
	if c.CustomProductTemplates != nil {
		return c.CustomProductTemplates(db)
	}
	return c.DefaultSeeder.SeedProductTemplates(db)
}

// SeedPricingTemplates returns custom or default pricing templates
func (c *CustomSeeder) SeedPricingTemplates(db *sql.DB) ([]models.PricingTemplate, error) {
	if c.CustomPricingTemplates != nil {
		return c.CustomPricingTemplates(db)
	}
	return c.DefaultSeeder.SeedPricingTemplates(db)
}

// ShouldSeed checks if seeding should occur
func (c *CustomSeeder) ShouldSeed(db *sql.DB) bool {
	if c.CustomShouldSeed != nil {
		return c.CustomShouldSeed(db)
	}
	return c.DefaultSeeder.ShouldSeed(db)
}

// SeedWithSeeder seeds the database using the provided seeder
func SeedWithSeeder(db *sql.DB, seeder Seeder) error {
	// Check if we should seed
	if !seeder.ShouldSeed(db) {
		return nil
	}

	now := apptime.NowTime()

	// Seed variables
	variables, err := seeder.SeedVariables(db)
	if err != nil {
		return err
	}
	for _, v := range variables {
		v.PrepareForCreate()
		var defaultValue *string
		if v.DefaultValue != nil {
			if s, ok := v.DefaultValue.(string); ok {
				defaultValue = &s
			}
		}
		_, err := db.Exec(`INSERT INTO ext_products_variables (name, display_name, value_type, type, default_value, description, status, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			v.Name, v.DisplayName, v.ValueType, v.Type, defaultValue, v.Description, v.Status, now, now)
		if err != nil {
			return fmt.Errorf("failed to seed variable %s: %w", v.Name, err)
		}
	}

	// Seed group templates
	groupTemplates, err := seeder.SeedGroupTemplates(db)
	if err != nil {
		return err
	}
	for _, gt := range groupTemplates {
		gt.PrepareForCreate()
		filterFieldsJSON, _ := json.Marshal(gt.FilterFieldsSchema)
		_, err := db.Exec(`INSERT INTO ext_products_group_templates (name, display_name, description, icon, filter_fields_schema, status, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
			gt.Name, gt.DisplayName, gt.Description, gt.Icon, filterFieldsJSON, gt.Status, now, now)
		if err != nil {
			return fmt.Errorf("failed to seed group template %s: %w", gt.Name, err)
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
		pt.PrepareForCreate()
		filterFieldsJSON, _ := json.Marshal(pt.FilterFieldsSchema)
		customFieldsJSON, _ := json.Marshal(pt.CustomFieldsSchema)
		pricingTemplatesJSON, _ := json.Marshal(pt.PricingTemplates)

		var intervalCount *int64
		if pt.BillingRecurringIntervalCount != nil {
			count := int64(*pt.BillingRecurringIntervalCount)
			intervalCount = &count
		}

		_, err := db.Exec(`INSERT INTO ext_products_product_templates
			(name, display_name, description, category, icon, filter_fields_schema, custom_fields_schema,
			 pricing_templates, billing_mode, billing_type, billing_recurring_interval, billing_recurring_interval_count,
			 status, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			pt.Name, pt.DisplayName, pt.Description, pt.Category, pt.Icon, filterFieldsJSON, customFieldsJSON,
			pricingTemplatesJSON, pt.BillingMode, pt.BillingType, pt.BillingRecurringInterval, intervalCount,
			pt.Status, now, now)
		if err != nil {
			fmt.Printf("SeedWithSeeder: Error creating product template %s: %v\n", pt.Name, err)
			return fmt.Errorf("failed to seed product template %s: %w", pt.Name, err)
		}
		fmt.Printf("SeedWithSeeder: Successfully created product template: %s\n", pt.Name)
	}

	// Seed pricing templates
	pricingTemplates, err := seeder.SeedPricingTemplates(db)
	if err != nil {
		return err
	}
	for _, pt := range pricingTemplates {
		pt.PrepareForCreate()
		variablesJSON, _ := json.Marshal(pt.Variables)
		_, err := db.Exec(`INSERT INTO ext_products_pricing_templates
			(name, display_name, description, price_formula, condition_formula, variables, category, status, created_at, updated_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			pt.Name, pt.DisplayName, pt.Description, pt.PriceFormula, pt.ConditionFormula, variablesJSON, pt.Category, pt.Status, now, now)
		if err != nil {
			return fmt.Errorf("failed to seed pricing template %s: %w", pt.Name, err)
		}
	}

	return nil
}
