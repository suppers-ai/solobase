package dynamicfields

import (
	"encoding/json"
	"fmt"
)

// SchemaBuilder helps build field schemas
type SchemaBuilder struct {
	schema *Schema
}

// NewSchemaBuilder creates a new schema builder
func NewSchemaBuilder(name string) *SchemaBuilder {
	return &SchemaBuilder{
		schema: &Schema{
			Name:   name,
			Fields: make([]*Field, 0),
		},
	}
}

// WithDescription adds a description to the schema
func (b *SchemaBuilder) WithDescription(desc string) *SchemaBuilder {
	b.schema.Description = desc
	return b
}

// AddField adds a field to the schema
func (b *SchemaBuilder) AddField(field *Field) *SchemaBuilder {
	b.schema.Fields = append(b.schema.Fields, field)
	return b
}

// Build returns the built schema
func (b *SchemaBuilder) Build() *Schema {
	return b.schema
}

// FieldBuilder helps build field definitions
type FieldBuilder struct {
	field *Field
}

// NewFieldBuilder creates a new field builder
func NewFieldBuilder(name string, fieldType FieldType) *FieldBuilder {
	return &FieldBuilder{
		field: &Field{
			Name: name,
			Type: fieldType,
		},
	}
}

// WithDisplayName sets the display name
func (b *FieldBuilder) WithDisplayName(name string) *FieldBuilder {
	b.field.DisplayName = name
	return b
}

// WithDescription sets the description
func (b *FieldBuilder) WithDescription(desc string) *FieldBuilder {
	b.field.Description = desc
	return b
}

// Required marks the field as required
func (b *FieldBuilder) Required() *FieldBuilder {
	b.field.Required = true
	return b
}

// WithDefault sets the default value
func (b *FieldBuilder) WithDefault(value interface{}) *FieldBuilder {
	b.field.DefaultValue = value
	return b
}

// WithPlaceholder sets the placeholder
func (b *FieldBuilder) WithPlaceholder(placeholder string) *FieldBuilder {
	b.field.Placeholder = placeholder
	return b
}

// WithValidation sets the validation rules
func (b *FieldBuilder) WithValidation(rules *ValidationRules) *FieldBuilder {
	b.field.Validation = rules
	return b
}

// WithOptions sets the options for enum fields
func (b *FieldBuilder) WithOptions(options []Option) *FieldBuilder {
	b.field.Options = options
	return b
}

// WithProperties sets the properties for object fields
func (b *FieldBuilder) WithProperties(props map[string]*Field) *FieldBuilder {
	b.field.Properties = props
	return b
}

// WithItems sets the item type for array fields
func (b *FieldBuilder) WithItems(items *Field) *FieldBuilder {
	b.field.Items = items
	return b
}

// Build returns the built field
func (b *FieldBuilder) Build() *Field {
	// Set default display name if not provided
	if b.field.DisplayName == "" {
		b.field.DisplayName = b.field.Name
	}
	return b.field
}

// Validate validates a schema
func (s *Schema) Validate() error {
	if s.Name == "" {
		return fmt.Errorf("schema name is required")
	}

	fieldNames := make(map[string]bool)
	for _, field := range s.Fields {
		if err := field.Validate(); err != nil {
			return fmt.Errorf("field '%s': %w", field.Name, err)
		}

		if fieldNames[field.Name] {
			return fmt.Errorf("duplicate field name: %s", field.Name)
		}
		fieldNames[field.Name] = true
	}

	return nil
}

// Validate validates a field definition
func (f *Field) Validate() error {
	if f.Name == "" {
		return fmt.Errorf("field name is required")
	}

	if !IsValidFieldType(f.Type) {
		return fmt.Errorf("invalid field type: %s", f.Type)
	}

	// Validate enum fields
	if f.Type == FieldTypeEnum && len(f.Options) == 0 {
		return fmt.Errorf("enum field must have options")
	}

	// Validate array fields
	if f.Type == FieldTypeArray && f.Items == nil {
		return fmt.Errorf("array field must have item type")
	}

	// Validate object fields
	if f.Type == FieldTypeObject && len(f.Properties) == 0 {
		return fmt.Errorf("object field must have properties")
	}

	// Validate nested fields
	if f.Items != nil {
		if err := f.Items.Validate(); err != nil {
			return fmt.Errorf("array items: %w", err)
		}
	}

	for name, prop := range f.Properties {
		if err := prop.Validate(); err != nil {
			return fmt.Errorf("property '%s': %w", name, err)
		}
	}

	return nil
}

// GetField returns a field by name
func (s *Schema) GetField(name string) (*Field, bool) {
	for _, field := range s.Fields {
		if field.Name == name {
			return field, true
		}
	}
	return nil, false
}

// ToJSON converts the schema to JSON
func (s *Schema) ToJSON() ([]byte, error) {
	return json.MarshalIndent(s, "", "  ")
}

// FromJSON creates a schema from JSON
func FromJSON(data []byte) (*Schema, error) {
	var schema Schema
	if err := json.Unmarshal(data, &schema); err != nil {
		return nil, err
	}

	if err := schema.Validate(); err != nil {
		return nil, err
	}

	return &schema, nil
}

// Clone creates a deep copy of the schema
func (s *Schema) Clone() *Schema {
	data, _ := json.Marshal(s)
	var clone Schema
	json.Unmarshal(data, &clone)
	return &clone
}

// MergeSchemas merges multiple schemas into one
func MergeSchemas(schemas ...*Schema) (*Schema, error) {
	if len(schemas) == 0 {
		return nil, fmt.Errorf("no schemas to merge")
	}

	merged := &Schema{
		Name:   schemas[0].Name,
		Fields: make([]*Field, 0),
	}

	fieldNames := make(map[string]bool)
	for _, schema := range schemas {
		for _, field := range schema.Fields {
			if fieldNames[field.Name] {
				continue // Skip duplicate fields
			}
			merged.Fields = append(merged.Fields, field)
			fieldNames[field.Name] = true
		}
	}

	return merged, nil
}
