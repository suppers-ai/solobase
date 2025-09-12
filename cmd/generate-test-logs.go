package main

import (
	"context"
	"fmt"
	"log"
	"math/rand"
	"time"

	"github.com/joho/godotenv"
	"github.com/suppers-ai/database"
	"github.com/suppers-ai/logger"
	"github.com/suppers-ai/solobase/config"
)

func main() {
	// Load environment variables
	if err := godotenv.Load(); err != nil {
		log.Println("No .env file found, using environment variables")
	}

	// Initialize configuration
	cfg := config.Load()

	// Initialize database
	dbConfig := &database.Config{
		Type:            cfg.DatabaseType,
		DSN:             cfg.DatabaseURL,
		Host:            cfg.DatabaseHost,
		Port:            cfg.DatabasePort,
		Database:        cfg.DatabaseName,
		Username:        cfg.DatabaseUser,
		Password:        cfg.DatabasePassword,
		SSLMode:         cfg.DatabaseSSLMode,
		MaxOpenConns:    100,
		MaxIdleConns:    10,
		ConnMaxLifetime: time.Hour,
		Debug:           false,
		AutoMigrate:     true,
	}

	db, err := database.New(dbConfig)
	if err != nil {
		log.Fatalf("Failed to initialize database: %v", err)
	}
	defer db.Close()

	// Initialize models
	logger.InitModels(cfg.DatabaseType)

	// Initialize logger with database output
	logLevel, _ := logger.ParseLevel(cfg.LogLevel)
	appLogger, err := logger.New(logger.Config{
		Level:  logLevel,
		Output: "database",
		Format: "json",
		Extra: map[string]interface{}{
			"database": db,
		},
	})
	if err != nil {
		log.Fatalf("Failed to initialize logger: %v", err)
	}

	rand.Seed(time.Now().UnixNano())

	// Generate test logs
	methods := []string{"GET", "POST", "PUT", "DELETE", "PATCH"}
	paths := []string{
		"/api/users",
		"/api/products",
		"/api/orders",
		"/dashboard",
		"/settings",
		"/api/auth/login",
		"/api/auth/logout",
		"/api/collections",
		"/api/storage/upload",
		"/api/storage/download",
		"/api/logs",
		"/api/metrics",
		"/api/extensions",
		"/admin/users",
		"/admin/settings",
	}
	userIPs := []string{
		"192.168.1.1",
		"10.0.0.1",
		"172.16.0.1",
		"203.0.113.1",
		"198.51.100.1",
	}
	userIDs := []string{
		"user-123",
		"user-456",
		"user-789",
		"admin-001",
		"",
	}

	fmt.Println("Generating test logs...")

	// Generate logs for the past 24 hours
	for i := 0; i < 500; i++ {
		// Random time in the past 24 hours - we'll use this later when we can set timestamps
		// hoursAgo := rand.Intn(24)
		// minutesAgo := rand.Intn(60)
		// timestamp := time.Now().Add(-time.Duration(hoursAgo)*time.Hour - time.Duration(minutesAgo)*time.Minute)

		// Random request details
		method := methods[rand.Intn(len(methods))]
		path := paths[rand.Intn(len(paths))]
		userIP := userIPs[rand.Intn(len(userIPs))]
		userID := userIDs[rand.Intn(len(userIDs))]

		// Random status code with weighted distribution
		statusRand := rand.Float64()
		var status int
		var level string
		var message string

		if statusRand < 0.7 { // 70% success
			status = 200 + rand.Intn(4)*100 // 200, 201, 202, 203
			level = "INFO"
			message = fmt.Sprintf("%s %s completed successfully", method, path)
		} else if statusRand < 0.85 { // 15% redirects
			status = 301 + rand.Intn(7) // 301-307
			level = "INFO"
			message = fmt.Sprintf("%s %s redirected", method, path)
		} else if statusRand < 0.95 { // 10% client errors
			status = 400 + rand.Intn(18) // 400-417
			level = "WARN"
			message = fmt.Sprintf("%s %s client error", method, path)
		} else { // 5% server errors
			status = 500 + rand.Intn(11) // 500-510
			level = "ERROR"
			message = fmt.Sprintf("%s %s server error", method, path)
		}

		// Random duration
		duration := time.Duration(rand.Intn(5000)) * time.Millisecond

		// Log with context
		ctx := context.Background()
		fields := []logger.Field{
			logger.String("method", method),
			logger.String("path", path),
			logger.Int("status", status),
			logger.Duration("duration", duration),
			logger.String("user_ip", userIP),
		}

		if userID != "" {
			fields = append(fields, logger.String("user_id", userID))
		}

		// Add error details for error logs
		if level == "ERROR" {
			fields = append(fields,
				logger.String("error", fmt.Sprintf("Internal server error processing %s", path)),
				logger.String("stack", fmt.Sprintf("goroutine 1 [running]:\nmain.handleRequest()\n\t/app/main.go:%d", 100+rand.Intn(400))),
			)
		} else if level == "WARN" && rand.Float64() < 0.3 {
			fields = append(fields, logger.String("error", fmt.Sprintf("Validation error: invalid %s parameter", []string{"id", "token", "email", "format"}[rand.Intn(4)])))
		}

		// Log based on level
		switch level {
		case "DEBUG":
			appLogger.Debug(ctx, message, fields...)
		case "INFO":
			appLogger.Info(ctx, message, fields...)
		case "WARN":
			appLogger.Warn(ctx, message, fields...)
		case "ERROR":
			appLogger.Error(ctx, message, fields...)
		}

		// Progress indicator
		if (i+1)%50 == 0 {
			fmt.Printf("Generated %d logs...\n", i+1)
		}
	}

	// Generate some debug logs
	for i := 0; i < 50; i++ {
		ctx := context.Background()
		appLogger.Debug(ctx, fmt.Sprintf("Debug message %d: Processing internal task", i),
			logger.String("component", []string{"auth", "storage", "database", "cache", "queue"}[rand.Intn(5)]),
			logger.String("operation", []string{"connect", "query", "insert", "update", "delete"}[rand.Intn(5)]),
			logger.Duration("elapsed", time.Duration(rand.Intn(100))*time.Millisecond),
		)
	}

	fmt.Println("Successfully generated 550 test logs!")
}
