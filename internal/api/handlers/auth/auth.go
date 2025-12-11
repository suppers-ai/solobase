package auth

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/extensions/core"
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/constants"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/utils"
	"golang.org/x/crypto/bcrypt"
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

type Claims struct {
	UserID string   `json:"user_id"`
	Email  string   `json:"email"`
	Roles  []string `json:"roles"` // Array of role names from IAM
	jwt.RegisteredClaims
}


// generateRefreshToken creates a secure random refresh token
func generateRefreshToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}

func HandleLogin(authService *services.AuthService, storageService *services.StorageService, extensionRegistry *core.ExtensionRegistry, iamService *iam.Service, db *database.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		log.Printf("Login request received")

		var req LoginRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			log.Printf("Failed to decode login request: %v", err)
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body: "+err.Error())
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
					"appID":     appID,
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
		http.SetCookie(w, &http.Cookie{
			Name:     "auth_token",
			Value:    accessToken,
			Path:     "/",
			HttpOnly: true,
			Secure:   r.TLS != nil,
			SameSite: http.SameSiteLaxMode,
			MaxAge:   int(constants.AccessTokenDuration.Seconds()),
		})

		// In read-only mode, skip refresh token storage (database writes are blocked)
		// Users will need to re-login when access token expires
		if os.Getenv("READONLY_MODE") != "true" {
			// Generate refresh token
			refreshTokenStr, err := generateRefreshToken()
			if err != nil {
				log.Printf("Failed to generate refresh token for %s: %v", req.Email, err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
				return
			}

			// Create a token family for rotation tracking
			familyID := uuid.New()

			// Store refresh token in database
			refreshToken := &auth.Token{
				ID:         uuid.New(),
				UserID:     user.ID,
				TokenHash:  auth.HashToken(refreshTokenStr),
				Type:       auth.TokenTypeRefresh,
				FamilyID:   &familyID,
				ExpiresAt:  time.Now().Add(constants.RefreshTokenDuration),
				CreatedAt:  time.Now(),
				IPAddress:  getClientIP(r),
				DeviceInfo: getUserAgent(r),
			}

			if err := db.Create(refreshToken).Error; err != nil {
				log.Printf("Failed to store refresh token for %s: %v", req.Email, err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to create session")
				return
			}

			// Set refresh token as httpOnly cookie (long-lived)
			http.SetCookie(w, &http.Cookie{
				Name:     "refresh_token",
				Value:    refreshTokenStr,
				Path:     "/api/auth", // Only sent to auth endpoints
				HttpOnly: true,
				Secure:   r.TLS != nil,
				SameSite: http.SameSiteLaxMode,
				MaxAge:   int(constants.RefreshTokenDuration.Seconds()),
			})
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

func HandleLogout(db *database.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Revoke refresh token if present
		if cookie, err := r.Cookie("refresh_token"); err == nil && cookie.Value != "" {
			tokenHash := auth.HashToken(cookie.Value)
			now := time.Now()
			db.Model(&auth.Token{}).
				Where("token_hash = ? AND type = ? AND revoked_at IS NULL", tokenHash, auth.TokenTypeRefresh).
				Update("revoked_at", now)
		}

		// Clear the access token cookie
		http.SetCookie(w, &http.Cookie{
			Name:     "auth_token",
			Value:    "",
			Path:     "/",
			HttpOnly: true,
			Secure:   r.TLS != nil,
			SameSite: http.SameSiteLaxMode,
			MaxAge:   -1,
		})

		// Clear the refresh token cookie
		http.SetCookie(w, &http.Cookie{
			Name:     "refresh_token",
			Value:    "",
			Path:     "/api/auth",
			HttpOnly: true,
			Secure:   r.TLS != nil,
			SameSite: http.SameSiteLaxMode,
			MaxAge:   -1,
		})

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Logged out successfully"})
	}
}

// HandleRefreshToken handles token refresh requests
func HandleRefreshToken(db *database.DB, iamService *iam.Service, authService *services.AuthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get refresh token from cookie
		cookie, err := r.Cookie("refresh_token")
		if err != nil || cookie.Value == "" {
			utils.JSONError(w, http.StatusUnauthorized, "No refresh token")
			return
		}

		refreshTokenStr := cookie.Value
		tokenHash := auth.HashToken(refreshTokenStr)

		// Find the refresh token in database
		var token auth.Token
		if err := db.Where("token_hash = ? AND type = ? AND revoked_at IS NULL", tokenHash, auth.TokenTypeRefresh).
			First(&token).Error; err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid refresh token")
			return
		}

		// Check if token is expired
		if time.Now().After(token.ExpiresAt) {
			utils.JSONError(w, http.StatusUnauthorized, "Refresh token expired")
			return
		}

		// Check if token was already used (replay attack detection)
		if token.UsedAt != nil {
			// Token reuse detected - revoke entire token family
			log.Printf("WARNING: Refresh token reuse detected for user %s, revoking token family", token.UserID)
			now := time.Now()
			db.Model(&auth.Token{}).
				Where("family_id = ?", token.FamilyID).
				Update("revoked_at", now)
			utils.JSONError(w, http.StatusUnauthorized, "Token reuse detected")
			return
		}

		// Mark current token as used
		now := time.Now()
		db.Model(&token).Update("used_at", now)

		// Get user
		user, err := authService.GetUserByID(token.UserID.String())
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
		newRefreshToken := &auth.Token{
			ID:        uuid.New(),
			UserID:    user.ID,
			TokenHash: auth.HashToken(newRefreshTokenStr),
			Type:      auth.TokenTypeRefresh,
			FamilyID:  token.FamilyID, // Same family for rotation tracking
			ExpiresAt: time.Now().Add(constants.RefreshTokenDuration),
			CreatedAt: time.Now(),
			IPAddress: getClientIP(r),
			DeviceInfo: getUserAgent(r),
		}

		if err := db.Create(newRefreshToken).Error; err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create session")
			return
		}

		// Set new access token cookie
		http.SetCookie(w, &http.Cookie{
			Name:     "auth_token",
			Value:    accessToken,
			Path:     "/",
			HttpOnly: true,
			Secure:   r.TLS != nil,
			SameSite: http.SameSiteLaxMode,
			MaxAge:   int(constants.AccessTokenDuration.Seconds()),
		})

		// Set new refresh token cookie
		http.SetCookie(w, &http.Cookie{
			Name:     "refresh_token",
			Value:    newRefreshTokenStr,
			Path:     "/api/auth",
			HttpOnly: true,
			Secure:   r.TLS != nil,
			SameSite: http.SameSiteLaxMode,
			MaxAge:   int(constants.RefreshTokenDuration.Seconds()),
		})

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Token refreshed"})
	}
}

func HandleSignup(authService *services.AuthService, iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req SignupRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		// Hash password
		hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to process password")
			return
		}

		user := &auth.User{
			Email:    req.Email,
			Password: string(hashedPassword),
			// Role is now handled by IAM system
			// Metadata can be handled separately if needed
		}

		if err := authService.CreateUser(user); err != nil {
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
		// Get user ID from context (set by auth middleware)
		userID := r.Context().Value("userID")
		if userID == nil {
			utils.JSONError(w, http.StatusUnauthorized, "User not authenticated")
			return
		}

		userIDStr, ok := userID.(string)
		if !ok {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid user ID")
			return
		}

		// Parse request body
		var updates map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		// Remove fields that users shouldn't be able to update themselves
		delete(updates, "id")
		delete(updates, "password") // Password should be updated via change-password endpoint
		delete(updates, "role")     // Role changes should go through IAM
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
	CurrentPassword string `json:"current_password"`
	NewPassword     string `json:"new_password"`
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
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		// Verify current password
		if err := bcrypt.CompareHashAndPassword([]byte(user.Password), []byte(req.CurrentPassword)); err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Current password is incorrect")
			return
		}

		// Validate new password
		if len(req.NewPassword) < 8 {
			utils.JSONError(w, http.StatusBadRequest, "Password must be at least 8 characters")
			return
		}

		// Hash new password
		hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.NewPassword), bcrypt.DefaultCost)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to process password")
			return
		}

		// Update password in database
		if err := authService.UpdateUserPassword(user.ID.String(), string(hashedPassword)); err != nil {
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

	claims := &Claims{
		UserID: user.ID.String(),
		Email:  user.Email,
		Roles:  roleNames,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(constants.AccessTokenDuration)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(commonjwt.GetJWTSecret())
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
