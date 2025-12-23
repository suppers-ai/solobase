//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type tokenRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewTokenRepository creates a new SQLite token repository
func NewTokenRepository(sqlDB *sql.DB, queries *db.Queries) repos.TokenRepository {
	return &tokenRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *tokenRepository) Create(ctx context.Context, token *auth.Token) (*auth.Token, error) {
	now := apptime.NowString()
	if token.ID == uuid.Nil {
		token.ID = uuid.New()
	}

	var familyID, provider, providerUID, accessToken, deviceInfo, ipAddress *string
	if token.FamilyID != nil {
		s := token.FamilyID.String()
		familyID = &s
	}
	if token.Provider != nil {
		provider = token.Provider
	}
	if token.ProviderUID != nil {
		providerUID = token.ProviderUID
	}
	if token.AccessToken != nil {
		accessToken = token.AccessToken
	}
	if token.DeviceInfo != nil {
		deviceInfo = token.DeviceInfo
	}
	if token.IPAddress != nil {
		ipAddress = token.IPAddress
	}

	var tokenHash, tokenStr *string
	if token.TokenHash != "" {
		tokenHash = &token.TokenHash
	}
	if token.Token != "" {
		tokenStr = &token.Token
	}

	dbToken, err := r.queries.CreateToken(ctx, db.CreateTokenParams{
		ID:          token.ID.String(),
		UserID:      token.UserID.String(),
		TokenHash:   tokenHash,
		Token:       tokenStr,
		Type:        token.Type,
		FamilyID:    familyID,
		Provider:    provider,
		ProviderUid: providerUID,
		AccessToken: accessToken,
		OauthExpiry: token.OAuthExpiry,
		ExpiresAt:   apptime.NewNullTime(token.ExpiresAt),
		CreatedAt:   now,
		DeviceInfo:  deviceInfo,
		IpAddress:   ipAddress,
	})
	if err != nil {
		return nil, err
	}
	return convertDBTokenToModel(dbToken), nil
}

func (r *tokenRepository) GetByID(ctx context.Context, id string) (*auth.Token, error) {
	dbToken, err := r.queries.GetTokenByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBTokenToModel(dbToken), nil
}

func (r *tokenRepository) GetByHash(ctx context.Context, hash string) (*auth.Token, error) {
	dbToken, err := r.queries.GetTokenByHash(ctx, &hash)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBTokenToModel(dbToken), nil
}

func (r *tokenRepository) GetByToken(ctx context.Context, token string) (*auth.Token, error) {
	dbToken, err := r.queries.GetTokenByToken(ctx, &token)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBTokenToModel(dbToken), nil
}

func (r *tokenRepository) GetByProviderUID(ctx context.Context, provider, uid string) (*auth.Token, error) {
	dbToken, err := r.queries.GetTokenByProviderUID(ctx, db.GetTokenByProviderUIDParams{
		Provider:    &provider,
		ProviderUid: &uid,
	})
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBTokenToModel(dbToken), nil
}

func (r *tokenRepository) ListByUserID(ctx context.Context, userID string) ([]*auth.Token, error) {
	dbTokens, err := r.queries.ListTokensByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}
	tokens := make([]*auth.Token, len(dbTokens))
	for i, t := range dbTokens {
		tokens[i] = convertDBTokenToModel(t)
	}
	return tokens, nil
}

func (r *tokenRepository) ListByFamily(ctx context.Context, familyID string) ([]*auth.Token, error) {
	dbTokens, err := r.queries.ListTokensByFamily(ctx, &familyID)
	if err != nil {
		return nil, err
	}
	tokens := make([]*auth.Token, len(dbTokens))
	for i, t := range dbTokens {
		tokens[i] = convertDBTokenToModel(t)
	}
	return tokens, nil
}

func (r *tokenRepository) UpdateUsed(ctx context.Context, id string, usedAt apptime.Time) error {
	return r.queries.UpdateTokenUsed(ctx, db.UpdateTokenUsedParams{
		ID:     id,
		UsedAt: apptime.NewNullTime(usedAt),
	})
}

func (r *tokenRepository) Revoke(ctx context.Context, id string) error {
	return r.queries.RevokeToken(ctx, db.RevokeTokenParams{
		ID:        id,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *tokenRepository) RevokeByUserID(ctx context.Context, userID string) error {
	return r.queries.RevokeTokensByUserID(ctx, db.RevokeTokensByUserIDParams{
		UserID:    userID,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *tokenRepository) RevokeByFamily(ctx context.Context, familyID string) error {
	return r.queries.RevokeTokensByFamily(ctx, db.RevokeTokensByFamilyParams{
		FamilyID:  &familyID,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *tokenRepository) RevokeByType(ctx context.Context, userID, tokenType string) error {
	return r.queries.RevokeTokensByType(ctx, db.RevokeTokensByTypeParams{
		UserID:    userID,
		Type:      tokenType,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *tokenRepository) Delete(ctx context.Context, id string) error {
	return r.queries.DeleteToken(ctx, id)
}

func (r *tokenRepository) DeleteExpired(ctx context.Context) error {
	return r.queries.DeleteExpiredTokens(ctx, apptime.NewNullTimeNow())
}

// API Key Repository

type apiKeyRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewAPIKeyRepository creates a new SQLite API key repository
func NewAPIKeyRepository(sqlDB *sql.DB, queries *db.Queries) repos.APIKeyRepository {
	return &apiKeyRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *apiKeyRepository) Create(ctx context.Context, key *auth.APIKey) (*auth.APIKey, error) {
	now := apptime.NowString()
	if key.ID == uuid.Nil {
		key.ID = uuid.New()
	}

	scopes := []byte(key.Scopes)

	dbKey, err := r.queries.CreateAPIKey(ctx, db.CreateAPIKeyParams{
		ID:        key.ID.String(),
		UserID:    key.UserID.String(),
		Name:      key.Name,
		KeyPrefix: key.KeyPrefix,
		KeyHash:   key.KeyHash,
		Scopes:    scopes,
		ExpiresAt: key.ExpiresAt,
		CreatedAt: now,
		UpdatedAt: now,
	})
	if err != nil {
		return nil, err
	}
	return convertDBAPIKeyToModel(dbKey), nil
}

func (r *apiKeyRepository) GetByID(ctx context.Context, id string) (*auth.APIKey, error) {
	dbKey, err := r.queries.GetAPIKeyByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBAPIKeyToModel(dbKey), nil
}

func (r *apiKeyRepository) GetByHash(ctx context.Context, hash string) (*auth.APIKey, error) {
	dbKey, err := r.queries.GetAPIKeyByHash(ctx, hash)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBAPIKeyToModel(dbKey), nil
}

func (r *apiKeyRepository) GetByPrefix(ctx context.Context, prefix string) (*auth.APIKey, error) {
	dbKey, err := r.queries.GetAPIKeyByPrefix(ctx, prefix)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBAPIKeyToModel(dbKey), nil
}

func (r *apiKeyRepository) ListByUserID(ctx context.Context, userID string) ([]*auth.APIKey, error) {
	dbKeys, err := r.queries.ListAPIKeysByUserID(ctx, userID)
	if err != nil {
		return nil, err
	}
	keys := make([]*auth.APIKey, len(dbKeys))
	for i, k := range dbKeys {
		keys[i] = convertDBAPIKeyToModel(k)
	}
	return keys, nil
}

func (r *apiKeyRepository) Update(ctx context.Context, key *auth.APIKey) error {
	now := apptime.NowString()
	scopes := []byte(key.Scopes)

	return r.queries.UpdateAPIKey(ctx, db.UpdateAPIKeyParams{
		ID:        key.ID.String(),
		Name:      key.Name,
		Scopes:    scopes,
		ExpiresAt: key.ExpiresAt,
		UpdatedAt: now,
	})
}

func (r *apiKeyRepository) UpdateLastUsed(ctx context.Context, id string, lastUsed apptime.Time, ip string) error {
	return r.queries.UpdateAPIKeyLastUsed(ctx, db.UpdateAPIKeyLastUsedParams{
		ID:         id,
		LastUsedAt: apptime.NewNullTime(lastUsed),
		LastUsedIp: &ip,
	})
}

func (r *apiKeyRepository) Revoke(ctx context.Context, id string) error {
	return r.queries.RevokeAPIKey(ctx, db.RevokeAPIKeyParams{
		ID:        id,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *apiKeyRepository) RevokeByUserID(ctx context.Context, userID string) error {
	return r.queries.RevokeAPIKeysByUserID(ctx, db.RevokeAPIKeysByUserIDParams{
		UserID:    userID,
		RevokedAt: apptime.NewNullTimeNow(),
	})
}

func (r *apiKeyRepository) Delete(ctx context.Context, id string) error {
	return r.queries.DeleteAPIKey(ctx, id)
}

// Conversion helpers

func convertDBTokenToModel(dbToken db.AuthToken) *auth.Token {
	var familyID *uuid.UUID
	if dbToken.FamilyID != nil {
		id := uuid.MustParse(*dbToken.FamilyID)
		familyID = &id
	}

	var tokenHash, tokenStr string
	if dbToken.TokenHash != nil {
		tokenHash = *dbToken.TokenHash
	}
	if dbToken.Token != nil {
		tokenStr = *dbToken.Token
	}

	var expiresAt apptime.Time
	if dbToken.ExpiresAt.Valid {
		expiresAt = dbToken.ExpiresAt.Time
	}

	return &auth.Token{
		ID:          uuid.MustParse(dbToken.ID),
		UserID:      uuid.MustParse(dbToken.UserID),
		TokenHash:   tokenHash,
		Token:       tokenStr,
		Type:        dbToken.Type,
		FamilyID:    familyID,
		Provider:    dbToken.Provider,
		ProviderUID: dbToken.ProviderUid,
		AccessToken: dbToken.AccessToken,
		OAuthExpiry: dbToken.OauthExpiry,
		ExpiresAt:   expiresAt,
		UsedAt:      dbToken.UsedAt,
		RevokedAt:   dbToken.RevokedAt,
		CreatedAt:   apptime.MustParse(dbToken.CreatedAt),
		DeviceInfo:  dbToken.DeviceInfo,
		IPAddress:   dbToken.IpAddress,
	}
}

func convertDBAPIKeyToModel(dbKey db.ApiKey) *auth.APIKey {
	return &auth.APIKey{
		ID:         uuid.MustParse(dbKey.ID),
		UserID:     uuid.MustParse(dbKey.UserID),
		Name:       dbKey.Name,
		KeyPrefix:  dbKey.KeyPrefix,
		KeyHash:    dbKey.KeyHash,
		Scopes:     string(dbKey.Scopes),
		ExpiresAt:  dbKey.ExpiresAt,
		LastUsedAt: dbKey.LastUsedAt,
		LastUsedIP: dbKey.LastUsedIp,
		RevokedAt:  dbKey.RevokedAt,
		CreatedAt:  apptime.MustParse(dbKey.CreatedAt),
		UpdatedAt:  apptime.MustParse(dbKey.UpdatedAt),
	}
}

// Ensure implementations
var _ repos.TokenRepository = (*tokenRepository)(nil)
var _ repos.APIKeyRepository = (*apiKeyRepository)(nil)
