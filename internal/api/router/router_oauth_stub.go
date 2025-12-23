//go:build wasm || tinygo

package router

import (
	"github.com/gorilla/mux"
)

// setupOAuthRoutes is a no-op for WASM/TinyGo builds (OAuth not supported)
func (a *API) setupOAuthRoutes(router *mux.Router) {
	// OAuth is not available in WASM/TinyGo builds
}
