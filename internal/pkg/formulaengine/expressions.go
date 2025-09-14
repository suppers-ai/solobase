package formulaengine

import (
	"context"
	"fmt"
	"math"
	"strings"
	"time"
)

// LiteralExpression represents a literal value
type LiteralExpression struct {
	value    interface{}
	exprType ExpressionType
}

func (e *LiteralExpression) Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error) {
	return e.value, nil
}

func (e *LiteralExpression) String() string {
	return fmt.Sprintf("%v", e.value)
}

func (e *LiteralExpression) Type() ExpressionType {
	return e.exprType
}

// VariableExpression represents a variable reference
type VariableExpression struct {
	name string
}

func (e *VariableExpression) Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error) {
	return resolver.GetVariable(ctx, e.name)
}

func (e *VariableExpression) String() string {
	return e.name
}

func (e *VariableExpression) Type() ExpressionType {
	return TypeUnknown
}

// BinaryExpression represents a binary operation
type BinaryExpression struct {
	left     Expression
	operator string
	right    Expression
}

func (e *BinaryExpression) Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error) {
	leftVal, err := e.left.Evaluate(ctx, resolver)
	if err != nil {
		return nil, err
	}

	rightVal, err := e.right.Evaluate(ctx, resolver)
	if err != nil {
		return nil, err
	}

	switch e.operator {
	case "+":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left + right, nil

	case "-":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left - right, nil

	case "*":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left * right, nil

	case "/":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		if right == 0 {
			return nil, fmt.Errorf("division by zero")
		}
		return left / right, nil

	case "%":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		if right == 0 {
			return nil, fmt.Errorf("modulo by zero")
		}
		return math.Mod(left, right), nil

	case "==":
		return compareEqual(leftVal, rightVal), nil

	case "!=":
		return !compareEqual(leftVal, rightVal), nil

	case "<":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left < right, nil

	case "<=":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left <= right, nil

	case ">":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left > right, nil

	case ">=":
		left, err := toFloat64(leftVal)
		if err != nil {
			return nil, err
		}
		right, err := toFloat64(rightVal)
		if err != nil {
			return nil, err
		}
		return left >= right, nil

	case "&&":
		left, err := toBool(leftVal)
		if err != nil {
			return nil, err
		}
		if !left {
			return false, nil
		}
		right, err := toBool(rightVal)
		if err != nil {
			return nil, err
		}
		return right, nil

	case "||":
		left, err := toBool(leftVal)
		if err != nil {
			return nil, err
		}
		if left {
			return true, nil
		}
		right, err := toBool(rightVal)
		if err != nil {
			return nil, err
		}
		return right, nil

	default:
		return nil, fmt.Errorf("unknown operator: %s", e.operator)
	}
}

func (e *BinaryExpression) String() string {
	return fmt.Sprintf("(%s %s %s)", e.left.String(), e.operator, e.right.String())
}

func (e *BinaryExpression) Type() ExpressionType {
	switch e.operator {
	case "+", "-", "*", "/", "%":
		return TypeNumber
	case "==", "!=", "<", "<=", ">", ">=", "&&", "||":
		return TypeBoolean
	default:
		return TypeUnknown
	}
}

// TernaryExpression represents a ternary conditional expression (condition ? true : false)
type TernaryExpression struct {
	condition Expression
	trueExpr  Expression
	falseExpr Expression
}

func (e *TernaryExpression) Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error) {
	condVal, err := e.condition.Evaluate(ctx, resolver)
	if err != nil {
		return nil, err
	}

	cond, err := toBool(condVal)
	if err != nil {
		return nil, err
	}

	if cond {
		return e.trueExpr.Evaluate(ctx, resolver)
	}
	return e.falseExpr.Evaluate(ctx, resolver)
}

func (e *TernaryExpression) String() string {
	return fmt.Sprintf("(%s ? %s : %s)", e.condition.String(), e.trueExpr.String(), e.falseExpr.String())
}

func (e *TernaryExpression) Type() ExpressionType {
	return TypeUnknown
}

// FunctionExpression represents a function call
type FunctionExpression struct {
	name string
	args []Expression
}

func (e *FunctionExpression) Evaluate(ctx context.Context, resolver VariableResolver) (interface{}, error) {
	// Evaluate arguments
	argVals := make([]interface{}, len(e.args))
	for i, arg := range e.args {
		val, err := arg.Evaluate(ctx, resolver)
		if err != nil {
			return nil, err
		}
		argVals[i] = val
	}

	// Execute function
	switch strings.ToLower(e.name) {
	case "min":
		if len(argVals) < 2 {
			return nil, fmt.Errorf("min requires at least 2 arguments")
		}
		min, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		for i := 1; i < len(argVals); i++ {
			val, err := toFloat64(argVals[i])
			if err != nil {
				return nil, err
			}
			if val < min {
				min = val
			}
		}
		return min, nil

	case "max":
		if len(argVals) < 2 {
			return nil, fmt.Errorf("max requires at least 2 arguments")
		}
		max, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		for i := 1; i < len(argVals); i++ {
			val, err := toFloat64(argVals[i])
			if err != nil {
				return nil, err
			}
			if val > max {
				max = val
			}
		}
		return max, nil

	case "abs":
		if len(argVals) != 1 {
			return nil, fmt.Errorf("abs requires exactly 1 argument")
		}
		val, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		return math.Abs(val), nil

	case "round":
		if len(argVals) < 1 || len(argVals) > 2 {
			return nil, fmt.Errorf("round requires 1 or 2 arguments")
		}
		val, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		precision := 0.0
		if len(argVals) == 2 {
			precision, err = toFloat64(argVals[1])
			if err != nil {
				return nil, err
			}
		}
		multiplier := math.Pow(10, precision)
		return math.Round(val*multiplier) / multiplier, nil

	case "floor":
		if len(argVals) != 1 {
			return nil, fmt.Errorf("floor requires exactly 1 argument")
		}
		val, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		return math.Floor(val), nil

	case "ceil":
		if len(argVals) != 1 {
			return nil, fmt.Errorf("ceil requires exactly 1 argument")
		}
		val, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		return math.Ceil(val), nil

	case "pow":
		if len(argVals) != 2 {
			return nil, fmt.Errorf("pow requires exactly 2 arguments")
		}
		base, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		exp, err := toFloat64(argVals[1])
		if err != nil {
			return nil, err
		}
		return math.Pow(base, exp), nil

	case "sqrt":
		if len(argVals) != 1 {
			return nil, fmt.Errorf("sqrt requires exactly 1 argument")
		}
		val, err := toFloat64(argVals[0])
		if err != nil {
			return nil, err
		}
		if val < 0 {
			return nil, fmt.Errorf("sqrt of negative number")
		}
		return math.Sqrt(val), nil

	case "if":
		if len(argVals) != 3 {
			return nil, fmt.Errorf("if requires exactly 3 arguments")
		}
		cond, err := toBool(argVals[0])
		if err != nil {
			return nil, err
		}
		if cond {
			return argVals[1], nil
		}
		return argVals[2], nil

	case "days":
		// Calculate days between two dates
		if len(argVals) != 2 {
			return nil, fmt.Errorf("days requires exactly 2 arguments")
		}
		// This would need date parsing logic
		// For now, return a placeholder
		return 0.0, fmt.Errorf("date functions not yet implemented")

	default:
		return nil, fmt.Errorf("unknown function: %s", e.name)
	}
}

func (e *FunctionExpression) String() string {
	args := make([]string, len(e.args))
	for i, arg := range e.args {
		args[i] = arg.String()
	}
	return fmt.Sprintf("%s(%s)", e.name, strings.Join(args, ", "))
}

func (e *FunctionExpression) Type() ExpressionType {
	switch strings.ToLower(e.name) {
	case "min", "max", "abs", "round", "floor", "ceil", "pow", "sqrt", "days":
		return TypeNumber
	case "if":
		return TypeUnknown
	default:
		return TypeUnknown
	}
}

// Helper function to compare equality
func compareEqual(left, right interface{}) bool {
	// Try numeric comparison first
	leftNum, leftErr := toFloat64(left)
	rightNum, rightErr := toFloat64(right)
	if leftErr == nil && rightErr == nil {
		return leftNum == rightNum
	}

	// Try string comparison
	leftStr := toString(left)
	rightStr := toString(right)
	return leftStr == rightStr
}

// parseDateString attempts to parse a date string
func parseDateString(s string) (time.Time, error) {
	formats := []string{
		"2006-01-02",
		"2006-01-02 15:04:05",
		time.RFC3339,
	}

	for _, format := range formats {
		if t, err := time.Parse(format, s); err == nil {
			return t, nil
		}
	}

	return time.Time{}, fmt.Errorf("cannot parse date: %s", s)
}
