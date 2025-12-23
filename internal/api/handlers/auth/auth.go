package auth

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/base64"
	"log"
	"net/http"
	"strings"

	"github.com/suppers-ai/solobase/extensions/core"
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/constants"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/env"
	"github.com/suppers-ai/solobase/internal/iam"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/utils"
)

// SetJWTSecret delegates to the common JWT package to ensure consistency
func SetJWTSecret(secret string) error {
	return commonjwt.SetJWTSecret(secret)
}

type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type LoginResponse struct {
	Token string     `json:"token"`
	User  *auth.User `json:"user"`
}

type SignupRequest struct {
	Email    string                 `json:"email"`
	Password string                 `json:"password"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// generateRefreshToken creates a secure random refresh token
func generateRefreshToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}

// isProduction checks if we're running in production mode
func isProduction() bool {
	env := env.GetEnv("ENVIRONMENT")
	return env == "production" || env == "prod"
}

// createAuthCookie creates a cookie with proper settings for cross-origin auth
// In production: Secure=true, SameSite=None (required for cross-origin)
// In development: Secure=false, SameSite=Lax (works on localhost)
func createAuthCookie(name, value string, maxAge int) *http.Cookie {
	cookie := &http.Cookie{
		Name:     name,
		Value:    value,
		Path:     "/",
		HttpOnly: true,
		MaxAge:   maxAge,
	}

	if isProduction() {
		// Production: cross-origin requires SameSite=None and Secure=true
		cookie.Secure = true
		cookie.SameSite = http.SameSiteNoneMode
	} else {
		// Development: SameSite=Lax works for localhost
		cookie.Secure = false
		cookie.SameSite = http.SameSiteLaxMode
	}

	return cookie
}

func HandleLogin(authService *services.AuthService, storageService *services.StorageService, extensionRegistry *core.ExtensionRegistry, iamService *iam.Service, sqlDB *sql.DB) http.HandlerFunc {
	// Handle WASM mode where services may be nil
	var queries *db.Queries
	if sqlDB != nil {
		queries = db.New(sqlDB)
	}

	return func(w http.ResponseWriter, r *http.Request) {
		// Check if auth service is available (WASM mode check)
		if authService == nil || queries == nil {
			utils.JSONError(w, http.StatusServiceUnavailable, "Authentication service not available in WASM mode without database")
			return
		}

		log.Printf("Login request received")

		var req LoginRequest
		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		log.Printf("Login attempt for email: %s", req.Email)

		user, err := authService.AuthenticateUser(req.Email, req.Password)
		if err != nil {
			log.Printf("Authentication failed for %s: %v", req.Email, err)
			utils.JSONError(w, http.StatusUnauthorized, "Invalid credentials")
			return
		}

		// Execute PostLogin hooks for extensions
		if extensionRegistry != nil {
			// Get app ID from storage service if available
			appID := "solobase" // default
			if storageService != nil {
				appID = storageService.GetAppID()
			}

			log.Printf("Executing PostLogin hooks with appID=%s for user %s", appID, user.Email)

			hookCtx := &core.HookContext{
				Request:  r,
				Response: w,
				Data: map[string]interface{}{
					"userID":    user.ID.String(),
					"userEmail": user.Email,
					// Role info now comes from IAM system
					"appID": appID,
				},
				Services: nil, // Services will be set by the registry
			}

			// Set the storage service in the hook context if available
			if storageService != nil && hookCtx.Services != nil {
				// The registry should have already set up services
				// We just add the storage reference for backwards compatibility
			}

			// Execute post-login hooks (e.g., CloudStorage extension will create "My Files" folder)
			if err := extensionRegistry.ExecuteHooks(r.Context(), core.HookPostLogin, hookCtx); err != nil {
				// Log the error but don't fail the login
				log.Printf("Warning: PostLogin hook failed: %v", err)
			} else {
				log.Printf("PostLogin hooks executed successfully")
			}
		} else {
			log.Printf("Warning: extensionRegistry is nil, skipping PostLogin hooks")
		}

		// Generate JWT access token with IAM roles
		accessToken, err := generateAccessToken(user, iamService)
		if err != nil {
			log.Printf("Failed to generate access token for %s: %v", req.Email, err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		// Set access token as httpOnly cookie (short-lived)
		http.SetCookie(w, createAuthCookie("auth_token", accessToken, int(constants.AccessTokenDuration.Seconds())))

		// In read-only mode, skip refresh token storage (database writes are blocked)
		// Users will need to re-login when access token expires
		if env.GetEnv("READONLY_MODE") != "true" {
			// Generate refresh token
			refreshTokenStr, err := generateRefreshToken()
			if err != nil {
				log.Printf("Failed to generate refresh token for %s: %v", req.Email, err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
				return
			}

			// Create a token family for rotation tracking
			familyID := uuid.New().String()
			tokenHash := auth.HashToken(refreshTokenStr)
			ipAddress := getClientIP(r)
			deviceInfo := getUserAgent(r)

			// Store refresh token in database using sqlc
			_, err = queries.CreateToken(r.Context(), db.CreateTokenParams{
				ID:         uuid.New().String(),
				UserID:     user.ID.String(),
				TokenHash:  &tokenHash,
				Type:       string(auth.TokenTypeRefresh),
				FamilyID:   &familyID,
				ExpiresAt:  apptime.NullTime{Time: apptime.NowTime().Add(constants.RefreshTokenDuration), Valid: true},
				CreatedAt:  apptime.NowString(),
				IpAddress:  ipAddress,
				DeviceInfo: deviceInfo,
			})

			if err != nil {
				log.Printf("Failed to store refresh token for %s: %v", req.Email, err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to create session")
				return
			}

			// Set refresh token as httpOnly cookie (long-lived)
			refreshCookie := createAuthCookie("refresh_token", refreshTokenStr, int(constants.RefreshTokenDuration.Seconds()))
			refreshCookie.Path = "/api/auth" // Only sent to auth endpoints
			http.SetCookie(w, refreshCookie)
		}

		// Get roles for response
		var roleNames []string
		if iamService != nil {
			roles, err := iamService.GetUserRoles(context.Background(), user.ID.String())
			if err != nil {
				log.Printf("Warning: Failed to fetch roles for login response: %v", err)
				roleNames = []string{}
			} else {
				roleNames = roles
			}
		}

		log.Printf("Login successful for %s with roles: %v", req.Email, roleNames)
		// Don't send token in response body for security
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"data":    auth.NewUserResponse(user, roleNames),
			"message": "Login successful",
		})
	}
}

func HandleLogout(sqlDB *sql.DB) http.HandlerFunc {
	queries := db.New(sqlDB)

	return func(w http.ResponseWriter, r *http.Request) {
		// Revoke refresh token if present
		if cookie, err := r.Cookie("refresh_token"); err == nil && cookie.Value != "" {
			tokenHash := auth.HashToken(cookie.Value)
			now := apptime.NowTime()
			// Get token by hash first, then revoke by ID
			if token, err := queries.GetTokenByHash(r.Context(), &tokenHash); err == nil {
				_ = queries.RevokeToken(r.Context(), db.RevokeTokenParams{
					RevokedAt: apptime.NullTime{Time: now, Valid: true},
					ID:        token.ID,
				})
			}
		}

		// Clear the access token cookie
		http.SetCookie(w, createAuthCookie("auth_token", "", -1))

		// Clear the refresh token cookie
		clearRefreshCookie := createAuthCookie("refresh_token", "", -1)
		clearRefreshCookie.Path = "/api/auth"
		http.SetCookie(w, clearRefreshCookie)

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Logged out successfully"})
	}
}

// HandleRefreshToken handles token refresh requests
func HandleRefreshToken(sqlDB *sql.DB, iamService *iam.Service, authService *services.AuthService) http.HandlerFunc {
	queries := db.New(sqlDB)

	return func(w http.ResponseWriter, r *http.Request) {
		// Get refresh token from cookie
		cookie, err := r.Cookie("refresh_token")
		if err != nil || cookie.Value == "" {
			utils.JSONError(w, http.StatusUnauthorized, "No refresh token")
			return
		}

		refreshTokenStr := cookie.Value
		tokenHash := auth.HashToken(refreshTokenStr)

		// Find the refresh token in database using sqlc
		token, err := queries.GetTokenByHash(r.Context(), &tokenHash)
		if err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid refresh token")
			return
		}

		// Check if token is revoked
		if token.RevokedAt.Valid {
			utils.JSONError(w, http.StatusUnauthorized, "Token has been revoked")
			return
		}

		// Check token type
		if token.Type != string(auth.TokenTypeRefresh) {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid token type")
			return
		}

		// Check if token is expired
		if token.ExpiresAt.Valid && apptime.NowTime().After(token.ExpiresAt.Time) {
			utils.JSONError(w, http.StatusUnauthorized, "Refresh token expired")
			return
		}

		// Check if token was already used (replay attack detection)
		if token.UsedAt.Valid {
			// Token reuse detected - revoke entire token family
			log.Printf("WARNING: Refresh token reuse detected for user %s, revoking token family", token.UserID)
			now := apptime.NowTime()
			_ = queries.RevokeTokensByFamily(r.Context(), db.RevokeTokensByFamilyParams{
				RevokedAt: apptime.NullTime{Time: now, Valid: true},
				FamilyID:  token.FamilyID,
			})
			utils.JSONError(w, http.StatusUnauthorized, "Token reuse detected")
			return
		}

		// Mark current token as used
		now := apptime.NowTime()
		_ = queries.UpdateTokenUsed(r.Context(), db.UpdateTokenUsedParams{
			UsedAt: apptime.NullTime{Time: now, Valid: true},
			ID:     token.ID,
		})

		// Get user
		user, err := authService.GetUserByID(token.UserID)
		if err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "User not found")
			return
		}

		// Generate new access token
		accessToken, err := generateAccessToken(user, iamService)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		// Generate new refresh token (rotation)
		newRefreshTokenStr, err := generateRefreshToken()
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		// Store new refresh token with same family ID
		newTokenHash := auth.HashToken(newRefreshTokenStr)
		ipAddress := getClientIP(r)
		deviceInfo := getUserAgent(r)

		_, err = queries.CreateToken(r.Context(), db.CreateTokenParams{
			ID:         uuid.New().String(),
			UserID:     user.ID.String(),
			TokenHash:  &newTokenHash,
			Type:       string(auth.TokenTypeRefresh),
			FamilyID:   token.FamilyID, // Same family for rotation tracking
			ExpiresAt:  apptime.NullTime{Time: apptime.NowTime().Add(constants.RefreshTokenDuration), Valid: true},
			CreatedAt:  apptime.NowString(),
			IpAddress:  ipAddress,
			DeviceInfo: deviceInfo,
		})

		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create session")
			return
		}

		// Set new access token cookie
		http.SetCookie(w, createAuthCookie("auth_token", accessToken, int(constants.AccessTokenDuration.Seconds())))

		// Set new refresh token cookie
		newRefreshCookie := createAuthCookie("refresh_token", newRefreshTokenStr, int(constants.RefreshTokenDuration.Seconds()))
		newRefreshCookie.Path = "/api/auth"
		http.SetCookie(w, newRefreshCookie)

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Token refreshed"})
	}
}

func HandleSignup(authService *services.AuthService, iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		log.Printf("Signup request received")

		var req SignupRequest
		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		log.Printf("Signup attempt for email: %s", req.Email)

		// Hash password using adapter (bcrypt for standard, sha256 for WASM)
		hashedPassword, err := crypto.HashPassword(req.Password)
		if err != nil {
			log.Printf("Failed to hash password for %s: %v", req.Email, err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to process password")
			return
		}

		log.Printf("Password hashed successfully for %s", req.Email)

		user := &auth.User{
			Email:    req.Email,
			Password: hashedPassword,
			// Role is now handled by IAM system
			// Metadata can be handled separately if needed
		}

		if err := authService.CreateUser(user); err != nil {
			log.Printf("Failed to create user %s: %v", req.Email, err)
			utils.JSONError(w, http.StatusBadRequest, "Failed to create user")
			return
		}

		// Assign default 'user' role to new signups
		if iamService != nil {
			if err := iamService.AssignRoleToUser(context.Background(), user.ID.String(), "user"); err != nil {
				log.Printf("Warning: Failed to assign default user role to %s: %v", user.Email, err)
				// Don't fail the signup, just log the warning
			}
		}

		utils.JSONResponse(w, http.StatusCreated, user)
	}
}

func HandleGetCurrentUser() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		user := r.Context().Value("user").(*auth.User)
		roles, _ := r.Context().Value("user_roles").([]string)

		utils.JSONResponse(w, http.StatusOK, auth.NewUserResponse(user, roles))
	}
}

// HandleUpdateCurrentUser handles updating the current user's profile
func HandleUpdateCurrentUser(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userIDStr, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		var updates map[string]interface{}
		if !utils.DecodeJSONBody(w, r, &updates) {
			return
		}

		// Remove fields that users shouldn't be able to update themselves
		delete(updates, "id")
		delete(updates, "password")  // Password should be updated via change-password endpoint
		delete(updates, "role")      // Role changes should go through IAM
		delete(updates, "confirmed") // Email confirmation status
		delete(updates, "created_at")
		delete(updates, "deleted_at")

		// Update user in database
		updatedUser, err := userService.UpdateUser(userIDStr, updates)
		if err != nil {
			log.Printf("Failed to update user profile for %s: %v", userIDStr, err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update profile")
			return
		}

		utils.JSONResponse(w, http.StatusOK, updatedUser)
	}
}

type ChangePasswordRequest struct {
	CurrentPassword string `json:"currentPassword"`
	NewPassword     string `json:"newPassword"`
}

func HandleChangePassword(authService *services.AuthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user from context (set by auth middleware)
		user, ok := r.Context().Value("user").(*auth.User)
		if !ok {
			utils.JSONError(w, http.StatusUnauthorized, "User not authenticated")
			return
		}

		var req ChangePasswordRequest
		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		// Verify current password using adapter
		if err := crypto.ComparePassword(user.Password, req.CurrentPassword); err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Current password is incorrect")
			return
		}

		// Validate new password
		if len(req.NewPassword) < 8 {
			utils.JSONError(w, http.StatusBadRequest, "Password must be at least 8 characters")
			return
		}

		// Hash new password using adapter
		hashedPassword, err := crypto.HashPassword(req.NewPassword)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to process password")
			return
		}

		// Update password in database
		if err := authService.UpdateUserPassword(user.ID.String(), hashedPassword); err != nil {
			log.Printf("Failed to update password for user %s: %v", user.Email, err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update password")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Password updated successfully"})
	}
}

// generateAccessToken creates a short-lived JWT access token
func generateAccessToken(user *auth.User, iamService *iam.Service) (string, error) {
	// Fetch user's roles from IAM
	var roleNames []string
	if iamService != nil {
		roles, err := iamService.GetUserRoles(context.Background(), user.ID.String())
		if err != nil {
			log.Printf("Warning: Failed to fetch user roles from IAM: %v", err)
			// Continue with empty roles rather than failing login
			roleNames = []string{}
		} else {
			// Extract role names (roles is already []string)
			roleNames = roles
		}
	}

	// Use build-tag specific token generation (standard vs WASM)
	return createSignedToken(user.ID.String(), user.Email, roleNames)
}

// getClientIP extracts the client IP address from the request
func getClientIP(r *http.Request) *string {
	var ip string
	// Check for forwarded headers (when behind proxy/load balancer)
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		// X-Forwarded-For can contain multiple IPs; the first one is the client
		ips := strings.Split(xff, ",")
		if len(ips) > 0 {
			ip = strings.TrimSpace(ips[0])
		}
	} else if xri := r.Header.Get("X-Real-IP"); xri != "" {
		ip = xri
	} else {
		// Fall back to RemoteAddr
		ip = r.RemoteAddr
		// Remove port if present
		if idx := strings.LastIndex(ip, ":"); idx != -1 {
			ip = ip[:idx]
		}
	}
	if ip == "" {
		return nil
	}
	return &ip
}

// getUserAgent extracts the User-Agent from the request
func getUserAgent(r *http.Request) *string {
	ua := r.Header.Get("User-Agent")
	if ua == "" {
		return nil
	}
	return &ua
}
