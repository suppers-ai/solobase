package storage

import "fmt"

type ErrorCode string

const (
	ErrorCodeNotFound       ErrorCode = "NOT_FOUND"
	ErrorCodeAccessDenied   ErrorCode = "ACCESS_DENIED"
	ErrorCodeInvalidConfig  ErrorCode = "INVALID_CONFIG"
	ErrorCodeNetworkError   ErrorCode = "NETWORK_ERROR"
	ErrorCodeInternalError  ErrorCode = "INTERNAL_ERROR"
	ErrorCodeInvalidRequest ErrorCode = "INVALID_REQUEST"
)

type Error struct {
	Code    ErrorCode
	Message string
	Cause   error
}

func (e *Error) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("%s: %s (cause: %v)", e.Code, e.Message, e.Cause)
	}
	return fmt.Sprintf("%s: %s", e.Code, e.Message)
}

func (e *Error) Unwrap() error {
	return e.Cause
}

func IsNotFound(err error) bool {
	if err == nil {
		return false
	}
	e, ok := err.(*Error)
	return ok && e.Code == ErrorCodeNotFound
}

func IsAccessDenied(err error) bool {
	if err == nil {
		return false
	}
	e, ok := err.(*Error)
	return ok && e.Code == ErrorCodeAccessDenied
}
