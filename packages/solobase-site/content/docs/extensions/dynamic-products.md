---
title: "Dynamic Products"
description: "Create custom product schemas with flexible fields"
weight: 20
---

# Dynamic Products Extension

The Dynamic Products extension enables you to define custom product schemas tailored to your specific business needs. Instead of being limited to predefined fields, you can create exactly the product structure you need with custom fields, validations, and search capabilities.

## Features

- **Custom Product Schemas** - Define your own product structure
- **Flexible Field Types** - Support for text, numbers, dates, arrays, and more
- **Advanced Search** - Search across custom fields efficiently
- **Validation Rules** - Set custom validation for each field
- **Bulk Operations** - Import/export products with custom schemas
- **Version Control** - Track schema changes over time

## Installation

```bash
solobase extension install dynamic-products
```

## Quick Start

### Define a Product Schema

```json
{
  "schema_name": "Electronics",
  "fields": [
    {
      "name": "brand",
      "type": "string",
      "required": true,
      "searchable": true
    },
    {
      "name": "specifications",
      "type": "object",
      "fields": [
        {
          "name": "processor",
          "type": "string"
        },
        {
          "name": "ram_gb",
          "type": "number",
          "min": 1,
          "max": 1024
        },
        {
          "name": "storage_gb",
          "type": "number"
        }
      ]
    },
    {
      "name": "features",
      "type": "array",
      "items": {
        "type": "string"
      }
    },
    {
      "name": "warranty_years",
      "type": "number",
      "default": 1
    }
  ]
}
```

## Configuration

Add to your `solobase.config.yml`:

```yaml
extensions:
  dynamic-products:
    enabled: true
    max_schemas: 50
    max_fields_per_schema: 100
    enable_versioning: true
    search_engine: "elasticsearch" # or "postgres"
```

## API Endpoints

### Create Product Schema

```http
POST /api/extensions/dynamic-products/schemas
Content-Type: application/json

{
  "name": "Clothing",
  "description": "Schema for clothing products",
  "fields": [
    {
      "name": "size",
      "type": "select",
      "options": ["XS", "S", "M", "L", "XL", "XXL"],
      "required": true
    },
    {
      "name": "color",
      "type": "string",
      "required": true,
      "searchable": true
    },
    {
      "name": "material",
      "type": "array",
      "items": {
        "type": "string"
      }
    },
    {
      "name": "care_instructions",
      "type": "text"
    }
  ]
}
```

### Create Product

```http
POST /api/extensions/dynamic-products/products
Content-Type: application/json

{
  "schema_id": "clothing-schema-id",
  "data": {
    "name": "Premium Cotton T-Shirt",
    "size": "L",
    "color": "Navy Blue",
    "material": ["100% Cotton", "Organic"],
    "care_instructions": "Machine wash cold, tumble dry low",
    "price": 29.99
  }
}
```

### Search Products

```http
GET /api/extensions/dynamic-products/products/search?q=cotton&schema=clothing&color=blue
```

## Field Types

### Text Fields
```json
{
  "name": "description",
  "type": "text",
  "min_length": 10,
  "max_length": 5000,
  "searchable": true
}
```

### Number Fields
```json
{
  "name": "price",
  "type": "number",
  "min": 0,
  "max": 999999.99,
  "decimal_places": 2,
  "required": true
}
```

### Date Fields
```json
{
  "name": "release_date",
  "type": "date",
  "format": "YYYY-MM-DD",
  "min": "2020-01-01"
}
```

### Select Fields
```json
{
  "name": "category",
  "type": "select",
  "options": [
    {"value": "electronics", "label": "Electronics"},
    {"value": "clothing", "label": "Clothing"},
    {"value": "books", "label": "Books"}
  ],
  "required": true
}
```

### Array Fields
```json
{
  "name": "tags",
  "type": "array",
  "items": {
    "type": "string",
    "max_length": 50
  },
  "max_items": 20
}
```

### Object Fields
```json
{
  "name": "dimensions",
  "type": "object",
  "fields": [
    {"name": "length", "type": "number"},
    {"name": "width", "type": "number"},
    {"name": "height", "type": "number"},
    {"name": "unit", "type": "select", "options": ["cm", "inch"]}
  ]
}
```

## Advanced Features

### Custom Validations

```javascript
{
  "name": "sku",
  "type": "string",
  "validation": {
    "pattern": "^[A-Z]{3}-[0-9]{6}$",
    "message": "SKU must be in format XXX-000000"
  }
}
```

### Computed Fields

```javascript
{
  "name": "full_name",
  "type": "computed",
  "formula": "{{brand}} {{model}} {{year}}"
}
```

### Field Dependencies

```javascript
{
  "name": "warranty_type",
  "type": "select",
  "options": ["standard", "extended"],
  "visible_when": {
    "field": "price",
    "operator": ">",
    "value": 100
  }
}
```

## Schema Examples

### Real Estate Listing
```json
{
  "name": "RealEstateListing",
  "fields": [
    {"name": "property_type", "type": "select", "options": ["house", "apartment", "condo"]},
    {"name": "bedrooms", "type": "number", "min": 0, "max": 20},
    {"name": "bathrooms", "type": "number", "min": 0, "max": 10, "decimal_places": 1},
    {"name": "square_feet", "type": "number", "min": 0},
    {"name": "amenities", "type": "array", "items": {"type": "string"}},
    {"name": "location", "type": "object", "fields": [
      {"name": "address", "type": "string"},
      {"name": "city", "type": "string"},
      {"name": "zip", "type": "string"},
      {"name": "coordinates", "type": "object", "fields": [
        {"name": "lat", "type": "number"},
        {"name": "lng", "type": "number"}
      ]}
    ]}
  ]
}
```

### Digital Course
```json
{
  "name": "DigitalCourse",
  "fields": [
    {"name": "title", "type": "string", "required": true},
    {"name": "instructor", "type": "string", "required": true},
    {"name": "duration_hours", "type": "number", "min": 0.5},
    {"name": "difficulty", "type": "select", "options": ["beginner", "intermediate", "advanced"]},
    {"name": "topics", "type": "array", "items": {"type": "string"}},
    {"name": "prerequisites", "type": "array", "items": {"type": "string"}},
    {"name": "certificate_available", "type": "boolean", "default": false}
  ]
}
```

## Best Practices

1. **Plan Your Schema** - Design your schema carefully before implementation
2. **Use Appropriate Types** - Choose the right field type for your data
3. **Enable Search Strategically** - Only mark fields as searchable if needed
4. **Version Your Schemas** - Keep track of schema changes
5. **Validate Early** - Add validation rules to prevent bad data
6. **Document Fields** - Provide clear descriptions for each field

## Migration Guide

### From Static to Dynamic Products

```javascript
// Export existing products
const products = await exportStaticProducts();

// Create new schema matching your needs
const schema = await createSchema({
  name: "MigratedProducts",
  fields: mapFieldsFromStatic()
});

// Import products to new schema
await importToDynamicProducts(products, schema.id);
```

## Performance Optimization

- Index searchable fields
- Use pagination for large datasets
- Cache frequently accessed schemas
- Optimize complex object structures
- Use batch operations for bulk updates

## Support

For help with the Dynamic Products extension:

- [GitHub Issues](https://github.com/suppers-ai/solobase-extensions/issues)
- [Documentation](https://docs.solobase.ai/extensions/dynamic-products)
- [Discord Community](https://discord.gg/solobase)