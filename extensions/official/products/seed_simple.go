package products

import (
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"gorm.io/gorm"
)

// SeedSimpleData creates simplified initial data for the products extension
func SeedSimpleData(db *gorm.DB) error {
	// Check if already seeded
	var count int64
	db.Model(&models.GroupTemplate{}).Count(&count)
	if count > 0 {
		return nil
	}

	// Create simple group templates
	groupTemplates := []models.GroupTemplate{
		{
			Name:        "restaurant",
			DisplayName: "Restaurant",
			Description: "Food service establishment",
			Icon:        "utensils",
			Status:      "active",
			Fields: []models.FieldDefinition{
				{
					ID:          "filter_enum_1",
					Name:        "Cuisine Type",
					Type:        "enum",
					Required:    true,
					Description: "Type of cuisine served",
					Constraints: models.FieldConstraints{
						Options: []string{"Italian", "Chinese", "Mexican", "American", "Indian", "Japanese", "Thai", "Other"},
					},
				},
				{
					ID:          "filter_numeric_1",
					Name:        "Seating Capacity",
					Type:        "numeric",
					Required:    false,
					Description: "Number of seats available",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_boolean_1",
					Name:        "Accepts Reservations",
					Type:        "boolean",
					Required:    false,
					Description: "Whether the restaurant accepts reservations",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_text_1",
					Name:        "Special Features",
					Type:        "text",
					Required:    false,
					Description: "Special features or amenities",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "retail_store",
			DisplayName: "Retail Store",
			Description: "Physical or online retail business",
			Icon:        "shopping-bag",
			Status:      "active",
			Fields: []models.FieldDefinition{
				{
					ID:          "filter_enum_1",
					Name:        "Store Type",
					Type:        "enum",
					Required:    true,
					Description: "Type of retail store",
					Constraints: models.FieldConstraints{
						Options: []string{"Physical", "Online", "Hybrid"},
					},
				},
				{
					ID:          "filter_text_1",
					Name:        "Location",
					Type:        "text",
					Required:    false,
					Description: "Store location or headquarters",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_boolean_1",
					Name:        "Offers Delivery",
					Type:        "boolean",
					Required:    false,
					Description: "Whether the store offers delivery",
					Constraints: models.FieldConstraints{},
				},
			},
		},
	}

	for _, et := range groupTemplates {
		if err := db.Create(&et).Error; err != nil {
			return err
		}
	}

	// Create simple product templates
	productTemplates := []models.ProductTemplate{
		{
			Name:        "physical_product",
			DisplayName: "Physical Product",
			Description: "Tangible goods that require shipping",
			Category:    "product",
			Icon:        "package",
			BillingMode: "instant",
			BillingType: "one-time",
			Status:      "active",
			Fields: []models.FieldDefinition{
				{
					ID:          "filter_text_1",
					Name:        "SKU",
					Type:        "text",
					Required:    true,
					Description: "Stock Keeping Unit",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_numeric_1",
					Name:        "Weight",
					Type:        "numeric",
					Required:    false,
					Description: "Product weight in kg",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_boolean_1",
					Name:        "Fragile",
					Type:        "boolean",
					Required:    false,
					Description: "Requires special handling",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "service",
			DisplayName: "Service",
			Description: "Time-based or project-based services",
			Category:    "service",
			Icon:        "briefcase",
			BillingMode: "approval",
			BillingType: "one-time",
			Status:      "active",
			Fields: []models.FieldDefinition{
				{
					ID:          "filter_numeric_1",
					Name:        "Duration (hours)",
					Type:        "numeric",
					Required:    true,
					Description: "Service duration in hours",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_text_1",
					Name:        "Location",
					Type:        "text",
					Required:    false,
					Description: "Service location",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          "filter_enum_1",
					Name:        "Skill Level",
					Type:        "enum",
					Required:    true,
					Description: "Required skill level",
					Constraints: models.FieldConstraints{
						Options: []string{"Beginner", "Intermediate", "Advanced", "Expert"},
					},
				},
			},
		},
	}

	for _, pt := range productTemplates {
		if err := db.Create(&pt).Error; err != nil {
			return err
		}
	}

	// Create sample pricing templates
	pricingTemplates := []models.PricingTemplate{
		{
			Name:             "volume_discount",
			DisplayName:      "Volume Discount",
			Description:      "Quantity-based tiered pricing",
			Category:         "discount",
			PriceFormula:     "base_price * quantity * (quantity >= 100 ? 0.7 : quantity >= 50 ? 0.8 : quantity >= 10 ? 0.9 : 1.0)",
			ConditionFormula: "quantity >= 10",
			IsActive:         true,
			Variables: models.JSONB{
				"required": []string{"base_price", "quantity"},
			},
		},
		{
			Name:             "member_discount",
			DisplayName:      "Member Discount",
			Description:      "Special pricing for members",
			Category:         "membership",
			PriceFormula:     "base_price * (is_premium_member ? 0.75 : is_member ? 0.9 : 1.0)",
			ConditionFormula: "is_member == true",
			IsActive:         true,
			Variables: models.JSONB{
				"required": []string{"base_price", "is_member", "is_premium_member"},
			},
		},
	}

	for _, pt := range pricingTemplates {
		if err := db.Create(&pt).Error; err != nil {
			return err
		}
	}

	// Create sample variables (only user variables in DB, system variables are hard-coded)
	variables := []models.Variable{
		{
			Name:        "base_price",
			DisplayName: "Base Price",
			ValueType:   "number",
			Type:        "user",
			Description: "Base price before any modifications",
			IsActive:    true,
		},
		{
			Name:        "quantity",
			DisplayName: "Quantity",
			ValueType:   "number",
			Type:        "user",
			Description: "Number of items being purchased",
			IsActive:    true,
		},
		{
			Name:        "is_member",
			DisplayName: "Is Member",
			ValueType:   "boolean",
			Type:        "user",
			Description: "Whether the customer is a member",
			IsActive:    true,
		},
		{
			Name:        "is_premium_member",
			DisplayName: "Is Premium Member",
			ValueType:   "boolean",
			Type:        "user",
			Description: "Whether the customer is a premium member",
			IsActive:    true,
		},
		{
			Name:         "discount_percentage",
			DisplayName:  "Discount Percentage",
			ValueType:    "number",
			Type:         "user",
			Description:  "Discount percentage to apply",
			DefaultValue: 0,
			IsActive:     true,
		},
		{
			Name:         "tax_rate",
			DisplayName:  "Tax Rate",
			ValueType:    "number",
			Type:         "user",
			Description:  "Applicable tax rate",
			DefaultValue: 0.08,
			IsActive:     true,
		},
	}

	for _, v := range variables {
		if err := db.Create(&v).Error; err != nil {
			return err
		}
	}

	return nil
}
