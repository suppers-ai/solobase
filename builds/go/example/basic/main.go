// Package main provides a minimal bootstrap example for Solobase.
//
// This example shows how to configure and run Solobase with custom options.
// For production deployments, ensure you set proper environment variables
// or configuration values.
package main

import (
	"log"
	"os"

	"github.com/suppers-ai/solobase"
)

func main() {
	// Create app with custom options
	app := solobase.NewWithOptions(&solobase.Options{
		DatabaseType:         getEnv("DATABASE_TYPE", "sqlite"),
		DatabaseURL:          getEnv("DATABASE_URL", "file:./data/solobase.db"),
		JWTSecret:            getEnv("JWT_SECRET", "change-me-in-production-minimum-32-chars"),
		Port:                 getEnv("PORT", "8090"),
		DefaultAdminEmail:    getEnv("DEFAULT_ADMIN_EMAIL", ""),
		DefaultAdminPassword: getEnv("DEFAULT_ADMIN_PASSWORD", ""),
	})

	if err := app.Initialize(); err != nil {
		log.Fatalf("Failed to initialize: %v", err)
	}

	log.Printf("Starting Solobase on port %s...", getEnv("PORT", "8090"))
	if err := app.Start(); err != nil {
		log.Fatalf("Failed to start: %v", err)
	}
}

func getEnv(key, defaultVal string) string {
	if val := os.Getenv(key); val != "" {
		return val
	}
	return defaultVal
}
