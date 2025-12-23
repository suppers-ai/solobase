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

type userRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewUserRepository creates a new SQLite user repository
func NewUserRepository(sqlDB *sql.DB, queries *db.Queries) repos.UserRepository {
	return &userRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *userRepository) GetByID(ctx context.Context, id string) (*auth.User, error) {
	dbUser, err := r.queries.GetUserByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) GetByEmail(ctx context.Context, email string) (*auth.User, error) {
	dbUser, err := r.queries.GetUserByEmail(ctx, email)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) GetByUsername(ctx context.Context, username string) (*auth.User, error) {
	dbUser, err := r.queries.GetUserByUsername(ctx, &username)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) Create(ctx context.Context, user *auth.User) (*auth.User, error) {
	now := apptime.NowString()
	if user.ID == uuid.Nil {
		user.ID = uuid.New()
	}

	confirmed := int64(0)
	if user.Confirmed {
		confirmed = 1
	}

	dbUser, err := r.queries.CreateUser(ctx, db.CreateUserParams{
		ID:          user.ID.String(),
		Email:       user.Email,
		Password:    user.Password,
		Username:    strPtr(user.Username),
		Confirmed:   &confirmed,
		FirstName:   strPtr(user.FirstName),
		LastName:    strPtr(user.LastName),
		DisplayName: strPtr(user.DisplayName),
		Phone:       strPtr(user.Phone),
		Location:    strPtr(user.Location),
		Metadata:    strPtr(user.Metadata),
		CreatedAt:   now,
		UpdatedAt:   now,
	})
	if err != nil {
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) Update(ctx context.Context, user *auth.User) error {
	now := apptime.NowString()
	confirmed := int64(0)
	if user.Confirmed {
		confirmed = 1
	}

	return r.queries.UpdateUser(ctx, db.UpdateUserParams{
		ID:          user.ID.String(),
		Email:       user.Email,
		Username:    strPtr(user.Username),
		Confirmed:   &confirmed,
		FirstName:   strPtr(user.FirstName),
		LastName:    strPtr(user.LastName),
		DisplayName: strPtr(user.DisplayName),
		Phone:       strPtr(user.Phone),
		Location:    strPtr(user.Location),
		Metadata:    strPtr(user.Metadata),
		UpdatedAt:   now,
	})
}

func (r *userRepository) SoftDelete(ctx context.Context, id string) error {
	now := apptime.NowString()
	return r.queries.SoftDeleteUser(ctx, db.SoftDeleteUserParams{
		ID:        id,
		DeletedAt: apptime.NewNullTimeNow(),
		UpdatedAt: now,
	})
}

func (r *userRepository) HardDelete(ctx context.Context, id string) error {
	return r.queries.HardDeleteUser(ctx, id)
}

func (r *userRepository) List(ctx context.Context, opts repos.ListUsersOptions) (*repos.PaginatedResult[*auth.User], error) {
	total, err := r.queries.CountUsers(ctx)
	if err != nil {
		return nil, err
	}

	dbUsers, err := r.queries.ListUsers(ctx, db.ListUsersParams{
		Limit:  int64(opts.Limit),
		Offset: int64(opts.Offset),
	})
	if err != nil {
		return nil, err
	}

	users := make([]*auth.User, len(dbUsers))
	for i, u := range dbUsers {
		users[i] = convertDBUserToModel(u)
	}

	return &repos.PaginatedResult[*auth.User]{
		Items: users,
		Total: total,
	}, nil
}

func (r *userRepository) Count(ctx context.Context) (int64, error) {
	return r.queries.CountUsers(ctx)
}

func (r *userRepository) UpdatePassword(ctx context.Context, id, hashedPassword string) error {
	now := apptime.NowString()
	return r.queries.UpdateUserPassword(ctx, db.UpdateUserPasswordParams{
		ID:        id,
		Password:  hashedPassword,
		UpdatedAt: now,
	})
}

func (r *userRepository) UpdateLastLogin(ctx context.Context, id string, loginTime apptime.Time) error {
	now := apptime.NowString()
	return r.queries.UpdateUserLastLogin(ctx, db.UpdateUserLastLoginParams{
		ID:        id,
		LastLogin: apptime.NewNullTime(loginTime),
		UpdatedAt: now,
	})
}

func (r *userRepository) UpdateLoginAttempt(ctx context.Context, id string, count int, lastAttempt apptime.NullTime) error {
	now := apptime.NowString()
	attemptCount := int64(count)
	return r.queries.UpdateUserLoginAttempt(ctx, db.UpdateUserLoginAttemptParams{
		ID:           id,
		AttemptCount: &attemptCount,
		LastAttempt:  lastAttempt,
		UpdatedAt:    now,
	})
}

func (r *userRepository) SetConfirmToken(ctx context.Context, id, token, selector string) error {
	now := apptime.NowString()
	return r.queries.SetUserConfirmToken(ctx, db.SetUserConfirmTokenParams{
		ID:              id,
		ConfirmToken:    &token,
		ConfirmSelector: &selector,
		UpdatedAt:       now,
	})
}

func (r *userRepository) ClearConfirmToken(ctx context.Context, id string) error {
	now := apptime.NowString()
	confirmed := int64(1)
	return r.queries.UpdateUserConfirmation(ctx, db.UpdateUserConfirmationParams{
		ID:        id,
		Confirmed: &confirmed,
		UpdatedAt: now,
	})
}

func (r *userRepository) GetByConfirmSelector(ctx context.Context, selector string) (*auth.User, error) {
	dbUser, err := r.queries.GetUserByConfirmSelector(ctx, &selector)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) SetRecoverToken(ctx context.Context, id, token, selector string, expiry apptime.NullTime) error {
	now := apptime.NowString()
	return r.queries.SetUserRecoverToken(ctx, db.SetUserRecoverTokenParams{
		ID:              id,
		RecoverToken:    &token,
		RecoverSelector: &selector,
		RecoverTokenExp: expiry,
		UpdatedAt:       now,
	})
}

func (r *userRepository) ClearRecoverToken(ctx context.Context, id string) error {
	now := apptime.NowString()
	return r.queries.ClearUserRecoverToken(ctx, db.ClearUserRecoverTokenParams{
		ID:        id,
		UpdatedAt: now,
	})
}

func (r *userRepository) GetByRecoverSelector(ctx context.Context, selector string) (*auth.User, error) {
	dbUser, err := r.queries.GetUserByRecoverSelector(ctx, &selector)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBUserToModel(dbUser), nil
}

func (r *userRepository) SetTOTP(ctx context.Context, id, secret, backup string, recoveryCodes *string) error {
	now := apptime.NowString()
	return r.queries.SetUserTOTP(ctx, db.SetUserTOTPParams{
		ID:               id,
		TotpSecret:       &secret,
		TotpSecretBackup: &backup,
		RecoveryCodes:    recoveryCodes,
		UpdatedAt:        now,
	})
}

func (r *userRepository) ClearTOTP(ctx context.Context, id string) error {
	now := apptime.NowString()
	return r.queries.ClearUserTOTP(ctx, db.ClearUserTOTPParams{
		ID:        id,
		UpdatedAt: now,
	})
}

func (r *userRepository) SetSMSPhone(ctx context.Context, id, phone string) error {
	now := apptime.NowString()
	return r.queries.SetUserSMSPhone(ctx, db.SetUserSMSPhoneParams{
		ID:             id,
		SmsPhoneNumber: &phone,
		UpdatedAt:      now,
	})
}

// Helper functions

func strPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func convertDBUserToModel(dbUser db.AuthUser) *auth.User {
	var confirmed bool
	if dbUser.Confirmed != nil && *dbUser.Confirmed == 1 {
		confirmed = true
	}

	var username, firstName, lastName, displayName, phone, location, metadata string
	if dbUser.Username != nil {
		username = *dbUser.Username
	}
	if dbUser.FirstName != nil {
		firstName = *dbUser.FirstName
	}
	if dbUser.LastName != nil {
		lastName = *dbUser.LastName
	}
	if dbUser.DisplayName != nil {
		displayName = *dbUser.DisplayName
	}
	if dbUser.Phone != nil {
		phone = *dbUser.Phone
	}
	if dbUser.Location != nil {
		location = *dbUser.Location
	}
	if dbUser.Metadata != nil {
		metadata = *dbUser.Metadata
	}

	var attemptCount int
	if dbUser.AttemptCount != nil {
		attemptCount = int(*dbUser.AttemptCount)
	}

	return &auth.User{
		ID:              uuid.MustParse(dbUser.ID),
		Email:           dbUser.Email,
		Password:        dbUser.Password,
		Username:        username,
		Confirmed:       confirmed,
		FirstName:       firstName,
		LastName:        lastName,
		DisplayName:     displayName,
		Phone:           phone,
		Location:        location,
		ConfirmToken:    dbUser.ConfirmToken,
		ConfirmSelector: dbUser.ConfirmSelector,
		RecoverToken:    dbUser.RecoverToken,
		RecoverTokenExp: dbUser.RecoverTokenExp,
		RecoverSelector: dbUser.RecoverSelector,
		AttemptCount:    attemptCount,
		LastAttempt:     dbUser.LastAttempt,
		LastLogin:       dbUser.LastLogin,
		Metadata:        metadata,
		CreatedAt:       apptime.MustParse(dbUser.CreatedAt),
		UpdatedAt:       apptime.MustParse(dbUser.UpdatedAt),
		DeletedAt:       dbUser.DeletedAt,
		TOTPSecret:      dbUser.TotpSecret,
		TOTPSecretBackup: dbUser.TotpSecretBackup,
		SMSPhoneNumber:  dbUser.SmsPhoneNumber,
		RecoveryCodes:   dbUser.RecoveryCodes,
	}
}

// Ensure userRepository implements UserRepository
var _ repos.UserRepository = (*userRepository)(nil)
