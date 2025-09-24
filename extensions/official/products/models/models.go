package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"gorm.io/gorm"
	"time"
)

// JSONB handles JSON data in database
type JSONB map[string]interface{}

func (j JSONB) Value() (driver.Value, error) {
	if j == nil {
		return nil, nil
	}
	return json.Marshal(j)
}

func (j *JSONB) Scan(value interface{}) error {
	if value == nil {
		*j = make(JSONB)
		return nil
	}
	bytes, ok := value.([]byte)
	if !ok {
		return nil
	}
	return json.Unmarshal(bytes, j)
}

// Variable represents a pricing variable
type Variable struct {
	ID           uint        `gorm:"primaryKey" json:"id"`
	Name         string      `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName  string      `json:"display_name"`
	ValueType    string      `json:"value_type"` // number, string, boolean
	Type         string      `json:"type"`       // user, system
	DefaultValue interface{} `gorm:"type:jsonb" json:"default_value"`
	Description  string      `json:"description"`
	IsActive     bool        `gorm:"default:true" json:"is_active"`
	CreatedAt    time.Time   `json:"created_at"`
	UpdatedAt    time.Time   `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (Variable) TableName() string {
	return "ext_products_variables"
}

// FieldConstraints defines constraints for a field
type FieldConstraints struct {
	Required    bool        `json:"required,omitempty"`
	Min         *float64    `json:"min,omitempty"`
	Max         *float64    `json:"max,omitempty"`
	MinLength   *int        `json:"min_length,omitempty"`
	MaxLength   *int        `json:"max_length,omitempty"`
	Pattern     string      `json:"pattern,omitempty"`
	Options     []string    `json:"options,omitempty"` // For select/enum types
	Default     interface{} `json:"default,omitempty"`
	Placeholder string      `json:"placeholder,omitempty"`
}

// FieldDefinition defines a custom field with its constraints (for filter fields)
type FieldDefinition struct {
	ID          string           `json:"id"`   // Must be a valid FilterFieldID value (e.g., "filter_text_1", "filter_numeric_1")
	Name        string           `json:"name"` // Display name for the field
	Type        string           `json:"type"` // numeric, text, boolean, enum, location
	Required    bool             `json:"required"`
	Description string           `json:"description,omitempty"`
	Constraints FieldConstraints `json:"constraints"`
}

// CustomFieldDefinition defines a field stored in CustomFields JSON (non-indexed)
type CustomFieldDefinition struct {
	ID          string      `json:"id"`   // Unique identifier for the custom field
	Name        string      `json:"name"` // Display name for the field
	Type        string      `json:"type"` // text, numeric, boolean, enum, color, date, email, url, textarea
	Required    bool        `json:"required"`
	Description string      `json:"description,omitempty"`
	Default     interface{} `json:"default,omitempty"`
	Section     string      `json:"section,omitempty"` // Section/tab this field belongs to
	Order       int         `json:"order,omitempty"`   // Display order within section
	// Type-specific constraints
	Options     []string `json:"options,omitempty"`      // For enum/select types
	Min         *float64 `json:"min,omitempty"`          // For numeric types
	Max         *float64 `json:"max,omitempty"`          // For numeric types
	MinLength   *int     `json:"min_length,omitempty"`   // For text types
	MaxLength   *int     `json:"max_length,omitempty"`   // For text types
	Pattern     string   `json:"pattern,omitempty"`      // Regex pattern for validation
	Placeholder string   `json:"placeholder,omitempty"`   // UI placeholder text
	Rows        *int     `json:"rows,omitempty"`          // For textarea type
}

// Validate checks if the FieldDefinition is valid
func (f *FieldDefinition) Validate() error {
	// Validate that ID is a valid filter field
	if _, valid := ValidateFilterFieldID(f.ID); !valid {
		return fmt.Errorf("invalid field ID: %s", f.ID)
	}

	// Validate that the field type matches the filter type
	filterID := FilterFieldID(f.ID)
	expectedType := filterID.GetType()

	// Map field types to filter types
	var actualFilterType string
	switch f.Type {
	case "text", "location", "color": // color values stored as text (hex)
		actualFilterType = "text"
	case "numeric", "number":
		actualFilterType = "numeric"
	case "boolean":
		actualFilterType = "boolean"
	case "enum", "select", "multiselect":
		actualFilterType = "enum"
	default:
		return fmt.Errorf("invalid field type: %s", f.Type)
	}

	if actualFilterType != expectedType {
		return fmt.Errorf("field type %s does not match filter type %s for ID %s", f.Type, expectedType, f.ID)
	}

	return nil
}

// GroupTemplate represents a template for business groups
type GroupTemplate struct {
	ID          uint              `gorm:"primaryKey" json:"id"`
	Name        string            `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName string            `json:"display_name"`
	Description string            `json:"description"`
	Icon        string            `json:"icon,omitempty"`
	Fields      []FieldDefinition `gorm:"type:jsonb;serializer:json" json:"fields"` // Custom field definitions
	Status      string            `gorm:"default:'active'" json:"status"`           // active, pending, deleted
	CreatedAt   time.Time         `json:"created_at"`
	UpdatedAt   time.Time         `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (GroupTemplate) TableName() string {
	return "ext_products_group_templates"
}

// Group represents a business group (restaurant, store, etc)
type Group struct {
	ID              uint          `gorm:"primaryKey" json:"id"`
	UserID          string        `gorm:"index;size:36;not null" json:"user_id"`
	GroupTemplateID uint          `gorm:"index;not null" json:"group_template_id"`
	GroupTemplate   GroupTemplate `json:"group_template,omitempty" gorm:"foreignKey:GroupTemplateID"`
	Name            string        `gorm:"not null" json:"name"`
	Description     string        `json:"description"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `gorm:"index" json:"filter_numeric_1,omitempty"`
	FilterNumeric2 *float64 `gorm:"index" json:"filter_numeric_2,omitempty"`
	FilterNumeric3 *float64 `gorm:"index" json:"filter_numeric_3,omitempty"`
	FilterNumeric4 *float64 `gorm:"index" json:"filter_numeric_4,omitempty"`
	FilterNumeric5 *float64 `gorm:"index" json:"filter_numeric_5,omitempty"`

	FilterText1 *string `gorm:"index" json:"filter_text_1,omitempty"`
	FilterText2 *string `gorm:"index" json:"filter_text_2,omitempty"`
	FilterText3 *string `gorm:"index" json:"filter_text_3,omitempty"`
	FilterText4 *string `gorm:"index" json:"filter_text_4,omitempty"`
	FilterText5 *string `gorm:"index" json:"filter_text_5,omitempty"`

	FilterBoolean1 *bool `gorm:"index" json:"filter_boolean_1,omitempty"`
	FilterBoolean2 *bool `gorm:"index" json:"filter_boolean_2,omitempty"`
	FilterBoolean3 *bool `gorm:"index" json:"filter_boolean_3,omitempty"`
	FilterBoolean4 *bool `gorm:"index" json:"filter_boolean_4,omitempty"`
	FilterBoolean5 *bool `gorm:"index" json:"filter_boolean_5,omitempty"`

	FilterEnum1 *string `gorm:"index" json:"filter_enum_1,omitempty"`
	FilterEnum2 *string `gorm:"index" json:"filter_enum_2,omitempty"`
	FilterEnum3 *string `gorm:"index" json:"filter_enum_3,omitempty"`
	FilterEnum4 *string `gorm:"index" json:"filter_enum_4,omitempty"`
	FilterEnum5 *string `gorm:"index" json:"filter_enum_5,omitempty"`

	FilterLocation1 *string `gorm:"index" json:"filter_location_1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `gorm:"index" json:"filter_location_2,omitempty"`
	FilterLocation3 *string `gorm:"index" json:"filter_location_3,omitempty"`
	FilterLocation4 *string `gorm:"index" json:"filter_location_4,omitempty"`
	FilterLocation5 *string `gorm:"index" json:"filter_location_5,omitempty"`

	CustomFields JSONB     `gorm:"type:jsonb" json:"custom_fields"` // Additional non-indexed fields
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (Group) TableName() string {
	return "ext_products_groups"
}

// ProductTemplate represents a template for products
type ProductTemplate struct {
	ID                            uint                      `gorm:"primaryKey" json:"id"`
	Name                          string                    `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName                   string                    `json:"display_name"`
	Description                   string                    `json:"description"`
	Category                      string                    `json:"category,omitempty"`
	Icon                          string                    `json:"icon,omitempty"`
	Fields                        []FieldDefinition         `gorm:"type:jsonb;serializer:json" json:"fields"`                      // Filter field definitions (indexed)
	CustomFieldsSchema            []CustomFieldDefinition   `gorm:"type:jsonb;serializer:json" json:"custom_fields_schema"`        // Custom field definitions (non-indexed, stored in CustomFields)
	PricingTemplates              []uint                    `gorm:"type:jsonb;serializer:json" json:"pricing_templates"`           // IDs of pricing templates to use
	BillingMode                   string                    `gorm:"default:'instant';not null" json:"billing_mode"`                // instant, approval
	BillingType                   string                    `gorm:"default:'one-time';not null" json:"billing_type"`               // one-time, recurring
	BillingRecurringInterval      *string                   `json:"billing_recurring_interval,omitempty"`                          // day, week, month, year
	BillingRecurringIntervalCount *int                      `gorm:"default:1" json:"billing_recurring_interval_count,omitempty"`
	Status                        string                    `gorm:"default:'active'" json:"status"` // active, pending, deleted
	CreatedAt                     time.Time                 `json:"created_at"`
	UpdatedAt                     time.Time                 `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (ProductTemplate) TableName() string {
	return "ext_products_product_templates"
}

// Product represents a product
type Product struct {
	ID                uint            `gorm:"primaryKey" json:"id"`
	GroupID           uint            `gorm:"index;not null" json:"group_id"`
	Group             Group           `json:"group,omitempty" gorm:"foreignKey:GroupID"`
	ProductTemplateID uint            `gorm:"index;not null" json:"product_template_id"`
	ProductTemplate   ProductTemplate `json:"product_template,omitempty" gorm:"foreignKey:ProductTemplateID"`
	Name              string          `gorm:"not null" json:"name"`
	Description       string          `json:"description"`
	BasePrice         float64         `json:"base_price"`
	Currency          string          `gorm:"default:'USD'" json:"currency"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `gorm:"index" json:"filter_numeric_1,omitempty"`
	FilterNumeric2 *float64 `gorm:"index" json:"filter_numeric_2,omitempty"`
	FilterNumeric3 *float64 `gorm:"index" json:"filter_numeric_3,omitempty"`
	FilterNumeric4 *float64 `gorm:"index" json:"filter_numeric_4,omitempty"`
	FilterNumeric5 *float64 `gorm:"index" json:"filter_numeric_5,omitempty"`

	FilterText1 *string `gorm:"index" json:"filter_text_1,omitempty"`
	FilterText2 *string `gorm:"index" json:"filter_text_2,omitempty"`
	FilterText3 *string `gorm:"index" json:"filter_text_3,omitempty"`
	FilterText4 *string `gorm:"index" json:"filter_text_4,omitempty"`
	FilterText5 *string `gorm:"index" json:"filter_text_5,omitempty"`

	FilterBoolean1 *bool `gorm:"index" json:"filter_boolean_1,omitempty"`
	FilterBoolean2 *bool `gorm:"index" json:"filter_boolean_2,omitempty"`
	FilterBoolean3 *bool `gorm:"index" json:"filter_boolean_3,omitempty"`
	FilterBoolean4 *bool `gorm:"index" json:"filter_boolean_4,omitempty"`
	FilterBoolean5 *bool `gorm:"index" json:"filter_boolean_5,omitempty"`

	FilterEnum1 *string `gorm:"index" json:"filter_enum_1,omitempty"`
	FilterEnum2 *string `gorm:"index" json:"filter_enum_2,omitempty"`
	FilterEnum3 *string `gorm:"index" json:"filter_enum_3,omitempty"`
	FilterEnum4 *string `gorm:"index" json:"filter_enum_4,omitempty"`
	FilterEnum5 *string `gorm:"index" json:"filter_enum_5,omitempty"`

	FilterLocation1 *string `gorm:"index" json:"filter_location_1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `gorm:"index" json:"filter_location_2,omitempty"`
	FilterLocation3 *string `gorm:"index" json:"filter_location_3,omitempty"`
	FilterLocation4 *string `gorm:"index" json:"filter_location_4,omitempty"`
	FilterLocation5 *string `gorm:"index" json:"filter_location_5,omitempty"`

	CustomFields   JSONB     `gorm:"type:jsonb" json:"custom_fields"` // Additional non-indexed fields
	Variables      JSONB     `gorm:"type:jsonb" json:"variables"`     // Product-specific variable values
	PricingFormula string    `json:"pricing_formula"`                 // Override formula
	Active         bool      `gorm:"default:true" json:"active"`
	CreatedAt      time.Time `json:"created_at"`
	UpdatedAt      time.Time `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (Product) TableName() string {
	return "ext_products_products"
}

// PricingTemplate represents a reusable pricing template
type PricingTemplate struct {
	ID               uint      `gorm:"primaryKey" json:"id"`
	Name             string    `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName      string    `json:"display_name"`
	Description      string    `json:"description"`
	PriceFormula     string    `gorm:"not null" json:"price_formula"` // Formula to calculate price
	ConditionFormula string    `json:"condition_formula,omitempty"`   // Formula to determine if template applies
	Variables        JSONB     `gorm:"type:jsonb" json:"variables"`   // Required variables for this template
	Category         string    `json:"category"`
	IsActive         bool      `gorm:"default:true" json:"is_active"`
	CreatedAt        time.Time `json:"created_at"`
	UpdatedAt        time.Time `json:"updated_at"`
}

// TableName specifies the table name with extension prefix
func (PricingTemplate) TableName() string {
	return "ext_products_pricing_templates"
}

// RegisterModels registers all models with GORM for auto-migration
func RegisterModels(db *gorm.DB) error {
	// Import the extensions package for the auto-migrate function
	return extensionsMigrate(db)
}

// extensionsMigrate uses the extension auto-migrate with proper prefix
func extensionsMigrate(db *gorm.DB) error {
	models := []interface{}{
		&Variable{},
		&GroupTemplate{},
		&Group{},
		&ProductTemplate{},
		&Product{},
		&PricingTemplate{},
		&Purchase{},
	}

	// This will be called from the extension.go file which has access to the extensions package
	// For now, we'll use regular AutoMigrate and the extension will handle the prefix
	return db.AutoMigrate(models...)
}

