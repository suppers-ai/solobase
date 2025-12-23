package core

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/internal/config"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
)

// ExtensionServices provides controlled access to core application services
type ExtensionServices struct {
	// Core services with controlled access
	db      database.Database
	auth    *auth.Service
	logger  logger.Logger
	storage *services.EnhancedStorageService
	config  *config.Config
	stats   *services.StatsService
	iam     *iam.Service

	// Extension-specific context
	extensionName string
	schemaName    string
}

// NewExtensionServices creates extension services
func NewExtensionServices(
	db database.Database,
	auth *auth.Service,
	logger logger.Logger,
	storage *services.EnhancedStorageService,
	config *config.Config,
	stats *services.StatsService,
	iamSvc *iam.Service,
) *ExtensionServices {
	return &ExtensionServices{
		db:      db,
		auth:    auth,
		logger:  logger,
		storage: storage,
		config:  config,
		stats:   stats,
		iam:     iamSvc,
	}
}

// ForExtension creates extension-specific services
func (s *ExtensionServices) ForExtension(extensionName string) *ExtensionServices {
	return &ExtensionServices{
		db:            s.db,
		auth:          s.auth,
		logger:        s.logger,
		storage:       s.storage,
		config:        s.config,
		stats:         s.stats,
		iam:           s.iam,
		extensionName: extensionName,
		schemaName:    fmt.Sprintf("ext_%s", extensionName),
	}
}

// Database returns the extension database interface
func (s *ExtensionServices) Database() ExtensionDatabase {
	return &extensionDatabase{
		db:         s.db,
		schemaName: s.schemaName,
	}
}

// Auth returns the extension auth interface
func (s *ExtensionServices) Auth() ExtensionAuth {
	return &extensionAuth{
		auth: s.auth,
	}
}

// Logger returns the extension logger
func (s *ExtensionServices) Logger() ExtensionLogger {
	return &extensionLogger{
		logger:    s.logger,
		extension: s.extensionName,
	}
}

// Storage returns the extension storage interface
func (s *ExtensionServices) Storage() ExtensionStorage {
	return &extensionStorage{
		storage:   s.storage,
		extension: s.extensionName,
	}
}

// Config returns the extension config interface
func (s *ExtensionServices) Config() ExtensionConfigInterface {
	return &extensionConfig{
		config:    s.config,
		extension: s.extensionName,
	}
}

// IAM returns the IAM service for role checking
func (s *ExtensionServices) IAM() *iam.Service {
	return s.iam
}

// ExtensionDatabase provides schema-isolated database access
type ExtensionDatabase interface {
	Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error)
	Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error)
	Transaction(ctx context.Context, fn func(ExtensionTx) error) error
}

// ExtensionTx represents a database transaction
type ExtensionTx interface {
	Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error)
	Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error)
	Commit() error
	Rollback() error
}

// extensionDatabase implements ExtensionDatabase
type extensionDatabase struct {
	db         database.Database
	schemaName string
}

func (d *extensionDatabase) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	// Prefix query with schema name
	prefixedQuery := d.prefixQuery(query)
	return d.db.Query(ctx, prefixedQuery, args...)
}

func (d *extensionDatabase) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	// Prefix query with schema name
	prefixedQuery := d.prefixQuery(query)
	return d.db.Exec(ctx, prefixedQuery, args...)
}

func (d *extensionDatabase) Transaction(ctx context.Context, fn func(ExtensionTx) error) error {
	tx, err := d.db.BeginTx(ctx)
	if err != nil {
		return err
	}

	extTx := &extensionTx{
		tx:         tx,
		schemaName: d.schemaName,
	}

	if err := fn(extTx); err != nil {
		tx.Rollback()
		return err
	}

	return tx.Commit()
}

func (d *extensionDatabase) prefixQuery(query string) string {
	// Simple schema prefixing - in production, use proper SQL parser
	// For now, set search_path
	return fmt.Sprintf("SET search_path TO %s; %s", d.schemaName, query)
}

// extensionTx implements ExtensionTx
type extensionTx struct {
	tx         database.Transaction
	schemaName string
}

func (t *extensionTx) Query(ctx context.Context, query string, args ...interface{}) (database.Rows, error) {
	prefixedQuery := t.prefixQuery(query)
	return t.tx.Query(ctx, prefixedQuery, args...)
}

func (t *extensionTx) Exec(ctx context.Context, query string, args ...interface{}) (database.Result, error) {
	prefixedQuery := t.prefixQuery(query)
	return t.tx.Exec(ctx, prefixedQuery, args...)
}

func (t *extensionTx) Commit() error {
	return t.tx.Commit()
}

func (t *extensionTx) Rollback() error {
	return t.tx.Rollback()
}

func (t *extensionTx) prefixQuery(query string) string {
	return fmt.Sprintf("SET search_path TO %s; %s", t.schemaName, query)
}

// ExtensionAuth provides controlled auth access
type ExtensionAuth interface {
	GetUser(ctx context.Context, userID string) (interface{}, error)
	ValidateToken(ctx context.Context, token string) (interface{}, error)
	CheckPermission(ctx context.Context, userID string, permission string) bool
}

// extensionAuth implements ExtensionAuth
type extensionAuth struct {
	auth *auth.Service
}

func (a *extensionAuth) GetUser(ctx context.Context, userID string) (interface{}, error) {
	return nil, fmt.Errorf("ExtensionAuth.GetUser: not available for extensions")
}

func (a *extensionAuth) ValidateToken(ctx context.Context, token string) (interface{}, error) {
	return nil, fmt.Errorf("ExtensionAuth.ValidateToken: not available for extensions")
}

func (a *extensionAuth) CheckPermission(ctx context.Context, userID string, permission string) bool {
	return false
}

// ExtensionLogger provides extension-scoped logging
type ExtensionLogger interface {
	Debug(ctx context.Context, msg string, fields ...logger.Field)
	Info(ctx context.Context, msg string, fields ...logger.Field)
	Warn(ctx context.Context, msg string, fields ...logger.Field)
	Error(ctx context.Context, msg string, fields ...logger.Field)
}

// extensionLogger implements ExtensionLogger
type extensionLogger struct {
	logger    logger.Logger
	extension string
}

func (l *extensionLogger) Debug(ctx context.Context, msg string, fields ...logger.Field) {
	fields = append(fields, logger.String("extension", l.extension))
	l.logger.Debug(ctx, msg, fields...)
}

func (l *extensionLogger) Info(ctx context.Context, msg string, fields ...logger.Field) {
	fields = append(fields, logger.String("extension", l.extension))
	l.logger.Info(ctx, msg, fields...)
}

func (l *extensionLogger) Warn(ctx context.Context, msg string, fields ...logger.Field) {
	fields = append(fields, logger.String("extension", l.extension))
	l.logger.Warn(ctx, msg, fields...)
}

func (l *extensionLogger) Error(ctx context.Context, msg string, fields ...logger.Field) {
	fields = append(fields, logger.String("extension", l.extension))
	l.logger.Error(ctx, msg, fields...)
}

// ExtensionStorage provides controlled storage access
type ExtensionStorage interface {
	Upload(ctx context.Context, bucket, path string, content []byte) error
	Download(ctx context.Context, bucket, path string) ([]byte, error)
	Delete(ctx context.Context, bucket, path string) error
	List(ctx context.Context, bucket, prefix string) ([]string, error)
}

// extensionStorage implements ExtensionStorage
type extensionStorage struct {
	storage   *services.EnhancedStorageService
	extension string
}

func (s *extensionStorage) Upload(ctx context.Context, bucket, path string, content []byte) error {
	return fmt.Errorf("ExtensionStorage.Upload: not available for extensions")
}

func (s *extensionStorage) Download(ctx context.Context, bucket, path string) ([]byte, error) {
	return nil, fmt.Errorf("ExtensionStorage.Download: not available for extensions")
}

func (s *extensionStorage) Delete(ctx context.Context, bucket, path string) error {
	return fmt.Errorf("ExtensionStorage.Delete: not available for extensions")
}

func (s *extensionStorage) List(ctx context.Context, bucket, prefix string) ([]string, error) {
	return nil, fmt.Errorf("ExtensionStorage.List: not available for extensions")
}

// ExtensionConfigInterface provides extension configuration
type ExtensionConfigInterface interface {
	Get(key string) interface{}
	GetString(key string) string
	GetInt(key string) int
	GetBool(key string) bool
	GetStringSlice(key string) []string
}

// extensionConfig implements ExtensionConfigInterface
type extensionConfig struct {
	config    *config.Config
	extension string
}

func (c *extensionConfig) Get(key string) interface{} {
	// Extension config not available - extensions should use their own config
	return nil
}

func (c *extensionConfig) GetString(key string) string {
	v := c.Get(key)
	if s, ok := v.(string); ok {
		return s
	}
	return ""
}

func (c *extensionConfig) GetInt(key string) int {
	v := c.Get(key)
	if i, ok := v.(int); ok {
		return i
	}
	return 0
}

func (c *extensionConfig) GetBool(key string) bool {
	v := c.Get(key)
	if b, ok := v.(bool); ok {
		return b
	}
	return false
}

func (c *extensionConfig) GetStringSlice(key string) []string {
	v := c.Get(key)
	if s, ok := v.([]string); ok {
		return s
	}
	return []string{}
}
