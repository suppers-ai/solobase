package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	_ "github.com/lib/pq"
	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/internal/pkg/mailer"
	"github.com/volatiletech/authboss/v3"
)

func main() {
	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "postgresql://postgres:password@localhost:5432/authdb?sslmode=disable"
	}

	// Create database connection using the database package
	dbConfig := database.Config{
		Driver:   "postgres",
		Host:     "localhost",
		Port:     5432,
		Database: "authdb",
		Username: "postgres",
		Password: "password",
		SSLMode:  "disable",
	}

	// You can also parse from DSN if needed
	db, err := database.New("postgres")
	if err != nil {
		log.Fatal("Failed to create database instance:", err)
	}

	if err := db.Connect(context.Background(), dbConfig); err != nil {
		log.Fatal("Failed to connect to database:", err)
	}
	defer db.Close()

	// Create mailer instance
	var mailService mailer.Mailer
	if os.Getenv("SMTP_HOST") != "" {
		mailService, err = mailer.New(mailer.Config{
			Provider: "smtp",
			From: mailer.Address{
				Name:  "Auth Service",
				Email: os.Getenv("EMAIL_FROM"),
			},
			Timeout: 10 * time.Second,
			Extra: map[string]interface{}{
				"smtp_host":      os.Getenv("SMTP_HOST"),
				"smtp_port":      587,
				"smtp_username":  os.Getenv("SMTP_USERNAME"),
				"smtp_password":  os.Getenv("SMTP_PASSWORD"),
				"smtp_start_tls": true,
			},
		})
		if err != nil {
			log.Printf("Warning: Failed to create mailer: %v", err)
			// Use mock mailer for development
			mailService = mailer.NewMock()
		}
	} else {
		// Use mock mailer if SMTP is not configured
		mailService = mailer.NewMock()
	}

	authService, err := auth.New(auth.Config{
		DB:          db,
		Mailer:      mailService,
		RootURL:     "http://localhost:8080",
		BCryptCost:  12,
		SessionName: "auth_session",
		SessionKey:  []byte(os.Getenv("SESSION_KEY")),
		CookieKey:   []byte(os.Getenv("COOKIE_KEY")),
		CSRFKey:     []byte(os.Getenv("CSRF_KEY")),
		OAuth2Providers: map[string]auth.OAuth2Provider{
			"google": {
				ClientID:     os.Getenv("GOOGLE_CLIENT_ID"),
				ClientSecret: os.Getenv("GOOGLE_CLIENT_SECRET"),
				Scopes:       []string{"email", "profile"},
			},
			"github": {
				ClientID:     os.Getenv("GITHUB_CLIENT_ID"),
				ClientSecret: os.Getenv("GITHUB_CLIENT_SECRET"),
				Scopes:       []string{"user:email"},
			},
		},
	})
	if err != nil {
		log.Fatal("Failed to initialize auth service:", err)
	}

	go func() {
		ticker := time.NewTicker(1 * time.Hour)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				if err := authService.CleanupSessions(context.Background()); err != nil {
					log.Printf("Failed to cleanup sessions: %v", err)
				}
			}
		}
	}()

	mux := http.NewServeMux()

	mux.Handle("/auth/", http.StripPrefix("/auth", authService.Router()))

	publicMux := authService.LoadClientStateMiddleware(mux)

	protectedHandler := authService.RequireAuth(http.HandlerFunc(protectedEndpoint))
	mux.Handle("/api/protected", protectedHandler)

	mux.HandleFunc("/api/public", publicEndpoint)

	adminHandler := authService.RequireAdmin(func(user authboss.User) bool {
		return false
	})(http.HandlerFunc(adminEndpoint))
	mux.Handle("/api/admin", adminHandler)

	mux.HandleFunc("/api/user", func(w http.ResponseWriter, r *http.Request) {
		user, err := authService.CurrentUser(r)
		if err != nil {
			http.Error(w, "Not authenticated", http.StatusUnauthorized)
			return
		}

		// In authboss, the PID (Primary ID) is typically the email
		// The authboss.User interface doesn't have GetEmail method
		// You need to type assert to the concrete type or use GetPID

		response := map[string]interface{}{
			"id":    user.GetPID(),
			"email": user.GetPID(), // PID is the email in authboss
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
	})

	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		html := `
<!DOCTYPE html>
<html>
<head>
    <title>Auth Example</title>
</head>
<body>
    <h1>Authentication Example</h1>
    <h2>Authentication Endpoints</h2>
    <ul>
        <li><a href="/auth/login">Login</a></li>
        <li><a href="/auth/register">Register</a></li>
        <li><a href="/auth/logout">Logout</a></li>
        <li><a href="/auth/recover">Recover Password</a></li>
        <li><a href="/auth/confirm">Confirm Email</a></li>
        <li><a href="/auth/oauth2/google">Login with Google</a></li>
        <li><a href="/auth/oauth2/github">Login with GitHub</a></li>
    </ul>
    <h2>API Endpoints</h2>
    <ul>
        <li><a href="/api/public">Public Endpoint</a></li>
        <li><a href="/api/protected">Protected Endpoint (requires auth)</a></li>
        <li><a href="/api/admin">Admin Endpoint (requires admin)</a></li>
        <li><a href="/api/user">Current User Info</a></li>
    </ul>
</body>
</html>
		`
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprint(w, html)
	})

	fmt.Println("Server starting on http://localhost:8080")
	log.Fatal(http.ListenAndServe(":8080", publicMux))
}

func protectedEndpoint(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"message": "This is a protected endpoint",
		"status":  "authenticated",
	})
}

func publicEndpoint(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"message": "This is a public endpoint",
		"status":  "public",
	})
}

func adminEndpoint(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"message": "This is an admin endpoint",
		"status":  "admin",
	})
}
