package extensions

import (
	"gorm.io/gorm"
)

// DatabaseWrapper wraps GORM database for extensions
type DatabaseWrapper struct {
	db *gorm.DB
}

// NewDatabaseWrapper creates a new database wrapper
func NewDatabaseWrapper(db *gorm.DB) *DatabaseWrapper {
	return &DatabaseWrapper{db: db}
}

// GetDB returns the underlying GORM database
func (w *DatabaseWrapper) GetDB() *gorm.DB {
	return w.db
}
