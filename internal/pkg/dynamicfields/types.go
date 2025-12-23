package dynamicfields

import (
	"encoding/json"
	"fmt"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

// FieldType represents the type of a dynamic field
type FieldType string

const (
	FieldTypeText     FieldType = "text"
	FieldTypeNumber   FieldType = "number"
	FieldTypeBoolean  FieldType = "boolean"
	FieldTypeDate     FieldType = "date"
	FieldTypeDateTime FieldType = "datetime"
	FieldTypeEmail    FieldType = "email"
	FieldTypeURL      FieldType = "url"
	FieldTypeEnum     FieldType = "enum"
	FieldTypeArray    FieldType = "array"
	FieldTypeObject   FieldType = "object"
	FieldTypeFile     FieldType = "file"
	FieldTypeImage    FieldType = "image"
	FieldTypeGeo      FieldType = "geo" // Geographic point
)

// Field represents a dynamic field definition
type Field struct {
	Name         string                 `json:"name"`
	DisplayName  string                 `json:"displayName"`
	Type         FieldType              `json:"type"`
	Description  string                 `json:"description,omitempty"`
	Required     bool                   `json:"required"`
	DefaultValue interface{}            `json:"defaultValue,omitempty"`
	Placeholder  string                 `json:"placeholder,omitempty"`
	Validation   *ValidationRules       `json:"validation,omitempty"`
	Options      []Option               `json:"options,omitempty"`    // For enum fields
	Properties   map[string]*Field      `json:"properties,omitempty"` // For object fields
	Items        *Field                 `json:"items,omitempty"`      // For array fields
	Metadata     map[string]interface{} `json:"metadata,omitempty"`
}

// Option represents an option for enum fields
type Option struct {
	Value    interface{} `json:"value"`
	Label    string      `json:"label"`
	Icon     string      `json:"icon,omitempty"`
	Disabled bool        `json:"disabled,omitempty"`
}

// ValidationRules contains validation rules for a field
type ValidationRules struct {
	// Text validations
	MinLength *int    `json:"minLength,omitempty"`
	MaxLength *int    `json:"maxLength,omitempty"`
	Pattern   *string `json:"pattern,omitempty"` // Regex pattern

	// Number validations
	Min       *float64 `json:"min,omitempty"`
	Max       *float64 `json:"max,omitempty"`
	Step      *float64 `json:"step,omitempty"`
	Precision *int     `json:"precision,omitempty"` // Decimal places

	// Date validations
	MinDate *apptime.Time `json:"minDate,omitempty"`
	MaxDate *apptime.Time `json:"maxDate,omitempty"`

	// Array validations
	MinItems *int `json:"minItems,omitempty"`
	MaxItems *int `json:"maxItems,omitempty"`
	Unique   bool `json:"unique,omitempty"`

	// File validations
	MaxSize      *int64   `json:"maxSize,omitempty"`      // In bytes
	AllowedTypes []string `json:"allowedTypes,omitempty"` // MIME types

	// Custom validation
	Custom *CustomValidation `json:"custom,omitempty"`
}

// CustomValidation represents a custom validation function
type CustomValidation struct {
	Function string `json:"function"` // Function name or expression
	Message  string `json:"message"`  // Error message
}

// Schema represents a collection of fields
type Schema struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description,omitempty"`
	Fields      []*Field               `json:"fields"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// Value represents a field value with its metadata
type Value struct {
	Field     string      `json:"field"`
	Value     interface{} `json:"value"`
	IsValid   bool        `json:"isValid"`
	Error     string      `json:"error,omitempty"`
	UpdatedAt apptime.Time   `json:"updatedAt"`
}

// Document represents a collection of field values
type Document struct {
	ID        string                 `json:"id"`
	SchemaID  string                 `json:"schemaId"`
	Values    map[string]interface{} `json:"values"`
	Metadata  map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt apptime.Time              `json:"createdAt"`
	UpdatedAt apptime.Time              `json:"updatedAt"`
}

// ValidationError represents a field validation error
type ValidationError struct {
	Field   string `json:"field"`
	Type    string `json:"type"`
	Message string `json:"message"`
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("field '%s': %s", e.Field, e.Message)
}

// ValidationErrors represents multiple validation errors
type ValidationErrors struct {
	Errors []*ValidationError `json:"errors"`
}

func (e *ValidationErrors) Error() string {
	if len(e.Errors) == 0 {
		return "validation failed"
	}
	return fmt.Sprintf("validation failed: %d errors", len(e.Errors))
}

// Add adds a validation error
func (e *ValidationErrors) Add(field, errType, message string) {
	e.Errors = append(e.Errors, &ValidationError{
		Field:   field,
		Type:    errType,
		Message: message,
	})
}

// HasErrors returns true if there are validation errors
func (e *ValidationErrors) HasErrors() bool {
	return len(e.Errors) > 0
}

// GeoPoint represents a geographic point
type GeoPoint struct {
	Lat float64 `json:"lat"`
	Lng float64 `json:"lng"`
}

// MarshalJSON implements json.Marshaler
func (p GeoPoint) MarshalJSON() ([]byte, error) {
	return json.Marshal(map[string]float64{
		"lat": p.Lat,
		"lng": p.Lng,
	})
}

// UnmarshalJSON implements json.Unmarshaler
func (p *GeoPoint) UnmarshalJSON(data []byte) error {
	var m map[string]float64
	if err := json.Unmarshal(data, &m); err != nil {
		return err
	}
	p.Lat = m["lat"]
	p.Lng = m["lng"]
	return nil
}

// IsValidFieldType checks if a field type is valid
func IsValidFieldType(ft FieldType) bool {
	switch ft {
	case FieldTypeText, FieldTypeNumber, FieldTypeBoolean,
		FieldTypeDate, FieldTypeDateTime, FieldTypeEmail,
		FieldTypeURL, FieldTypeEnum, FieldTypeArray,
		FieldTypeObject, FieldTypeFile, FieldTypeImage, FieldTypeGeo:
		return true
	default:
		return false
	}
}
