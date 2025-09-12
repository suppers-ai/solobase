package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/volatiletech/authboss/v3"
)

func RequireAdmin(ab *authboss.Authboss, isAdmin func(authboss.User) bool) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			user, err := ab.CurrentUser(r)
			if err != nil || user == nil {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			if !isAdmin(user) {
				http.Error(w, "Forbidden", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

func CSRF(ab *authboss.Authboss) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if r.Method == "POST" || r.Method == "PUT" || r.Method == "DELETE" || r.Method == "PATCH" {
				token := r.Header.Get("X-CSRF-Token")
				if token == "" {
					token = r.FormValue("csrf_token")
				}

				if token == "" {
					http.Error(w, "CSRF token missing", http.StatusForbidden)
					return
				}
			}

			next.ServeHTTP(w, r)
		})
	}
}

func RequireRole(ab *authboss.Authboss, role string, getRoles func(authboss.User) []string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			user, err := ab.CurrentUser(r)
			if err != nil || user == nil {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			roles := getRoles(user)
			for _, userRole := range roles {
				if userRole == role {
					next.ServeHTTP(w, r)
					return
				}
			}

			http.Error(w, "Forbidden", http.StatusForbidden)
		})
	}
}

func RequirePermission(ab *authboss.Authboss, permission string, getPermissions func(authboss.User) []string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			user, err := ab.CurrentUser(r)
			if err != nil || user == nil {
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}

			permissions := getPermissions(user)
			for _, userPermission := range permissions {
				if userPermission == permission || strings.HasPrefix(permission, userPermission+":") {
					next.ServeHTTP(w, r)
					return
				}
			}

			http.Error(w, "Forbidden", http.StatusForbidden)
		})
	}
}

func APIAuth(ab *authboss.Authboss, getTokenUser func(ctx context.Context, token string) (authboss.User, error)) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" {
				user, err := ab.CurrentUser(r)
				if err == nil && user != nil {
					next.ServeHTTP(w, r)
					return
				}

				http.Error(w, "Authorization required", http.StatusUnauthorized)
				return
			}

			parts := strings.SplitN(authHeader, " ", 2)
			if len(parts) != 2 || parts[0] != "Bearer" {
				http.Error(w, "Invalid authorization header", http.StatusUnauthorized)
				return
			}

			user, err := getTokenUser(r.Context(), parts[1])
			if err != nil || user == nil {
				http.Error(w, "Invalid token", http.StatusUnauthorized)
				return
			}

			ctx := context.WithValue(r.Context(), authboss.CTXKeyUser, user)
			ctx = context.WithValue(ctx, authboss.CTXKeyPID, user.GetPID())
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func RateLimitByUser(ab *authboss.Authboss, limiter func(userID string) bool) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userID, err := ab.CurrentUserID(r)
			if err == nil && userID != "" {
				if limiter(userID) {
					http.Error(w, "Too many requests", http.StatusTooManyRequests)
					return
				}
			}

			next.ServeHTTP(w, r)
		})
	}
}
