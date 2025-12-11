package middleware

import (
	"github.com/golang-jwt/jwt/v5"
)

type Claims struct {
	UserID string   `json:"user_id"`
	Email  string   `json:"email"`
	Roles  []string `json:"roles"` // Array of role names from IAM
	jwt.RegisteredClaims
}
