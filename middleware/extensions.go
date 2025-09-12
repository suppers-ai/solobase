package middleware

import (
	"context"
	"net/http"

	"github.com/suppers-ai/solobase/extensions/core"
)

type contextKey string

const ExtensionRegistryKey contextKey = "extension_registry"

// WithExtensions adds the extension registry to the request context
func WithExtensions(registry *core.ExtensionRegistry) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := context.WithValue(r.Context(), ExtensionRegistryKey, registry)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// GetExtensionRegistry retrieves the extension registry from the context
func GetExtensionRegistry(ctx context.Context) *core.ExtensionRegistry {
	if registry, ok := ctx.Value(ExtensionRegistryKey).(*core.ExtensionRegistry); ok {
		return registry
	}
	return nil
}
