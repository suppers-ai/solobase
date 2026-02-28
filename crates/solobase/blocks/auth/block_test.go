package auth

import (
	"context"
	"os"
	"testing"

	adaptercrypto "github.com/suppers-ai/solobase/adapters/crypto"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
	"github.com/suppers-ai/waffle-go/waffletest"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

const testSecret = "test-secret-key-minimum-32-characters-long!!"

func TestMain(m *testing.M) {
	// The adapters/crypto package-level functions require default providers to be set.
	adaptercrypto.SetDefaultHasher(adaptercrypto.NewArgon2Hasher())
	adaptercrypto.SetDefaultCrypto(adaptercrypto.NewStandardCrypto())
	os.Exit(m.Run())
}

func setupAuth(t *testing.T) (*AuthBlock, waffle.Context, database.Service) {
	t.Helper()

	authManifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	iamManifest, err := os.ReadFile("../admin/block.json")
	require.NoError(t, err)

	db := waffletest.SetupDBFromManifest(t, authManifest, iamManifest)
	crypto := adaptercrypto.NewStandardService(testSecret)
	ctx := waffletest.NewContextWithCrypto(db, crypto)

	t.Setenv("ENABLE_SIGNUP", "true")
	block := NewAuthBlock()
	waffletest.InitBlock(t, block, ctx)

	return block, ctx, db
}

// createTestUser hashes the password and inserts a user into the database.
func createTestUser(t *testing.T, db database.Service, crypto *adaptercrypto.StandardService, email, password string) string {
	t.Helper()
	hashed, err := crypto.Hash(password)
	require.NoError(t, err)

	record, err := db.Create(context.Background(), usersCollection, map[string]any{
		"email":    email,
		"password": hashed,
	})
	require.NoError(t, err)
	return record.ID
}

func TestLoginSuccess(t *testing.T) {
	block, ctx, db := setupAuth(t)
	crypto := adaptercrypto.NewStandardService(testSecret)

	createTestUser(t, db, crypto, "alice@test.com", "password123")

	msg := waffletest.Create("/auth/login", LoginRequest{
		Email:    "alice@test.com",
		Password: "password123",
	})

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action, "expected respond action")
	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)

	data, ok := resp["data"].(map[string]any)
	require.True(t, ok, "expected data field in response")

	user, ok := data["user"].(map[string]any)
	require.True(t, ok, "expected user field in data")
	assert.Equal(t, "alice@test.com", user["email"])

	// Password should be stripped from response
	_, hasPassword := user["password"]
	assert.False(t, hasPassword, "password should be sanitized from response")

	assert.Equal(t, "Login successful", resp["message"])
}

func TestLoginWrongPassword(t *testing.T) {
	block, ctx, db := setupAuth(t)
	crypto := adaptercrypto.NewStandardService(testSecret)

	createTestUser(t, db, crypto, "alice@test.com", "password123")

	msg := waffletest.Create("/auth/login", LoginRequest{
		Email:    "alice@test.com",
		Password: "wrongpassword",
	})

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionError, result.Action, "expected error action")
	assert.Equal(t, 401, waffletest.Status(result))
	require.NotNil(t, result.Error)
	assert.Equal(t, "unauthorized", result.Error.Code)
}

func TestLoginNonexistentUser(t *testing.T) {
	block, ctx, _ := setupAuth(t)

	msg := waffletest.Create("/auth/login", LoginRequest{
		Email:    "nobody@test.com",
		Password: "password123",
	})

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionError, result.Action)
	assert.Equal(t, 401, waffletest.Status(result))
}

func TestSignupSuccess(t *testing.T) {
	block, ctx, _ := setupAuth(t)

	msg := waffletest.Create("/auth/signup", SignupRequest{
		Email:    "newuser@test.com",
		Password: "securepassword",
	})

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action, "expected respond action")
	assert.Equal(t, 201, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "newuser@test.com", resp["email"])

	// Password should not appear in response
	_, hasPassword := resp["password"]
	assert.False(t, hasPassword, "password should be sanitized from response")
}

func TestSignupDisabled(t *testing.T) {
	authManifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	iamManifest, err := os.ReadFile("../admin/block.json")
	require.NoError(t, err)

	db := waffletest.SetupDBFromManifest(t, authManifest, iamManifest)
	crypto := adaptercrypto.NewStandardService(testSecret)
	ctx := waffletest.NewContextWithCrypto(db, crypto)

	t.Setenv("ENABLE_SIGNUP", "false")
	block := NewAuthBlock()
	waffletest.InitBlock(t, block, ctx)

	msg := waffletest.Create("/auth/signup", SignupRequest{
		Email:    "newuser@test.com",
		Password: "securepassword",
	})

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionError, result.Action)
	assert.Equal(t, 403, waffletest.Status(result))
}

func TestGetCurrentUser(t *testing.T) {
	block, ctx, db := setupAuth(t)
	crypto := adaptercrypto.NewStandardService(testSecret)

	userID := createTestUser(t, db, crypto, "alice@test.com", "password123")

	msg := waffletest.Retrieve("/auth/me")
	waffletest.WithAuth(msg, userID, "alice@test.com")
	waffletest.WithRoles(msg, "user")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)

	user, ok := resp["user"].(map[string]any)
	require.True(t, ok, "expected user field in response")
	assert.Equal(t, "alice@test.com", user["email"])

	roles, ok := resp["roles"].([]any)
	require.True(t, ok, "expected roles field in response")
	assert.Contains(t, roles, "user")
}

func TestGetCurrentUserUnauthenticated(t *testing.T) {
	block, ctx, _ := setupAuth(t)

	msg := waffletest.Retrieve("/auth/me")
	// No auth set

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionError, result.Action)
	assert.Equal(t, 401, waffletest.Status(result))
}

func TestChangePassword(t *testing.T) {
	block, ctx, db := setupAuth(t)
	crypto := adaptercrypto.NewStandardService(testSecret)

	userID := createTestUser(t, db, crypto, "alice@test.com", "oldpassword1")

	msg := waffletest.Create("/auth/change-password", ChangePasswordRequest{
		CurrentPassword: "oldpassword1",
		NewPassword:     "newpassword1",
	})
	waffletest.WithAuth(msg, userID, "alice@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	// Verify login works with new password
	loginMsg := waffletest.Create("/auth/login", LoginRequest{
		Email:    "alice@test.com",
		Password: "newpassword1",
	})
	loginResult := block.Handle(ctx, loginMsg)
	assert.Equal(t, 200, waffletest.Status(loginResult))

	// Old password should fail
	loginMsg2 := waffletest.Create("/auth/login", LoginRequest{
		Email:    "alice@test.com",
		Password: "oldpassword1",
	})
	loginResult2 := block.Handle(ctx, loginMsg2)
	assert.Equal(t, 401, waffletest.Status(loginResult2))
}

func TestChangePasswordWrongCurrent(t *testing.T) {
	block, ctx, db := setupAuth(t)
	crypto := adaptercrypto.NewStandardService(testSecret)

	userID := createTestUser(t, db, crypto, "alice@test.com", "oldpassword1")

	msg := waffletest.Create("/auth/change-password", ChangePasswordRequest{
		CurrentPassword: "wrongpassword",
		NewPassword:     "newpassword1",
	})
	waffletest.WithAuth(msg, userID, "alice@test.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionError, result.Action)
	assert.Equal(t, 401, waffletest.Status(result))
}
