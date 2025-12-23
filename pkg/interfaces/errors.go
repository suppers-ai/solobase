package interfaces

import "errors"

// Common errors used across interfaces
var (
	// ErrNotImplemented indicates a feature is not implemented
	ErrNotImplemented = errors.New("not implemented")

	// ErrNotSupported indicates a feature is not supported in this runtime
	ErrNotSupported = errors.New("not supported in this runtime")

	// ErrInvalidCredentials indicates invalid authentication credentials
	ErrInvalidCredentials = errors.New("invalid credentials")

	// ErrUserNotFound indicates the user was not found
	ErrUserNotFound = errors.New("user not found")

	// ErrUserExists indicates the user already exists
	ErrUserExists = errors.New("user already exists")

	// ErrTokenExpired indicates the token has expired
	ErrTokenExpired = errors.New("token expired")

	// ErrTokenInvalid indicates the token is invalid
	ErrTokenInvalid = errors.New("invalid token")

	// ErrBucketNotFound indicates the storage bucket was not found
	ErrBucketNotFound = errors.New("bucket not found")

	// ErrBucketExists indicates the storage bucket already exists
	ErrBucketExists = errors.New("bucket already exists")

	// ErrObjectNotFound indicates the storage object was not found
	ErrObjectNotFound = errors.New("object not found")

	// ErrConnectionFailed indicates a connection failure
	ErrConnectionFailed = errors.New("connection failed")

	// ErrTransactionFailed indicates a transaction failure
	ErrTransactionFailed = errors.New("transaction failed")
)
