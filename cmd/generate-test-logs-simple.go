package main

import (
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"time"

	"github.com/google/uuid"
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

	// Get the GORM DB
	gormDB := db.GetGORM()

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
		// Random time in the past 24 hours
		hoursAgo := rand.Intn(24)
		minutesAgo := rand.Intn(60)
		createdAt := time.Now().Add(-time.Duration(hoursAgo)*time.Hour - time.Duration(minutesAgo)*time.Minute)

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
			status = 200 + rand.Intn(4) // 200-203
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

		// Build context map
		context := map[string]interface{}{
			"method":   method,
			"path":     path,
			"status":   status,
			"duration": duration.Nanoseconds(),
			"user_ip":  userIP,
		}

		if userID != "" {
			context["user_id"] = userID
		}

		// Add error details for error logs
		if level == "ERROR" {
			context["error"] = fmt.Sprintf("Internal server error processing %s", path)
			context["stack"] = fmt.Sprintf("goroutine 1 [running]:\nmain.handleRequest()\n\t/app/main.go:%d", 100+rand.Intn(400))
		} else if level == "WARN" && rand.Float64() < 0.3 {
			context["error"] = fmt.Sprintf("Validation error: invalid %s parameter", []string{"id", "token", "email", "format"}[rand.Intn(4)])
		}

		// Convert context to JSON
		contextJSON, _ := json.Marshal(context)

		// Create log entry
		logEntry := logger.LogModel{
			ID:        uuid.New(),
			Level:     level,
			Message:   message,
			Fields:    contextJSON,
			CreatedAt: createdAt,
		}

		// Insert into database
		tableName := "logger.logs"
		if cfg.DatabaseType == "sqlite" {
			tableName = "logs"
		}

		if err := gormDB.Table(tableName).Create(&logEntry).Error; err != nil {
			log.Printf("Failed to insert log: %v", err)
		}

		// Progress indicator
		if (i+1)%50 == 0 {
			fmt.Printf("Generated %d logs...\n", i+1)
		}
	}

	// Generate some debug logs
	for i := 0; i < 50; i++ {
		createdAt := time.Now().Add(-time.Duration(rand.Intn(1440)) * time.Minute)

		context := map[string]interface{}{
			"component": []string{"auth", "storage", "database", "cache", "queue"}[rand.Intn(5)],
			"operation": []string{"connect", "query", "insert", "update", "delete"}[rand.Intn(5)],
			"elapsed":   time.Duration(rand.Intn(100)) * time.Millisecond,
		}

		contextJSON, _ := json.Marshal(context)

		logEntry := logger.LogModel{
			ID:        uuid.New(),
			Level:     "DEBUG",
			Message:   fmt.Sprintf("Debug message %d: Processing internal task", i),
			Fields:    contextJSON,
			CreatedAt: createdAt,
		}

		tableName := "logger.logs"
		if cfg.DatabaseType == "sqlite" {
			tableName = "logs"
		}

		if err := gormDB.Table(tableName).Create(&logEntry).Error; err != nil {
			log.Printf("Failed to insert debug log: %v", err)
		}
	}

	fmt.Println("Successfully generated 550 test logs!")
}
