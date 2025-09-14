# Formula Engine

A standalone, pure Go formula calculation engine for dynamic pricing and conditional logic evaluation.

## Features

- **Formula Calculation**: Evaluate mathematical expressions with variables
- **Condition Evaluation**: Evaluate boolean conditions
- **Rule-Based Pricing**: Apply pricing rules based on conditions
- **Built-in Functions**: Mathematical functions like min, max, round, floor, ceil, etc.
- **Type Safety**: Strong type checking and conversion
- **No External Dependencies**: Pure Go implementation (except for testing)
- **Thread-Safe**: Safe for concurrent use

## Installation

```bash
go get github.com/suppers-ai/formulaengine
```

## Usage

### Basic Calculation

```go
package main

import (
    "context"
    "fmt"
    "github.com/suppers-ai/formulaengine"
)

func main() {
    // Create a new engine
    engine := formulaengine.NewEngine()
    
    // Define variables
    variables := map[string]interface{}{
        "basePrice": 100.0,
        "quantity":  5,
        "discount":  10.0,
    }
    
    // Create a resolver
    resolver := formulaengine.NewSimpleResolver(variables)
    
    // Calculate formula
    formula := "(basePrice * quantity) * (1 - discount / 100)"
    result, err := engine.Calculate(context.Background(), formula, resolver)
    if err != nil {
        panic(err)
    }
    
    fmt.Printf("Result: %.2f\n", result) // Result: 450.00
}
```

### Condition Evaluation

```go
// Evaluate conditions
condition := "quantity >= 10 && discount > 0"
isTrue, err := engine.EvaluateCondition(context.Background(), condition, resolver)
if err != nil {
    panic(err)
}
fmt.Printf("Condition is: %v\n", isTrue)
```

### Rule-Based Pricing

```go
// Define pricing rules
rules := []formulaengine.Rule{
    {Condition: "quantity >= 100", Calculation: "basePrice * quantity * 0.7"},
    {Condition: "quantity >= 50", Calculation: "basePrice * quantity * 0.8"},
    {Condition: "quantity >= 10", Calculation: "basePrice * quantity * 0.9"},
    {Condition: "true", Calculation: "basePrice * quantity"},
}

// Evaluate rules
result, err := engine.EvaluateRules(context.Background(), rules, resolver)
if err != nil {
    panic(err)
}

fmt.Printf("Price: %.2f\n", result.Value)
fmt.Printf("Rule Applied: %s\n", result.RuleApplied.Condition)
```

### Custom Variable Resolver

```go
type MyResolver struct {
    db *sql.DB
}

func (r *MyResolver) GetVariable(ctx context.Context, name string) (interface{}, error) {
    // Fetch variable from database
    var value float64
    err := r.db.QueryRowContext(ctx, 
        "SELECT value FROM variables WHERE name = ?", name).Scan(&value)
    return value, err
}

func (r *MyResolver) HasVariable(ctx context.Context, name string) bool {
    var exists bool
    r.db.QueryRowContext(ctx, 
        "SELECT EXISTS(SELECT 1 FROM variables WHERE name = ?)", name).Scan(&exists)
    return exists
}

func (r *MyResolver) GetAllVariables(ctx context.Context) (map[string]interface{}, error) {
    // Implementation
    return nil, nil
}
```

## Supported Operators

### Arithmetic Operators
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division
- `%` Modulo

### Comparison Operators
- `==` Equal
- `!=` Not equal
- `<` Less than
- `<=` Less than or equal
- `>` Greater than
- `>=` Greater than or equal

### Logical Operators
- `&&` Logical AND
- `||` Logical OR

### Ternary Operator
- `condition ? trueValue : falseValue`

## Built-in Functions

- `min(a, b, ...)` - Returns the minimum value
- `max(a, b, ...)` - Returns the maximum value
- `abs(x)` - Returns the absolute value
- `round(x, precision)` - Rounds to specified decimal places
- `floor(x)` - Rounds down
- `ceil(x)` - Rounds up
- `pow(base, exp)` - Power function
- `sqrt(x)` - Square root
- `if(condition, trueValue, falseValue)` - Conditional function

## Examples

### E-commerce Pricing

```go
// Quantity-based discounts with shipping
formula := `
    (basePrice * quantity * 
        (quantity >= 100 ? 0.7 : 
         quantity >= 50 ? 0.8 : 
         quantity >= 10 ? 0.9 : 1.0)) +
    (weight * shippingRate) +
    (expressShipping ? 20 : 0)
`

variables := map[string]interface{}{
    "basePrice":       29.99,
    "quantity":        75,
    "weight":          2.5,
    "shippingRate":    4.99,
    "expressShipping": true,
}
```

### Subscription Pricing

```go
// SaaS tier-based pricing
formula := `
    seats * (
        seats > 100 ? 8 : 
        seats > 20 ? 10 : 
        seats > 5 ? 12 : 15
    ) * (billingCycle == 'annual' ? 10 : 1)
`

variables := map[string]interface{}{
    "seats":        25,
    "billingCycle": "annual",
}
```

### Service Pricing

```go
// Professional services with urgency multiplier
formula := `
    hourlyRate * hours * 
    (urgency == 'emergency' ? 2.0 : 
     urgency == 'urgent' ? 1.5 : 1.0) *
    (1 + complexity * 0.1)
`

variables := map[string]interface{}{
    "hourlyRate": 150,
    "hours":      8,
    "urgency":    "urgent",
    "complexity": 3, // 1-5 scale
}
```

## Error Handling

The engine provides detailed error information:

```go
err := engine.ValidateFormula("2 + (3 * 4")
if err != nil {
    // Error: parse error: expected closing parenthesis
}

result, err := engine.Calculate(ctx, "price * quantity", resolver)
if err != nil {
    // Error: variable 'price' not found
}
```

## Performance

The engine is designed for high performance:
- Formulas are parsed once and can be cached
- No reflection in hot paths
- Minimal allocations
- Thread-safe for concurrent use

## Testing

Run tests with:

```bash
go test -v ./...
```

Run benchmarks:

```bash
go test -bench=. -benchmem
```

## License

MIT License