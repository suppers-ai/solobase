//go:build wasm || tinygo

package auth

import (
	"net/http"

	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/utils"
)

// OAuthProviderConfig holds configuration for an OAuth provider (stub for WASM)
type OAuthProviderConfig struct{}

// OAuthManager manages OAuth providers (stub for WASM)
type OAuthManager struct{}

// NewOAuthManager creates a new OAuth manager (stub for WASM - OAuth not available)
func NewOAuthManager(authService *services.AuthService, iamService *iam.Service, redirectBaseURL string) *OAuthManager {
	return &OAuthManager{}
}

// RegisterProvider registers an OAuth provider (stub for WASM - not available)
func (om *OAuthManager) RegisterProvider(name, clientID, clientSecret string, scopes []string) error {
	return nil // OAuth not available in WASM
}

// GetAvailableProviders returns an empty list (OAuth not available in WASM)
func (om *OAuthManager) GetAvailableProviders() []string {
	return []string{}
}

// HandleGetProviders returns empty list (OAuth not available in WASM)
func (om *OAuthManager) HandleGetProviders() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"providers": []string{},
			"message":   "OAuth is not available in WASM builds",
		})
	}
}

// HandleOAuthLogin returns error (OAuth not available in WASM)
func (om *OAuthManager) HandleOAuthLogin() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONError(w, http.StatusNotImplemented, "OAuth is not available in WASM builds")
	}
}

// HandleOAuthCallback returns error (OAuth not available in WASM)
func (om *OAuthManager) HandleOAuthCallback() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONError(w, http.StatusNotImplemented, "OAuth is not available in WASM builds")
	}
}
