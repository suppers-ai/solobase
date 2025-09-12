package core

import (
	"fmt"
	"gorm.io/gorm"
	"gorm.io/gorm/schema"
)

// ExtensionNamer implements schema.Namer to add extension prefix to table names
type ExtensionNamer struct {
	extensionName string
	defaultNamer  schema.Namer
}

// NewExtensionNamer creates a new namer with extension prefix
func NewExtensionNamer(extensionName string) *ExtensionNamer {
	return &ExtensionNamer{
		extensionName: extensionName,
		defaultNamer:  schema.NamingStrategy{},
	}
}

// TableName generates table name with extension prefix
func (n *ExtensionNamer) TableName(str string) string {
	// Use the default namer to get the base table name
	baseName := n.defaultNamer.TableName(str)
	// Add the extension prefix
	return fmt.Sprintf("ext_%s_%s", n.extensionName, baseName)
}

// ColumnName delegates to default namer
func (n *ExtensionNamer) ColumnName(table, column string) string {
	return n.defaultNamer.ColumnName(table, column)
}

// JoinTableName delegates to default namer with prefix
func (n *ExtensionNamer) JoinTableName(str string) string {
	baseName := n.defaultNamer.JoinTableName(str)
	return fmt.Sprintf("ext_%s_%s", n.extensionName, baseName)
}

// RelationshipFKName delegates to default namer
func (n *ExtensionNamer) RelationshipFKName(rel schema.Relationship) string {
	return n.defaultNamer.RelationshipFKName(rel)
}

// CheckerName delegates to default namer
func (n *ExtensionNamer) CheckerName(table, column string) string {
	return n.defaultNamer.CheckerName(table, column)
}

// IndexName delegates to default namer
func (n *ExtensionNamer) IndexName(table, column string) string {
	return n.defaultNamer.IndexName(table, column)
}

// SchemaName delegates to default namer
func (n *ExtensionNamer) SchemaName(table string) string {
	return n.defaultNamer.SchemaName(table)
}

// tableNameSetter is an interface for models that can set their table name
type tableNameSetter interface {
	TableName() string
}

// ExtensionAutoMigrate performs auto-migration with extension-specific table prefix
// This is now deprecated since we use TableName methods directly
func ExtensionAutoMigrate(db *gorm.DB, extensionName string, models ...interface{}) error {
	// Just use regular AutoMigrate since models now have TableName methods with prefixes
	return db.AutoMigrate(models...)
}
