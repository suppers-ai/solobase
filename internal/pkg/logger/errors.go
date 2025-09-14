package logger

import "errors"

var (
	// Configuration errors
	ErrInvalidConfig    = errors.New("invalid logger configuration")
	ErrInvalidLevel     = errors.New("invalid log level")
	ErrInvalidOutput    = errors.New("invalid output type")
	ErrDatabaseRequired = errors.New("database connection required")

	// Operation errors
	ErrNotImplemented = errors.New("feature not implemented")
	ErrWriteFailed    = errors.New("failed to write log")
	ErrFlushFailed    = errors.New("failed to flush logs")
	ErrRotationFailed = errors.New("failed to rotate log file")
	ErrQueryFailed    = errors.New("failed to query logs")

	// Database errors
	ErrDatabaseConnection = errors.New("database connection error")
	ErrMigrationFailed    = errors.New("failed to run migrations")
	ErrInsertFailed       = errors.New("failed to insert log")
)
