package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"gorm.io/gorm"
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
	DisplayName  string      `json:"displayName"`
	ValueType    string      `json:"valueType"` // number, string, boolean
	Type         string      `json:"type"`       // user, system
	DefaultValue interface{} `gorm:"type:jsonb" json:"defaultValue"`
	Description  string      `json:"description"`
	Status       string      `gorm:"default:'active'" json:"status"` // active, pending, deleted
	CreatedAt    time.Time   `json:"createdAt"`
	UpdatedAt    time.Time   `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Variable) TableName() string {
	return "ext_products_variables"
}

// FieldDefinition defines a field for both filter fields (indexed) and custom fields (JSON stored)
type FieldDefinition struct {
	ID          string           `json:"id"`                    // For filter fields: "filter_text_1", etc. For custom fields: any unique ID
	Name        string           `json:"name"`                  // Display name for the field
	Type        string           `json:"type"`                  // text, numeric, boolean, enum, color, date, email, url, textarea, etc.
	Required    bool             `json:"required,omitempty"`
	Description string           `json:"description,omitempty"`
	Section     string           `json:"section,omitempty"`     // Section/tab this field belongs to (for UI organization)
	Order       int              `json:"order,omitempty"`       // Display order within section
	Constraints FieldConstraints `json:"constraints,omitempty"` // All validation constraints
}

// FieldConstraints defines constraints for a field
type FieldConstraints struct {
	Required       bool        `json:"required,omitempty"`
	Min            *float64    `json:"min,omitempty"`          // For numeric/range types
	Max            *float64    `json:"max,omitempty"`          // For numeric/range types
	MinLength      *int        `json:"minLength,omitempty"`   // For text types
	MaxLength      *int        `json:"maxLength,omitempty"`   // For text types
	Pattern        string      `json:"pattern,omitempty"`      // Regex pattern for validation
	Options        []string    `json:"options,omitempty"`      // For select/enum types
	Default        interface{} `json:"default,omitempty"`      // Default value
	Placeholder    string      `json:"placeholder,omitempty"`  // UI placeholder text
	Rows           *int        `json:"rows,omitempty"`         // For textarea type
	Step           *float64    `json:"step,omitempty"`         // For numeric/range types
	EditableByUser bool        `json:"editableByUser,omitempty"` // Whether the user can edit this field
}

// ValidateAsFilterField checks if the FieldDefinition is valid as a filter field
func (f *FieldDefinition) ValidateAsFilterField() error {
	// Validate that ID is a valid filter field
	if _, valid := ValidateFilterFieldID(f.ID); !valid {
		return fmt.Errorf("invalid filter field ID: %s", f.ID)
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
		return fmt.Errorf("invalid field type for filter: %s", f.Type)
	}

	if actualFilterType != expectedType {
		return fmt.Errorf("field type %s does not match filter type %s for ID %s", f.Type, expectedType, f.ID)
	}

	return nil
}

// Validate checks if the FieldDefinition is valid (for any field type)
func (f *FieldDefinition) Validate() error {
	// Check if it's a filter field ID
	if strings.HasPrefix(f.ID, "filter_") {
		return f.ValidateAsFilterField()
	}

	// For custom fields, just ensure basic requirements are met
	if f.ID == "" {
		return fmt.Errorf("field ID cannot be empty")
	}
	if f.Name == "" {
		return fmt.Errorf("field name cannot be empty")
	}
	if f.Type == "" {
		return fmt.Errorf("field type cannot be empty")
	}

	return nil
}

// GroupTemplate represents a template for business groups
type GroupTemplate struct {
	ID          uint              `gorm:"primaryKey" json:"id"`
	Name        string            `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName string            `json:"displayName"`
	Description string            `json:"description"`
	Icon        string            `json:"icon,omitempty"`
	FilterFieldsSchema []FieldDefinition `gorm:"type:jsonb;serializer:json" json:"filterFieldsSchema"` // Filter field definitions
	Status      string            `gorm:"default:'active'" json:"status"`           // active, pending, deleted
	CreatedAt   time.Time         `json:"createdAt"`
	UpdatedAt   time.Time         `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (GroupTemplate) TableName() string {
	return "ext_products_group_templates"
}

// Group represents a business group (restaurant, store, etc)
type Group struct {
	ID              uint          `gorm:"primaryKey" json:"id"`
	UserID          string        `gorm:"index;size:36;not null" json:"userId"`
	GroupTemplateID uint          `gorm:"index;not null" json:"groupTemplateId"`
	GroupTemplate   GroupTemplate `json:"groupTemplate,omitempty" gorm:"foreignKey:GroupTemplateID"`
	Name            string        `gorm:"not null" json:"name"`
	Description     string        `json:"description"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `gorm:"index" json:"filterNumeric1,omitempty"`
	FilterNumeric2 *float64 `gorm:"index" json:"filterNumeric2,omitempty"`
	FilterNumeric3 *float64 `gorm:"index" json:"filterNumeric3,omitempty"`
	FilterNumeric4 *float64 `gorm:"index" json:"filterNumeric4,omitempty"`
	FilterNumeric5 *float64 `gorm:"index" json:"filterNumeric5,omitempty"`

	FilterText1 *string `gorm:"index" json:"filterText1,omitempty"`
	FilterText2 *string `gorm:"index" json:"filterText2,omitempty"`
	FilterText3 *string `gorm:"index" json:"filterText3,omitempty"`
	FilterText4 *string `gorm:"index" json:"filterText4,omitempty"`
	FilterText5 *string `gorm:"index" json:"filterText5,omitempty"`

	FilterBoolean1 *bool `gorm:"index" json:"filterBoolean1,omitempty"`
	FilterBoolean2 *bool `gorm:"index" json:"filterBoolean2,omitempty"`
	FilterBoolean3 *bool `gorm:"index" json:"filterBoolean3,omitempty"`
	FilterBoolean4 *bool `gorm:"index" json:"filterBoolean4,omitempty"`
	FilterBoolean5 *bool `gorm:"index" json:"filterBoolean5,omitempty"`

	FilterEnum1 *string `gorm:"index" json:"filterEnum1,omitempty"`
	FilterEnum2 *string `gorm:"index" json:"filterEnum2,omitempty"`
	FilterEnum3 *string `gorm:"index" json:"filterEnum3,omitempty"`
	FilterEnum4 *string `gorm:"index" json:"filterEnum4,omitempty"`
	FilterEnum5 *string `gorm:"index" json:"filterEnum5,omitempty"`

	FilterLocation1 *string `gorm:"index" json:"filterLocation1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `gorm:"index" json:"filterLocation2,omitempty"`
	FilterLocation3 *string `gorm:"index" json:"filterLocation3,omitempty"`
	FilterLocation4 *string `gorm:"index" json:"filterLocation4,omitempty"`
	FilterLocation5 *string `gorm:"index" json:"filterLocation5,omitempty"`

	CustomFields JSONB     `gorm:"type:jsonb" json:"customFields"` // Additional non-indexed fields
	CreatedAt    time.Time `json:"createdAt"`
	UpdatedAt    time.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Group) TableName() string {
	return "ext_products_groups"
}

// ProductTemplate represents a template for products
type ProductTemplate struct {
	ID                            uint                      `gorm:"primaryKey" json:"id"`
	Name                          string                    `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName                   string                    `json:"displayName"`
	Description                   string                    `json:"description"`
	Category                      string                    `json:"category,omitempty"`
	Icon                          string                    `json:"icon,omitempty"`
	FilterFieldsSchema            []FieldDefinition         `gorm:"type:jsonb;serializer:json" json:"filterFieldsSchema"`        // Filter field definitions (indexed, mapped to filter columns)
	CustomFieldsSchema            []FieldDefinition         `gorm:"type:jsonb;serializer:json" json:"customFieldsSchema"`        // Custom field definitions (non-indexed, stored in CustomFields JSON)
	PricingTemplates              []uint                    `gorm:"type:jsonb;serializer:json" json:"pricingTemplates"`           // IDs of pricing templates to use
	BillingMode                   string                    `gorm:"default:'instant';not null" json:"billingMode"`                // instant, approval
	BillingType                   string                    `gorm:"default:'one-time';not null" json:"billingType"`               // one-time, recurring
	BillingRecurringInterval      *string                   `json:"billingRecurringInterval,omitempty"`                          // day, week, month, year
	BillingRecurringIntervalCount *int                      `gorm:"default:1" json:"billingRecurringIntervalCount,omitempty"`
	Status                        string                    `gorm:"default:'active'" json:"status"` // active, pending, deleted
	CreatedAt                     time.Time                 `json:"createdAt"`
	UpdatedAt                     time.Time                 `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (ProductTemplate) TableName() string {
	return "ext_products_product_templates"
}

// Product represents a product
type Product struct {
	ID                uint            `gorm:"primaryKey" json:"id"`
	GroupID           uint            `gorm:"index;not null" json:"groupId"`
	Group             Group           `json:"group,omitempty" gorm:"foreignKey:GroupID"`
	ProductTemplateID uint            `gorm:"index;not null" json:"productTemplateId"`
	ProductTemplate   ProductTemplate `json:"productTemplate,omitempty" gorm:"foreignKey:ProductTemplateID"`
	Name              string          `gorm:"not null" json:"name"`
	Description       string          `json:"description"`
	BasePrice         float64         `json:"basePrice"`
	Currency          string          `gorm:"default:'USD'" json:"currency"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `gorm:"index" json:"filterNumeric1,omitempty"`
	FilterNumeric2 *float64 `gorm:"index" json:"filterNumeric2,omitempty"`
	FilterNumeric3 *float64 `gorm:"index" json:"filterNumeric3,omitempty"`
	FilterNumeric4 *float64 `gorm:"index" json:"filterNumeric4,omitempty"`
	FilterNumeric5 *float64 `gorm:"index" json:"filterNumeric5,omitempty"`

	FilterText1 *string `gorm:"index" json:"filterText1,omitempty"`
	FilterText2 *string `gorm:"index" json:"filterText2,omitempty"`
	FilterText3 *string `gorm:"index" json:"filterText3,omitempty"`
	FilterText4 *string `gorm:"index" json:"filterText4,omitempty"`
	FilterText5 *string `gorm:"index" json:"filterText5,omitempty"`

	FilterBoolean1 *bool `gorm:"index" json:"filterBoolean1,omitempty"`
	FilterBoolean2 *bool `gorm:"index" json:"filterBoolean2,omitempty"`
	FilterBoolean3 *bool `gorm:"index" json:"filterBoolean3,omitempty"`
	FilterBoolean4 *bool `gorm:"index" json:"filterBoolean4,omitempty"`
	FilterBoolean5 *bool `gorm:"index" json:"filterBoolean5,omitempty"`

	FilterEnum1 *string `gorm:"index" json:"filterEnum1,omitempty"`
	FilterEnum2 *string `gorm:"index" json:"filterEnum2,omitempty"`
	FilterEnum3 *string `gorm:"index" json:"filterEnum3,omitempty"`
	FilterEnum4 *string `gorm:"index" json:"filterEnum4,omitempty"`
	FilterEnum5 *string `gorm:"index" json:"filterEnum5,omitempty"`

	FilterLocation1 *string `gorm:"index" json:"filterLocation1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `gorm:"index" json:"filterLocation2,omitempty"`
	FilterLocation3 *string `gorm:"index" json:"filterLocation3,omitempty"`
	FilterLocation4 *string `gorm:"index" json:"filterLocation4,omitempty"`
	FilterLocation5 *string `gorm:"index" json:"filterLocation5,omitempty"`

	CustomFields   JSONB     `gorm:"type:jsonb" json:"customFields"` // Additional non-indexed fields
	Variables      JSONB     `gorm:"type:jsonb" json:"variables"`     // Product-specific variable values
	PricingFormula string    `json:"pricingFormula"`                 // Override formula
	Active         bool      `gorm:"default:true" json:"active"`
	CreatedAt      time.Time `json:"createdAt"`
	UpdatedAt      time.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Product) TableName() string {
	return "ext_products_products"
}

// PricingTemplate represents a reusable pricing template
type PricingTemplate struct {
	ID               uint      `gorm:"primaryKey" json:"id"`
	Name             string    `gorm:"uniqueIndex;not null" json:"name"`
	DisplayName      string    `json:"displayName"`
	Description      string    `json:"description"`
	PriceFormula     string    `gorm:"not null" json:"priceFormula"` // Formula to calculate price
	ConditionFormula string    `json:"conditionFormula,omitempty"`   // Formula to determine if template applies
	Variables        JSONB     `gorm:"type:jsonb" json:"variables"`   // Required variables for this template
	Category         string    `json:"category"`
	Status           string    `gorm:"default:'active'" json:"status"` // active, pending, deleted
	CreatedAt        time.Time `json:"createdAt"`
	UpdatedAt        time.Time `json:"updatedAt"`
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

