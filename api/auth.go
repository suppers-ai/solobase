package api

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/golang-jwt/jwt/v5"
	auth "github.com/suppers-ai/auth"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
	"golang.org/x/crypto/bcrypt"
)

var jwtSecret []byte // Will be set from config

// SetJWTSecret sets the JWT secret from config
func SetJWTSecret(secret string) {
	jwtSecret = []byte(secret)
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
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	Role   string `json:"role"`
	jwt.RegisteredClaims
}

func HandleLogin(authService *services.AuthService, storageService *services.StorageService, extensionRegistry *core.ExtensionRegistry) http.HandlerFunc {
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
					"userRole":  user.Role,
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

		// Generate JWT token
		token, err := generateToken(user)
		if err != nil {
			log.Printf("Failed to generate token for %s: %v", req.Email, err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		log.Printf("Login successful for %s", req.Email)
		utils.JSONResponse(w, http.StatusOK, LoginResponse{
			Token: token,
			User:  user,
		})
	}
}

func HandleLogout() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// In a JWT-based system, logout is handled client-side
		// We could implement a token blacklist here if needed
		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Logged out successfully"})
	}
}

func HandleSignup(authService *services.AuthService) http.HandlerFunc {
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
			Role:     "user",
			// Metadata can be handled separately if needed
		}

		if err := authService.CreateUser(user); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Failed to create user")
			return
		}

		utils.JSONResponse(w, http.StatusCreated, user)
	}
}

func HandleGetCurrentUser() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		user := r.Context().Value("user").(*auth.User)
		utils.JSONResponse(w, http.StatusOK, user)
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

func generateToken(user *auth.User) (string, error) {
	claims := &Claims{
		UserID: user.ID.String(),
		Email:  user.Email,
		Role:   user.Role,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(jwtSecret)
}
