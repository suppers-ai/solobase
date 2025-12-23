package middleware

import (
	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
)

// Claims for JWT tokens
type Claims struct {
	UserID string   `json:"userId"`
	Email  string   `json:"email"`
	Roles  []string `json:"roles"`
	Exp    int64    `json:"exp"`
	Iat    int64    `json:"iat"`
}

// parseToken parses and validates a JWT token, returning the claims
func parseToken(tokenString string) (*Claims, error) {
	secret := string(commonjwt.GetJWTSecret())
	jwtClaims, err := crypto.VerifyToken(tokenString, secret)
	if err != nil {
		return nil, err
	}

	return &Claims{
		UserID: jwtClaims.UserID,
		Email:  jwtClaims.Email,
		Roles:  jwtClaims.Roles,
		Exp:    jwtClaims.Exp,
		Iat:    jwtClaims.Iat,
	}, nil
}
