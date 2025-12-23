//go:build !wasm && !tinygo

package router

import (
	"log"

	"github.com/gorilla/mux"
	auth "github.com/suppers-ai/solobase/internal/api/handlers/auth"
)

// setupOAuthRoutes sets up OAuth authentication routes
func (a *API) setupOAuthRoutes(router *mux.Router) {
	// Get OAuth configuration from environment
	oauthManager := auth.NewOAuthManager(a.AuthService, a.IAMService, getBaseURL())

	// Register Google OAuth if configured
	googleClientID := getEnv("GOOGLE_CLIENT_ID", "")
	googleClientSecret := getEnv("GOOGLE_CLIENT_SECRET", "")
	if googleClientID != "" && googleClientSecret != "" {
		if err := oauthManager.RegisterProvider("google", googleClientID, googleClientSecret, nil); err != nil {
			log.Printf("Warning: Failed to register Google OAuth: %v", err)
		}
	}

	// Register Microsoft OAuth if configured
	microsoftClientID := getEnv("MICROSOFT_CLIENT_ID", "")
	microsoftClientSecret := getEnv("MICROSOFT_CLIENT_SECRET", "")
	if microsoftClientID != "" && microsoftClientSecret != "" {
		if err := oauthManager.RegisterProvider("microsoft", microsoftClientID, microsoftClientSecret, nil); err != nil {
			log.Printf("Warning: Failed to register Microsoft OAuth: %v", err)
		}
	}

	// Register Facebook OAuth if configured
	facebookClientID := getEnv("FACEBOOK_CLIENT_ID", "")
	facebookClientSecret := getEnv("FACEBOOK_CLIENT_SECRET", "")
	if facebookClientID != "" && facebookClientSecret != "" {
		if err := oauthManager.RegisterProvider("facebook", facebookClientID, facebookClientSecret, nil); err != nil {
			log.Printf("Warning: Failed to register Facebook OAuth: %v", err)
		}
	}

	// Public endpoint to get available OAuth providers
	router.HandleFunc("/auth/oauth/providers", oauthManager.HandleGetProviders()).Methods("GET", "OPTIONS")

	// OAuth login endpoint
	router.HandleFunc("/auth/oauth/login", oauthManager.HandleOAuthLogin()).Methods("GET", "OPTIONS")

	// OAuth callback endpoint
	router.HandleFunc("/auth/oauth/callback/{provider}", oauthManager.HandleOAuthCallback()).Methods("GET", "OPTIONS")
}
