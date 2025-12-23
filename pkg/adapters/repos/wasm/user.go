//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type userRepository struct{}

// Core CRUD

func (r *userRepository) GetByID(ctx context.Context, id string) (*auth.User, error) {
	return nil, ErrNotImplemented
}

func (r *userRepository) GetByEmail(ctx context.Context, email string) (*auth.User, error) {
	return nil, ErrNotImplemented
}

func (r *userRepository) GetByUsername(ctx context.Context, username string) (*auth.User, error) {
	return nil, ErrNotImplemented
}

func (r *userRepository) Create(ctx context.Context, user *auth.User) (*auth.User, error) {
	return nil, ErrNotImplemented
}

func (r *userRepository) Update(ctx context.Context, user *auth.User) error {
	return ErrNotImplemented
}

func (r *userRepository) SoftDelete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *userRepository) HardDelete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

// Listing with pagination

func (r *userRepository) List(ctx context.Context, opts repos.ListUsersOptions) (*repos.PaginatedResult[*auth.User], error) {
	return nil, ErrNotImplemented
}

func (r *userRepository) Count(ctx context.Context) (int64, error) {
	return 0, ErrNotImplemented
}

// Authentication-specific

func (r *userRepository) UpdatePassword(ctx context.Context, id, hashedPassword string) error {
	return ErrNotImplemented
}

func (r *userRepository) UpdateLastLogin(ctx context.Context, id string, loginTime apptime.Time) error {
	return ErrNotImplemented
}

func (r *userRepository) UpdateLoginAttempt(ctx context.Context, id string, count int, lastAttempt apptime.NullTime) error {
	return ErrNotImplemented
}

// Confirmation

func (r *userRepository) SetConfirmToken(ctx context.Context, id, token, selector string) error {
	return ErrNotImplemented
}

func (r *userRepository) ClearConfirmToken(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *userRepository) GetByConfirmSelector(ctx context.Context, selector string) (*auth.User, error) {
	return nil, ErrNotImplemented
}

// Recovery

func (r *userRepository) SetRecoverToken(ctx context.Context, id, token, selector string, expiry apptime.NullTime) error {
	return ErrNotImplemented
}

func (r *userRepository) ClearRecoverToken(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *userRepository) GetByRecoverSelector(ctx context.Context, selector string) (*auth.User, error) {
	return nil, ErrNotImplemented
}

// 2FA

func (r *userRepository) SetTOTP(ctx context.Context, id, secret, backup string, recoveryCodes *string) error {
	return ErrNotImplemented
}

func (r *userRepository) ClearTOTP(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *userRepository) SetSMSPhone(ctx context.Context, id, phone string) error {
	return ErrNotImplemented
}

// Ensure userRepository implements UserRepository
var _ repos.UserRepository = (*userRepository)(nil)
