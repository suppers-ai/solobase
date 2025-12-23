package auth

import (
	"context"
	"database/sql"
	"log"
	"net/http"

	"github.com/gorilla/mux"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/utils"
)

// API Key prefix for all generated keys
const APIKeyPrefix = "sb_"

// CreateAPIKeyRequest is the request body for creating an API key
type CreateAPIKeyRequest struct {
	Name      string     `json:"name"`
	ExpiresAt *apptime.Time `json:"expiresAt,omitempty"` // Optional expiration
}

// CreateAPIKeyResponse is the response for creating an API key
type CreateAPIKeyResponse struct {
	ID        uuid.UUID  `json:"id"`
	Name      string     `json:"name"`
	Key       string     `json:"key"`       // Full key - only returned once!
	KeyPrefix string     `json:"keyPrefix"` // Prefix for display
	ExpiresAt *apptime.Time `json:"expiresAt,omitempty"`
	CreatedAt apptime.Time  `json:"createdAt"`
}

// APIKeyResponse is the response for listing API keys (without the full key)
type APIKeyResponse struct {
	ID         uuid.UUID  `json:"id"`
	Name       string     `json:"name"`
	KeyPrefix  string     `json:"keyPrefix"`
	ExpiresAt  *apptime.Time `json:"expiresAt,omitempty"`
	LastUsedAt *apptime.Time `json:"lastUsedAt,omitempty"`
	LastUsedIP *string    `json:"lastUsedIp,omitempty"`
	CreatedAt  apptime.Time  `json:"createdAt"`
}

// HandleCreateAPIKey creates a new API key for the authenticated user
func HandleCreateAPIKey(sqlDB *sql.DB) http.HandlerFunc {
	storage := auth.NewStorage(sqlDB)

	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		userUUID, err := uuid.Parse(userID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid user ID")
			return
		}

		var req CreateAPIKeyRequest
		if !utils.DecodeJSONBody(w, r, &req) {
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
			ExpiresAt: apptime.FromTimePtr(req.ExpiresAt),
			CreatedAt: apptime.NowTime(),
			UpdatedAt: apptime.NowTime(),
		}

		if err := storage.CreateAPIKey(context.Background(), apiKey); err != nil {
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
			ExpiresAt: apiKey.ExpiresAt.ToTimePtr(),
			CreatedAt: apiKey.CreatedAt,
		})
	}
}

// HandleListAPIKeys lists all API keys for the authenticated user
func HandleListAPIKeys(sqlDB *sql.DB) http.HandlerFunc {
	storage := auth.NewStorage(sqlDB)

	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		userUUID, err := uuid.Parse(userID)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid user ID")
			return
		}

		// Get API keys for user
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
				ExpiresAt:  key.ExpiresAt.ToTimePtr(),
				LastUsedAt: key.LastUsedAt.ToTimePtr(),
				LastUsedIP: key.LastUsedIP,
				CreatedAt:  key.CreatedAt,
			}
		}

		utils.JSONResponse(w, http.StatusOK, response)
	}
}

// HandleRevokeAPIKey revokes an API key
func HandleRevokeAPIKey(sqlDB *sql.DB) http.HandlerFunc {
	storage := auth.NewStorage(sqlDB)

	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
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
		if err := storage.RevokeAPIKey(context.Background(), keyUUID, userUUID); err != nil {
			log.Printf("Failed to revoke API key: %v", err)
			utils.JSONError(w, http.StatusNotFound, "API key not found or already revoked")
			return
		}

		log.Printf("API key revoked for user %s: %s", userID, keyID)
		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "API key revoked"})
	}
}
