package dynamicfields

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestSchemaBuilder(t *testing.T) {
	schema := NewSchemaBuilder("user_profile").
		WithDescription("User profile schema").
		AddField(
			NewFieldBuilder("name", FieldTypeText).
				WithDisplayName("Full Name").
				WithDescription("User's full name").
				Required().
				Build(),
		).
		AddField(
			NewFieldBuilder("age", FieldTypeNumber).
				WithDisplayName("Age").
				WithValidation(&ValidationRules{
					Min: ptrFloat64(0),
					Max: ptrFloat64(150),
				}).
				Build(),
		).
		AddField(
			NewFieldBuilder("email", FieldTypeEmail).
				WithDisplayName("Email Address").
				Required().
				WithPlaceholder("user@example.com").
				Build(),
		).
		Build()

	assert.Equal(t, "user_profile", schema.Name)
	assert.Equal(t, "User profile schema", schema.Description)
	assert.Len(t, schema.Fields, 3)

	// Check first field
	nameField := schema.Fields[0]
	assert.Equal(t, "name", nameField.Name)
	assert.Equal(t, "Full Name", nameField.DisplayName)
	assert.Equal(t, FieldTypeText, nameField.Type)
	assert.True(t, nameField.Required)

	// Check email field
	emailField := schema.Fields[2]
	assert.Equal(t, "email", emailField.Name)
	assert.Equal(t, "Email Address", emailField.DisplayName)
	assert.Equal(t, "user@example.com", emailField.Placeholder)
}

func TestFieldBuilder(t *testing.T) {
	tests := []struct {
		name     string
		build    func() *Field
		validate func(*testing.T, *Field)
	}{
		{
			name: "enum field",
			build: func() *Field {
				return NewFieldBuilder("status", FieldTypeEnum).
					WithDisplayName("Status").
					WithOptions([]Option{
						{Value: "active", Label: "Active"},
						{Value: "inactive", Label: "Inactive"},
					}).
					WithDefault("active").
					Build()
			},
			validate: func(t *testing.T, f *Field) {
				assert.Equal(t, FieldTypeEnum, f.Type)
				assert.Len(t, f.Options, 2)
				assert.Equal(t, "active", f.DefaultValue)
			},
		},
		{
			name: "array field",
			build: func() *Field {
				return NewFieldBuilder("tags", FieldTypeArray).
					WithItems(&Field{Type: FieldTypeText}).
					WithValidation(&ValidationRules{
						MinItems: ptrInt(1),
						MaxItems: ptrInt(10),
					}).
					Build()
			},
			validate: func(t *testing.T, f *Field) {
				assert.Equal(t, FieldTypeArray, f.Type)
				assert.NotNil(t, f.Items)
				assert.Equal(t, FieldTypeText, f.Items.Type)
			},
		},
		{
			name: "object field",
			build: func() *Field {
				return NewFieldBuilder("address", FieldTypeObject).
					WithProperties(map[string]*Field{
						"street": {Name: "street", Type: FieldTypeText},
						"city":   {Name: "city", Type: FieldTypeText},
					}).
					Build()
			},
			validate: func(t *testing.T, f *Field) {
				assert.Equal(t, FieldTypeObject, f.Type)
				assert.Len(t, f.Properties, 2)
				assert.NotNil(t, f.Properties["street"])
				assert.NotNil(t, f.Properties["city"])
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			field := tt.build()
			tt.validate(t, field)
		})
	}
}

func TestSchema_Validate(t *testing.T) {
	tests := []struct {
		name    string
		schema  *Schema
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid schema",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "field1", Type: FieldTypeText},
					{Name: "field2", Type: FieldTypeNumber},
				},
			},
			wantErr: false,
		},
		{
			name: "missing schema name",
			schema: &Schema{
				Fields: []*Field{
					{Name: "field1", Type: FieldTypeText},
				},
			},
			wantErr: true,
			errMsg:  "schema name is required",
		},
		{
			name: "duplicate field names",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "field1", Type: FieldTypeText},
					{Name: "field1", Type: FieldTypeNumber},
				},
			},
			wantErr: true,
			errMsg:  "duplicate field name: field1",
		},
		{
			name: "invalid field",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "", Type: FieldTypeText},
				},
			},
			wantErr: true,
			errMsg:  "field name is required",
		},
		{
			name: "enum without options",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "status", Type: FieldTypeEnum},
				},
			},
			wantErr: true,
			errMsg:  "enum field must have options",
		},
		{
			name: "array without items",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "tags", Type: FieldTypeArray},
				},
			},
			wantErr: true,
			errMsg:  "array field must have item type",
		},
		{
			name: "object without properties",
			schema: &Schema{
				Name: "test",
				Fields: []*Field{
					{Name: "data", Type: FieldTypeObject},
				},
			},
			wantErr: true,
			errMsg:  "object field must have properties",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.schema.Validate()
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

func TestSchema_GetField(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{Name: "field1", Type: FieldTypeText},
			{Name: "field2", Type: FieldTypeNumber},
		},
	}

	// Existing field
	field, found := schema.GetField("field1")
	assert.True(t, found)
	assert.NotNil(t, field)
	assert.Equal(t, "field1", field.Name)

	// Non-existing field
	field, found = schema.GetField("field3")
	assert.False(t, found)
	assert.Nil(t, field)
}

func TestSchema_ToJSON_FromJSON(t *testing.T) {
	original := &Schema{
		Name:        "test",
		Description: "Test schema",
		Fields: []*Field{
			{
				Name:        "name",
				DisplayName: "Name",
				Type:        FieldTypeText,
				Required:    true,
			},
			{
				Name: "age",
				Type: FieldTypeNumber,
				Validation: &ValidationRules{
					Min: ptrFloat64(0),
					Max: ptrFloat64(150),
				},
			},
		},
	}

	// Convert to JSON
	jsonData, err := original.ToJSON()
	require.NoError(t, err)
	assert.NotEmpty(t, jsonData)

	// Parse from JSON
	parsed, err := FromJSON(jsonData)
	require.NoError(t, err)

	// Verify parsed schema
	assert.Equal(t, original.Name, parsed.Name)
	assert.Equal(t, original.Description, parsed.Description)
	assert.Len(t, parsed.Fields, 2)

	// Verify fields
	assert.Equal(t, original.Fields[0].Name, parsed.Fields[0].Name)
	assert.Equal(t, original.Fields[0].Required, parsed.Fields[0].Required)
	assert.Equal(t, *original.Fields[1].Validation.Min, *parsed.Fields[1].Validation.Min)
}

func TestSchema_Clone(t *testing.T) {
	original := &Schema{
		Name: "test",
		Fields: []*Field{
			{Name: "field1", Type: FieldTypeText},
			{Name: "field2", Type: FieldTypeNumber},
		},
	}

	clone := original.Clone()

	// Verify clone is equal
	assert.Equal(t, original.Name, clone.Name)
	assert.Len(t, clone.Fields, 2)

	// Modify clone
	clone.Name = "modified"
	clone.Fields[0].Name = "modified_field"

	// Verify original is unchanged
	assert.Equal(t, "test", original.Name)
	assert.Equal(t, "field1", original.Fields[0].Name)
}

func TestMergeSchemas(t *testing.T) {
	schema1 := &Schema{
		Name: "base",
		Fields: []*Field{
			{Name: "field1", Type: FieldTypeText},
			{Name: "field2", Type: FieldTypeNumber},
		},
	}

	schema2 := &Schema{
		Name: "extension",
		Fields: []*Field{
			{Name: "field2", Type: FieldTypeBoolean}, // Duplicate, should be skipped
			{Name: "field3", Type: FieldTypeEmail},
		},
	}

	schema3 := &Schema{
		Name: "another",
		Fields: []*Field{
			{Name: "field4", Type: FieldTypeDate},
		},
	}

	merged, err := MergeSchemas(schema1, schema2, schema3)
	require.NoError(t, err)

	assert.Equal(t, "base", merged.Name)
	assert.Len(t, merged.Fields, 4)

	// Verify fields
	fieldNames := make([]string, len(merged.Fields))
	for i, f := range merged.Fields {
		fieldNames[i] = f.Name
	}
	assert.Contains(t, fieldNames, "field1")
	assert.Contains(t, fieldNames, "field2")
	assert.Contains(t, fieldNames, "field3")
	assert.Contains(t, fieldNames, "field4")

	// Verify field2 is from schema1 (first occurrence)
	field2, found := merged.GetField("field2")
	assert.True(t, found)
	assert.Equal(t, FieldTypeNumber, field2.Type)
}

func TestMergeSchemas_Empty(t *testing.T) {
	merged, err := MergeSchemas()
	assert.Error(t, err)
	assert.Nil(t, merged)
	assert.Contains(t, err.Error(), "no schemas to merge")
}

func TestIsValidFieldType(t *testing.T) {
	validTypes := []FieldType{
		FieldTypeText,
		FieldTypeNumber,
		FieldTypeBoolean,
		FieldTypeDate,
		FieldTypeDateTime,
		FieldTypeEmail,
		FieldTypeURL,
		FieldTypeEnum,
		FieldTypeArray,
		FieldTypeObject,
		FieldTypeFile,
		FieldTypeImage,
		FieldTypeGeo,
	}

	for _, ft := range validTypes {
		assert.True(t, IsValidFieldType(ft), "Type %s should be valid", ft)
	}

	invalidTypes := []FieldType{
		"invalid",
		"unknown",
		"",
	}

	for _, ft := range invalidTypes {
		assert.False(t, IsValidFieldType(ft), "Type %s should be invalid", ft)
	}
}

func TestComplexNestedSchema(t *testing.T) {
	// Create a complex nested schema
	schema := NewSchemaBuilder("product").
		AddField(
			NewFieldBuilder("variants", FieldTypeArray).
				WithItems(&Field{
					Name: "variant",
					Type: FieldTypeObject,
					Properties: map[string]*Field{
						"size": {
							Name: "size",
							Type: FieldTypeEnum,
							Options: []Option{
								{Value: "S", Label: "Small"},
								{Value: "M", Label: "Medium"},
								{Value: "L", Label: "Large"},
							},
						},
						"price": {
							Name: "price",
							Type: FieldTypeNumber,
							Validation: &ValidationRules{
								Min: ptrFloat64(0),
							},
						},
						"attributes": {
							Name: "attributes",
							Type: FieldTypeObject,
							Properties: map[string]*Field{
								"color":    {Name: "color", Type: FieldTypeText},
								"material": {Name: "material", Type: FieldTypeText},
							},
						},
					},
				}).
				Build(),
		).
		Build()

	// Validate the schema
	err := schema.Validate()
	require.NoError(t, err)

	// Convert to JSON and back
	jsonData, err := schema.ToJSON()
	require.NoError(t, err)

	parsed, err := FromJSON(jsonData)
	require.NoError(t, err)

	// Verify structure
	assert.Equal(t, "product", parsed.Name)
	assert.Len(t, parsed.Fields, 1)

	variantsField := parsed.Fields[0]
	assert.Equal(t, FieldTypeArray, variantsField.Type)
	assert.NotNil(t, variantsField.Items)
	assert.Equal(t, FieldTypeObject, variantsField.Items.Type)
	assert.Len(t, variantsField.Items.Properties, 3)

	// Check nested properties
	sizeField := variantsField.Items.Properties["size"]
	assert.Equal(t, FieldTypeEnum, sizeField.Type)
	assert.Len(t, sizeField.Options, 3)

	attributesField := variantsField.Items.Properties["attributes"]
	assert.Equal(t, FieldTypeObject, attributesField.Type)
	assert.Len(t, attributesField.Properties, 2)
}

func TestValidationErrors(t *testing.T) {
	errors := &ValidationErrors{
		Errors: make([]*ValidationError, 0),
	}

	// No errors initially
	assert.False(t, errors.HasErrors())

	// Add errors
	errors.Add("field1", "required", "Field is required")
	errors.Add("field2", "min", "Value too small")

	assert.True(t, errors.HasErrors())
	assert.Len(t, errors.Errors, 2)
	assert.Contains(t, errors.Error(), "2 errors")

	// Check individual errors
	assert.Equal(t, "field1", errors.Errors[0].Field)
	assert.Equal(t, "required", errors.Errors[0].Type)
	assert.Equal(t, "field 'field1': Field is required", errors.Errors[0].Error())
}

func TestGeoPoint_JSON(t *testing.T) {
	point := GeoPoint{Lat: 40.7128, Lng: -74.0060}

	// Marshal to JSON
	data, err := json.Marshal(point)
	require.NoError(t, err)

	// Unmarshal from JSON
	var parsed GeoPoint
	err = json.Unmarshal(data, &parsed)
	require.NoError(t, err)

	assert.Equal(t, point.Lat, parsed.Lat)
	assert.Equal(t, point.Lng, parsed.Lng)
}
