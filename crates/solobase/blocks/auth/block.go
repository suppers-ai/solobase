package auth

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/suppers-ai/solobase/adapters/auth/oauth"
	"github.com/suppers-ai/solobase/core/apptime"
	pkgauth "github.com/suppers-ai/solobase/core/auth"
	"github.com/suppers-ai/solobase/core/constants"
	"github.com/suppers-ai/solobase/core/env"
	"github.com/suppers-ai/solobase/core/envutil"
	"github.com/suppers-ai/solobase/core/iam"
	"github.com/suppers-ai/solobase/core/uuid"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const BlockName = "auth-feature"

const (
	usersCollection  = "auth_users"
	tokensCollection = "auth_tokens"
	apiKeysCollection = "api_keys"
)

// AuthBlock handles authentication using ctx.Services().Database.
type AuthBlock struct {
	router        *waffle.Router
	enableSignup  bool
	oauthProvider oauth.Provider
}

func NewAuthBlock() *AuthBlock {
	b := &AuthBlock{}
	b.router = waffle.NewRouter()
	// Public routes
	b.router.Create("/auth/login", b.handleLogin)
	b.router.Create("/auth/signup", b.handleSignup)
	b.router.Create("/auth/refresh", b.handleRefreshToken)
	// Protected routes
	b.router.Create("/auth/logout", b.handleLogout)
	b.router.Retrieve("/auth/me", b.handleGetCurrentUser)
	b.router.Update("/auth/me", b.handleUpdateCurrentUser)
	b.router.Create("/auth/change-password", b.handleChangePassword)
	// API key management
	b.router.Retrieve("/auth/api-keys", b.handleListAPIKeys)
	b.router.Create("/auth/api-keys", b.handleCreateAPIKey)
	b.router.Delete("/auth/api-keys/{keyId}", b.handleRevokeAPIKey)
	// OAuth routes
	b.router.Retrieve("/auth/oauth/providers", b.handleOAuthProviders)
	b.router.Retrieve("/auth/oauth/login", b.handleOAuthLogin)
	b.router.Retrieve("/auth/oauth/callback/{provider}", b.handleOAuthCallback)
	b.router.Create("/internal/oauth/sync-user", b.handleOAuthSyncUser)
	return b
}

func (b *AuthBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Authentication routes",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
	}
}

func (b *AuthBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *AuthBlock) Lifecycle(ctx waffle.Context, evt waffle.LifecycleEvent) error {
	if evt.Type == waffle.Init {
		// Read signup config from environment (default: enabled)
		signupEnv := os.Getenv("ENABLE_SIGNUP")
		b.enableSignup = signupEnv == "" || signupEnv == "true" || signupEnv == "1"

		dbSvc := ctx.Services().Database
		cryptoSvc := ctx.Services().Crypto
		netSvc := ctx.Services().Network

		baseURL := env.GetEnv("BASE_URL")
		if baseURL == "" {
			baseURL = "http://localhost:8080"
		}

		if netSvc != nil {
			provider := oauth.NewStandardProvider(dbSvc, cryptoSvc, netSvc, baseURL)

			// Register configured providers from env vars
			if id, secret := env.GetEnv("GOOGLE_CLIENT_ID"), env.GetEnv("GOOGLE_CLIENT_SECRET"); id != "" && secret != "" {
				if err := provider.RegisterProvider("google", id, secret, nil); err != nil {
					log.Printf("Warning: Failed to register Google OAuth: %v", err)
				}
			}
			if id, secret := env.GetEnv("MICROSOFT_CLIENT_ID"), env.GetEnv("MICROSOFT_CLIENT_SECRET"); id != "" && secret != "" {
				if err := provider.RegisterProvider("microsoft", id, secret, nil); err != nil {
					log.Printf("Warning: Failed to register Microsoft OAuth: %v", err)
				}
			}
			if id, secret := env.GetEnv("FACEBOOK_CLIENT_ID"), env.GetEnv("FACEBOOK_CLIENT_SECRET"); id != "" && secret != "" {
				if err := provider.RegisterProvider("facebook", id, secret, nil); err != nil {
					log.Printf("Warning: Failed to register Facebook OAuth: %v", err)
				}
			}

			b.oauthProvider = provider
		} else {
			// No network service — OAuth not available
			b.oauthProvider = &noopOAuthProvider{}
		}
	}
	return nil
}

// formatCookie formats a cookie for the Set-Cookie header.
func formatCookie(name, value, path string, maxAge int, httpOnly bool) string {
	cookie := fmt.Sprintf("%s=%s; Path=%s; Max-Age=%d", name, value, path, maxAge)
	if httpOnly {
		cookie += "; HttpOnly"
	}
	if envutil.IsProduction() {
		cookie += "; Secure; SameSite=None"
	} else {
		cookie += "; SameSite=Lax"
	}
	return cookie
}

// --- Login ---

func (b *AuthBlock) handleLogin(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var body LoginRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	// Find user by email
	userRecord, err := database.GetByField(context.Background(), db, usersCollection, "email", body.Email)
	if err != nil {
		return waffle.Error(msg, 401, "unauthorized", "Invalid credentials")
	}

	cryptoSvc := ctx.Services().Crypto

	password, _ := userRecord.Data["password"].(string)
	if err := cryptoSvc.CompareHash(body.Password, password); err != nil {
		return waffle.Error(msg, 401, "unauthorized", "Invalid credentials")
	}

	// Update last login (non-critical)
	_, _ = db.Update(context.Background(), usersCollection, userRecord.ID, map[string]any{
		"last_login": apptime.NowTime().Format(apptime.TimeFormat),
	})

	userEmail, _ := userRecord.Data["email"].(string)
	accessToken, err := GenerateAccessToken(cryptoSvc, userRecord.ID, userEmail, db)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to generate token")
	}

	resp := waffle.NewResponse(msg, 200).
		SetCookie(formatCookie("auth_token", accessToken, "/", int(constants.AccessTokenDuration.Seconds()), true))

	// Generate refresh token
	if !envutil.IsReadOnly() {
		refreshTokenStr, err := generateRefreshToken()
		if err != nil {
			return waffle.Error(msg, 500, "internal_error", "Failed to generate token")
		}

		familyID := uuid.New().String()
		tokenHash := pkgauth.HashToken(refreshTokenStr)
		ipAddress := getIPFromMeta(msg)
		deviceInfo := getHeaderPtr(msg, "User-Agent")

		expiresAt := apptime.NowTime().Add(constants.RefreshTokenDuration)

		tokenData := map[string]any{
			"user_id":    userRecord.ID,
			"token_hash": tokenHash,
			"type":       "refresh",
			"family_id":  familyID,
			"expires_at": expiresAt.Format(apptime.TimeFormat),
		}
		if ipAddress != nil {
			tokenData["ip_address"] = *ipAddress
		}
		if deviceInfo != nil {
			tokenData["device_info"] = *deviceInfo
		}

		if _, err := db.Create(context.Background(), tokensCollection, tokenData); err != nil {
			return waffle.Error(msg, 500, "internal_error", "Failed to create session")
		}

		resp.SetCookie(formatCookie("refresh_token", refreshTokenStr, "/api/auth", int(constants.RefreshTokenDuration.Seconds()), true))
	}

	// Get roles for response
	roleNames, err := iam.GetUserRoles(context.Background(), db, userRecord.ID)
	if err != nil {
		roleNames = []string{}
	}

	return resp.JSON(map[string]any{
		"data": map[string]any{
			"user":  sanitizeUserData(userRecord.Data),
			"roles": roleNames,
		},
		"message": "Login successful",
	})
}

// --- Signup ---

func (b *AuthBlock) handleSignup(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	if !b.enableSignup {
		return waffle.Error(msg, 403, "forbidden", "User registration is disabled")
	}

	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var body SignupRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	hashedPassword, err := ctx.Services().Crypto.Hash(body.Password)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to process password")
	}

	userData := map[string]any{
		"email":    body.Email,
		"password": hashedPassword,
	}

	record, err := db.Create(context.Background(), usersCollection, userData)
	if err != nil {
		return waffle.Error(msg, 400, "bad_request", "Failed to create user")
	}

	// Assign default role
	if err := iam.AssignRole(context.Background(), db, record.ID, "user", "system"); err != nil {
		log.Printf("Warning: Failed to assign default user role to %s: %v", body.Email, err)
	}

	return waffle.JSONRespond(msg, 201, sanitizeUserData(record.Data))
}

// --- Refresh Token ---

func (b *AuthBlock) handleRefreshToken(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	refreshTokenStr := msg.Cookie("refresh_token")
	if refreshTokenStr == "" {
		return waffle.Error(msg, 401, "unauthorized", "No refresh token")
	}

	tokenHash := pkgauth.HashToken(refreshTokenStr)

	// Find token by hash
	tokenRecord, err := database.GetByField(context.Background(), db, tokensCollection, "token_hash", tokenHash)
	if err != nil {
		return waffle.Error(msg, 401, "unauthorized", "Invalid refresh token")
	}

	// Check revoked
	if tokenRecord.Data["revoked_at"] != nil && tokenRecord.Data["revoked_at"] != "" {
		return waffle.Error(msg, 401, "unauthorized", "Token has been revoked")
	}

	// Check type
	if fmt.Sprintf("%v", tokenRecord.Data["type"]) != "refresh" {
		return waffle.Error(msg, 401, "unauthorized", "Invalid token type")
	}

	// Check expiry
	expiresAtStr, _ := tokenRecord.Data["expires_at"].(string)
	if expiresAtStr != "" {
		expiresAt, err := apptime.ParseWithLayout(apptime.TimeFormat, expiresAtStr)
		if err == nil && apptime.NowTime().After(expiresAt) {
			return waffle.Error(msg, 401, "unauthorized", "Refresh token expired")
		}
	}

	// Replay attack detection
	if tokenRecord.Data["used_at"] != nil && tokenRecord.Data["used_at"] != "" {
		userID := fmt.Sprintf("%v", tokenRecord.Data["user_id"])
		log.Printf("WARNING: Refresh token reuse detected for user %s, revoking token family", userID)
		if familyID, ok := tokenRecord.Data["family_id"].(string); ok && familyID != "" {
			b.revokeTokenFamily(db, familyID)
		}
		return waffle.Error(msg, 401, "unauthorized", "Token reuse detected")
	}

	// Mark current token as used
	_, _ = db.Update(context.Background(), tokensCollection, tokenRecord.ID, map[string]any{
		"used_at": apptime.NowTime().Format(apptime.TimeFormat),
	})

	// Get user
	userID := fmt.Sprintf("%v", tokenRecord.Data["user_id"])
	userRecord, err := db.Get(context.Background(), usersCollection, userID)
	if err != nil {
		return waffle.Error(msg, 401, "unauthorized", "User not found")
	}

	refreshUserEmail, _ := userRecord.Data["email"].(string)
	accessToken, err := GenerateAccessToken(ctx.Services().Crypto, userRecord.ID, refreshUserEmail, db)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to generate token")
	}

	// Create new refresh token
	newRefreshTokenStr, err := generateRefreshToken()
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to generate token")
	}

	newTokenHash := pkgauth.HashToken(newRefreshTokenStr)
	ipAddress := getIPFromMeta(msg)
	deviceInfo := getHeaderPtr(msg, "User-Agent")
	newExpiresAt := apptime.NowTime().Add(constants.RefreshTokenDuration)

	familyID, _ := tokenRecord.Data["family_id"].(string)
	if familyID == "" {
		familyID = uuid.New().String()
	}

	newTokenData := map[string]any{
		"user_id":    userID,
		"token_hash": newTokenHash,
		"type":       "refresh",
		"family_id":  familyID,
		"expires_at": newExpiresAt.Format(apptime.TimeFormat),
	}
	if ipAddress != nil {
		newTokenData["ip_address"] = *ipAddress
	}
	if deviceInfo != nil {
		newTokenData["device_info"] = *deviceInfo
	}

	if _, err := db.Create(context.Background(), tokensCollection, newTokenData); err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to create session")
	}

	return waffle.NewResponse(msg, 200).
		SetCookie(formatCookie("auth_token", accessToken, "/", int(constants.AccessTokenDuration.Seconds()), true)).
		SetCookie(formatCookie("refresh_token", newRefreshTokenStr, "/api/auth", int(constants.RefreshTokenDuration.Seconds()), true)).
		JSON(map[string]string{"message": "Token refreshed"})
}

// --- Logout ---

func (b *AuthBlock) handleLogout(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database

	// Revoke refresh token if present
	if refreshToken := msg.Cookie("refresh_token"); refreshToken != "" && db != nil {
		tokenHash := pkgauth.HashToken(refreshToken)
		tokenRecord, err := database.GetByField(context.Background(), db, tokensCollection, "token_hash", tokenHash)
		if err == nil {
			_, _ = db.Update(context.Background(), tokensCollection, tokenRecord.ID, map[string]any{
				"revoked_at": apptime.NowTime().Format(apptime.TimeFormat),
			})
		}
	}

	return waffle.NewResponse(msg, 200).
		SetCookie(formatCookie("auth_token", "", "/", -1, true)).
		SetCookie(formatCookie("refresh_token", "", "/api/auth", -1, true)).
		JSON(map[string]string{"message": "Logged out successfully"})
}

// --- Get Current User ---

func (b *AuthBlock) handleGetCurrentUser(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	userRecord, err := db.Get(context.Background(), usersCollection, userID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", "User not found")
	}

	roles := msg.UserRoles()
	return waffle.JSONRespond(msg, 200, map[string]any{
		"user":  sanitizeUserData(userRecord.Data),
		"roles": roles,
	})
}

// --- Update Current User ---

func (b *AuthBlock) handleUpdateCurrentUser(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	var updates map[string]any
	if err := msg.Decode(&updates); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	// Remove fields that users shouldn't update
	for _, key := range []string{"id", "password", "role", "confirmed", "created_at", "deleted_at"} {
		delete(updates, key)
	}

	record, err := db.Update(context.Background(), usersCollection, userID, updates)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to update profile")
	}

	return waffle.JSONRespond(msg, 200, sanitizeUserData(record.Data))
}

// --- Change Password ---

func (b *AuthBlock) handleChangePassword(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	var body ChangePasswordRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	userRecord, err := db.Get(context.Background(), usersCollection, userID)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to verify user")
	}

	currentHash, _ := userRecord.Data["password"].(string)
	if err := ctx.Services().Crypto.CompareHash(body.CurrentPassword, currentHash); err != nil {
		return waffle.Error(msg, 401, "unauthorized", "Current password is incorrect")
	}

	if len(body.NewPassword) < 8 {
		return waffle.Error(msg, 400, "bad_request", "Password must be at least 8 characters")
	}

	hashedPassword, err := ctx.Services().Crypto.Hash(body.NewPassword)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to process password")
	}

	if _, err := db.Update(context.Background(), usersCollection, userID, map[string]any{
		"password": hashedPassword,
	}); err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to update password")
	}

	return waffle.JSONRespond(msg, 200, map[string]string{"message": "Password updated successfully"})
}

// --- API Keys ---

func (b *AuthBlock) handleCreateAPIKey(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	var body CreateAPIKeyRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}
	if body.Name == "" {
		return waffle.Error(msg, 400, "bad_request", "Name is required")
	}

	fullKey, keyPrefix, keyHash, err := pkgauth.GenerateAPIKey(APIKeyPrefix)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to generate API key")
	}

	keyData := map[string]any{
		"user_id":    userID,
		"name":       body.Name,
		"key_prefix": keyPrefix,
		"key_hash":   keyHash,
	}
	if body.ExpiresAt != nil {
		keyData["expires_at"] = body.ExpiresAt.Format(apptime.TimeFormat)
	}

	record, err := db.Create(context.Background(), apiKeysCollection, keyData)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to create API key")
	}

	return waffle.JSONRespond(msg, 201, map[string]any{
		"id":        record.ID,
		"name":      body.Name,
		"key":       fullKey,
		"keyPrefix": keyPrefix,
		"createdAt": record.Data["created_at"],
	})
}

func (b *AuthBlock) handleListAPIKeys(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	records, err := database.ListAll(context.Background(), db, apiKeysCollection,
		database.Filter{Field: "user_id", Operator: database.OpEqual, Value: userID},
		database.Filter{Field: "revoked_at", Operator: database.OpIsNull},
	)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to list API keys")
	}

	var response []map[string]any
	for _, r := range records {
		item := map[string]any{
			"id":        r.ID,
			"name":      r.Data["name"],
			"keyPrefix": r.Data["key_prefix"],
			"createdAt": r.Data["created_at"],
		}
		if v := r.Data["expires_at"]; v != nil && v != "" {
			item["expiresAt"] = v
		}
		if v := r.Data["last_used_at"]; v != nil && v != "" {
			item["lastUsedAt"] = v
		}
		if v := r.Data["last_used_ip"]; v != nil && v != "" {
			item["lastUsedIp"] = v
		}
		response = append(response, item)
	}

	return waffle.JSONRespond(msg, 200, response)
}

func (b *AuthBlock) handleRevokeAPIKey(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.UserID()
	if userID == "" {
		return waffle.Error(msg, 401, "unauthorized", "User not authenticated")
	}

	keyID := msg.Var("keyId")
	if keyID == "" {
		return waffle.Error(msg, 400, "bad_request", "Key ID is required")
	}

	// Verify ownership
	record, err := db.Get(context.Background(), apiKeysCollection, keyID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", "API key not found or already revoked")
	}
	if fmt.Sprintf("%v", record.Data["user_id"]) != userID {
		return waffle.Error(msg, 404, "not_found", "API key not found or already revoked")
	}

	_, err = db.Update(context.Background(), apiKeysCollection, keyID, map[string]any{
		"revoked_at": apptime.NowTime().Format(apptime.TimeFormat),
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to revoke API key")
	}

	return waffle.JSONRespond(msg, 200, map[string]string{"message": "API key revoked"})
}

// --- Helpers ---

func (b *AuthBlock) revokeTokenFamily(db database.Service, familyID string) {
	records, err := database.ListAll(context.Background(), db, tokensCollection,
		database.Filter{Field: "family_id", Operator: database.OpEqual, Value: familyID},
	)
	if err != nil {
		return
	}
	for _, r := range records {
		_, _ = db.Update(context.Background(), tokensCollection, r.ID, map[string]any{
			"revoked_at": apptime.NowTime().Format(apptime.TimeFormat),
		})
	}
}

// sanitizeUserData removes sensitive fields from user data.
func sanitizeUserData(data map[string]any) map[string]any {
	result := make(map[string]any, len(data))
	for k, v := range data {
		result[k] = v
	}
	for _, key := range []string{
		"password", "confirm_token", "confirm_selector",
		"recover_token", "recover_selector", "recover_token_exp",
		"totp_secret", "totp_secret_backup", "recovery_codes",
	} {
		delete(result, key)
	}
	return result
}

func getIPFromMeta(msg *waffle.Message) *string {
	// Try X-Forwarded-For first, then X-Real-IP, then remote addr
	if ip := msg.Header("X-Forwarded-For"); ip != "" {
		return &ip
	}
	if ip := msg.Header("X-Real-Ip"); ip != "" {
		return &ip
	}
	addr := msg.RemoteAddr()
	if addr == "" {
		return nil
	}
	// Strip port from remote addr
	if idx := len(addr) - 1; idx > 0 {
		for i := idx; i >= 0; i-- {
			if addr[i] == ':' {
				ip := addr[:i]
				return &ip
			}
		}
	}
	return &addr
}

func getHeaderPtr(msg *waffle.Message, name string) *string {
	val := msg.Header(name)
	if val == "" {
		return nil
	}
	return &val
}
