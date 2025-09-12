package formulaengine

import (
	"context"
	"fmt"
	"strings"
)

// ConditionEvaluator evaluates boolean conditions
type ConditionEvaluator struct {
	parser *FormulaParser
}

// NewConditionEvaluator creates a new condition evaluator
func NewConditionEvaluator() *ConditionEvaluator {
	return &ConditionEvaluator{
		parser: NewFormulaParser(),
	}
}

// Evaluate checks if a condition is true
func (e *ConditionEvaluator) Evaluate(ctx context.Context, condition string, resolver VariableResolver) (bool, error) {
	// Handle special cases - only check for exact matches, don't lowercase everything
	trimmed := strings.TrimSpace(condition)
	lower := strings.ToLower(trimmed)
	if trimmed == "" || lower == "true" || lower == "always" {
		return true, nil
	}
	if lower == "false" || lower == "never" {
		return false, nil
	}

	// Use original condition for parsing (preserves variable case)

	// Parse and evaluate the condition
	expr, err := e.parser.ParseCondition(condition)
	if err != nil {
		return false, fmt.Errorf("parse error: %w", err)
	}

	result, err := expr.Evaluate(ctx, resolver)
	if err != nil {
		return false, fmt.Errorf("evaluation error: %w", err)
	}

	return toBool(result)
}

// ValidateCondition checks if a condition is syntactically valid
func (e *ConditionEvaluator) ValidateCondition(condition string) error {
	// Handle special cases
	trimmed := strings.TrimSpace(condition)
	lower := strings.ToLower(trimmed)
	if trimmed == "" || lower == "true" || lower == "always" || lower == "false" || lower == "never" {
		return nil
	}

	_, err := e.parser.ParseCondition(condition)
	return err
}

// EvaluateMultiple evaluates multiple conditions and returns results
func (e *ConditionEvaluator) EvaluateMultiple(ctx context.Context, conditions []string, resolver VariableResolver) ([]bool, error) {
	results := make([]bool, len(conditions))

	for i, condition := range conditions {
		result, err := e.Evaluate(ctx, condition, resolver)
		if err != nil {
			return nil, fmt.Errorf("condition %d: %w", i, err)
		}
		results[i] = result
	}

	return results, nil
}

// FindFirstMatch finds the first condition that evaluates to true
func (e *ConditionEvaluator) FindFirstMatch(ctx context.Context, conditions []string, resolver VariableResolver) (int, error) {
	for i, condition := range conditions {
		match, err := e.Evaluate(ctx, condition, resolver)
		if err != nil {
			return -1, fmt.Errorf("condition %d: %w", i, err)
		}
		if match {
			return i, nil
		}
	}
	return -1, nil
}
