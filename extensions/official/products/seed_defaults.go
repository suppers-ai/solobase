package products

import "github.com/suppers-ai/solobase/extensions/official/products/models"

// DefaultVariables returns the default variables for seeding
// This is exported so custom seeders can extend the defaults
func DefaultVariables() []models.Variable {
	return []models.Variable{
		// Product variables
		{Name: "quantity", DisplayName: "Quantity", ValueType: "number", Type: "user", DefaultValue: 1.0, Description: "Number of items"},
		{Name: "weight", DisplayName: "Weight (kg)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product weight in kilograms"},
		{Name: "volume", DisplayName: "Volume (L)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product volume in liters"},
		{Name: "dimensions_length", DisplayName: "Length (cm)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product length"},
		{Name: "dimensions_width", DisplayName: "Width (cm)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product width"},
		{Name: "dimensions_height", DisplayName: "Height (cm)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product height"},

		// Order variables
		{Name: "order_total", DisplayName: "Order Total", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Total order value before discounts"},
		{Name: "order_item_count", DisplayName: "Item Count", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Number of different items in order"},
		{Name: "order_quantity_total", DisplayName: "Total Quantity", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Total quantity of all items"},

		// Shipping variables
		{Name: "shipping_distance", DisplayName: "Shipping Distance (km)", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Distance to shipping destination"},
		{Name: "shipping_method", DisplayName: "Shipping Method", ValueType: "string", Type: "user", DefaultValue: "standard", Description: "Selected shipping method"},
		{Name: "shipping_zone", DisplayName: "Shipping Zone", ValueType: "string", Type: "user", DefaultValue: "local", Description: "Shipping zone based on destination"},
		{Name: "shipping_rate_per_kg", DisplayName: "Rate per kg", ValueType: "number", Type: "user", DefaultValue: 4.99, Description: "Shipping rate per kilogram"},
		{Name: "express_shipping", DisplayName: "Express Shipping", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether express shipping is selected"},
		{Name: "express_shipping_surcharge", DisplayName: "Express Surcharge", ValueType: "number", Type: "user", DefaultValue: 20.0, Description: "Additional charge for express shipping"},

		// Promotion variables
		{Name: "discount_percentage", DisplayName: "Discount %", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Discount percentage"},
		{Name: "discount_amount", DisplayName: "Discount Amount", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Fixed discount amount"},
		{Name: "promo_code", DisplayName: "Promo Code", ValueType: "string", Type: "user", DefaultValue: "", Description: "Applied promotional code"},
		{Name: "bulk_discount_threshold", DisplayName: "Bulk Threshold", ValueType: "number", Type: "user", DefaultValue: 10.0, Description: "Minimum quantity for bulk discount"},
		{Name: "bulk_discount_rate", DisplayName: "Bulk Discount Rate", ValueType: "number", Type: "user", DefaultValue: 0.1, Description: "Discount rate for bulk purchases"},

		// Location variables
		{Name: "tax_rate", DisplayName: "Tax Rate", ValueType: "number", Type: "user", DefaultValue: 0.08, Description: "Local tax rate"},
		{Name: "currency", DisplayName: "Currency", ValueType: "string", Type: "user", DefaultValue: "USD", Description: "Transaction currency"},
		{Name: "country", DisplayName: "Country", ValueType: "string", Type: "user", DefaultValue: "US", Description: "Destination country"},
		{Name: "state", DisplayName: "State/Province", ValueType: "string", Type: "user", DefaultValue: "", Description: "Destination state or province"},
		{Name: "city", DisplayName: "City", ValueType: "string", Type: "user", DefaultValue: "", Description: "Destination city"},
		{Name: "postal_code", DisplayName: "Postal Code", ValueType: "string", Type: "user", DefaultValue: "", Description: "Destination postal code"},

		// Service variables
		{Name: "urgency", DisplayName: "Urgency Level", ValueType: "string", Type: "user", DefaultValue: "normal", Description: "Service urgency level"},
		{Name: "complexity", DisplayName: "Complexity", ValueType: "number", Type: "user", DefaultValue: 1.0, Description: "Service complexity (1-5 scale)"},
		{Name: "hourly_rate", DisplayName: "Hourly Rate", ValueType: "number", Type: "user", DefaultValue: 100.0, Description: "Base hourly rate for services"},
		{Name: "hours", DisplayName: "Hours", ValueType: "number", Type: "user", DefaultValue: 1.0, Description: "Number of service hours"},
		{Name: "service_level", DisplayName: "Service Level", ValueType: "string", Type: "user", DefaultValue: "standard", Description: "Selected service level"},

		// System variables
		{Name: "base_price", DisplayName: "Base Price", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Base product price"},
		{Name: "cost", DisplayName: "Cost", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Product cost"},
		{Name: "margin", DisplayName: "Margin", ValueType: "number", Type: "user", DefaultValue: 0.3, Description: "Default profit margin"},
		{Name: "subtotal", DisplayName: "Subtotal", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Subtotal before taxes and fees"},
		{Name: "total_discount", DisplayName: "Total Discount", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Total discount amount applied"},
		{Name: "total_tax", DisplayName: "Total Tax", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Total tax amount"},
		{Name: "total_fees", DisplayName: "Total Fees", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Total fees amount"},
		{Name: "final_price", DisplayName: "Final Price", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Final calculated price"},
		{Name: "calculation_date", DisplayName: "Calculation Date", ValueType: "date", Type: "user", DefaultValue: "now", Description: "Date of price calculation"},
		{Name: "is_taxable", DisplayName: "Is Taxable", ValueType: "boolean", Type: "user", DefaultValue: true, Description: "Whether product is taxable"},
		{Name: "tax_exempt", DisplayName: "Tax Exempt", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether customer is tax exempt"},

		// Time-based variables
		{Name: "current_hour", DisplayName: "Current Hour", ValueType: "number", Type: "user", DefaultValue: 12, Description: "Current hour (0-23)"},
		{Name: "current_day_of_week", DisplayName: "Day of Week", ValueType: "number", Type: "user", DefaultValue: 1, Description: "Current day of week (1-7)"},
		{Name: "current_month", DisplayName: "Current Month", ValueType: "number", Type: "user", DefaultValue: 1, Description: "Current month (1-12)"},
		{Name: "is_weekend", DisplayName: "Is Weekend", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether it's weekend"},
		{Name: "is_holiday", DisplayName: "Is Holiday", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether it's a holiday"},
		{Name: "is_peak_time", DisplayName: "Is Peak Time", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether it's peak hours"},
		{Name: "days_until_delivery", DisplayName: "Days Until Delivery", ValueType: "number", Type: "user", DefaultValue: 0, Description: "Number of days until delivery"},
		{Name: "lead_time", DisplayName: "Lead Time", ValueType: "number", Type: "user", DefaultValue: 1, Description: "Product lead time in days"},

		// Customer variables
		{Name: "customer_type", DisplayName: "Customer Type", ValueType: "string", Type: "user", DefaultValue: "regular", Description: "Type of customer"},
		{Name: "is_member", DisplayName: "Is Member", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether customer is a member"},
		{Name: "is_premium_member", DisplayName: "Is Premium Member", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether customer is a premium member"},
		{Name: "customer_loyalty_points", DisplayName: "Loyalty Points", ValueType: "number", Type: "user", DefaultValue: 0, Description: "Customer's loyalty points"},
		{Name: "customer_lifetime_value", DisplayName: "Customer LTV", ValueType: "number", Type: "user", DefaultValue: 0, Description: "Customer lifetime value"},
		{Name: "is_first_purchase", DisplayName: "First Purchase", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether this is customer's first purchase"},
		{Name: "referral_code", DisplayName: "Referral Code", ValueType: "string", Type: "user", DefaultValue: "", Description: "Referral code used"},

		// Inventory variables
		{Name: "stock_level", DisplayName: "Stock Level", ValueType: "number", Type: "user", DefaultValue: 100, Description: "Current stock level"},
		{Name: "low_stock_threshold", DisplayName: "Low Stock Threshold", ValueType: "number", Type: "user", DefaultValue: 10, Description: "Low stock alert threshold"},
		{Name: "is_low_stock", DisplayName: "Is Low Stock", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether stock is low"},
		{Name: "backorder_allowed", DisplayName: "Backorder Allowed", ValueType: "boolean", Type: "user", DefaultValue: false, Description: "Whether backorders are allowed"},
		{Name: "reserved_quantity", DisplayName: "Reserved Quantity", ValueType: "number", Type: "user", DefaultValue: 0, Description: "Quantity reserved in other orders"},

		// Fee variables
		{Name: "processing_fee", DisplayName: "Processing Fee", ValueType: "number", Type: "user", DefaultValue: 2.99, Description: "Payment processing fee"},
		{Name: "handling_fee", DisplayName: "Handling Fee", ValueType: "number", Type: "user", DefaultValue: 0.0, Description: "Order handling fee"},
		{Name: "rush_fee", DisplayName: "Rush Fee", ValueType: "number", Type: "user", DefaultValue: 25.0, Description: "Rush order fee"},
		{Name: "cancellation_fee", DisplayName: "Cancellation Fee", ValueType: "number", Type: "user", DefaultValue: 10.0, Description: "Order cancellation fee"},
		{Name: "restocking_fee", DisplayName: "Restocking Fee", ValueType: "number", Type: "user", DefaultValue: 0.15, Description: "Restocking fee percentage"},
	}
}

// DefaultGroupTemplates returns the default group templates
// This is exported so custom seeders can extend the defaults
func DefaultGroupTemplates() []models.GroupTemplate {
	return []models.GroupTemplate{
		{
			Name:        "restaurant",
			DisplayName: "Restaurant",
			Description: "Food service establishment",
			Icon:        "utensils",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterEnum1),
					Name:        "Cuisine Type",
					Type:        "enum",
					Required:    true,
					Description: "Type of cuisine served",
					Constraints: models.FieldConstraints{
						Options: []string{"Italian", "Chinese", "Mexican", "American", "Indian", "Japanese", "Thai", "Other"},
					},
				},
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Seating Capacity",
					Type:        "numeric",
					Required:    false,
					Description: "Number of seats available",
					Constraints: models.FieldConstraints{
						Min: func() *float64 { v := 1.0; return &v }(),
						Max: func() *float64 { v := 1000.0; return &v }(),
					},
				},
				{
					ID:          string(models.FilterNumeric2),
					Name:        "Delivery Radius",
					Type:        "numeric",
					Required:    false,
					Description: "Delivery service radius in miles",
					Constraints: models.FieldConstraints{
						Min: func() *float64 { v := 0.0; return &v }(),
						Max: func() *float64 { v := 50.0; return &v }(),
					},
				},
				{
					ID:          string(models.FilterBoolean1),
					Name:        "Accepts Reservations",
					Type:        "boolean",
					Required:    false,
					Description: "Whether the restaurant accepts reservations",
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
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterEnum1),
					Name:        "Store Type",
					Type:        "enum",
					Required:    true,
					Description: "Type of retail store",
					Constraints: models.FieldConstraints{
						Options: []string{"Physical", "Online", "Hybrid"},
					},
				},
				{
					ID:          string(models.FilterText1),
					Name:        "Location",
					Type:        "text",
					Required:    false,
					Description: "Store location or headquarters",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "service_provider",
			DisplayName: "Service Provider",
			Description: "Professional or technical services",
			Icon:        "briefcase",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterText1),
					Name:        "Service Type",
					Type:        "text",
					Description: "Type of service provided",
					Constraints: models.FieldConstraints{
						Required: true,
					},
				},
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Team Size",
					Type:        "numeric",
					Description: "Number of team members",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "subscription_service",
			DisplayName: "Subscription Service",
			Description: "Recurring subscription-based business",
			Icon:        "credit-card",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterEnum1),
					Name:        "Billing Cycle",
					Type:        "enum",
					Description: "Billing frequency",
					Constraints: models.FieldConstraints{
						Options:  []string{"Monthly", "Quarterly", "Yearly"},
						Required: true,
					},
				},
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Trial Period (days)",
					Type:        "numeric",
					Description: "Number of trial days offered",
					Constraints: models.FieldConstraints{},
				},
			},
		},
	}
}

// DefaultProductTemplates returns the default product templates
// This is exported so custom seeders can extend the defaults
func DefaultProductTemplates() []models.ProductTemplate {
	return []models.ProductTemplate{
		{
			Name:        "physical_product",
			DisplayName: "Physical Product",
			Description: "Tangible goods that require shipping",
			Category:    "physical",
			Icon:        "package",
			BillingMode: "instant",
			BillingType: "one-time",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterText1),
					Name:        "SKU",
					Type:        "text",
					Description: "Stock Keeping Unit",
					Constraints: models.FieldConstraints{
						Required: true,
					},
				},
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Weight (kg)",
					Type:        "numeric",
					Description: "Product weight in kilograms",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          string(models.FilterText2),
					Name:        "Manufacturer",
					Type:        "text",
					Description: "Product manufacturer",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "digital_product",
			DisplayName: "Digital Product",
			Description: "Downloadable or online products",
			Category:    "digital",
			Icon:        "download",
			BillingMode: "instant",
			BillingType: "one-time",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterNumeric1),
					Name:        "File Size (MB)",
					Type:        "numeric",
					Description: "Size of the digital file in megabytes",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          string(models.FilterText1),
					Name:        "File Format",
					Type:        "text",
					Description: "File format (PDF, ZIP, etc.)",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          string(models.FilterEnum1),
					Name:        "License Type",
					Type:        "enum",
					Description: "Type of license for this digital product",
					Constraints: models.FieldConstraints{
						Options: []string{"Single User", "Team", "Enterprise", "Unlimited"},
					},
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
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Duration (hours)",
					Type:        "numeric",
					Description: "Service duration in hours",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          string(models.FilterText1),
					Name:        "Service Location",
					Type:        "text",
					Description: "Where the service will be provided",
					Constraints: models.FieldConstraints{},
				},
				{
					ID:          string(models.FilterText2),
					Name:        "Special Requirements",
					Type:        "text",
					Description: "Any special requirements for the service",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:                          "subscription",
			DisplayName:                   "Subscription",
			Description:                   "Recurring subscription products",
			Category:                      "subscription",
			Icon:                          "calendar",
			BillingMode:                   "instant",
			BillingType:                   "recurring",
			BillingRecurringInterval:      func() *string { v := "month"; return &v }(),
			BillingRecurringIntervalCount: func() *int { v := 1; return &v }(),
			Status:                        "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterEnum1),
					Name:        "Billing Cycle",
					Type:        "enum",
					Description: "Subscription billing frequency",
					Constraints: models.FieldConstraints{
						Options:  []string{"Monthly", "Quarterly", "Annual"},
						Required: true,
						Default:  "Monthly",
					},
				},
				{
					ID:          string(models.FilterNumeric2),
					Name:        "User Limit",
					Type:        "numeric",
					Description: "Maximum number of users",
					Constraints: models.FieldConstraints{},
				},
			},
		},
		{
			Name:        "bundle",
			DisplayName: "Product Bundle",
			Description: "Collection of multiple products",
			Category:    "physical",
			Icon:        "box",
			BillingMode: "instant",
			BillingType: "one-time",
			Status:      "active",
			FilterFieldsSchema:[]models.FieldDefinition{
				{
					ID:          string(models.FilterNumeric1),
					Name:        "Number of Items",
					Type:        "numeric",
					Description: "Number of items in the bundle",
					Constraints: models.FieldConstraints{
						Required: true,
					},
				},
				{
					ID:          string(models.FilterNumeric2),
					Name:        "Savings (%)",
					Type:        "numeric",
					Description: "Percentage saved by purchasing the bundle",
					Constraints: models.FieldConstraints{
						Default: 10,
					},
				},
			},
		},
	}
}

// DefaultPricingTemplates returns the default pricing templates
// This is exported so custom seeders can extend the defaults
func DefaultPricingTemplates() []models.PricingTemplate {
	return []models.PricingTemplate{
		{
			Name:         "volume_discount",
			DisplayName:  "Volume Discount",
			Description:  "Quantity-based tiered pricing",
			Category:     "discount",
			PriceFormula: "base_price * quantity * (quantity >= 100 ? 0.7 : quantity >= 50 ? 0.8 : quantity >= 10 ? 0.9 : 1.0)",
			Variables: models.JSONB{
				"required": []string{"base_price", "quantity"},
			},
		},
		{
			Name:         "progressive_calculation",
			DisplayName:  "Progressive Calculation",
			Description:  "Step-by-step price calculation with running total",
			Category:     "complex",
			PriceFormula: "((running_total = base_price * quantity) + (running_total * tax_rate * (tax_exempt ? 0 : 1)) + processing_fee + (express_shipping ? express_shipping_surcharge : 0))",
			Variables: models.JSONB{
				"required": []string{"base_price", "quantity", "running_total", "tax_rate", "tax_exempt", "processing_fee"},
			},
		},
		{
			Name:         "time_based_pricing",
			DisplayName:  "Time-based Pricing",
			Description:  "Different prices based on time of day or day of week",
			Category:     "dynamic",
			PriceFormula: "base_price * (is_peak_time ? 1.5 : is_weekend ? 1.2 : 1.0)",
			Variables: models.JSONB{
				"required": []string{"base_price", "is_peak_time", "is_weekend"},
			},
		},
		{
			Name:         "zone_based_shipping",
			DisplayName:  "Zone-based Shipping",
			Description:  "Shipping costs based on delivery zones",
			Category:     "shipping",
			PriceFormula: "weight * (shipping_zone == 'international' ? 15.99 : shipping_zone == 'national' ? 7.99 : 3.99)",
			Variables: models.JSONB{
				"required": []string{"weight", "shipping_zone"},
			},
		},
		{
			Name:         "membership_discount",
			DisplayName:  "Membership Discount",
			Description:  "Special pricing for members",
			Category:     "membership",
			PriceFormula: "base_price * (is_premium_member ? 0.75 : is_member ? 0.9 : 1.0)",
			Variables: models.JSONB{
				"required": []string{"base_price", "is_member", "is_premium_member"},
			},
		},
		{
			Name:         "cost_plus_margin",
			DisplayName:  "Cost Plus Margin",
			Description:  "Simple cost plus margin pricing",
			Category:     "standard",
			PriceFormula: "cost * (1 + margin)",
			Variables: models.JSONB{
				"required": []string{"cost", "margin"},
			},
		},
		{
			Name:         "bundle_pricing",
			DisplayName:  "Bundle Pricing",
			Description:  "Discounted pricing for product bundles",
			Category:     "bundle",
			PriceFormula: "(item1_price + item2_price + item3_price) * bundle_discount_rate",
			Variables: models.JSONB{
				"required": []string{"item1_price", "item2_price", "item3_price", "bundle_discount_rate"},
			},
		},
		{
			Name:         "surge_pricing",
			DisplayName:  "Surge Pricing",
			Description:  "Dynamic pricing based on demand",
			Category:     "dynamic",
			PriceFormula: "base_price * demand_multiplier * (1 + (current_utilization / 100))",
			Variables: models.JSONB{
				"required": []string{"base_price", "demand_multiplier", "current_utilization"},
			},
		},
		{
			Name:         "subscription_tiers",
			DisplayName:  "Subscription Tiers",
			Description:  "Tiered subscription pricing",
			Category:     "subscription",
			PriceFormula: "seats * (seats > 100 ? 8 : seats > 20 ? 10 : seats > 5 ? 12 : 15) * (billing_cycle == 'annual' ? 10 : 1)",
			Variables: models.JSONB{
				"required": []string{"seats", "billing_cycle"},
			},
		},
		{
			Name:         "complete_pricing",
			DisplayName:  "Complete Pricing with All Factors",
			Description:  "Comprehensive pricing including all discounts, taxes, and fees",
			Category:     "complete",
			PriceFormula: "((subtotal = base_price * quantity) - (total_discount = subtotal * (discount_percentage / 100 + (is_member ? 0.1 : 0) + (is_first_purchase ? 0.15 : 0))) + (total_tax = (subtotal - total_discount) * tax_rate * (tax_exempt ? 0 : 1)) + (total_fees = processing_fee + handling_fee + (express_shipping ? rush_fee : 0)))",
			Variables: models.JSONB{
				"required": []string{"base_price", "quantity", "subtotal", "total_discount", "total_tax", "total_fees", "tax_rate"},
			},
		},
		{
			Name:         "dynamic_markup",
			DisplayName:  "Dynamic Markup Based on Inventory",
			Description:  "Adjust pricing based on stock levels and demand",
			Category:     "dynamic",
			PriceFormula: "base_price * (1 + margin) * (is_low_stock ? 1.2 : 1.0) * (is_peak_time ? 1.15 : 1.0) * quantity",
			Variables: models.JSONB{
				"required": []string{"base_price", "margin", "is_low_stock", "is_peak_time", "quantity"},
			},
		},
		{
			Name:         "loyalty_rewards",
			DisplayName:  "Loyalty Rewards Pricing",
			Description:  "Price calculation with loyalty points redemption",
			Category:     "loyalty",
			PriceFormula: "max(0, (base_price * quantity * (1 - discount_percentage / 100)) - (customer_loyalty_points * 0.01))",
			Variables: models.JSONB{
				"required": []string{"base_price", "quantity", "discount_percentage", "customer_loyalty_points"},
			},
		},
	}
}