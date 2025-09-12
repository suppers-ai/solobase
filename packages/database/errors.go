package database

import "errors"

var (
	// ErrUnsupportedDriver is returned when an unsupported database driver is specified
	ErrUnsupportedDriver = errors.New("unsupported database driver")

	// ErrNoConnection is returned when database operations are attempted without a connection
	ErrNoConnection = errors.New("no database connection established")

	// ErrTransactionFailed is returned when a transaction operation fails
	ErrTransactionFailed = errors.New("transaction operation failed")

	// ErrQueryFailed is returned when a query execution fails
	ErrQueryFailed = errors.New("query execution failed")

	// ErrConnectionFailed is returned when connection to database fails
	ErrConnectionFailed = errors.New("failed to connect to database")
)
