package auth

import (
	"context"
)

// ContextKey is the type for auth context keys
type ContextKey string

// Context keys for authentication
const (
	CtxKeyUser   ContextKey = "auth_user"
	CtxKeyUserID ContextKey = "auth_user_id"
)

// GetUserFromContext retrieves the user from context
func GetUserFromContext(ctx context.Context) (*User, bool) {
	user, ok := ctx.Value(CtxKeyUser).(*User)
	return user, ok
}

// GetUserIDFromContext retrieves the user ID from context
func GetUserIDFromContext(ctx context.Context) (string, bool) {
	userID, ok := ctx.Value(CtxKeyUserID).(string)
	return userID, ok
}

// SetUserInContext adds the user to the context
func SetUserInContext(ctx context.Context, user *User) context.Context {
	ctx = context.WithValue(ctx, CtxKeyUser, user)
	ctx = context.WithValue(ctx, CtxKeyUserID, user.ID.String())
	return ctx
}
