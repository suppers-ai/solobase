package config

import (
	"log"
	"os"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/utils"
)

type StorageConfig struct {
	Type             string
	S3Endpoint       string
	S3AccessKey      string
	S3SecretKey      string
	S3Bucket         string
	S3Region         string
	S3UseSSL         bool
	LocalStoragePath string
}

type Config struct {
	// Server
	Port        string
	Environment string

	// Database
	Database database.Config

	// Storage
	Storage StorageConfig

	// Mail
	SMTPHost     string
	SMTPPort     int
	SMTPUsername string
	SMTPPassword string
	SMTPFrom     string
	SMTPUseTLS   bool

	// Auth
	JWTSecret    string
	EnableSignup bool
	EnableAPI    bool

	// Admin
	AdminEmail     string
	AdminPassword  string
	DisableAdminUI bool

	// Logging
	LogLevel string

	// CORS
	CORSAllowedOrigins []string
	CORSAllowedMethods []string
	CORSAllowedHeaders []string

	// Rate limiting
	RateLimitEnabled           bool
	RateLimitRequestsPerMinute int
}

func Load() *Config {
	dbType := getEnv("DATABASE_TYPE", "sqlite")

	// Normalize database type to lowercase
	dbType = strings.ToLower(dbType)
	if dbType == "postgresql" {
		dbType = "postgres" // Normalize postgresql to postgres
	} else if dbType == "sqlite3" {
		dbType = "sqlite" // Normalize sqlite3 to sqlite
	}

	cfg := &Config{
		Port:        getEnv("PORT", "8080"),
		Environment: getEnv("ENVIRONMENT", "development"),

		// Database
		Database: database.Config{
			Type:     dbType,
			Host:     getEnv("DATABASE_HOST", "localhost"),
			Port:     getEnvInt("DATABASE_PORT", 5432),
			Database: getEnv("DATABASE_NAME", "solobase"),
			Username: getEnv("DATABASE_USER", "postgres"),
			Password: getEnv("DATABASE_PASSWORD", "postgres"),
			SSLMode:  getEnv("DATABASE_SSL_MODE", "disable"),
		},

		// Storage
		Storage: StorageConfig{
			Type:             getEnv("STORAGE_TYPE", "local"),
			S3Endpoint:       getEnv("S3_ENDPOINT", ""),
			S3AccessKey:      getEnv("S3_ACCESS_KEY", ""),
			S3SecretKey:      getEnv("S3_SECRET_KEY", ""),
			S3Bucket:         getEnv("S3_BUCKET", "solobase"),
			S3Region:         getEnv("S3_REGION", "us-east-1"),
			S3UseSSL:         getEnvBool("S3_USE_SSL", false),
			LocalStoragePath: getEnv("LOCAL_STORAGE_PATH", "./.data/storage"),
		},

		// Mail
		SMTPHost:     getEnv("SMTP_HOST", "localhost"),
		SMTPPort:     getEnvInt("SMTP_PORT", 1025),
		SMTPUsername: getEnv("SMTP_USERNAME", ""),
		SMTPPassword: getEnv("SMTP_PASSWORD", ""),
		SMTPFrom:     getEnv("SMTP_FROM", "noreply@solobase.local"),
		SMTPUseTLS:   getEnvBool("SMTP_USE_TLS", false),

		// Auth - Use consistent secret for development
		JWTSecret:    getJWTSecret(),
		EnableSignup: getEnvBool("ENABLE_SIGNUP", true),
		EnableAPI:    getEnvBool("ENABLE_API", true),

		// Admin - Require secure password
		AdminEmail:    getEnv("DEFAULT_ADMIN_EMAIL", "admin@example.com"),
		AdminPassword: getSecureAdminPassword(),

		// Logging
		LogLevel: getEnv("LOG_LEVEL", "INFO"),

		// CORS
		CORSAllowedOrigins: getEnvSlice("CORS_ALLOWED_ORIGINS", []string{"*"}),
		CORSAllowedMethods: getEnvSlice("CORS_ALLOWED_METHODS", []string{"GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"}),
		CORSAllowedHeaders: getEnvSlice("CORS_ALLOWED_HEADERS", []string{"Content-Type", "Authorization"}),

		// Rate limiting
		RateLimitEnabled:           getEnvBool("RATE_LIMIT_ENABLED", true),
		RateLimitRequestsPerMinute: getEnvInt("RATE_LIMIT_REQUESTS_PER_MINUTE", 60),
	}

	// Handle DATABASE_URL if provided
	dbURL := getEnv("DATABASE_URL", "")
	if dbURL != "" {
		if dbType == "sqlite" {
			// For SQLite, DATABASE_URL is the file path
			cfg.Database.Database = dbURL
		} else {
			// For PostgreSQL, parse the URL
			parseDatabaseURL(cfg, dbURL)
		}
	} else if dbType == "sqlite" {
		// Default SQLite path
		cfg.Database.Database = getEnv("SQLITE_PATH", "./.data/solobase.db")
	}

	return cfg
}

func parseDatabaseURL(cfg *Config, url string) {
	// Simple parsing of postgres://user:pass@host:port/db?sslmode=disable

	// Remove postgres:// prefix
	url = strings.TrimPrefix(url, "postgres://")
	url = strings.TrimPrefix(url, "postgresql://")

	// Split by @
	parts := strings.Split(url, "@")
	if len(parts) != 2 {
		return
	}

	// Parse user:pass
	userPass := strings.Split(parts[0], ":")
	if len(userPass) == 2 {
		cfg.Database.Username = userPass[0]
		cfg.Database.Password = userPass[1]
	}

	// Parse host:port/db?params
	hostPart := parts[1]

	// Extract params
	if idx := strings.Index(hostPart, "?"); idx != -1 {
		params := hostPart[idx+1:]
		hostPart = hostPart[:idx]

		// Parse params
		for _, param := range strings.Split(params, "&") {
			kv := strings.Split(param, "=")
			if len(kv) == 2 && kv[0] == "sslmode" {
				cfg.Database.SSLMode = kv[1]
			}
		}
	}

	// Parse host:port/db
	if idx := strings.LastIndex(hostPart, "/"); idx != -1 {
		cfg.Database.Database = hostPart[idx+1:]
		hostPart = hostPart[:idx]
	}

	// Parse host:port
	if idx := strings.LastIndex(hostPart, ":"); idx != -1 {
		cfg.Database.Host = hostPart[:idx]
		if port, err := strconv.Atoi(hostPart[idx+1:]); err == nil {
			cfg.Database.Port = port
		}
	} else {
		cfg.Database.Host = hostPart
		cfg.Database.Port = 5432
	}

	if cfg.Database.SSLMode == "" {
		cfg.Database.SSLMode = "disable"
	}
}

func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getEnvInt(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if i, err := strconv.Atoi(value); err == nil {
			return i
		}
	}
	return defaultValue
}

func getEnvBool(key string, defaultValue bool) bool {
	if value := os.Getenv(key); value != "" {
		if b, err := strconv.ParseBool(value); err == nil {
			return b
		}
	}
	return defaultValue
}

func getEnvSlice(key string, defaultValue []string) []string {
	if value := os.Getenv(key); value != "" {
		return strings.Split(value, ",")
	}
	return defaultValue
}

// getJWTSecret returns a consistent JWT secret for development or generates one for production
func getJWTSecret() string {
	// Check if explicitly set
	if value := os.Getenv("JWT_SECRET"); value != "" {
		return value
	}

	// In development, use a consistent secret so sessions persist across restarts
	env := getEnv("ENVIRONMENT", "development")
	if env == "development" {
		devSecret := "solobase-dev-jwt-secret-do-not-use-in-production-2024"
		log.Printf("Using development JWT secret (sessions will persist across restarts)")
		return devSecret
	}

	// In production, generate a secure secret
	secret, err := utils.GenerateSecureToken(32)
	if err != nil {
		log.Fatalf("Failed to generate secure JWT secret: %v\n", err)
	}

	log.Printf("Generated secure JWT secret. Save this in your environment: JWT_SECRET=%s\n", secret)
	return secret
}

// getSecureAdminPassword requires admin password to be set or generates one
func getSecureAdminPassword() string {
	password := os.Getenv("DEFAULT_ADMIN_PASSWORD")

	if password == "" {
		// Generate secure password
		generated, err := utils.GenerateSecurePassword()
		if err != nil {
			log.Fatalf("Failed to generate secure admin password: %v\n", err)
		}
		password = generated
		log.Printf("\n"+
			"========================================\n"+
			"IMPORTANT: Generated admin credentials:\n"+
			"Email: %s\n"+
			"Password: %s\n"+
			"Save these credentials securely!\n"+
			"Set DEFAULT_ADMIN_PASSWORD environment variable to use a custom password.\n"+
			"========================================\n",
			getEnv("DEFAULT_ADMIN_EMAIL", "admin@example.com"),
			password)
		return password
	}

	// Validate existing password strength
	if err := utils.ValidatePasswordStrength(password); err != nil {
		log.Printf("WARNING: Admin password does not meet security requirements: %v\n", err)
	}

	return password
}
