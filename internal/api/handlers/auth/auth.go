package auth

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/golang-jwt/jwt/v5"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/extensions/core"
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/iam"
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

func HandleLogin(authService *services.AuthService, storageService *services.StorageService, extensionRegistry *core.ExtensionRegistry, iamService *iam.Service) http.HandlerFunc {
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

		// Generate JWT token with IAM roles
		token, err := generateToken(user, iamService)
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
		utils.JSONResponse(w, http.StatusOK, user)
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

func generateToken(user *auth.User, iamService *iam.Service) (string, error) {
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
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(commonjwt.GetJWTSecret())
}
