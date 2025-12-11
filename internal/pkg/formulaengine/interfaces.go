package formulaengine

import "context"

// VariableResolver provides variable values for calculations
type VariableResolver interface {
	// GetVariable retrieves the value of a variable by name
	GetVariable(ctx context.Context, name string) (interface{}, error)
	// HasVariable checks if a variable exists
	HasVariable(ctx context.Context, name string) bool
	// GetAllVariables returns all available variables
	GetAllVariables(ctx context.Context) (map[string]interface{}, error)
}

// Calculator performs formula calculations
type Calculator interface {
	// Calculate evaluates a formula with the given variables
	Calculate(ctx context.Context, formula string, resolver VariableResolver) (float64, error)
	// ValidateFormula checks if a formula is syntactically valid
	ValidateFormula(formula string) error
}

// Evaluator evaluates conditions
type Evaluator interface {
	// Evaluate checks if a condition is true
	Evaluate(ctx context.Context, condition string, resolver VariableResolver) (bool, error)
	// ValidateCondition checks if a condition is syntactically valid
	ValidateCondition(condition string) error
}

// Parser parses formulas and conditions
type Parser interface {
	// ParseFormula parses a formula string into an AST
	ParseFormula(formula string) (Expression, error)
	// ParseCondition parses a condition string into an AST
	ParseCondition(condition string) (Expression, error)
}

// Expression represents a parsed expression tree
type Expression interface {
	// Evaluate calculates the value of the expression
	Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error)
	// String returns the string representation
	String() string
	// Type returns the expected result type
	Type() ExpressionType
}

// ExpressionType represents the type of an expression
type ExpressionType int

const (
	TypeUnknown ExpressionType = iota
	TypeNumber
	TypeString
	TypeBoolean
	TypeDate
	TypeArray
)

// Rule represents a pricing rule with condition and calculation
type Rule struct {
	Condition   string `json:"condition"`
	Calculation string `json:"calculation"`
}

// CalculationResult represents the result of a calculation
type CalculationResult struct {
	Value       float64                `json:"value"`
	Formula     string                 `json:"formula"`
	Variables   map[string]interface{} `json:"variables"`
	RuleApplied *Rule                  `json:"ruleApplied,omitempty"`
	Error       string                 `json:"error,omitempty"`
}
