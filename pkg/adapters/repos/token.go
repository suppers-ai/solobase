package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
)

// TokenRepository provides token operations
type TokenRepository interface {
	// CRUD
	Create(ctx context.Context, token *auth.Token) (*auth.Token, error)
	GetByID(ctx context.Context, id string) (*auth.Token, error)
	GetByHash(ctx context.Context, hash string) (*auth.Token, error)
	GetByToken(ctx context.Context, token string) (*auth.Token, error)
	GetByProviderUID(ctx context.Context, provider, uid string) (*auth.Token, error)

	// Listing
	ListByUserID(ctx context.Context, userID string) ([]*auth.Token, error)
	ListByFamily(ctx context.Context, familyID string) ([]*auth.Token, error)

	// Updates
	UpdateUsed(ctx context.Context, id string, usedAt apptime.Time) error
	Revoke(ctx context.Context, id string) error
	RevokeByUserID(ctx context.Context, userID string) error
	RevokeByFamily(ctx context.Context, familyID string) error
	RevokeByType(ctx context.Context, userID, tokenType string) error

	// Cleanup
	Delete(ctx context.Context, id string) error
	DeleteExpired(ctx context.Context) error
}

// APIKeyRepository provides API key operations
type APIKeyRepository interface {
	// CRUD
	Create(ctx context.Context, key *auth.APIKey) (*auth.APIKey, error)
	GetByID(ctx context.Context, id string) (*auth.APIKey, error)
	GetByHash(ctx context.Context, hash string) (*auth.APIKey, error)
	GetByPrefix(ctx context.Context, prefix string) (*auth.APIKey, error)

	// Listing
	ListByUserID(ctx context.Context, userID string) ([]*auth.APIKey, error)

	// Updates
	Update(ctx context.Context, key *auth.APIKey) error
	UpdateLastUsed(ctx context.Context, id string, lastUsed apptime.Time, ip string) error
	Revoke(ctx context.Context, id string) error
	RevokeByUserID(ctx context.Context, userID string) error

	// Delete
	Delete(ctx context.Context, id string) error
}
