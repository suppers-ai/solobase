package auth

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/constants"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/utils"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/facebook"
	"golang.org/x/oauth2/google"
	"golang.org/x/oauth2/microsoft"
)

// OAuthProviderConfig holds configuration for an OAuth provider
type OAuthProviderConfig struct {
	Config      *oauth2.Config
	UserInfoURL string
}

// OAuthManager manages OAuth providers
type OAuthManager struct {
	providers       map[string]OAuthProviderConfig
	authService     *services.AuthService
	iamService      *iam.Service
	redirectBaseURL string
}

// NewOAuthManager creates a new OAuth manager
func NewOAuthManager(authService *services.AuthService, iamService *iam.Service, redirectBaseURL string) *OAuthManager {
	return &OAuthManager{
		providers:       make(map[string]OAuthProviderConfig),
		authService:     authService,
		iamService:      iamService,
		redirectBaseURL: redirectBaseURL,
	}
}

// RegisterProvider registers an OAuth provider
func (om *OAuthManager) RegisterProvider(name, clientID, clientSecret string, scopes []string) error {
	if clientID == "" || clientSecret == "" {
		return fmt.Errorf("OAuth provider %s is not configured (missing client ID or secret)", name)
	}

	redirectURL := fmt.Sprintf("%s/api/auth/oauth/callback/%s", om.redirectBaseURL, name)

	var endpoint oauth2.Endpoint
	var userInfoURL string

	switch name {
	case "google":
		endpoint = google.Endpoint
		userInfoURL = "https://www.googleapis.com/oauth2/v2/userinfo"
		if len(scopes) == 0 {
			scopes = []string{
				"https://www.googleapis.com/auth/userinfo.email",
				"https://www.googleapis.com/auth/userinfo.profile",
			}
		}
	case "microsoft":
		endpoint = microsoft.AzureADEndpoint("")
		userInfoURL = "https://graph.microsoft.com/v1.0/me"
		if len(scopes) == 0 {
			scopes = []string{"User.Read"}
		}
	case "facebook":
		endpoint = facebook.Endpoint
		userInfoURL = "https://graph.facebook.com/me?fields=id,name,email"
		if len(scopes) == 0 {
			scopes = []string{"email", "public_profile"}
		}
	default:
		return fmt.Errorf("unsupported OAuth provider: %s", name)
	}

	om.providers[name] = OAuthProviderConfig{
		Config: &oauth2.Config{
			ClientID:     clientID,
			ClientSecret: clientSecret,
			RedirectURL:  redirectURL,
			Scopes:       scopes,
			Endpoint:     endpoint,
		},
		UserInfoURL: userInfoURL,
	}

	log.Printf("Registered OAuth provider: %s with redirect URL: %s", name, redirectURL)
	return nil
}

// GetAvailableProviders returns a list of configured OAuth providers
func (om *OAuthManager) GetAvailableProviders() []string {
	providers := make([]string, 0, len(om.providers))
	for name := range om.providers {
		providers = append(providers, name)
	}
	return providers
}

// HandleGetProviders returns the list of configured OAuth providers
func (om *OAuthManager) HandleGetProviders() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		providers := om.GetAvailableProviders()
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"providers": providers,
		})
	}
}

// generateStateToken generates a random state token for OAuth
func generateStateToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}

// HandleOAuthLogin handles the OAuth login initiation
func (om *OAuthManager) HandleOAuthLogin() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		provider := r.URL.Query().Get("provider")
		if provider == "" {
			utils.JSONError(w, http.StatusBadRequest, "Provider parameter is required")
			return
		}

		providerConfig, ok := om.providers[provider]
		if !ok {
			utils.JSONError(w, http.StatusBadRequest, fmt.Sprintf("Unsupported provider: %s", provider))
			return
		}

		// Generate state token for CSRF protection
		state, err := generateStateToken()
		if err != nil {
			log.Printf("Failed to generate state token: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate state token")
			return
		}

		// Store state in cookie for verification
		stateCookie := createAuthCookie("oauth_state", state, 600) // 10 minutes
		http.SetCookie(w, stateCookie)

		// Get authorization URL
		url := providerConfig.Config.AuthCodeURL(state, oauth2.AccessTypeOffline)

		// Return the URL as JSON or redirect
		if r.Header.Get("Accept") == "application/json" {
			utils.JSONResponse(w, http.StatusOK, map[string]string{
				"url": url,
			})
		} else {
			http.Redirect(w, r, url, http.StatusTemporaryRedirect)
		}
	}
}

// OAuthUserInfo represents user information from OAuth providers
type OAuthUserInfo struct {
	ID    string
	Email string
	Name  string
}

// getUserInfo fetches user information from the OAuth provider
func (om *OAuthManager) getUserInfo(ctx context.Context, provider string, token *oauth2.Token) (*OAuthUserInfo, error) {
	providerConfig, ok := om.providers[provider]
	if !ok {
		return nil, fmt.Errorf("provider not found: %s", provider)
	}

	client := providerConfig.Config.Client(ctx, token)
	resp, err := client.Get(providerConfig.UserInfoURL)
	if err != nil {
		return nil, fmt.Errorf("failed to get user info: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to get user info: status %d", resp.StatusCode)
	}

	var userInfo OAuthUserInfo
	var rawData map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&rawData); err != nil {
		return nil, fmt.Errorf("failed to decode user info: %w", err)
	}

	// Parse user info based on provider
	switch provider {
	case "google":
		userInfo.ID = getString(rawData, "id")
		userInfo.Email = getString(rawData, "email")
		userInfo.Name = getString(rawData, "name")
	case "microsoft":
		userInfo.ID = getString(rawData, "id")
		userInfo.Email = getString(rawData, "mail")
		if userInfo.Email == "" {
			userInfo.Email = getString(rawData, "userPrincipalName")
		}
		userInfo.Name = getString(rawData, "displayName")
	case "facebook":
		userInfo.ID = getString(rawData, "id")
		userInfo.Email = getString(rawData, "email")
		userInfo.Name = getString(rawData, "name")
	}

	if userInfo.ID == "" {
		return nil, fmt.Errorf("user ID not found in OAuth response")
	}

	return &userInfo, nil
}

// getString safely gets a string value from a map
func getString(m map[string]interface{}, key string) string {
	if val, ok := m[key]; ok {
		if str, ok := val.(string); ok {
			return str
		}
	}
	return ""
}

// HandleOAuthCallback handles the OAuth callback
func (om *OAuthManager) HandleOAuthCallback() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get provider from URL path parameter
		vars := mux.Vars(r)
		provider := vars["provider"]
		if provider == "" {
			utils.JSONError(w, http.StatusBadRequest, "Provider parameter is required")
			return
		}

		providerConfig, ok := om.providers[provider]
		if !ok {
			utils.JSONError(w, http.StatusBadRequest, fmt.Sprintf("Unsupported provider: %s", provider))
			return
		}

		// Verify state token
		stateCookie, err := r.Cookie("oauth_state")
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Missing state cookie")
			return
		}

		stateParam := r.URL.Query().Get("state")
		if stateParam != stateCookie.Value {
			utils.JSONError(w, http.StatusBadRequest, "Invalid state parameter")
			return
		}

		// Clear state cookie
		http.SetCookie(w, createAuthCookie("oauth_state", "", -1))

		// Get authorization code
		code := r.URL.Query().Get("code")
		if code == "" {
			errorMsg := r.URL.Query().Get("error")
			if errorMsg == "" {
				errorMsg = "Authorization code not found"
			}
			utils.JSONError(w, http.StatusBadRequest, errorMsg)
			return
		}

		// Exchange code for token
		ctx := context.Background()
		token, err := providerConfig.Config.Exchange(ctx, code)
		if err != nil {
			log.Printf("Failed to exchange code for token: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to exchange authorization code")
			return
		}

		// Get user info from provider
		userInfo, err := om.getUserInfo(ctx, provider, token)
		if err != nil {
			log.Printf("Failed to get user info: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to get user information")
			return
		}

		if userInfo.Email == "" {
			utils.JSONError(w, http.StatusBadRequest, "Email not provided by OAuth provider")
			return
		}

		// Find or create user
		user, err := om.findOrCreateOAuthUser(ctx, provider, userInfo, token)
		if err != nil {
			log.Printf("Failed to find or create OAuth user: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create user account")
			return
		}

		// Generate JWT access token
		jwtToken, err := generateAccessToken(user, om.iamService)
		if err != nil {
			log.Printf("Failed to generate JWT token: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate authentication token")
			return
		}

		// Set access token as httpOnly cookie for security
		http.SetCookie(w, createAuthCookie("auth_token", jwtToken, int(constants.AccessTokenDuration.Seconds())))

		// Return token as JSON or redirect to frontend
		redirectURL := r.URL.Query().Get("redirect_uri")
		if redirectURL == "" {
			// Default redirect to frontend callback page
			redirectURL = "/auth/oauth/callback"
		}

		// Redirect to frontend callback page (no token in URL for security)
		finalURL := fmt.Sprintf("%s?success=true&provider=%s", redirectURL, provider)
		http.Redirect(w, r, finalURL, http.StatusTemporaryRedirect)
	}
}

// findOrCreateOAuthUser finds an existing OAuth user or creates a new one
func (om *OAuthManager) findOrCreateOAuthUser(ctx context.Context, provider string, userInfo *OAuthUserInfo, token *oauth2.Token) (*auth.User, error) {
	// Try to find user by OAuth provider and ID (via Token table)
	user, err := om.authService.FindUserByOAuthToken(provider, userInfo.ID)
	if err == nil && user != nil {
		// Update OAuth token in Token table
		var expiry *time.Time
		if !token.Expiry.IsZero() {
			expiry = &token.Expiry
		}
		if err := om.authService.CreateOrUpdateOAuthToken(user.ID, provider, userInfo.ID, token.AccessToken, expiry); err != nil {
			return nil, fmt.Errorf("failed to update OAuth token: %w", err)
		}

		// Update last login
		now := time.Now()
		user.LastLogin = &now
		if err := om.authService.UpdateUser(user); err != nil {
			log.Printf("Warning: Failed to update last login: %v", err)
		}

		return user, nil
	}

	// Try to find user by email
	user, err = om.authService.FindUserByEmail(userInfo.Email)
	if err == nil && user != nil {
		// Link OAuth account to existing user via Token table
		var expiry *time.Time
		if !token.Expiry.IsZero() {
			expiry = &token.Expiry
		}
		if err := om.authService.CreateOrUpdateOAuthToken(user.ID, provider, userInfo.ID, token.AccessToken, expiry); err != nil {
			return nil, fmt.Errorf("failed to link OAuth account: %w", err)
		}

		// Update last login
		now := time.Now()
		user.LastLogin = &now
		if err := om.authService.UpdateUser(user); err != nil {
			log.Printf("Warning: Failed to update last login: %v", err)
		}

		return user, nil
	}

	// Create new user
	newUser := &auth.User{
		ID:          uuid.New(),
		Email:       userInfo.Email,
		DisplayName: userInfo.Name,
		Confirmed:   true, // OAuth users are confirmed by default
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}

	// Create user in database
	if err := om.authService.CreateUser(newUser); err != nil {
		return nil, fmt.Errorf("failed to create OAuth user: %w", err)
	}

	// Create OAuth token in Token table
	var expiry *time.Time
	if !token.Expiry.IsZero() {
		expiry = &token.Expiry
	}
	if err := om.authService.CreateOrUpdateOAuthToken(newUser.ID, provider, userInfo.ID, token.AccessToken, expiry); err != nil {
		log.Printf("Warning: Failed to store OAuth token: %v", err)
	}

	// Assign default 'user' role
	if om.iamService != nil {
		if err := om.iamService.AssignRoleToUser(ctx, newUser.ID.String(), "user"); err != nil {
			log.Printf("Warning: Failed to assign default user role to %s: %v", newUser.Email, err)
		}
	}

	return newUser, nil
}
