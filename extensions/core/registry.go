package core

import (
	"context"
	"fmt"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/logger"
)

// ExtensionRegistry manages all extensions and their lifecycle
type ExtensionRegistry struct {
	mu         sync.RWMutex
	extensions map[string]Extension
	hooks      map[HookType][]HookRegistration
	routes     []RouteRegistration
	middleware []MiddlewareRegistration
	logger     logger.Logger
	services   *ExtensionServices

	// Runtime state
	enabled map[string]bool
	status  map[string]*ExtensionStatus
	metrics map[string]*ExtensionMetrics

	// Error handling
	errorHandler ExtensionErrorHandler
	panicHandler ExtensionPanicHandler
}

// NewExtensionRegistry creates a new extension registry
func NewExtensionRegistry(logger logger.Logger, services *ExtensionServices) *ExtensionRegistry {
	return &ExtensionRegistry{
		extensions:   make(map[string]Extension),
		hooks:        make(map[HookType][]HookRegistration),
		routes:       make([]RouteRegistration, 0),
		middleware:   make([]MiddlewareRegistration, 0),
		enabled:      make(map[string]bool),
		status:       make(map[string]*ExtensionStatus),
		metrics:      make(map[string]*ExtensionMetrics),
		logger:       logger,
		services:     services,
		errorHandler: defaultErrorHandler(logger),
		panicHandler: defaultPanicHandler(logger),
	}
}

// Register registers a new extension
func (r *ExtensionRegistry) Register(ext Extension) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	metadata := ext.Metadata()

	// Check if extension already registered
	if _, exists := r.extensions[metadata.Name]; exists {
		return fmt.Errorf("extension %s already registered", metadata.Name)
	}

	// Validate version compatibility
	if err := r.validateCompatibility(metadata); err != nil {
		return fmt.Errorf("extension %s compatibility check failed: %w", metadata.Name, err)
	}

	// Store extension
	r.extensions[metadata.Name] = ext
	r.status[metadata.Name] = &ExtensionStatus{
		Name:    metadata.Name,
		Version: metadata.Version,
		State:   "disabled",
		Enabled: false,
		Loaded:  false,
		Health: &HealthStatus{
			Status:      "uninitialized",
			LastChecked: time.Now(),
		},
		Resources: ExtensionResources{},
		Endpoints: []EndpointInfo{},
		Metrics:   ExtensionMetrics{},
	}

	r.logger.Info(context.Background(), "Extension registered: "+metadata.Name+" v"+metadata.Version)

	return nil
}

// Enable enables an extension
func (r *ExtensionRegistry) Enable(name string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	ext, exists := r.extensions[name]
	if !exists {
		return fmt.Errorf("extension %s not found", name)
	}

	if r.enabled[name] {
		return fmt.Errorf("extension %s already enabled", name)
	}

	// Initialize the extension
	ctx := context.Background()
	if err := r.initializeExtension(ctx, ext); err != nil {
		return fmt.Errorf("failed to initialize extension %s: %w", name, err)
	}

	// Start the extension
	if err := ext.Start(ctx); err != nil {
		return fmt.Errorf("failed to start extension %s: %w", name, err)
	}

	// Update health status
	health, _ := ext.Health(ctx)
	if health == nil {
		health = &HealthStatus{
			Status:      "healthy",
			LastChecked: time.Now(),
		}
	}

	r.enabled[name] = true
	r.status[name].State = "enabled"
	r.status[name].Enabled = true
	r.status[name].Loaded = true
	r.status[name].Health = health
	now := time.Now()
	r.status[name].LoadedAt = now
	r.status[name].EnabledAt = &now

	r.logger.Info(ctx, "Extension enabled: "+name)

	return nil
}

// Disable disables an extension
func (r *ExtensionRegistry) Disable(name string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	ext, exists := r.extensions[name]
	if !exists {
		return fmt.Errorf("extension %s not found", name)
	}

	if !r.enabled[name] {
		return fmt.Errorf("extension %s not enabled", name)
	}

	// Stop the extension
	ctx := context.Background()
	if err := ext.Stop(ctx); err != nil {
		r.logger.Error(ctx, "Failed to stop extension cleanly: "+name)
	}

	// Remove registered routes, hooks, etc.
	r.unregisterExtensionResources(name)

	r.enabled[name] = false
	r.status[name].State = "disabled"
	r.status[name].Enabled = false
	r.status[name].Loaded = false

	r.logger.Info(ctx, "Extension disabled: "+name)

	return nil
}

// Unregister completely removes an extension from the registry
func (r *ExtensionRegistry) Unregister(name string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	// Check if extension exists
	if _, exists := r.extensions[name]; !exists {
		return fmt.Errorf("extension %s not found", name)
	}

	// Disable if enabled
	if r.enabled[name] {
		// Need to unlock before calling Disable to avoid deadlock
		r.mu.Unlock()
		r.Disable(name)
		r.mu.Lock()
	}

	// Remove extension resources
	r.unregisterExtensionResources(name)

	// Remove from registry
	delete(r.extensions, name)
	delete(r.enabled, name)
	delete(r.status, name)
	delete(r.metrics, name)

	r.logger.Info(context.Background(), "Extension unregistered: "+name)

	return nil
}

// Get returns an extension by name
func (r *ExtensionRegistry) Get(name string) (Extension, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	ext, exists := r.extensions[name]
	return ext, exists
}

// List returns metadata for all registered extensions
func (r *ExtensionRegistry) List() []ExtensionMetadata {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]ExtensionMetadata, 0, len(r.extensions))
	for _, ext := range r.extensions {
		result = append(result, ext.Metadata())
	}

	return result
}

// GetStatus returns the status of an extension
func (r *ExtensionRegistry) GetStatus(name string) (*ExtensionStatus, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	status, exists := r.status[name]
	if !exists {
		return nil, fmt.Errorf("extension %s not found", name)
	}

	// Create a copy to avoid race conditions
	statusCopy := *status
	return &statusCopy, nil
}

// GetAll returns all registered extensions
func (r *ExtensionRegistry) GetAll() []Extension {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]Extension, 0, len(r.extensions))
	for _, ext := range r.extensions {
		result = append(result, ext)
	}

	return result
}

// GetMetrics returns metrics for an extension
func (r *ExtensionRegistry) GetMetrics(name string) (*ExtensionMetrics, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	metrics, exists := r.metrics[name]
	if !exists {
		return nil, fmt.Errorf("extension %s not found", name)
	}

	// Create a copy to avoid race conditions
	metricsCopy := *metrics
	return &metricsCopy, nil
}

// RegisterRoutes registers routes with the provided router
func (r *ExtensionRegistry) RegisterRoutes(router *mux.Router) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	for _, route := range r.routes {
		// Wrap handler with panic recovery
		wrappedHandler := r.wrapExtensionHandler(route.Extension, route.Handler)

		// Register the route
		router.
			Path(route.Path).
			Methods(route.Methods...).
			Handler(wrappedHandler)

		r.logger.Debug(context.Background(), "Registered extension route")
	}
}

// ApplyMiddleware applies all registered middleware to a handler
func (r *ExtensionRegistry) ApplyMiddleware(handler http.Handler) http.Handler {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Apply middleware in reverse order (so first registered middleware runs first)
	for i := len(r.middleware) - 1; i >= 0; i-- {
		mw := r.middleware[i]
		handler = mw.Handler(handler)
	}

	return handler
}

// ExecuteHooks executes hooks of the specified type
func (r *ExtensionRegistry) ExecuteHooks(ctx context.Context, hookType HookType, hookCtx *HookContext) error {
	r.mu.RLock()
	hooks := r.hooks[hookType]
	r.mu.RUnlock()

	for _, hook := range hooks {
		// Check if hook should apply to this path
		if len(hook.Paths) > 0 {
			if hookCtx.Request == nil {
				continue
			}
			if !r.matchPath(hookCtx.Request.URL.Path, hook.Paths) {
				continue
			}
		}

		// Execute hook with panic recovery
		if err := r.executeHookSafely(ctx, hook, hookCtx); err != nil {
			r.logger.Error(ctx, fmt.Sprintf("Hook execution failed: %s/%s (%s)", hook.Extension, hook.Name, string(hook.Type)))
			// Continue with other hooks even if one fails
		}
	}

	return nil
}

// initializeExtension initializes an extension
func (r *ExtensionRegistry) initializeExtension(ctx context.Context, ext Extension) error {
	metadata := ext.Metadata()

	// Create extension-specific services
	extServices := r.services.ForExtension(metadata.Name)

	// Initialize the extension
	if err := ext.Initialize(ctx, extServices); err != nil {
		return err
	}

	// Register routes
	router := NewExtensionRouter(metadata.Name, r)
	if err := ext.RegisterRoutes(router); err != nil {
		return fmt.Errorf("failed to register routes: %w", err)
	}

	// Add registered routes to the registry
	r.routes = append(r.routes, router.Routes...)

	// Register middleware
	middleware := ext.RegisterMiddleware()
	r.middleware = append(r.middleware, middleware...)

	// Register hooks
	hooks := ext.RegisterHooks()
	for _, hook := range hooks {
		if r.hooks[hook.Type] == nil {
			r.hooks[hook.Type] = []HookRegistration{}
		}
		r.hooks[hook.Type] = append(r.hooks[hook.Type], hook)
	}

	// Update resource count
	r.status[metadata.Name].Resources = ExtensionResources{
		Routes:     len(router.Routes),
		Middleware: len(middleware),
		Hooks:      len(hooks),
		Templates:  len(ext.RegisterTemplates()),
		Assets:     len(ext.RegisterStaticAssets()),
	}

	return nil
}

// unregisterExtensionResources removes all resources registered by an extension
func (r *ExtensionRegistry) unregisterExtensionResources(name string) {
	// Remove routes
	newRoutes := []RouteRegistration{}
	for _, route := range r.routes {
		if route.Extension != name {
			newRoutes = append(newRoutes, route)
		}
	}
	r.routes = newRoutes

	// Remove middleware
	newMiddleware := []MiddlewareRegistration{}
	for _, mw := range r.middleware {
		if mw.Extension != name {
			newMiddleware = append(newMiddleware, mw)
		}
	}
	r.middleware = newMiddleware

	// Remove hooks
	for hookType, hooks := range r.hooks {
		newHooks := []HookRegistration{}
		for _, hook := range hooks {
			if hook.Extension != name {
				newHooks = append(newHooks, hook)
			}
		}
		r.hooks[hookType] = newHooks
	}
}

// validateCompatibility checks if an extension is compatible with the current version
func (r *ExtensionRegistry) validateCompatibility(metadata ExtensionMetadata) error {
	// TODO: Implement version checking against current Solobase version
	// For now, always return success
	return nil
}

// wrapExtensionHandler wraps an extension handler with panic recovery
func (r *ExtensionRegistry) wrapExtensionHandler(extension string, handler http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		defer func() {
			if recovered := recover(); recovered != nil {
				r.panicHandler(extension, recovered)

				// Disable the extension
				r.Disable(extension)

				// Return error response
				http.Error(w, "Extension error", http.StatusInternalServerError)
			}
		}()

		// Track metrics
		start := time.Now()

		// Create wrapped response writer to capture status
		wrapped := &responseWriter{ResponseWriter: w, statusCode: http.StatusOK}

		// Execute handler
		handler.ServeHTTP(wrapped, req)

		// Update metrics
		r.updateMetrics(extension, time.Since(start), wrapped.statusCode)
	})
}

// executeHookSafely executes a hook with panic recovery
func (r *ExtensionRegistry) executeHookSafely(ctx context.Context, hook HookRegistration, hookCtx *HookContext) (err error) {
	defer func() {
		if recovered := recover(); recovered != nil {
			r.panicHandler(hook.Extension, recovered)
			err = fmt.Errorf("hook panic: %v", recovered)
		}
	}()

	return hook.Handler(ctx, hookCtx)
}

// matchPath checks if a path matches any of the patterns
func (r *ExtensionRegistry) matchPath(path string, patterns []string) bool {
	for _, pattern := range patterns {
		// Simple prefix matching for now
		// TODO: Implement more sophisticated pattern matching
		if len(pattern) > 0 && path[:len(pattern)] == pattern {
			return true
		}
	}
	return false
}

// updateMetrics updates metrics for an extension
func (r *ExtensionRegistry) updateMetrics(extension string, latency time.Duration, statusCode int) {
	r.mu.Lock()
	defer r.mu.Unlock()

	metrics, exists := r.metrics[extension]
	if !exists {
		metrics = &ExtensionMetrics{}
		r.metrics[extension] = metrics
	}

	metrics.RequestCount++
	if statusCode >= 400 {
		metrics.ErrorCount++
	}

	// Update average latency (simple moving average)
	if metrics.AverageLatency == 0 {
		metrics.AverageLatency = latency
	} else {
		metrics.AverageLatency = (metrics.AverageLatency + latency) / 2
	}

	// Update P95 and P99 (simplified - in production use proper percentile tracking)
	if latency > metrics.P95Latency {
		metrics.P99Latency = metrics.P95Latency
		metrics.P95Latency = latency
	}
}

// SetErrorHandler sets the error handler for extensions
func (r *ExtensionRegistry) SetErrorHandler(handler ExtensionErrorHandler) {
	r.errorHandler = handler
}

// SetPanicHandler sets the panic handler for extensions
func (r *ExtensionRegistry) SetPanicHandler(handler ExtensionPanicHandler) {
	r.panicHandler = handler
}

// defaultErrorHandler returns the default error handler
func defaultErrorHandler(log logger.Logger) ExtensionErrorHandler {
	return func(err *ExtensionError) {
		log.Error(context.Background(), fmt.Sprintf("Extension error [%s/%s]: %s", err.Extension, err.Type, err.Message))
	}
}

// defaultPanicHandler returns the default panic handler
func defaultPanicHandler(log logger.Logger) ExtensionPanicHandler {
	return func(extension string, recovered interface{}) {
		log.Error(context.Background(), fmt.Sprintf("Extension panic recovered [%s]: %v", extension, recovered))
	}
}

// responseWriter wraps http.ResponseWriter to capture status code
type responseWriter struct {
	http.ResponseWriter
	statusCode int
	written    bool
}

func (w *responseWriter) WriteHeader(statusCode int) {
	if !w.written {
		w.statusCode = statusCode
		w.written = true
		w.ResponseWriter.WriteHeader(statusCode)
	}
}

func (w *responseWriter) Write(b []byte) (int, error) {
	if !w.written {
		w.written = true
	}
	return w.ResponseWriter.Write(b)
}
