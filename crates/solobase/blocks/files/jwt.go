package files

import (
	cryptosvc "github.com/wafer-run/wafer-go/services/crypto"
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
func parseStorageToken(svc cryptosvc.Service, tokenString string) (string, error) {
	claims, err := svc.Verify(tokenString)
	if err != nil {
		return "", err
	}
	userID, _ := claims["user_id"].(string)
	return userID, nil
}
