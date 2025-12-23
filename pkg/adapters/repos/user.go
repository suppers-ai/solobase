package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
)

// ListUsersOptions configures user listing
type ListUsersOptions struct {
	Pagination
	// Future: filters, sorting
}

// UserRepository provides user data operations
type UserRepository interface {
	// Core CRUD
	GetByID(ctx context.Context, id string) (*auth.User, error)
	GetByEmail(ctx context.Context, email string) (*auth.User, error)
	GetByUsername(ctx context.Context, username string) (*auth.User, error)
	Create(ctx context.Context, user *auth.User) (*auth.User, error)
	Update(ctx context.Context, user *auth.User) error
	SoftDelete(ctx context.Context, id string) error
	HardDelete(ctx context.Context, id string) error

	// Listing with pagination
	List(ctx context.Context, opts ListUsersOptions) (*PaginatedResult[*auth.User], error)
	Count(ctx context.Context) (int64, error)

	// Authentication-specific
	UpdatePassword(ctx context.Context, id, hashedPassword string) error
	UpdateLastLogin(ctx context.Context, id string, loginTime apptime.Time) error
	UpdateLoginAttempt(ctx context.Context, id string, count int, lastAttempt apptime.NullTime) error

	// Confirmation
	SetConfirmToken(ctx context.Context, id, token, selector string) error
	ClearConfirmToken(ctx context.Context, id string) error
	GetByConfirmSelector(ctx context.Context, selector string) (*auth.User, error)

	// Recovery
	SetRecoverToken(ctx context.Context, id, token, selector string, expiry apptime.NullTime) error
	ClearRecoverToken(ctx context.Context, id string) error
	GetByRecoverSelector(ctx context.Context, selector string) (*auth.User, error)

	// 2FA
	SetTOTP(ctx context.Context, id, secret, backup string, recoveryCodes *string) error
	ClearTOTP(ctx context.Context, id string) error
	SetSMSPhone(ctx context.Context, id, phone string) error
}
