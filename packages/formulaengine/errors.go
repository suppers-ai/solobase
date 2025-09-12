package formulaengine

import "fmt"

// Error types for formula engine
type ErrorType int

const (
	ErrTypeUnknown ErrorType = iota
	ErrTypeParse
	ErrTypeEvaluation
	ErrTypeVariableNotFound
	ErrTypeTypeMismatch
	ErrTypeDivisionByZero
	ErrTypeInvalidFunction
	ErrTypeInvalidArgument
)

// EngineError represents an error from the formula engine
type EngineError struct {
	Type    ErrorType
	Message string
	Formula string
	Pos     int
}

func (e *EngineError) Error() string {
	if e.Formula != "" {
		return fmt.Sprintf("%s in formula: %s", e.Message, e.Formula)
	}
	return e.Message
}

// NewParseError creates a new parse error
func NewParseError(formula string, pos int, message string) *EngineError {
	return &EngineError{
		Type:    ErrTypeParse,
		Message: message,
		Formula: formula,
		Pos:     pos,
	}
}

// NewEvaluationError creates a new evaluation error
func NewEvaluationError(message string) *EngineError {
	return &EngineError{
		Type:    ErrTypeEvaluation,
		Message: message,
	}
}

// NewVariableNotFoundError creates a new variable not found error
func NewVariableNotFoundError(varName string) *EngineError {
	return &EngineError{
		Type:    ErrTypeVariableNotFound,
		Message: fmt.Sprintf("variable '%s' not found", varName),
	}
}

// NewTypeMismatchError creates a new type mismatch error
func NewTypeMismatchError(expected, got string) *EngineError {
	return &EngineError{
		Type:    ErrTypeTypeMismatch,
		Message: fmt.Sprintf("type mismatch: expected %s, got %s", expected, got),
	}
}

// NewDivisionByZeroError creates a new division by zero error
func NewDivisionByZeroError() *EngineError {
	return &EngineError{
		Type:    ErrTypeDivisionByZero,
		Message: "division by zero",
	}
}

// NewInvalidFunctionError creates a new invalid function error
func NewInvalidFunctionError(funcName string) *EngineError {
	return &EngineError{
		Type:    ErrTypeInvalidFunction,
		Message: fmt.Sprintf("unknown function: %s", funcName),
	}
}

// NewInvalidArgumentError creates a new invalid argument error
func NewInvalidArgumentError(funcName string, expected, got int) *EngineError {
	return &EngineError{
		Type:    ErrTypeInvalidArgument,
		Message: fmt.Sprintf("function %s: expected %d arguments, got %d", funcName, expected, got),
	}
}
