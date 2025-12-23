//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type tokenRepository struct{}

func (r *tokenRepository) Create(ctx context.Context, token *auth.Token) (*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) GetByID(ctx context.Context, id string) (*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) GetByHash(ctx context.Context, hash string) (*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) GetByToken(ctx context.Context, token string) (*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) GetByProviderUID(ctx context.Context, provider, uid string) (*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) ListByUserID(ctx context.Context, userID string) ([]*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) ListByFamily(ctx context.Context, familyID string) ([]*auth.Token, error) {
	return nil, ErrNotImplemented
}

func (r *tokenRepository) UpdateUsed(ctx context.Context, id string, usedAt apptime.Time) error {
	return ErrNotImplemented
}

func (r *tokenRepository) Revoke(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *tokenRepository) RevokeByUserID(ctx context.Context, userID string) error {
	return ErrNotImplemented
}

func (r *tokenRepository) RevokeByFamily(ctx context.Context, familyID string) error {
	return ErrNotImplemented
}

func (r *tokenRepository) RevokeByType(ctx context.Context, userID, tokenType string) error {
	return ErrNotImplemented
}

func (r *tokenRepository) Delete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *tokenRepository) DeleteExpired(ctx context.Context) error {
	return ErrNotImplemented
}

// API Key Repository

type apiKeyRepository struct{}

func (r *apiKeyRepository) Create(ctx context.Context, key *auth.APIKey) (*auth.APIKey, error) {
	return nil, ErrNotImplemented
}

func (r *apiKeyRepository) GetByID(ctx context.Context, id string) (*auth.APIKey, error) {
	return nil, ErrNotImplemented
}

func (r *apiKeyRepository) GetByHash(ctx context.Context, hash string) (*auth.APIKey, error) {
	return nil, ErrNotImplemented
}

func (r *apiKeyRepository) GetByPrefix(ctx context.Context, prefix string) (*auth.APIKey, error) {
	return nil, ErrNotImplemented
}

func (r *apiKeyRepository) ListByUserID(ctx context.Context, userID string) ([]*auth.APIKey, error) {
	return nil, ErrNotImplemented
}

func (r *apiKeyRepository) Update(ctx context.Context, key *auth.APIKey) error {
	return ErrNotImplemented
}

func (r *apiKeyRepository) UpdateLastUsed(ctx context.Context, id string, lastUsed apptime.Time, ip string) error {
	return ErrNotImplemented
}

func (r *apiKeyRepository) Revoke(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *apiKeyRepository) RevokeByUserID(ctx context.Context, userID string) error {
	return ErrNotImplemented
}

func (r *apiKeyRepository) Delete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

// Ensure implementations
var _ repos.TokenRepository = (*tokenRepository)(nil)
var _ repos.APIKeyRepository = (*apiKeyRepository)(nil)
