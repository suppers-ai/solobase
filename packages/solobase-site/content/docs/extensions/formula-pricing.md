---
title: "Formula Based Pricing"
description: "Create dynamic pricing models with unlimited flexibility"
weight: 10
---

# Formula Based Pricing Extension

The Formula Based Pricing extension allows you to create complex, dynamic pricing models using mathematical formulas and variables. Perfect for businesses with custom pricing requirements that go beyond simple fixed prices.

## Features

- **Custom Formulas** - Define pricing formulas using standard mathematical operations
- **Dynamic Variables** - Create unlimited variables that can be adjusted in real-time
- **Multiple Pricing Models** - Support different pricing strategies for different products or services
- **Calculation History** - Track all price calculations for auditing
- **API Integration** - Easy integration with your existing systems

## Installation

```bash
solobase extension install formula-pricing
```

## Quick Example

```javascript
// Define your pricing formula
const formula = "base_price * number_of_items * (1 + sales_tax/100)";

// Set variables
const variables = {
  base_price: 100,
  number_of_items: 5,
  sales_tax: 10
};

// Calculate result
const result = calculatePrice(formula, variables);
// Result: 550.00
```

## Configuration

Add to your `solobase.config.yml`:

```yaml
extensions:
  formula-pricing:
    enabled: true
    decimal_places: 2
    currency: "USD"
    allow_negative: false
```

## API Endpoints

### Create Formula

```http
POST /api/extensions/formula-pricing/formulas
Content-Type: application/json

{
  "name": "Standard Pricing",
  "formula": "base_price * quantity * (1 + margin/100)",
  "variables": {
    "base_price": {
      "type": "number",
      "default": 100,
      "min": 0
    },
    "quantity": {
      "type": "number",
      "default": 1,
      "min": 1
    },
    "margin": {
      "type": "number",
      "default": 20,
      "min": 0,
      "max": 100
    }
  }
}
```

### Calculate Price

```http
POST /api/extensions/formula-pricing/calculate
Content-Type: application/json

{
  "formula_id": "uuid-here",
  "variables": {
    "base_price": 150,
    "quantity": 3,
    "margin": 25
  }
}
```

Response:
```json
{
  "result": 562.50,
  "formula": "base_price * quantity * (1 + margin/100)",
  "calculation": "150 * 3 * (1 + 25/100)",
  "variables_used": {
    "base_price": 150,
    "quantity": 3,
    "margin": 25
  }
}
```

## Supported Operations

- Basic arithmetic: `+`, `-`, `*`, `/`
- Parentheses: `(`, `)`
- Power: `^`
- Modulo: `%`
- Functions: `min()`, `max()`, `round()`, `floor()`, `ceil()`, `abs()`

## Advanced Examples

### Tiered Pricing

```javascript
const formula = `
  base_price * 
  (quantity < 10 ? 1 : 
   quantity < 50 ? 0.9 : 
   quantity < 100 ? 0.8 : 0.7)
`;
```

### Volume Discount with Tax

```javascript
const formula = `
  (base_price * quantity - 
   max(0, (quantity - 10) * discount_per_unit)) * 
  (1 + tax_rate/100)
`;
```

### Service Pricing with Hours

```javascript
const formula = `
  hourly_rate * hours_worked * 
  (1 + urgency_factor/100) * 
  (is_weekend ? 1.5 : 1)
`;
```

## Variable Types

### Number Variables
```json
{
  "type": "number",
  "default": 0,
  "min": 0,
  "max": 1000,
  "step": 0.01
}
```

### Boolean Variables
```json
{
  "type": "boolean",
  "default": false
}
```

### Select Variables
```json
{
  "type": "select",
  "options": [
    {"value": 1, "label": "Standard"},
    {"value": 1.5, "label": "Express"},
    {"value": 2, "label": "Priority"}
  ],
  "default": 1
}
```

## Best Practices

1. **Validate Formulas** - Always validate formulas before saving
2. **Set Boundaries** - Use min/max values to prevent unrealistic calculations
3. **Document Variables** - Provide clear descriptions for each variable
4. **Test Edge Cases** - Test with minimum, maximum, and zero values
5. **Version Control** - Keep track of formula changes over time

## Troubleshooting

### Common Issues

**Formula returns NaN**
- Check for division by zero
- Ensure all variables are defined
- Validate mathematical operations

**Unexpected Results**
- Review operator precedence
- Check parentheses placement
- Verify variable values

**Performance Issues**
- Simplify complex formulas
- Cache frequently used calculations
- Use batch calculation endpoints

## Support

For issues or questions about the Formula Based Pricing extension:

- [GitHub Issues](https://github.com/suppers-ai/solobase-extensions/issues)
- [Documentation](https://docs.solobase.ai/extensions/formula-pricing)
- [Discord Community](https://discord.gg/solobase)