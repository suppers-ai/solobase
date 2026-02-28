package products

import "github.com/suppers-ai/solobase/blocks/products/models"

// Column constants for raw SQL SELECT statements (used with QueryRaw)
const variableColumns = `id, name, display_name, value_type, type, default_value, description, status, created_at, updated_at`
const groupTemplateColumns = `id, name, display_name, description, icon, filter_fields_schema, status, created_at, updated_at`
const groupColumns = `id, user_id, group_template_id, name, description,
	filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
	filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
	filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
	filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
	filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
	custom_fields, created_at, updated_at`
const productTemplateColumns = `id, name, display_name, description, category, icon,
	filter_fields_schema, custom_fields_schema, pricing_templates,
	billing_mode, billing_type, billing_recurring_interval, billing_recurring_interval_count,
	status, created_at, updated_at`
const productColumns = `id, group_id, product_template_id, name, description, base_price, base_price_cents, currency,
	filter_numeric_1, filter_numeric_2, filter_numeric_3, filter_numeric_4, filter_numeric_5,
	filter_text_1, filter_text_2, filter_text_3, filter_text_4, filter_text_5,
	filter_boolean_1, filter_boolean_2, filter_boolean_3, filter_boolean_4, filter_boolean_5,
	filter_enum_1, filter_enum_2, filter_enum_3, filter_enum_4, filter_enum_5,
	filter_location_1, filter_location_2, filter_location_3, filter_location_4, filter_location_5,
	custom_fields, variables, pricing_formula, active, created_at, updated_at`
const pricingTemplateColumns = `id, name, display_name, description, price_formula, condition_formula, variables, category, status, created_at, updated_at`

// groupTemplateWithJSON is used internally for transaction-based queries
type groupTemplateWithJSON struct {
	models.GroupTemplate
	FilterFieldsSchemaJSON []byte
}

// productTemplateWithJSON is used internally for transaction-based queries
type productTemplateWithJSON struct {
	models.ProductTemplate
	FilterFieldsSchemaJSON []byte
}
