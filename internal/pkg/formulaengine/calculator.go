package formulaengine

import (
	"context"
	"fmt"
	"math"
	"strconv"
	"strings"
)

// Engine is the main formula calculation engine
type Engine struct {
	parser    *FormulaParser
	evaluator *ConditionEvaluator
}

// NewEngine creates a new formula engine
func NewEngine() *Engine {
	return &Engine{
		parser:    NewFormulaParser(),
		evaluator: NewConditionEvaluator(),
	}
}

// Calculate evaluates a formula with the given variables
func (e *Engine) Calculate(ctx context.Context, formula string, resolver VariableResolver) (float64, error) {
	expr, err := e.parser.ParseFormula(formula)
	if err != nil {
		return 0, fmt.Errorf("parse error: %w", err)
	}

	result, err := expr.Evaluate(ctx, resolver)
	if err != nil {
		return 0, fmt.Errorf("evaluation error: %w", err)
	}

	return toFloat64(result)
}

// ValidateFormula checks if a formula is syntactically valid
func (e *Engine) ValidateFormula(formula string) error {
	_, err := e.parser.ParseFormula(formula)
	return err
}

// EvaluateCondition checks if a condition is true
func (e *Engine) EvaluateCondition(ctx context.Context, condition string, resolver VariableResolver) (bool, error) {
	return e.evaluator.Evaluate(ctx, condition, resolver)
}

// ValidateCondition checks if a condition is syntactically valid
func (e *Engine) ValidateCondition(condition string) error {
	return e.evaluator.ValidateCondition(condition)
}

// EvaluateRules evaluates a list of rules and returns the first matching calculation
func (e *Engine) EvaluateRules(ctx context.Context, rules []Rule, resolver VariableResolver) (*CalculationResult, error) {
	for _, rule := range rules {
		// Check if condition matches
		matches, err := e.EvaluateCondition(ctx, rule.Condition, resolver)
		if err != nil {
			return nil, fmt.Errorf("condition evaluation error: %w", err)
		}

		if matches {
			// Calculate the result
			value, err := e.Calculate(ctx, rule.Calculation, resolver)
			if err != nil {
				return nil, fmt.Errorf("calculation error: %w", err)
			}

			vars, _ := resolver.GetAllVariables(ctx)
			return &CalculationResult{
				Value:       value,
				Formula:     rule.Calculation,
				Variables:   vars,
				RuleApplied: &rule,
			}, nil
		}
	}

	return nil, fmt.Errorf("no matching rule found")
}

// SimpleVariableResolver is a basic implementation of VariableResolver
type SimpleVariableResolver struct {
	variables map[string]interface{}
}

// NewSimpleResolver creates a new simple variable resolver
func NewSimpleResolver(vars map[string]interface{}) *SimpleVariableResolver {
	return &SimpleVariableResolver{
		variables: vars,
	}
}

// GetVariable retrieves the value of a variable by name
func (r *SimpleVariableResolver) GetVariable(ctx context.Context, name string) (interface{}, error) {
	if val, ok := r.variables[name]; ok {
		return val, nil
	}
	return nil, fmt.Errorf("variable '%s' not found", name)
}

// HasVariable checks if a variable exists
func (r *SimpleVariableResolver) HasVariable(ctx context.Context, name string) bool {
	_, ok := r.variables[name]
	return ok
}

// GetAllVariables returns all available variables
func (r *SimpleVariableResolver) GetAllVariables(ctx context.Context) (map[string]interface{}, error) {
	result := make(map[string]interface{})
	for k, v := range r.variables {
		result[k] = v
	}
	return result, nil
}

// Helper functions

func toFloat64(val interface{}) (float64, error) {
	switch v := val.(type) {
	case float64:
		return v, nil
	case float32:
		return float64(v), nil
	case int:
		return float64(v), nil
	case int32:
		return float64(v), nil
	case int64:
		return float64(v), nil
	case string:
		return strconv.ParseFloat(v, 64)
	case bool:
		if v {
			return 1, nil
		}
		return 0, nil
	default:
		return 0, fmt.Errorf("cannot convert %T to float64", val)
	}
}

func toString(val interface{}) string {
	switch v := val.(type) {
	case string:
		return v
	case float64:
		if v == math.Floor(v) {
			return fmt.Sprintf("%.0f", v)
		}
		return fmt.Sprintf("%g", v)
	case int:
		return strconv.Itoa(v)
	case bool:
		return strconv.FormatBool(v)
	default:
		return fmt.Sprintf("%v", v)
	}
}

func toBool(val interface{}) (bool, error) {
	switch v := val.(type) {
	case bool:
		return v, nil
	case float64:
		return v != 0, nil
	case int:
		return v != 0, nil
	case string:
		v = strings.ToLower(strings.TrimSpace(v))
		if v == "true" || v == "1" || v == "yes" {
			return true, nil
		}
		if v == "false" || v == "0" || v == "no" || v == "" {
			return false, nil
		}
		return false, fmt.Errorf("cannot convert string '%s' to bool", v)
	default:
		return false, fmt.Errorf("cannot convert %T to bool", val)
	}
}
