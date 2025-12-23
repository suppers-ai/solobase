package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
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
	ID           uint         `json:"id"`
	Name         string       `json:"name"`
	DisplayName  string       `json:"displayName"`
	ValueType    string       `json:"valueType"` // number, string, boolean
	Type         string       `json:"type"`      // user, system
	DefaultValue interface{}  `json:"defaultValue"`
	Description  string       `json:"description"`
	Status       string       `json:"status"` // active, pending, deleted
	CreatedAt    apptime.Time `json:"createdAt"`
	UpdatedAt    apptime.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Variable) TableName() string {
	return "ext_products_variables"
}

// PrepareForCreate prepares the variable for insertion
func (v *Variable) PrepareForCreate() {
	now := apptime.NowTime()
	if v.CreatedAt.IsZero() {
		v.CreatedAt = now
	}
	v.UpdatedAt = now
	if v.Status == "" {
		v.Status = "active"
	}
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
	ID                 uint              `json:"id"`
	Name               string            `json:"name"`
	DisplayName        string            `json:"displayName"`
	Description        string            `json:"description"`
	Icon               string            `json:"icon,omitempty"`
	FilterFieldsSchema []FieldDefinition `json:"filterFieldsSchema"` // Filter field definitions
	Status             string            `json:"status"`                                     // active, pending, deleted
	CreatedAt          apptime.Time      `json:"createdAt"`
	UpdatedAt          apptime.Time      `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (GroupTemplate) TableName() string {
	return "ext_products_group_templates"
}

// PrepareForCreate prepares the group template for insertion
func (gt *GroupTemplate) PrepareForCreate() {
	now := apptime.NowTime()
	if gt.CreatedAt.IsZero() {
		gt.CreatedAt = now
	}
	gt.UpdatedAt = now
	if gt.Status == "" {
		gt.Status = "active"
	}
}

// Group represents a business group (restaurant, store, etc)
type Group struct {
	ID              uint          `json:"id"`
	UserID          string        `json:"userId"`
	GroupTemplateID uint          `json:"groupTemplateId"`
	GroupTemplate   GroupTemplate `json:"groupTemplate,omitempty"`
	Name            string        `json:"name"`
	Description     string        `json:"description"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `json:"filterNumeric1,omitempty"`
	FilterNumeric2 *float64 `json:"filterNumeric2,omitempty"`
	FilterNumeric3 *float64 `json:"filterNumeric3,omitempty"`
	FilterNumeric4 *float64 `json:"filterNumeric4,omitempty"`
	FilterNumeric5 *float64 `json:"filterNumeric5,omitempty"`

	FilterText1 *string `json:"filterText1,omitempty"`
	FilterText2 *string `json:"filterText2,omitempty"`
	FilterText3 *string `json:"filterText3,omitempty"`
	FilterText4 *string `json:"filterText4,omitempty"`
	FilterText5 *string `json:"filterText5,omitempty"`

	FilterBoolean1 *bool `json:"filterBoolean1,omitempty"`
	FilterBoolean2 *bool `json:"filterBoolean2,omitempty"`
	FilterBoolean3 *bool `json:"filterBoolean3,omitempty"`
	FilterBoolean4 *bool `json:"filterBoolean4,omitempty"`
	FilterBoolean5 *bool `json:"filterBoolean5,omitempty"`

	FilterEnum1 *string `json:"filterEnum1,omitempty"`
	FilterEnum2 *string `json:"filterEnum2,omitempty"`
	FilterEnum3 *string `json:"filterEnum3,omitempty"`
	FilterEnum4 *string `json:"filterEnum4,omitempty"`
	FilterEnum5 *string `json:"filterEnum5,omitempty"`

	FilterLocation1 *string `json:"filterLocation1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `json:"filterLocation2,omitempty"`
	FilterLocation3 *string `json:"filterLocation3,omitempty"`
	FilterLocation4 *string `json:"filterLocation4,omitempty"`
	FilterLocation5 *string `json:"filterLocation5,omitempty"`

	CustomFields JSONB        `json:"customFields"` // Additional non-indexed fields
	CreatedAt    apptime.Time `json:"createdAt"`
	UpdatedAt    apptime.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Group) TableName() string {
	return "ext_products_groups"
}

// PrepareForCreate prepares the group for insertion
func (g *Group) PrepareForCreate() {
	now := apptime.NowTime()
	if g.CreatedAt.IsZero() {
		g.CreatedAt = now
	}
	g.UpdatedAt = now
}

// ProductTemplate represents a template for products
type ProductTemplate struct {
	ID                            uint              `json:"id"`
	Name                          string            `json:"name"`
	DisplayName                   string            `json:"displayName"`
	Description                   string            `json:"description"`
	Category                      string            `json:"category,omitempty"`
	Icon                          string            `json:"icon,omitempty"`
	FilterFieldsSchema            []FieldDefinition `json:"filterFieldsSchema"` // Filter field definitions (indexed, mapped to filter columns)
	CustomFieldsSchema            []FieldDefinition `json:"customFieldsSchema"` // Custom field definitions (non-indexed, stored in CustomFields JSON)
	PricingTemplates              []uint            `json:"pricingTemplates"`   // IDs of pricing templates to use
	BillingMode                   string            `json:"billingMode"`                               // instant, approval
	BillingType                   string            `json:"billingType"`                               // one-time, recurring
	BillingRecurringInterval      *string           `json:"billingRecurringInterval,omitempty"`
	BillingRecurringIntervalCount *int              `json:"billingRecurringIntervalCount,omitempty"`
	Status                        string            `json:"status"` // active, pending, deleted
	CreatedAt                     apptime.Time      `json:"createdAt"`
	UpdatedAt                     apptime.Time      `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (ProductTemplate) TableName() string {
	return "ext_products_product_templates"
}

// PrepareForCreate prepares the product template for insertion
func (pt *ProductTemplate) PrepareForCreate() {
	now := apptime.NowTime()
	if pt.CreatedAt.IsZero() {
		pt.CreatedAt = now
	}
	pt.UpdatedAt = now
	if pt.Status == "" {
		pt.Status = "active"
	}
	if pt.BillingMode == "" {
		pt.BillingMode = "instant"
	}
	if pt.BillingType == "" {
		pt.BillingType = "one-time"
	}
	if pt.BillingRecurringIntervalCount == nil {
		defaultCount := 1
		pt.BillingRecurringIntervalCount = &defaultCount
	}
}

// Product represents a product
type Product struct {
	ID                uint            `json:"id"`
	GroupID           uint            `json:"groupId"`
	Group             Group           `json:"group,omitempty"`
	ProductTemplateID uint            `json:"productTemplateId"`
	ProductTemplate   ProductTemplate `json:"productTemplate,omitempty"`
	Name              string          `json:"name"`
	Description       string          `json:"description"`
	BasePrice         float64         `json:"basePrice"`
	Currency          string          `json:"currency"`

	// Filter columns for indexing and searching
	FilterNumeric1 *float64 `json:"filterNumeric1,omitempty"`
	FilterNumeric2 *float64 `json:"filterNumeric2,omitempty"`
	FilterNumeric3 *float64 `json:"filterNumeric3,omitempty"`
	FilterNumeric4 *float64 `json:"filterNumeric4,omitempty"`
	FilterNumeric5 *float64 `json:"filterNumeric5,omitempty"`

	FilterText1 *string `json:"filterText1,omitempty"`
	FilterText2 *string `json:"filterText2,omitempty"`
	FilterText3 *string `json:"filterText3,omitempty"`
	FilterText4 *string `json:"filterText4,omitempty"`
	FilterText5 *string `json:"filterText5,omitempty"`

	FilterBoolean1 *bool `json:"filterBoolean1,omitempty"`
	FilterBoolean2 *bool `json:"filterBoolean2,omitempty"`
	FilterBoolean3 *bool `json:"filterBoolean3,omitempty"`
	FilterBoolean4 *bool `json:"filterBoolean4,omitempty"`
	FilterBoolean5 *bool `json:"filterBoolean5,omitempty"`

	FilterEnum1 *string `json:"filterEnum1,omitempty"`
	FilterEnum2 *string `json:"filterEnum2,omitempty"`
	FilterEnum3 *string `json:"filterEnum3,omitempty"`
	FilterEnum4 *string `json:"filterEnum4,omitempty"`
	FilterEnum5 *string `json:"filterEnum5,omitempty"`

	FilterLocation1 *string `json:"filterLocation1,omitempty"` // Store as GeoJSON or lat,lng
	FilterLocation2 *string `json:"filterLocation2,omitempty"`
	FilterLocation3 *string `json:"filterLocation3,omitempty"`
	FilterLocation4 *string `json:"filterLocation4,omitempty"`
	FilterLocation5 *string `json:"filterLocation5,omitempty"`

	CustomFields   JSONB        `json:"customFields"` // Additional non-indexed fields
	Variables      JSONB        `json:"variables"`    // Product-specific variable values
	PricingFormula string       `json:"pricingFormula"`
	Active         bool         `json:"active"`
	CreatedAt      apptime.Time `json:"createdAt"`
	UpdatedAt      apptime.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (Product) TableName() string {
	return "ext_products_products"
}

// PrepareForCreate prepares the product for insertion
func (p *Product) PrepareForCreate() {
	now := apptime.NowTime()
	if p.CreatedAt.IsZero() {
		p.CreatedAt = now
	}
	p.UpdatedAt = now
	if p.Currency == "" {
		p.Currency = "USD"
	}
	if !p.Active {
		p.Active = true
	}
}

// PricingTemplate represents a reusable pricing template
type PricingTemplate struct {
	ID               uint         `json:"id"`
	Name             string       `json:"name"`
	DisplayName      string       `json:"displayName"`
	Description      string       `json:"description"`
	PriceFormula     string       `json:"priceFormula"`
	ConditionFormula string       `json:"conditionFormula,omitempty"`
	Variables        JSONB        `json:"variables"`
	Category         string       `json:"category"`
	Status           string       `json:"status"` // active, pending, deleted
	CreatedAt        apptime.Time `json:"createdAt"`
	UpdatedAt        apptime.Time `json:"updatedAt"`
}

// TableName specifies the table name with extension prefix
func (PricingTemplate) TableName() string {
	return "ext_products_pricing_templates"
}

// PrepareForCreate prepares the pricing template for insertion
func (pt *PricingTemplate) PrepareForCreate() {
	now := apptime.NowTime()
	if pt.CreatedAt.IsZero() {
		pt.CreatedAt = now
	}
	pt.UpdatedAt = now
	if pt.Status == "" {
		pt.Status = "active"
	}
}

