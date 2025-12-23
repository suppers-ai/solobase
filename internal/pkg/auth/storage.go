//go:build !wasm

package auth

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/internal/sqlc/hooks"
)

// ErrUserNotFound is returned when a user is not found
var ErrUserNotFound = errors.New("user not found")

// Storage provides user storage using SQLC
type Storage struct {
	db      *sql.DB
	queries *db.Queries
}

// NewStorage creates a new SQLC-based storage
func NewStorage(sqlDB *sql.DB) *Storage {
	return &Storage{
		db:      sqlDB,
		queries: db.New(sqlDB),
	}
}

// Load loads a user by key (ID or email)
func (s *Storage) Load(ctx context.Context, key string) (*User, error) {
	var dbUser db.AuthUser
	var err error

	// Check if key is email or ID
	if strings.Contains(key, "@") {
		dbUser, err = s.queries.GetUserByEmail(ctx, key)
	} else {
		dbUser, err = s.queries.GetUserByID(ctx, key)
	}

	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrUserNotFound
		}
		return nil, err
	}

	return dbUserToUser(&dbUser), nil
}

// Save updates an existing user
func (s *Storage) Save(ctx context.Context, user *User) error {
	var confirmed *int64
	if user.Confirmed {
		one := int64(1)
		confirmed = &one
	}

	return s.queries.UpdateUser(ctx, db.UpdateUserParams{
		ID:          user.ID.String(),
		Email:       user.Email,
		Password:    user.Password,
		Username:    stringPtr(user.Username),
		Confirmed:   confirmed,
		FirstName:   stringPtr(user.FirstName),
		LastName:    stringPtr(user.LastName),
		DisplayName: stringPtr(user.DisplayName),
		Phone:       stringPtr(user.Phone),
		Location:    stringPtr(user.Location),
		Metadata:    stringPtr(user.Metadata),
		UpdatedAt:   apptime.NowString(),
	})
}

// Create inserts a new user
func (s *Storage) Create(ctx context.Context, user *User) error {
	var confirmed *int64
	if user.Confirmed {
		one := int64(1)
		confirmed = &one
	}

	params := db.CreateUserParams{
		ID:          user.ID.String(),
		Email:       user.Email,
		Password:    user.Password,
		Username:    stringPtr(user.Username),
		Confirmed:   confirmed,
		FirstName:   stringPtr(user.FirstName),
		LastName:    stringPtr(user.LastName),
		DisplayName: stringPtr(user.DisplayName),
		Phone:       stringPtr(user.Phone),
		Location:    stringPtr(user.Location),
		Metadata:    stringPtr(user.Metadata),
	}

	// Apply hooks (generates ID, hashes password, sets timestamps)
	if err := hooks.UserBeforeCreate(&params); err != nil {
		return err
	}

	dbUser, err := s.queries.CreateUser(ctx, params)
	if err != nil {
		return err
	}

	// Update user with generated values
	user.ID = uuid.MustParse(dbUser.ID)
	user.CreatedAt = apptime.NewTime(apptime.MustParse(dbUser.CreatedAt))
	user.UpdatedAt = apptime.NewTime(apptime.MustParse(dbUser.UpdatedAt))

	return nil
}

// Additional helper methods

// GetUserByID retrieves a user by ID
func (s *Storage) GetUserByID(ctx context.Context, id uuid.UUID) (*User, error) {
	dbUser, err := s.queries.GetUserByID(ctx, id.String())
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("user not found")
		}
		return nil, err
	}
	return dbUserToUser(&dbUser), nil
}

// GetUserByEmail retrieves a user by email
func (s *Storage) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	dbUser, err := s.queries.GetUserByEmail(ctx, email)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("user not found")
		}
		return nil, err
	}
	return dbUserToUser(&dbUser), nil
}

// ListUsers retrieves all users with optional filters
func (s *Storage) ListUsers(ctx context.Context, offset, limit int) ([]User, int64, error) {
	// Get total count
	total, err := s.queries.CountUsers(ctx)
	if err != nil {
		return nil, 0, err
	}

	// Get paginated results
	dbUsers, err := s.queries.ListUsers(ctx, db.ListUsersParams{
		Offset: int64(offset),
		Limit:  int64(limit),
	})
	if err != nil {
		return nil, 0, err
	}

	users := make([]User, len(dbUsers))
	for i, dbUser := range dbUsers {
		users[i] = *dbUserToUser(&dbUser)
	}

	return users, total, nil
}

// UpdateUser updates a user
func (s *Storage) UpdateUser(ctx context.Context, user *User) error {
	var confirmed *int64
	if user.Confirmed {
		one := int64(1)
		confirmed = &one
	}

	return s.queries.UpdateUser(ctx, db.UpdateUserParams{
		ID:          user.ID.String(),
		Email:       user.Email,
		Password:    user.Password,
		Username:    stringPtr(user.Username),
		Confirmed:   confirmed,
		FirstName:   stringPtr(user.FirstName),
		LastName:    stringPtr(user.LastName),
		DisplayName: stringPtr(user.DisplayName),
		Phone:       stringPtr(user.Phone),
		Location:    stringPtr(user.Location),
		Metadata:    stringPtr(user.Metadata),
		UpdatedAt:   apptime.NowString(),
	})
}

// DeleteUser soft deletes a user
func (s *Storage) DeleteUser(ctx context.Context, id uuid.UUID) error {
	return s.queries.SoftDeleteUser(ctx, db.SoftDeleteUserParams{
		DeletedAt: apptime.FromTimePtr(ptrTime(apptime.NowTime())),
		UpdatedAt: apptime.NowString(),
		ID:        id.String(),
	})
}

// Token management

// CreateToken creates a new token
func (s *Storage) CreateToken(ctx context.Context, token *Token) error {
	params := db.CreateTokenParams{
		UserID:      token.UserID.String(),
		TokenHash:   stringPtr(token.TokenHash),
		Token:       stringPtr(token.Token),
		Type:        token.Type,
		FamilyID:    uuidPtrToString(token.FamilyID),
		Provider:    token.Provider,
		ProviderUid: token.ProviderUID,
		AccessToken: token.AccessToken,
		OauthExpiry: token.OAuthExpiry,
		ExpiresAt:   apptime.NewNullTime(token.ExpiresAt),
		DeviceInfo:  token.DeviceInfo,
		IpAddress:   token.IPAddress,
	}

	if err := hooks.TokenBeforeCreate(&params); err != nil {
		return err
	}

	dbToken, err := s.queries.CreateToken(ctx, params)
	if err != nil {
		return err
	}

	token.ID = uuid.MustParse(dbToken.ID)
	token.CreatedAt = apptime.NewTime(apptime.MustParse(dbToken.CreatedAt))

	return nil
}

// GetToken retrieves a token by token string
func (s *Storage) GetToken(ctx context.Context, token string) (*Token, error) {
	dbToken, err := s.queries.GetTokenByToken(ctx, &token)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("token not found")
		}
		return nil, err
	}
	return dbTokenToToken(&dbToken), nil
}

// UseToken marks a token as used
func (s *Storage) UseToken(ctx context.Context, token string) error {
	dbToken, err := s.queries.GetTokenByToken(ctx, &token)
	if err != nil {
		return err
	}

	return s.queries.UpdateTokenUsed(ctx, db.UpdateTokenUsedParams{
		UsedAt: apptime.FromTimePtr(ptrTime(apptime.NowTime())),
		ID:     dbToken.ID,
	})
}

// DeleteExpiredTokens removes expired tokens
func (s *Storage) DeleteExpiredTokens(ctx context.Context) error {
	return s.queries.DeleteExpiredTokens(ctx, apptime.FromTimePtr(ptrTime(apptime.NowTime())))
}

// API Key management methods

// CreateAPIKey creates a new API key for a user
func (s *Storage) CreateAPIKey(ctx context.Context, apiKey *APIKey) error {
	params := db.CreateAPIKeyParams{
		UserID:    apiKey.UserID.String(),
		Name:      apiKey.Name,
		KeyPrefix: apiKey.KeyPrefix,
		KeyHash:   apiKey.KeyHash,
		Scopes:    []byte(apiKey.Scopes),
		ExpiresAt: apiKey.ExpiresAt,
	}

	if err := hooks.APIKeyBeforeCreate(&params); err != nil {
		return err
	}

	dbKey, err := s.queries.CreateAPIKey(ctx, params)
	if err != nil {
		return err
	}

	apiKey.ID = uuid.MustParse(dbKey.ID)
	apiKey.CreatedAt = apptime.NewTime(apptime.MustParse(dbKey.CreatedAt))
	apiKey.UpdatedAt = apptime.NewTime(apptime.MustParse(dbKey.UpdatedAt))

	return nil
}

// GetAPIKeyByHash retrieves an API key by its hash
func (s *Storage) GetAPIKeyByHash(ctx context.Context, keyHash string) (*APIKey, error) {
	dbKey, err := s.queries.GetAPIKeyByHash(ctx, keyHash)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("API key not found")
		}
		return nil, err
	}
	return dbAPIKeyToAPIKey(&dbKey), nil
}

// GetAPIKeysByUserID retrieves all API keys for a user
func (s *Storage) GetAPIKeysByUserID(ctx context.Context, userID uuid.UUID) ([]APIKey, error) {
	dbKeys, err := s.queries.ListAPIKeysByUserID(ctx, userID.String())
	if err != nil {
		return nil, err
	}

	keys := make([]APIKey, len(dbKeys))
	for i, dbKey := range dbKeys {
		keys[i] = *dbAPIKeyToAPIKey(&dbKey)
	}

	return keys, nil
}

// GetAPIKeyByID retrieves an API key by its ID
func (s *Storage) GetAPIKeyByID(ctx context.Context, id uuid.UUID) (*APIKey, error) {
	dbKey, err := s.queries.GetAPIKeyByID(ctx, id.String())
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("API key not found")
		}
		return nil, err
	}
	return dbAPIKeyToAPIKey(&dbKey), nil
}

// RevokeAPIKey revokes an API key
func (s *Storage) RevokeAPIKey(ctx context.Context, id uuid.UUID, userID uuid.UUID) error {
	// First verify the key belongs to the user
	dbKey, err := s.queries.GetAPIKeyByID(ctx, id.String())
	if err != nil {
		return fmt.Errorf("API key not found or already revoked")
	}
	if dbKey.UserID != userID.String() {
		return fmt.Errorf("API key not found or already revoked")
	}

	return s.queries.RevokeAPIKey(ctx, db.RevokeAPIKeyParams{
		RevokedAt: apptime.FromTimePtr(ptrTime(apptime.NowTime())),
		UpdatedAt: apptime.NowString(),
		ID:        id.String(),
	})
}

// UpdateAPIKeyLastUsed updates the last used timestamp and IP
func (s *Storage) UpdateAPIKeyLastUsed(ctx context.Context, id uuid.UUID, ip string) error {
	return s.queries.UpdateAPIKeyLastUsed(ctx, db.UpdateAPIKeyLastUsedParams{
		LastUsedAt: apptime.FromTimePtr(ptrTime(apptime.NowTime())),
		LastUsedIp: &ip,
		UpdatedAt:  apptime.NowString(),
		ID:         id.String(),
	})
}

// OAuth methods

// FindUserByOAuthToken finds a user by their OAuth token (provider + provider UID)
func (s *Storage) FindUserByOAuthToken(ctx context.Context, provider, providerUID string) (*User, error) {
	dbToken, err := s.queries.GetTokenByProviderUID(ctx, db.GetTokenByProviderUIDParams{
		ProviderUid: &providerUID,
		Provider:    &provider,
	})
	if err != nil {
		return nil, err
	}

	dbUser, err := s.queries.GetUserByID(ctx, dbToken.UserID)
	if err != nil {
		return nil, err
	}

	return dbUserToUser(&dbUser), nil
}

// CreateOrUpdateOAuthToken creates or updates an OAuth token for a user
func (s *Storage) CreateOrUpdateOAuthToken(ctx context.Context, userID uuid.UUID, provider, providerUID, accessToken string, expiry *apptime.Time) error {
	// Revoke any existing OAuth tokens for this provider/user combo
	_ = s.queries.RevokeTokensByType(ctx, db.RevokeTokensByTypeParams{
		RevokedAt: apptime.FromTimePtr(ptrTime(apptime.NowTime())),
		UserID:    userID.String(),
		Type:      TokenTypeOAuth,
	})

	// Create new OAuth token
	params := db.CreateTokenParams{
		UserID:      userID.String(),
		Type:        TokenTypeOAuth,
		Provider:    &provider,
		ProviderUid: &providerUID,
		AccessToken: &accessToken,
		OauthExpiry: apptime.FromTimePtr(expiry),
		ExpiresAt:   apptime.FromTimePtr(ptrTime(apptime.NowTime().Add(365 * 24 * apptime.Hour))),
	}

	if err := hooks.TokenBeforeCreate(&params); err != nil {
		return err
	}

	_, err := s.queries.CreateToken(ctx, params)
	return err
}

// Helper functions to convert between DB types and domain types

func dbUserToUser(dbUser *db.AuthUser) *User {
	user := &User{
		ID:               uuid.MustParse(dbUser.ID),
		Email:            dbUser.Email,
		Password:         dbUser.Password,
		Username:         derefString(dbUser.Username),
		Confirmed:        dbUser.Confirmed != nil && *dbUser.Confirmed == 1,
		FirstName:        derefString(dbUser.FirstName),
		LastName:         derefString(dbUser.LastName),
		DisplayName:      derefString(dbUser.DisplayName),
		Phone:            derefString(dbUser.Phone),
		Location:         derefString(dbUser.Location),
		ConfirmToken:     dbUser.ConfirmToken,
		ConfirmSelector:  dbUser.ConfirmSelector,
		RecoverToken:     dbUser.RecoverToken,
		RecoverSelector:  dbUser.RecoverSelector,
		Metadata:         derefString(dbUser.Metadata),
		CreatedAt:        apptime.NewTime(apptime.MustParse(dbUser.CreatedAt)),
		UpdatedAt:        apptime.NewTime(apptime.MustParse(dbUser.UpdatedAt)),
		TOTPSecret:       dbUser.TotpSecret,
		TOTPSecretBackup: dbUser.TotpSecretBackup,
		SMSPhoneNumber:   dbUser.SmsPhoneNumber,
		RecoveryCodes:    dbUser.RecoveryCodes,
	}

	if dbUser.AttemptCount != nil {
		user.AttemptCount = int(*dbUser.AttemptCount)
	}
	if dbUser.LastAttempt.Valid {
		user.LastAttempt = apptime.NewNullTime(dbUser.LastAttempt.Time)
	}
	if dbUser.LastLogin.Valid {
		user.LastLogin = apptime.NewNullTime(dbUser.LastLogin.Time)
	}
	if dbUser.RecoverTokenExp.Valid {
		user.RecoverTokenExp = apptime.NewNullTime(dbUser.RecoverTokenExp.Time)
	}
	if dbUser.DeletedAt.Valid {
		user.DeletedAt = apptime.NewNullTime(dbUser.DeletedAt.Time)
	}

	return user
}

func dbTokenToToken(dbToken *db.AuthToken) *Token {
	token := &Token{
		ID:          uuid.MustParse(dbToken.ID),
		UserID:      uuid.MustParse(dbToken.UserID),
		Type:        dbToken.Type,
		Provider:    dbToken.Provider,
		ProviderUID: dbToken.ProviderUid,
		AccessToken: dbToken.AccessToken,
		DeviceInfo:  dbToken.DeviceInfo,
		IPAddress:   dbToken.IpAddress,
		CreatedAt:   apptime.NewTime(apptime.MustParse(dbToken.CreatedAt)),
	}

	if dbToken.TokenHash != nil {
		token.TokenHash = *dbToken.TokenHash
	}
	if dbToken.Token != nil {
		token.Token = *dbToken.Token
	}
	if dbToken.FamilyID != nil {
		fid := uuid.MustParse(*dbToken.FamilyID)
		token.FamilyID = &fid
	}
	if dbToken.OauthExpiry.Valid {
		token.OAuthExpiry = apptime.NewNullTime(dbToken.OauthExpiry.Time)
	}
	if dbToken.ExpiresAt.Valid {
		token.ExpiresAt = dbToken.ExpiresAt.Time
	}
	if dbToken.UsedAt.Valid {
		token.UsedAt = apptime.NewNullTime(dbToken.UsedAt.Time)
	}
	if dbToken.RevokedAt.Valid {
		token.RevokedAt = apptime.NewNullTime(dbToken.RevokedAt.Time)
	}

	return token
}

func dbAPIKeyToAPIKey(dbKey *db.ApiKey) *APIKey {
	key := &APIKey{
		ID:        uuid.MustParse(dbKey.ID),
		UserID:    uuid.MustParse(dbKey.UserID),
		Name:      dbKey.Name,
		KeyPrefix: dbKey.KeyPrefix,
		KeyHash:   dbKey.KeyHash,
		Scopes:    string(dbKey.Scopes),
		CreatedAt: apptime.NewTime(apptime.MustParse(dbKey.CreatedAt)),
		UpdatedAt: apptime.NewTime(apptime.MustParse(dbKey.UpdatedAt)),
	}

	if dbKey.ExpiresAt.Valid {
		key.ExpiresAt = dbKey.ExpiresAt
	}
	if dbKey.LastUsedAt.Valid {
		key.LastUsedAt = apptime.NewNullTime(dbKey.LastUsedAt.Time)
	}
	if dbKey.LastUsedIp != nil {
		key.LastUsedIP = dbKey.LastUsedIp
	}
	if dbKey.RevokedAt.Valid {
		key.RevokedAt = apptime.NewNullTime(dbKey.RevokedAt.Time)
	}

	return key
}

// Helper functions

func stringPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func derefString(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}

func ptrTime(t apptime.Time) *apptime.Time {
	return &t
}

func uuidPtrToString(u *uuid.UUID) *string {
	if u == nil {
		return nil
	}
	s := u.String()
	return &s
}
