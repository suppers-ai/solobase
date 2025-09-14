# Dynamic Fields Package

A flexible and type-safe dynamic field management system for Go applications. This package allows you to define schemas with various field types, validate data against those schemas, and perform data transformations.

## Features

- **Schema Definition**: Define flexible schemas with various field types
- **Field Types**: Support for text, number, boolean, date, email, URL, enum, array, object, file, image, and geo types
- **Validation**: Comprehensive validation rules including required fields, min/max values, patterns, and custom validators
- **Type Mapping**: Automatic type conversion and mapping between different formats
- **Builder Pattern**: Fluent API for building schemas and fields
- **JSON Support**: Full JSON serialization and deserialization
- **Nested Structures**: Support for complex nested objects and arrays

## Installation

```bash
go get github.com/suppers-ai/dynamicfields
```

## Quick Start

### Creating a Schema

```go
package main

import (
    "fmt"
    "github.com/suppers-ai/dynamicfields"
)

func main() {
    // Build a schema using the fluent API
    schema := dynamicfields.NewSchemaBuilder("user_profile").
        WithDescription("User profile information").
        AddField(
            dynamicfields.NewFieldBuilder("name", dynamicfields.FieldTypeText).
                WithDisplayName("Full Name").
                Required().
                WithValidation(&dynamicfields.ValidationRules{
                    MinLength: ptrInt(2),
                    MaxLength: ptrInt(100),
                }).
                Build(),
        ).
        AddField(
            dynamicfields.NewFieldBuilder("email", dynamicfields.FieldTypeEmail).
                WithDisplayName("Email Address").
                Required().
                WithPlaceholder("user@example.com").
                Build(),
        ).
        AddField(
            dynamicfields.NewFieldBuilder("age", dynamicfields.FieldTypeNumber).
                WithDisplayName("Age").
                WithValidation(&dynamicfields.ValidationRules{
                    Min: ptrFloat64(0),
                    Max: ptrFloat64(150),
                }).
                Build(),
        ).
        Build()

    // Validate the schema
    if err := schema.Validate(); err != nil {
        panic(err)
    }
}
```

### Validating Data

```go
// Create a validator
validator := dynamicfields.NewValidator(schema)

// Create a document with data
doc := &dynamicfields.Document{
    Values: map[string]interface{}{
        "name":  "John Doe",
        "email": "john@example.com",
        "age":   30,
    },
}

// Validate the document
if errors := validator.ValidateDocument(doc); errors != nil {
    fmt.Printf("Validation errors: %v\n", errors)
}
```

### Mapping Data

```go
// Create a mapper
mapper := dynamicfields.NewMapper(schema)

// Map raw data to a document
data := map[string]interface{}{
    "name":  "Jane Doe",
    "email": "jane@example.com",
    "age":   "25", // String will be converted to number
}

doc, err := mapper.MapToDocument("doc-123", data)
if err != nil {
    panic(err)
}
```

## Field Types

### Basic Types

- `FieldTypeText`: String values
- `FieldTypeNumber`: Numeric values (float64)
- `FieldTypeBoolean`: Boolean values
- `FieldTypeDate`: Date values (time.Time)
- `FieldTypeDateTime`: DateTime values (time.Time)
- `FieldTypeEmail`: Email addresses with validation
- `FieldTypeURL`: URLs with validation

### Complex Types

- `FieldTypeEnum`: Enumeration with predefined options
- `FieldTypeArray`: Array of items with optional item type
- `FieldTypeObject`: Nested object with properties
- `FieldTypeFile`: File metadata
- `FieldTypeImage`: Image metadata
- `FieldTypeGeo`: Geographic coordinates (lat/lng)

## Validation Rules

### Text Validation
```go
&ValidationRules{
    MinLength: ptrInt(5),
    MaxLength: ptrInt(100),
    Pattern:   ptrString("^[A-Za-z]+$"),
}
```

### Number Validation
```go
&ValidationRules{
    Min:       ptrFloat64(0),
    Max:       ptrFloat64(100),
    Step:      ptrFloat64(0.5),
    Precision: ptrInt(2),
}
```

### Date Validation
```go
minDate := time.Date(2020, 1, 1, 0, 0, 0, 0, time.UTC)
maxDate := time.Date(2025, 12, 31, 0, 0, 0, 0, time.UTC)

&ValidationRules{
    MinDate: &minDate,
    MaxDate: &maxDate,
}
```

### Array Validation
```go
&ValidationRules{
    MinItems: ptrInt(1),
    MaxItems: ptrInt(10),
    Unique:   true,
}
```

## Complex Examples

### Enum Field
```go
statusField := NewFieldBuilder("status", FieldTypeEnum).
    WithDisplayName("Order Status").
    WithOptions([]Option{
        {Value: "pending", Label: "Pending"},
        {Value: "processing", Label: "Processing"},
        {Value: "shipped", Label: "Shipped"},
        {Value: "delivered", Label: "Delivered"},
    }).
    WithDefault("pending").
    Required().
    Build()
```

### Array Field
```go
tagsField := NewFieldBuilder("tags", FieldTypeArray).
    WithDisplayName("Tags").
    WithItems(&Field{
        Type: FieldTypeText,
        Validation: &ValidationRules{
            MinLength: ptrInt(2),
            MaxLength: ptrInt(20),
        },
    }).
    WithValidation(&ValidationRules{
        MinItems: ptrInt(1),
        MaxItems: ptrInt(5),
        Unique:   true,
    }).
    Build()
```

### Nested Object Field
```go
addressField := NewFieldBuilder("address", FieldTypeObject).
    WithDisplayName("Address").
    WithProperties(map[string]*Field{
        "street": {
            Name:     "street",
            Type:     FieldTypeText,
            Required: true,
        },
        "city": {
            Name:     "city",
            Type:     FieldTypeText,
            Required: true,
        },
        "zipcode": {
            Name: "zipcode",
            Type: FieldTypeText,
            Validation: &ValidationRules{
                Pattern: ptrString("^\\d{5}$"),
            },
        },
        "location": {
            Name: "location",
            Type: FieldTypeGeo,
        },
    }).
    Required().
    Build()
```

## JSON Serialization

```go
// Convert schema to JSON
jsonData, err := schema.ToJSON()
if err != nil {
    panic(err)
}

// Load schema from JSON
loadedSchema, err := dynamicfields.FromJSON(jsonData)
if err != nil {
    panic(err)
}
```

## Schema Operations

### Cloning
```go
clonedSchema := schema.Clone()
```

### Merging
```go
merged, err := dynamicfields.MergeSchemas(schema1, schema2, schema3)
if err != nil {
    panic(err)
}
```

### Field Lookup
```go
field, exists := schema.GetField("email")
if exists {
    fmt.Printf("Field type: %s\n", field.Type)
}
```

## Data Transformations

### Apply Defaults
```go
data := map[string]interface{}{
    "name": "John",
}

// Apply default values for missing fields
dataWithDefaults := mapper.ApplyDefaults(data)
```

### Filter Fields
```go
data := map[string]interface{}{
    "name":    "John",
    "email":   "john@example.com",
    "unknown": "This field is not in schema",
}

// Remove fields not defined in schema
filtered := mapper.FilterFields(data)
```

### Extract Values
```go
// Extract values from document in external format
values := mapper.ExtractValues(doc)
```

## Testing

Run the tests:
```bash
go test ./...
```

Run with coverage:
```bash
go test -cover ./...
```

## License

This package is part of the Suppers AI Builder project.