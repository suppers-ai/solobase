package auth

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/utils"
)

// API Key prefix for all generated keys
const APIKeyPrefix = "sb_"

// CreateAPIKeyRequest is the request body for creating an API key
type CreateAPIKeyRequest struct {
	Name      string     `json:"name"`
	ExpiresAt *time.Time `json:"expiresAt,omitempty"` // Optional expiration
}

// CreateAPIKeyResponse is the response for creating an API key
type CreateAPIKeyResponse struct {
	ID        uuid.UUID  `json:"id"`
	Name      string     `json:"name"`
	Key       string     `json:"key"`       // Full key - only returned once!
	KeyPrefix string     `json:"keyPrefix"` // Prefix for display
	ExpiresAt *time.Time `json:"expiresAt,omitempty"`
	CreatedAt time.Time  `json:"createdAt"`
}

// APIKeyResponse is the response for listing API keys (without the full key)
type APIKeyResponse struct {
	ID         uuid.UUID  `json:"id"`
	Name       string     `json:"name"`
	KeyPrefix  string     `json:"keyPrefix"`
	ExpiresAt  *time.Time `json:"expiresAt,omitempty"`
	LastUsedAt *time.Time `json:"lastUsedAt,omitempty"`
	LastUsedIP *string    `json:"lastUsedIp,omitempty"`
	CreatedAt  time.Time  `json:"createdAt"`
}

// HandleCreateAPIKey creates a new API key for the authenticated user
func HandleCreateAPIKey(db *database.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user ID from context (set by auth middleware)
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			utils.JSONError(w, http.StatusUnauthorized, "Unauthorized")
			return
		}

		userUUID, err := uuid.Parse(userID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid user ID")
			return
		}

		// Parse request
		var req CreateAPIKeyRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		if req.Name == "" {
			utils.JSONError(w, http.StatusBadRequest, "Name is required")
			return
		}

		// Generate the API key
		fullKey, keyPrefix, keyHash, err := auth.GenerateAPIKey(APIKeyPrefix)
		if err != nil {
			log.Printf("Failed to generate API key: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate API key")
			return
		}

		// Create the API key record
		apiKey := &auth.APIKey{
			ID:        uuid.New(),
			UserID:    userUUID,
			Name:      req.Name,
			KeyPrefix: keyPrefix,
			KeyHash:   keyHash,
			ExpiresAt: req.ExpiresAt,
			CreatedAt: time.Now(),
			UpdatedAt: time.Now(),
		}

		if err := db.Create(apiKey).Error; err != nil {
			log.Printf("Failed to create API key: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create API key")
			return
		}

		log.Printf("API key created for user %s: %s (prefix: %s)", userID, req.Name, keyPrefix)

		// Return the full key - this is the only time it will be shown!
		utils.JSONResponse(w, http.StatusCreated, CreateAPIKeyResponse{
			ID:        apiKey.ID,
			Name:      apiKey.Name,
			Key:       fullKey,
			KeyPrefix: keyPrefix,
			ExpiresAt: apiKey.ExpiresAt,
			CreatedAt: apiKey.CreatedAt,
		})
	}
}

// HandleListAPIKeys lists all API keys for the authenticated user
func HandleListAPIKeys(db *database.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			utils.JSONError(w, http.StatusUnauthorized, "Unauthorized")
			return
		}

		userUUID, err := uuid.Parse(userID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid user ID")
			return
		}

		// Get API keys for user
		storage := auth.NewGormStorage(db.DB)
		apiKeys, err := storage.GetAPIKeysByUserID(context.Background(), userUUID)
		if err != nil {
			log.Printf("Failed to list API keys: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to list API keys")
			return
		}

		// Convert to response format (without full keys)
		response := make([]APIKeyResponse, len(apiKeys))
		for i, key := range apiKeys {
			response[i] = APIKeyResponse{
				ID:         key.ID,
				Name:       key.Name,
				KeyPrefix:  key.KeyPrefix,
				ExpiresAt:  key.ExpiresAt,
				LastUsedAt: key.LastUsedAt,
				LastUsedIP: key.LastUsedIP,
				CreatedAt:  key.CreatedAt,
			}
		}

		utils.JSONResponse(w, http.StatusOK, response)
	}
}

// HandleRevokeAPIKey revokes an API key
func HandleRevokeAPIKey(db *database.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			utils.JSONError(w, http.StatusUnauthorized, "Unauthorized")
			return
		}

		userUUID, err := uuid.Parse(userID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid user ID")
			return
		}

		// Get key ID from URL
		vars := mux.Vars(r)
		keyID := vars["keyId"]
		if keyID == "" {
			utils.JSONError(w, http.StatusBadRequest, "Key ID is required")
			return
		}

		keyUUID, err := uuid.Parse(keyID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid key ID")
			return
		}

		// Revoke the key
		storage := auth.NewGormStorage(db.DB)
		if err := storage.RevokeAPIKey(context.Background(), keyUUID, userUUID); err != nil {
			log.Printf("Failed to revoke API key: %v", err)
			utils.JSONError(w, http.StatusNotFound, "API key not found or already revoked")
			return
		}

		log.Printf("API key revoked for user %s: %s", userID, keyID)
		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "API key revoked"})
	}
}
