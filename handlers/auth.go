package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/middleware"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
	"golang.org/x/crypto/bcrypt"
)

type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type LoginResponse struct {
	Token  string `json:"token"`
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	Role   string `json:"role"`
}

// APILogin handles API login requests
func APILogin(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req LoginRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request")
			return
		}

		// Validate credentials
		ctx := r.Context()
		var user struct {
			ID       string `db:"id"`
			Password string `db:"password"`
			Role     string `db:"role"`
		}

		err := svc.Database().Get(ctx, &user, `
			SELECT id, password, role 
			FROM auth_users 
			WHERE email = $1 AND confirmed = true
		`, req.Email)

		if err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid credentials")
			return
		}

		// Check password
		if err := bcrypt.CompareHashAndPassword([]byte(user.Password), []byte(req.Password)); err != nil {
			utils.JSONError(w, http.StatusUnauthorized, "Invalid credentials")
			return
		}

		// Generate token
		token, err := middleware.GenerateToken(svc.Config().JWTSecret, user.ID, req.Email, user.Role)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		// Log successful login
		svc.Logger().Info(ctx, "User logged in via API",
			logger.String("email", req.Email),
			logger.String("user_id", user.ID))

		utils.JSONResponse(w, http.StatusOK, LoginResponse{
			Token:  token,
			UserID: user.ID,
			Email:  req.Email,
			Role:   user.Role,
		})
	}
}

type SignupRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

// APISignup handles API signup requests
func APISignup(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !svc.Config().EnableSignup {
			utils.JSONError(w, http.StatusForbidden, "Signup is disabled")
			return
		}

		var req SignupRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request")
			return
		}

		// Create user
		ctx := r.Context()
		user, err := svc.Auth().CreateUser(ctx, req.Email, req.Password, "user")
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Failed to create user")
			return
		}

		// Generate token
		token, err := middleware.GenerateToken(svc.Config().JWTSecret, user.GetPID(), req.Email, "user")
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		utils.JSONResponse(w, http.StatusCreated, LoginResponse{
			Token:  token,
			UserID: user.GetPID(),
			Email:  req.Email,
			Role:   "user",
		})
	}
}

// APIRefresh refreshes a JWT token
func APIRefresh(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user info from context (set by JWT middleware)
		userID := r.Context().Value("user_id").(string)
		email := r.Context().Value("user_email").(string)
		role := r.Context().Value("user_role").(string)

		// Generate new token
		token, err := middleware.GenerateToken(svc.Config().JWTSecret, userID, email, role)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate token")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]string{
			"token": token,
		})
	}
}
