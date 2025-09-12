package formulaengine

import (
	"context"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestEngine_Calculate(t *testing.T) {
	tests := []struct {
		name      string
		formula   string
		variables map[string]interface{}
		expected  float64
		wantErr   bool
	}{
		{
			name:      "simple addition",
			formula:   "2 + 3",
			variables: map[string]interface{}{},
			expected:  5,
		},
		{
			name:      "multiplication",
			formula:   "4 * 5",
			variables: map[string]interface{}{},
			expected:  20,
		},
		{
			name:      "division",
			formula:   "10 / 2",
			variables: map[string]interface{}{},
			expected:  5,
		},
		{
			name:      "with variables",
			formula:   "price * quantity",
			variables: map[string]interface{}{"price": 10.5, "quantity": 3},
			expected:  31.5,
		},
		{
			name:      "complex formula",
			formula:   "(basePrice + markup) * quantity * (1 - discount / 100)",
			variables: map[string]interface{}{"basePrice": 100, "markup": 20, "quantity": 2, "discount": 10},
			expected:  216,
		},
		{
			name:      "with functions",
			formula:   "min(100, price * 2)",
			variables: map[string]interface{}{"price": 60},
			expected:  100,
		},
		{
			name:      "max function",
			formula:   "max(10, 20, 15)",
			variables: map[string]interface{}{},
			expected:  20,
		},
		{
			name:      "round function",
			formula:   "round(10.567, 2)",
			variables: map[string]interface{}{},
			expected:  10.57,
		},
		{
			name:      "nested functions",
			formula:   "max(10, min(20, 15))",
			variables: map[string]interface{}{},
			expected:  15,
		},
		{
			name:      "division by zero",
			formula:   "10 / 0",
			variables: map[string]interface{}{},
			wantErr:   true,
		},
		{
			name:      "undefined variable",
			formula:   "price * quantity",
			variables: map[string]interface{}{"price": 10},
			wantErr:   true,
		},
	}

	engine := NewEngine()
	ctx := context.Background()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resolver := NewSimpleResolver(tt.variables)
			result, err := engine.Calculate(ctx, tt.formula, resolver)

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.InDelta(t, tt.expected, result, 0.001)
			}
		})
	}
}

func TestEngine_EvaluateCondition(t *testing.T) {
	tests := []struct {
		name      string
		condition string
		variables map[string]interface{}
		expected  bool
		wantErr   bool
	}{
		{
			name:      "simple true",
			condition: "true",
			variables: map[string]interface{}{},
			expected:  true,
		},
		{
			name:      "simple false",
			condition: "false",
			variables: map[string]interface{}{},
			expected:  false,
		},
		{
			name:      "always",
			condition: "always",
			variables: map[string]interface{}{},
			expected:  true,
		},
		{
			name:      "comparison",
			condition: "10 > 5",
			variables: map[string]interface{}{},
			expected:  true,
		},
		{
			name:      "variable comparison",
			condition: "quantity >= minQuantity",
			variables: map[string]interface{}{"quantity": 10, "minQuantity": 5},
			expected:  true,
		},
		{
			name:      "and operator",
			condition: "quantity > 5 && price < 100",
			variables: map[string]interface{}{"quantity": 10, "price": 50},
			expected:  true,
		},
		{
			name:      "or operator",
			condition: "quantity > 100 || vipCustomer == true",
			variables: map[string]interface{}{"quantity": 5, "vipCustomer": true},
			expected:  true,
		},
		{
			name:      "complex condition",
			condition: "(quantity >= 10 && price < 50) || discount > 20",
			variables: map[string]interface{}{"quantity": 5, "price": 60, "discount": 25},
			expected:  true,
		},
		{
			name:      "string comparison",
			condition: "userType == 'premium'",
			variables: map[string]interface{}{"userType": "premium"},
			expected:  true,
		},
		{
			name:      "not equal",
			condition: "status != 'inactive'",
			variables: map[string]interface{}{"status": "active"},
			expected:  true,
		},
	}

	engine := NewEngine()
	ctx := context.Background()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resolver := NewSimpleResolver(tt.variables)
			result, err := engine.EvaluateCondition(ctx, tt.condition, resolver)

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.expected, result)
			}
		})
	}
}

func TestEngine_EvaluateRules(t *testing.T) {
	rules := []Rule{
		{Condition: "quantity >= 100", Calculation: "basePrice * quantity * 0.8"},
		{Condition: "quantity >= 50", Calculation: "basePrice * quantity * 0.9"},
		{Condition: "quantity >= 10", Calculation: "basePrice * quantity * 0.95"},
		{Condition: "true", Calculation: "basePrice * quantity"},
	}

	tests := []struct {
		name      string
		variables map[string]interface{}
		expected  float64
	}{
		{
			name:      "bulk discount",
			variables: map[string]interface{}{"basePrice": 10, "quantity": 100},
			expected:  800, // 10 * 100 * 0.8
		},
		{
			name:      "medium discount",
			variables: map[string]interface{}{"basePrice": 10, "quantity": 50},
			expected:  450, // 10 * 50 * 0.9
		},
		{
			name:      "small discount",
			variables: map[string]interface{}{"basePrice": 10, "quantity": 10},
			expected:  95, // 10 * 10 * 0.95
		},
		{
			name:      "no discount",
			variables: map[string]interface{}{"basePrice": 10, "quantity": 5},
			expected:  50, // 10 * 5
		},
	}

	engine := NewEngine()
	ctx := context.Background()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resolver := NewSimpleResolver(tt.variables)
			result, err := engine.EvaluateRules(ctx, rules, resolver)

			require.NoError(t, err)
			assert.NotNil(t, result)
			assert.InDelta(t, tt.expected, result.Value, 0.001)
			assert.NotNil(t, result.RuleApplied)
		})
	}
}

func TestEngine_TernaryOperator(t *testing.T) {
	tests := []struct {
		name      string
		formula   string
		variables map[string]interface{}
		expected  float64
	}{
		{
			name:      "simple ternary",
			formula:   "quantity > 10 ? 100 : 50",
			variables: map[string]interface{}{"quantity": 15},
			expected:  100,
		},
		{
			name:      "ternary with calculation",
			formula:   "vip ? price * 0.8 : price",
			variables: map[string]interface{}{"vip": true, "price": 100},
			expected:  80,
		},
		{
			name:      "nested ternary",
			formula:   "quantity > 100 ? price * 0.7 : quantity > 50 ? price * 0.8 : price * 0.9",
			variables: map[string]interface{}{"quantity": 60, "price": 100},
			expected:  80,
		},
	}

	engine := NewEngine()
	ctx := context.Background()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resolver := NewSimpleResolver(tt.variables)
			result, err := engine.Calculate(ctx, tt.formula, resolver)

			require.NoError(t, err)
			assert.InDelta(t, tt.expected, result, 0.001)
		})
	}
}

func TestEngine_ValidateFormula(t *testing.T) {
	tests := []struct {
		name    string
		formula string
		wantErr bool
	}{
		{
			name:    "valid simple",
			formula: "2 + 3",
			wantErr: false,
		},
		{
			name:    "valid with variables",
			formula: "price * quantity",
			wantErr: false,
		},
		{
			name:    "valid with functions",
			formula: "min(10, max(5, x))",
			wantErr: false,
		},
		{
			name:    "invalid parentheses",
			formula: "2 + (3 * 4",
			wantErr: true,
		},
		{
			name:    "invalid operator",
			formula: "2 ++ 3",
			wantErr: true,
		},
		{
			name:    "empty formula",
			formula: "",
			wantErr: true,
		},
	}

	engine := NewEngine()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := engine.ValidateFormula(tt.formula)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}
