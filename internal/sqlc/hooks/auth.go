package hooks

import (
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// UserBeforeCreate prepares a user for creation
func UserBeforeCreate(params *db.CreateUserParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}

	// Hash password if provided
	if params.Password != "" {
		hashedPassword, err := crypto.HashPassword(params.Password)
		if err != nil {
			return err
		}
		params.Password = hashedPassword
	}

	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now

	return nil
}

// TokenBeforeCreate prepares a token for creation
func TokenBeforeCreate(params *db.CreateTokenParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	params.CreatedAt = apptime.NowString()
	return nil
}

// APIKeyBeforeCreate prepares an API key for creation
func APIKeyBeforeCreate(params *db.CreateAPIKeyParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now
	return nil
}

// VerifyPassword checks if the provided password matches the stored hash
func VerifyPassword(hashedPassword, password string) bool {
	return crypto.ComparePassword(hashedPassword, password) == nil
}

// HashPassword hashes a password
func HashPassword(password string) (string, error) {
	return crypto.HashPassword(password)
}
