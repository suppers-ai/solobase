package jwt

import (
	"fmt"
	"log"
	"sync"

	"github.com/suppers-ai/solobase/internal/env"
)

var (
	jwtSecret []byte
	once      sync.Once
)

// InitJWTSecret initializes the JWT secret with proper fallback logic
// This ensures consistent JWT secret handling across the application
func InitJWTSecret() {
	once.Do(func() {
		if err := SetJWTSecret(""); err != nil {
			log.Printf("WARNING: Failed to initialize JWT secret: %v", err)
		}
	})
}

// SetJWTSecret sets the JWT secret for authentication
func SetJWTSecret(secret string) error {
	if secret == "" {
		// Use environment variable as fallback
		secret = env.GetEnv("JWT_SECRET")
	}
	if secret == "" {
		// Use a default for development only
		if env.GetEnv("ENVIRONMENT") == "development" {
			secret = "dev-secret-key-do-not-use-in-production"
			log.Println("WARNING: Using default JWT secret for development. DO NOT use in production!")
		}
	}
	if secret == "" {
		return fmt.Errorf("JWT secret is required. Set JWT_SECRET environment variable or pass it in configuration")
	}
	jwtSecret = []byte(secret)
	return nil
}

// GetJWTSecret returns the JWT secret, initializing it if necessary
func GetJWTSecret() []byte {
	InitJWTSecret()
	return jwtSecret
}

// IsInitialized returns whether the JWT secret has been initialized
func IsInitialized() bool {
	return len(jwtSecret) > 0
}
