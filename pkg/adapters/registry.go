// Package adapters provides runtime-specific implementations of interfaces.
// Use build tags to select the appropriate adapters for each target:
//   - Standard (default): Full implementations using stdlib and external libs
//   - WASM (//go:build wasm): TinyGo-compatible implementations
package adapters

import (
	"fmt"
	"sync"

	"github.com/suppers-ai/solobase/pkg/adapters/repos"
	"github.com/suppers-ai/solobase/pkg/interfaces"
)

// Registry holds all registered adapters
type Registry struct {
	mu sync.RWMutex

	database       interfaces.Database
	storage        interfaces.Storage
	jwtSigner      interfaces.JWTSigner
	tokenGenerator interfaces.TokenGenerator
	httpServer     interfaces.HTTPServer
	httpClient     interfaces.HTTPClient
	logger         interfaces.Logger
	repos          repos.RepositoryFactory
}

// Global default registry
var defaultRegistry = &Registry{}

// Default returns the default adapter registry
func Default() *Registry {
	return defaultRegistry
}

// NewRegistry creates a new adapter registry
func NewRegistry() *Registry {
	return &Registry{}
}

// Database returns the registered database adapter
func (r *Registry) Database() interfaces.Database {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.database
}

// SetDatabase sets the database adapter
func (r *Registry) SetDatabase(db interfaces.Database) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.database = db
}

// Storage returns the registered storage adapter
func (r *Registry) Storage() interfaces.Storage {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.storage
}

// SetStorage sets the storage adapter
func (r *Registry) SetStorage(s interfaces.Storage) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.storage = s
}

// JWTSigner returns the registered JWT signer
func (r *Registry) JWTSigner() interfaces.JWTSigner {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.jwtSigner
}

// SetJWTSigner sets the JWT signer adapter
func (r *Registry) SetJWTSigner(s interfaces.JWTSigner) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.jwtSigner = s
}

// TokenGenerator returns the registered token generator
func (r *Registry) TokenGenerator() interfaces.TokenGenerator {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.tokenGenerator
}

// SetTokenGenerator sets the token generator adapter
func (r *Registry) SetTokenGenerator(g interfaces.TokenGenerator) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.tokenGenerator = g
}

// HTTPServer returns the registered HTTP server
func (r *Registry) HTTPServer() interfaces.HTTPServer {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.httpServer
}

// SetHTTPServer sets the HTTP server adapter
func (r *Registry) SetHTTPServer(s interfaces.HTTPServer) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.httpServer = s
}

// HTTPClient returns the registered HTTP client
func (r *Registry) HTTPClient() interfaces.HTTPClient {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.httpClient
}

// SetHTTPClient sets the HTTP client adapter
func (r *Registry) SetHTTPClient(c interfaces.HTTPClient) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.httpClient = c
}

// Logger returns the registered logger
func (r *Registry) Logger() interfaces.Logger {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.logger
}

// SetLogger sets the logger adapter
func (r *Registry) SetLogger(l interfaces.Logger) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.logger = l
}

// Repos returns the registered repository factory
func (r *Registry) Repos() repos.RepositoryFactory {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.repos
}

// SetRepos sets the repository factory adapter
func (r *Registry) SetRepos(rf repos.RepositoryFactory) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.repos = rf
}

// Validate ensures all required adapters are registered
func (r *Registry) Validate() error {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var missing []string

	if r.repos == nil {
		missing = append(missing, "repos")
	}
	if r.jwtSigner == nil {
		missing = append(missing, "jwtSigner")
	}

	if len(missing) > 0 {
		return fmt.Errorf("missing required adapters: %v", missing)
	}

	return nil
}

// Global helper functions for convenience

// GetDatabase returns the database from the default registry
func GetDatabase() interfaces.Database {
	return defaultRegistry.Database()
}

// SetDatabase sets the database in the default registry
func SetDatabase(db interfaces.Database) {
	defaultRegistry.SetDatabase(db)
}

// GetStorage returns the storage from the default registry
func GetStorage() interfaces.Storage {
	return defaultRegistry.Storage()
}

// SetStorage sets the storage in the default registry
func SetStorage(s interfaces.Storage) {
	defaultRegistry.SetStorage(s)
}

// GetJWTSigner returns the JWT signer from the default registry
func GetJWTSigner() interfaces.JWTSigner {
	return defaultRegistry.JWTSigner()
}

// SetJWTSigner sets the JWT signer in the default registry
func SetJWTSigner(s interfaces.JWTSigner) {
	defaultRegistry.SetJWTSigner(s)
}

// GetTokenGenerator returns the token generator from the default registry
func GetTokenGenerator() interfaces.TokenGenerator {
	return defaultRegistry.TokenGenerator()
}

// SetTokenGenerator sets the token generator in the default registry
func SetTokenGenerator(g interfaces.TokenGenerator) {
	defaultRegistry.SetTokenGenerator(g)
}

// GetHTTPServer returns the HTTP server from the default registry
func GetHTTPServer() interfaces.HTTPServer {
	return defaultRegistry.HTTPServer()
}

// SetHTTPServer sets the HTTP server in the default registry
func SetHTTPServer(s interfaces.HTTPServer) {
	defaultRegistry.SetHTTPServer(s)
}

// GetHTTPClient returns the HTTP client from the default registry
func GetHTTPClient() interfaces.HTTPClient {
	return defaultRegistry.HTTPClient()
}

// SetHTTPClient sets the HTTP client in the default registry
func SetHTTPClient(c interfaces.HTTPClient) {
	defaultRegistry.SetHTTPClient(c)
}

// GetLogger returns the logger from the default registry
func GetLogger() interfaces.Logger {
	return defaultRegistry.Logger()
}

// SetLogger sets the logger in the default registry
func SetLogger(l interfaces.Logger) {
	defaultRegistry.SetLogger(l)
}

// GetRepos returns the repository factory from the default registry
func GetRepos() repos.RepositoryFactory {
	return defaultRegistry.Repos()
}

// SetRepos sets the repository factory in the default registry
func SetRepos(rf repos.RepositoryFactory) {
	defaultRegistry.SetRepos(rf)
}
