package core

import (
	"fmt"
	"net/http"
	"strings"

	"github.com/gorilla/mux"
)

// ExtensionRouter provides a safe interface for extensions to register routes
type ExtensionRouter interface {
	HandleFunc(path string, handler http.HandlerFunc) RouteRegistration
	Handle(path string, handler http.Handler) RouteRegistration
	PathPrefix(prefix string) ExtensionRouter
	Use(middleware ...mux.MiddlewareFunc)

	// Restricted methods that require permissions
	RequireAuth(handler http.Handler) http.Handler
	RequireRole(role string, handler http.Handler) http.Handler
}

// extensionRouter implements ExtensionRouter
type extensionRouter struct {
	extension string
	registry  *ExtensionRegistry
	prefix    string
	Routes    []RouteRegistration
}

// NewExtensionRouter creates a new extension router
func NewExtensionRouter(extension string, registry *ExtensionRegistry) *extensionRouter {
	return &extensionRouter{
		extension: extension,
		registry:  registry,
		prefix:    fmt.Sprintf("/ext/%s", extension),
		Routes:    []RouteRegistration{},
	}
}

// HandleFunc registers a handler function for a path
func (r *extensionRouter) HandleFunc(path string, handler http.HandlerFunc) RouteRegistration {
	return r.Handle(path, handler)
}

// Handle registers a handler for a path
func (r *extensionRouter) Handle(path string, handler http.Handler) RouteRegistration {
	// Ensure path is under extension prefix
	fullPath := r.ensurePrefix(path)

	registration := RouteRegistration{
		Extension: r.extension,
		Path:      fullPath,
		Methods:   []string{"GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"},
		Handler:   handler,
		Protected: false,
		Roles:     []string{},
	}

	r.Routes = append(r.Routes, registration)
	r.registry.routes = append(r.registry.routes, registration)

	return registration
}

// PathPrefix creates a subrouter with a path prefix
func (r *extensionRouter) PathPrefix(prefix string) ExtensionRouter {
	return &extensionRouter{
		extension: r.extension,
		registry:  r.registry,
		prefix:    r.ensurePrefix(prefix),
		Routes:    r.Routes,
	}
}

// Use applies middleware to the router
func (r *extensionRouter) Use(middleware ...mux.MiddlewareFunc) {
	for _, mw := range middleware {
		r.registry.middleware = append(r.registry.middleware, MiddlewareRegistration{
			Extension: r.extension,
			Name:      fmt.Sprintf("%s-middleware", r.extension),
			Priority:  50, // Default priority
			Handler:   mw,
			Paths:     []string{r.prefix},
		})
	}
}

// RequireAuth wraps a handler to require authentication
func (r *extensionRouter) RequireAuth(handler http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		// Check if user is authenticated
		userID := req.Context().Value("user_id")
		if userID == nil {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		handler.ServeHTTP(w, req)
	})
}

// RequireRole wraps a handler to require a specific role
func (r *extensionRouter) RequireRole(role string, handler http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
		// Check if user has the required role
		userRole := req.Context().Value("user_role")
		if userRole == nil || userRole.(string) != role {
			http.Error(w, "Forbidden", http.StatusForbidden)
			return
		}

		handler.ServeHTTP(w, req)
	})
}

// ensurePrefix ensures the path has the extension prefix
func (r *extensionRouter) ensurePrefix(path string) string {
	// Remove leading slash if present
	path = strings.TrimPrefix(path, "/")

	// If path doesn't start with ext prefix, add it
	if !strings.HasPrefix(path, fmt.Sprintf("ext/%s", r.extension)) {
		if path == "" {
			return r.prefix
		}
		return fmt.Sprintf("%s/%s", r.prefix, path)
	}

	return "/" + path
}
