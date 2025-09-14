package dynamicfields

import (
	"fmt"
	"regexp"
	"strings"
	"time"
)

// Validator validates field values against their definitions
type Validator struct {
	schema *Schema
}

// NewValidator creates a new validator for a schema
func NewValidator(schema *Schema) *Validator {
	return &Validator{schema: schema}
}

// ValidateDocument validates all values in a document
func (v *Validator) ValidateDocument(doc *Document) *ValidationErrors {
	errors := &ValidationErrors{
		Errors: make([]*ValidationError, 0),
	}

	// Check required fields
	for _, field := range v.schema.Fields {
		value, exists := doc.Values[field.Name]

		if field.Required && (!exists || value == nil) {
			errors.Add(field.Name, "required", fmt.Sprintf("field '%s' is required", field.Name))
			continue
		}

		if exists && value != nil {
			if err := v.ValidateValue(field, value); err != nil {
				if ve, ok := err.(*ValidationError); ok {
					errors.Errors = append(errors.Errors, ve)
				} else {
					errors.Add(field.Name, "validation", err.Error())
				}
			}
		}
	}

	if errors.HasErrors() {
		return errors
	}
	return nil
}

// ValidateValue validates a single value against its field definition
func (v *Validator) ValidateValue(field *Field, value interface{}) error {
	// Type validation
	if err := v.validateType(field, value); err != nil {
		return err
	}

	// Special validation for email and URL types even without explicit validation rules
	if field.Type == FieldTypeEmail {
		if str, ok := value.(string); ok && !isValidEmail(str) {
			return &ValidationError{
				Field:   field.Name,
				Type:    "email",
				Message: "invalid email address",
			}
		}
	}

	if field.Type == FieldTypeURL {
		if str, ok := value.(string); ok && !isValidURL(str) {
			return &ValidationError{
				Field:   field.Name,
				Type:    "url",
				Message: "invalid URL",
			}
		}
	}

	// Validation rules
	if field.Validation != nil {
		if err := v.validateRules(field, value); err != nil {
			return err
		}
	}

	return nil
}

// validateType checks if the value matches the expected type
func (v *Validator) validateType(field *Field, value interface{}) error {
	switch field.Type {
	case FieldTypeText, FieldTypeEmail, FieldTypeURL:
		if _, ok := value.(string); !ok {
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected string, got %T", value),
			}
		}

	case FieldTypeNumber:
		switch value.(type) {
		case float64, float32, int, int32, int64:
			// Valid number types
		default:
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected number, got %T", value),
			}
		}

	case FieldTypeBoolean:
		if _, ok := value.(bool); !ok {
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected boolean, got %T", value),
			}
		}

	case FieldTypeDate, FieldTypeDateTime:
		switch v := value.(type) {
		case time.Time:
			// Valid
		case string:
			// Try to parse string as date
			var err error
			if field.Type == FieldTypeDate {
				_, err = time.Parse("2006-01-02", v)
			} else {
				_, err = time.Parse(time.RFC3339, v)
			}
			if err != nil {
				return &ValidationError{
					Field:   field.Name,
					Type:    "type",
					Message: fmt.Sprintf("invalid date format: %s", v),
				}
			}
		default:
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected date/time, got %T", value),
			}
		}

	case FieldTypeEnum:
		// Check if value is in options
		found := false
		for _, opt := range field.Options {
			if opt.Value == value {
				found = true
				break
			}
		}
		if !found {
			return &ValidationError{
				Field:   field.Name,
				Type:    "enum",
				Message: fmt.Sprintf("value '%v' is not a valid option", value),
			}
		}

	case FieldTypeArray:
		arr, ok := value.([]interface{})
		if !ok {
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected array, got %T", value),
			}
		}

		// Validate each item
		if field.Items != nil {
			for i, item := range arr {
				if err := v.ValidateValue(field.Items, item); err != nil {
					return &ValidationError{
						Field:   fmt.Sprintf("%s[%d]", field.Name, i),
						Type:    "item",
						Message: err.Error(),
					}
				}
			}
		}

	case FieldTypeObject:
		obj, ok := value.(map[string]interface{})
		if !ok {
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected object, got %T", value),
			}
		}

		// Validate properties
		for propName, propField := range field.Properties {
			if propValue, exists := obj[propName]; exists {
				if err := v.ValidateValue(propField, propValue); err != nil {
					return &ValidationError{
						Field:   fmt.Sprintf("%s.%s", field.Name, propName),
						Type:    "property",
						Message: err.Error(),
					}
				}
			} else if propField.Required {
				return &ValidationError{
					Field:   fmt.Sprintf("%s.%s", field.Name, propName),
					Type:    "required",
					Message: fmt.Sprintf("property '%s' is required", propName),
				}
			}
		}

	case FieldTypeGeo:
		switch v := value.(type) {
		case GeoPoint:
			if v.Lat < -90 || v.Lat > 90 {
				return &ValidationError{
					Field:   field.Name,
					Type:    "range",
					Message: "latitude must be between -90 and 90",
				}
			}
			if v.Lng < -180 || v.Lng > 180 {
				return &ValidationError{
					Field:   field.Name,
					Type:    "range",
					Message: "longitude must be between -180 and 180",
				}
			}
		case map[string]interface{}:
			lat, latOk := v["lat"].(float64)
			lng, lngOk := v["lng"].(float64)
			if !latOk || !lngOk {
				return &ValidationError{
					Field:   field.Name,
					Type:    "type",
					Message: "geo point must have lat and lng fields",
				}
			}
			if lat < -90 || lat > 90 {
				return &ValidationError{
					Field:   field.Name,
					Type:    "range",
					Message: "latitude must be between -90 and 90",
				}
			}
			if lng < -180 || lng > 180 {
				return &ValidationError{
					Field:   field.Name,
					Type:    "range",
					Message: "longitude must be between -180 and 180",
				}
			}
		default:
			return &ValidationError{
				Field:   field.Name,
				Type:    "type",
				Message: fmt.Sprintf("expected geo point, got %T", value),
			}
		}
	}

	return nil
}

// validateRules applies validation rules to a value
func (v *Validator) validateRules(field *Field, value interface{}) error {
	rules := field.Validation

	switch field.Type {
	case FieldTypeText, FieldTypeEmail, FieldTypeURL:
		str := value.(string)

		// Length validation
		if rules.MinLength != nil && len(str) < *rules.MinLength {
			return &ValidationError{
				Field:   field.Name,
				Type:    "min_length",
				Message: fmt.Sprintf("must be at least %d characters", *rules.MinLength),
			}
		}

		if rules.MaxLength != nil && len(str) > *rules.MaxLength {
			return &ValidationError{
				Field:   field.Name,
				Type:    "max_length",
				Message: fmt.Sprintf("must be at most %d characters", *rules.MaxLength),
			}
		}

		// Pattern validation
		if rules.Pattern != nil {
			matched, err := regexp.MatchString(*rules.Pattern, str)
			if err != nil {
				return &ValidationError{
					Field:   field.Name,
					Type:    "pattern",
					Message: fmt.Sprintf("invalid pattern: %s", err),
				}
			}
			if !matched {
				return &ValidationError{
					Field:   field.Name,
					Type:    "pattern",
					Message: "value does not match required pattern",
				}
			}
		}

		// Email validation
		if field.Type == FieldTypeEmail {
			if !isValidEmail(str) {
				return &ValidationError{
					Field:   field.Name,
					Type:    "email",
					Message: "invalid email address",
				}
			}
		}

		// URL validation
		if field.Type == FieldTypeURL {
			if !isValidURL(str) {
				return &ValidationError{
					Field:   field.Name,
					Type:    "url",
					Message: "invalid URL",
				}
			}
		}

	case FieldTypeNumber:
		num := toFloat64(value)

		// Range validation
		if rules.Min != nil && num < *rules.Min {
			return &ValidationError{
				Field:   field.Name,
				Type:    "min",
				Message: fmt.Sprintf("must be at least %v", *rules.Min),
			}
		}

		if rules.Max != nil && num > *rules.Max {
			return &ValidationError{
				Field:   field.Name,
				Type:    "max",
				Message: fmt.Sprintf("must be at most %v", *rules.Max),
			}
		}

		// Step validation
		if rules.Step != nil && *rules.Step > 0 {
			if !isValidStep(num, *rules.Min, *rules.Step) {
				return &ValidationError{
					Field:   field.Name,
					Type:    "step",
					Message: fmt.Sprintf("must be a multiple of %v", *rules.Step),
				}
			}
		}

	case FieldTypeDate, FieldTypeDateTime:
		var t time.Time
		switch v := value.(type) {
		case time.Time:
			t = v
		case string:
			var err error
			if field.Type == FieldTypeDate {
				t, err = time.Parse("2006-01-02", v)
			} else {
				t, err = time.Parse(time.RFC3339, v)
			}
			if err != nil {
				return &ValidationError{
					Field:   field.Name,
					Type:    "date",
					Message: "invalid date format",
				}
			}
		}

		// Date range validation
		if rules.MinDate != nil && t.Before(*rules.MinDate) {
			return &ValidationError{
				Field:   field.Name,
				Type:    "min_date",
				Message: fmt.Sprintf("must be after %s", rules.MinDate.Format("2006-01-02")),
			}
		}

		if rules.MaxDate != nil && t.After(*rules.MaxDate) {
			return &ValidationError{
				Field:   field.Name,
				Type:    "max_date",
				Message: fmt.Sprintf("must be before %s", rules.MaxDate.Format("2006-01-02")),
			}
		}

	case FieldTypeArray:
		arr := value.([]interface{})

		// Array size validation
		if rules.MinItems != nil && len(arr) < *rules.MinItems {
			return &ValidationError{
				Field:   field.Name,
				Type:    "min_items",
				Message: fmt.Sprintf("must have at least %d items", *rules.MinItems),
			}
		}

		if rules.MaxItems != nil && len(arr) > *rules.MaxItems {
			return &ValidationError{
				Field:   field.Name,
				Type:    "max_items",
				Message: fmt.Sprintf("must have at most %d items", *rules.MaxItems),
			}
		}

		// Unique validation
		if rules.Unique {
			seen := make(map[interface{}]bool)
			for _, item := range arr {
				if seen[item] {
					return &ValidationError{
						Field:   field.Name,
						Type:    "unique",
						Message: "array must contain unique values",
					}
				}
				seen[item] = true
			}
		}
	}

	return nil
}

// Helper functions

func isValidEmail(email string) bool {
	// Simple email validation - must have @ and domain with .
	if !strings.Contains(email, "@") {
		return false
	}
	parts := strings.Split(email, "@")
	if len(parts) != 2 {
		return false
	}
	if len(parts[0]) == 0 || len(parts[1]) == 0 {
		return false
	}
	if !strings.Contains(parts[1], ".") {
		return false
	}
	return true
}

func isValidURL(url string) bool {
	// Simple URL validation
	if !strings.HasPrefix(url, "http://") && !strings.HasPrefix(url, "https://") {
		return false
	}
	parts := strings.Split(url, "://")
	if len(parts) != 2 || len(parts[1]) == 0 {
		return false
	}
	return true
}

func toFloat64(value interface{}) float64 {
	switch v := value.(type) {
	case float64:
		return v
	case float32:
		return float64(v)
	case int:
		return float64(v)
	case int32:
		return float64(v)
	case int64:
		return float64(v)
	default:
		return 0
	}
}

func isValidStep(value, base, step float64) bool {
	diff := value - base
	remainder := diff - float64(int(diff/step))*step
	return remainder < 0.0001 || remainder > step-0.0001
}
