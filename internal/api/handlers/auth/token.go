package auth

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	commonjwt "github.com/suppers-ai/solobase/internal/common/jwt"
	"github.com/suppers-ai/solobase/internal/constants"
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

// createSignedToken creates a signed JWT token
func createSignedToken(userID, email string, roles []string) (string, error) {
	now := apptime.NowTime()
	claims := crypto.JWTClaims{
		UserID: userID,
		Email:  email,
		Roles:  roles,
		Exp:    now.Add(constants.AccessTokenDuration).Unix(),
		Iat:    now.Unix(),
	}

	secret := string(commonjwt.GetJWTSecret())
	return crypto.CreateToken(claims, secret)
}
