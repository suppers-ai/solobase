package dynamicfields

import (
	"strings"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestValidator_ValidateDocument(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{
				Name:     "name",
				Type:     FieldTypeText,
				Required: true,
			},
			{
				Name: "age",
				Type: FieldTypeNumber,
				Validation: &ValidationRules{
					Min: ptrFloat64(0),
					Max: ptrFloat64(150),
				},
			},
			{
				Name:     "email",
				Type:     FieldTypeEmail,
				Required: true,
			},
		},
	}

	validator := NewValidator(schema)

	tests := []struct {
		name    string
		doc     *Document
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid document",
			doc: &Document{
				Values: map[string]interface{}{
					"name":  "John Doe",
					"age":   30.0,
					"email": "john@example.com",
				},
			},
			wantErr: false,
		},
		{
			name: "missing required field",
			doc: &Document{
				Values: map[string]interface{}{
					"age": 30.0,
				},
			},
			wantErr: true,
			errMsg:  "field 'name' is required",
		},
		{
			name: "invalid type",
			doc: &Document{
				Values: map[string]interface{}{
					"name":  "John Doe",
					"age":   "thirty",
					"email": "john@example.com",
				},
			},
			wantErr: true,
			errMsg:  "expected number",
		},
		{
			name: "invalid email",
			doc: &Document{
				Values: map[string]interface{}{
					"name":  "John Doe",
					"age":   30.0,
					"email": "invalidemail", // No @ sign
				},
			},
			wantErr: true,
			errMsg:  "invalid email",
		},
		{
			name: "age out of range",
			doc: &Document{
				Values: map[string]interface{}{
					"name":  "John Doe",
					"age":   200.0,
					"email": "john@example.com",
				},
			},
			wantErr: true,
			errMsg:  "must be at most 150",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			errors := validator.ValidateDocument(tt.doc)
			if tt.wantErr {
				assert.NotNil(t, errors)
				if errors != nil {
					assert.True(t, errors.HasErrors())
					if tt.errMsg != "" {
						// Check individual error messages
						found := false
						for _, err := range errors.Errors {
							if contains(err.Message, tt.errMsg) {
								found = true
								break
							}
						}
						assert.True(t, found, "Expected error message containing '%s'", tt.errMsg)
					}
				}
			} else {
				assert.Nil(t, errors)
			}
		})
	}
}

func TestValidator_ValidateEnum(t *testing.T) {
	field := &Field{
		Name: "status",
		Type: FieldTypeEnum,
		Options: []Option{
			{Value: "active", Label: "Active"},
			{Value: "inactive", Label: "Inactive"},
		},
	}

	validator := NewValidator(&Schema{Fields: []*Field{field}})

	tests := []struct {
		name    string
		value   interface{}
		wantErr bool
	}{
		{
			name:    "valid option",
			value:   "active",
			wantErr: false,
		},
		{
			name:    "invalid option",
			value:   "pending",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidator_ValidateArray(t *testing.T) {
	field := &Field{
		Name: "tags",
		Type: FieldTypeArray,
		Items: &Field{
			Type: FieldTypeText,
		},
		Validation: &ValidationRules{
			MinItems: ptrInt(1),
			MaxItems: ptrInt(5),
			Unique:   true,
		},
	}

	validator := NewValidator(&Schema{Fields: []*Field{field}})

	tests := []struct {
		name    string
		value   interface{}
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid array",
			value:   []interface{}{"tag1", "tag2", "tag3"},
			wantErr: false,
		},
		{
			name:    "too few items",
			value:   []interface{}{},
			wantErr: true,
			errMsg:  "must have at least 1 items",
		},
		{
			name:    "too many items",
			value:   []interface{}{"tag1", "tag2", "tag3", "tag4", "tag5", "tag6"},
			wantErr: true,
			errMsg:  "must have at most 5 items",
		},
		{
			name:    "duplicate items",
			value:   []interface{}{"tag1", "tag2", "tag1"},
			wantErr: true,
			errMsg:  "must contain unique values",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
				if tt.errMsg != "" {
					assert.Contains(t, err.Error(), tt.errMsg)
				}
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidator_ValidateObject(t *testing.T) {
	field := &Field{
		Name: "address",
		Type: FieldTypeObject,
		Properties: map[string]*Field{
			"street": {
				Name:     "street",
				Type:     FieldTypeText,
				Required: true,
			},
			"city": {
				Name: "city",
				Type: FieldTypeText,
			},
			"zipcode": {
				Name: "zipcode",
				Type: FieldTypeText,
				Validation: &ValidationRules{
					Pattern: ptrString("^\\d{5}$"),
				},
			},
		},
	}

	validator := NewValidator(&Schema{Fields: []*Field{field}})

	tests := []struct {
		name    string
		value   interface{}
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid object",
			value: map[string]interface{}{
				"street":  "123 Main St",
				"city":    "New York",
				"zipcode": "12345",
			},
			wantErr: false,
		},
		{
			name: "missing required property",
			value: map[string]interface{}{
				"city":    "New York",
				"zipcode": "12345",
			},
			wantErr: true,
			errMsg:  "property 'street' is required",
		},
		{
			name: "invalid zipcode pattern",
			value: map[string]interface{}{
				"street":  "123 Main St",
				"city":    "New York",
				"zipcode": "ABC123",
			},
			wantErr: true,
			errMsg:  "does not match required pattern",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
				if tt.errMsg != "" {
					assert.Contains(t, err.Error(), tt.errMsg)
				}
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidator_ValidateGeoPoint(t *testing.T) {
	field := &Field{
		Name: "location",
		Type: FieldTypeGeo,
	}

	validator := NewValidator(&Schema{Fields: []*Field{field}})

	tests := []struct {
		name    string
		value   interface{}
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid GeoPoint",
			value:   GeoPoint{Lat: 40.7128, Lng: -74.0060},
			wantErr: false,
		},
		{
			name: "valid map",
			value: map[string]interface{}{
				"lat": 40.7128,
				"lng": -74.0060,
			},
			wantErr: false,
		},
		{
			name:    "invalid latitude",
			value:   GeoPoint{Lat: 91, Lng: 0},
			wantErr: true,
			errMsg:  "latitude must be between -90 and 90",
		},
		{
			name:    "invalid longitude",
			value:   GeoPoint{Lat: 0, Lng: 181},
			wantErr: true,
			errMsg:  "longitude must be between -180 and 180",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
				if tt.errMsg != "" {
					assert.Contains(t, err.Error(), tt.errMsg)
				}
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidator_ValidateDateRange(t *testing.T) {
	minDate := time.Date(2020, 1, 1, 0, 0, 0, 0, time.UTC)
	maxDate := time.Date(2025, 12, 31, 0, 0, 0, 0, time.UTC)

	field := &Field{
		Name: "birthdate",
		Type: FieldTypeDate,
		Validation: &ValidationRules{
			MinDate: &minDate,
			MaxDate: &maxDate,
		},
	}

	validator := NewValidator(&Schema{Fields: []*Field{field}})

	tests := []struct {
		name    string
		value   interface{}
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid date",
			value:   time.Date(2023, 6, 15, 0, 0, 0, 0, time.UTC),
			wantErr: false,
		},
		{
			name:    "valid date string",
			value:   "2023-06-15",
			wantErr: false,
		},
		{
			name:    "date before minimum",
			value:   time.Date(2019, 1, 1, 0, 0, 0, 0, time.UTC),
			wantErr: true,
			errMsg:  "must be after 2020-01-01",
		},
		{
			name:    "date after maximum",
			value:   time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC),
			wantErr: true,
			errMsg:  "must be before 2025-12-31",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
				if tt.errMsg != "" {
					assert.Contains(t, err.Error(), tt.errMsg)
				}
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidator_ValidateStringPatterns(t *testing.T) {
	tests := []struct {
		name      string
		fieldType FieldType
		value     string
		wantErr   bool
	}{
		{
			name:      "valid email",
			fieldType: FieldTypeEmail,
			value:     "test@example.com",
			wantErr:   false,
		},
		{
			name:      "invalid email - no @",
			fieldType: FieldTypeEmail,
			value:     "testexample.com",
			wantErr:   true,
		},
		{
			name:      "invalid email - no domain",
			fieldType: FieldTypeEmail,
			value:     "test@",
			wantErr:   true,
		},
		{
			name:      "valid URL",
			fieldType: FieldTypeURL,
			value:     "https://example.com",
			wantErr:   false,
		},
		{
			name:      "invalid URL - no protocol",
			fieldType: FieldTypeURL,
			value:     "example.com",
			wantErr:   true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			field := &Field{
				Name: "test",
				Type: tt.fieldType,
			}
			validator := NewValidator(&Schema{Fields: []*Field{field}})
			err := validator.ValidateValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

// Helper functions
func ptrInt(i int) *int {
	return &i
}

func ptrFloat64(f float64) *float64 {
	return &f
}

func ptrString(s string) *string {
	return &s
}

func contains(s, substr string) bool {
	return strings.Contains(s, substr)
}
