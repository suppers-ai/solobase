# Auth Library

A flexible, extensible authentication and user management library for Go applications using Authboss and PostgreSQL.

## Features

- **Complete Authentication Flow**: Registration, login, logout, password recovery, email confirmation
- **PostgreSQL Storage**: All user data and sessions stored in PostgreSQL with a dedicated schema
- **Session Management**: Database-backed sessions with automatic cleanup
- **OAuth2 Support**: Built-in support for Google, GitHub, and other providers
- **Security Features**: BCrypt password hashing, CSRF protection, account locking
- **Middleware**: Pre-built middleware for authentication, authorization, and rate limiting
- **Extensible Design**: Easy to customize and extend for your needs
- **Schema Isolation**: Uses PostgreSQL schema `auth` for clean separation

## Installation

```bash
go get github.com/suppers-ai/auth
```

## Quick Start

```go
package main

import (
    "context"
    "log"
    "net/http"
    _ "github.com/lib/pq"
    "github.com/suppers-ai/auth"
    "github.com/suppers-ai/database"
    "github.com/suppers-ai/mailer"
)

func main() {
    // Create database connection
    db, err := database.New("postgres")
    if err != nil {
        log.Fatal(err)
    }
    
    dbConfig := database.Config{
        Driver:   "postgres",
        Host:     "localhost",
        Port:     5432,
        Database: "authdb",
        Username: "user",
        Password: "pass",
    }
    
    if err := db.Connect(context.Background(), dbConfig); err != nil {
        log.Fatal(err)
    }
    
    // Create mailer instance
    mailService, err := mailer.New(mailer.Config{
        Provider: "smtp",
        From: mailer.Address{
            Name:  "My App",
            Email: "noreply@example.com",
        },
        Extra: map[string]interface{}{
            "smtp_host":     "smtp.example.com",
            "smtp_port":     587,
            "smtp_username": "username",
            "smtp_password": "password",
        },
    })
    if err != nil {
        log.Fatal(err)
    }
    
    // Initialize auth service
    authService, err := auth.New(auth.Config{
        DB:          db,
        Mailer:      mailService,
        RootURL:     "http://localhost:8080",
        SessionKey:  []byte("your-session-key"),
        CookieKey:   []byte("your-cookie-key"),
    })
    if err != nil {
        log.Fatal(err)
    }
    
    // Mount auth routes
    mux := http.NewServeMux()
    mux.Handle("/auth/", http.StripPrefix("/auth", authService.Router()))
    
    // Protect routes
    protected := authService.RequireAuth(yourHandler)
    mux.Handle("/protected", protected)
    
    http.ListenAndServe(":8080", authService.LoadClientStateMiddleware(mux))
}
```

## Database Setup

Run the migrations in `migrations/001_init.up.sql` to create the required schema and tables.

This will create:
- Schema: `auth`
- Tables: `auth.users`, `auth.sessions`, `auth.remember_tokens`

## Configuration

### Basic Configuration

```go
config := auth.Config{
    DB:          db,                    // database.Database interface
    Mailer:      mailService,           // mailer.Mailer interface
    RootURL:     "http://localhost",    // Your application URL
    BCryptCost:  12,                    // BCrypt cost (10-31, default 12)
    SessionName: "auth",                // Session cookie name
    SessionKey:  []byte("..."),         // 32 or 64 bytes
    CookieKey:   []byte("..."),         // 32 or 64 bytes
}
```

### Mailer Configuration

The auth package uses the centralized mailer package for sending emails:

```go
// SMTP Mailer
mailService, _ := mailer.New(mailer.Config{
    Provider: "smtp",
    From: mailer.Address{
        Name:  "My App",
        Email: "noreply@example.com",
    },
    Extra: map[string]interface{}{
        "smtp_host":     "smtp.example.com",
        "smtp_port":     587,
        "smtp_username": "username",
        "smtp_password": "password",
    },
})

// Mock Mailer (for testing)
mailService := mailer.NewMock()
```

### OAuth2 Providers

```go
config.OAuth2Providers = map[string]auth.OAuth2Provider{
    "google": {
        ClientID:     "your-client-id",
        ClientSecret: "your-client-secret",
        Scopes:       []string{"email", "profile"},
    },
}
```

## Available Endpoints

- `POST /auth/register` - User registration
- `POST /auth/login` - User login
- `POST /auth/logout` - User logout
- `POST /auth/recover` - Password recovery
- `POST /auth/confirm` - Email confirmation
- `GET /auth/oauth2/{provider}` - OAuth2 login

## Middleware

### Require Authentication
```go
protected := authService.RequireAuth(handler)
```

### Require No Authentication (for login pages)
```go
public := authService.RequireNoAuth(handler)
```

### Admin Only
```go
adminOnly := authService.RequireAdmin(func(user authboss.User) bool {
    // Your admin check logic
    // In authboss, PID (Primary ID) is typically the email
    return user.GetPID() == "admin@example.com"
})(handler)
```

### CSRF Protection
```go
protected := authService.CSRF(handler)
```

## API Methods

### Get Current User
```go
user, err := authService.CurrentUser(request)
```

### Create User Programmatically
```go
user, err := authService.CreateUser(ctx, "email@example.com", "password")
```

### Lock/Unlock User
```go
err := authService.LockUser(ctx, userID)
err := authService.UnlockUser(ctx, userID)
```

### Session Cleanup
```go
// Run periodically to clean expired sessions
err := authService.CleanupSessions(ctx)
```

## Database Schema

All tables are created in the `auth` schema:

- `auth.users` - User accounts and authentication data
- `auth.sessions` - Active user sessions
- `auth.remember_tokens` - Remember me tokens

## Security Considerations

1. **Always use HTTPS in production**
2. **Generate secure random keys** for sessions and cookies
3. **Set appropriate BCrypt cost** (12+ recommended)
4. **Enable CSRF protection** for state-changing operations
5. **Implement rate limiting** for authentication endpoints
6. **Regular session cleanup** to remove expired sessions

## License

MIT