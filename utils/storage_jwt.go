package utils

import (
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
)

// StorageClaims represents JWT claims for storage operations
type StorageClaims struct {
	UserID string `json:"userId"`
	Email  string `json:"email"`
	Role   string `json:"role"`
	Exp    int64  `json:"exp"`
	Iat    int64  `json:"iat"`
}

// parseStorageToken parses and validates a JWT token for storage operations
func parseStorageToken(tokenString string) (string, error) {
	secret := string(commonjwt.GetJWTSecret())
	claims, err := crypto.VerifyToken(tokenString, secret)
	if err != nil {
		return "", err
	}
	return claims.UserID, nil
}
