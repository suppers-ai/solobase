package admin

import (
	"context"
	"os"
	"testing"

	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
	"github.com/suppers-ai/waffle-go/waffletest"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func setupAdmin(t *testing.T) (*AdminBlock, waffle.Context, database.Service) {
	t.Helper()

	authManifest, err := os.ReadFile("../auth/block.json")
	require.NoError(t, err)

	adminManifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	db := waffletest.SetupDBFromManifest(t, authManifest, adminManifest)
	ctx := waffletest.NewContext(db)

	w := waffle.New()
	block := NewAdminBlock()
	ctx.SetService("waffle.runtime", w)
	waffletest.InitBlock(t, block, ctx)

	return block, ctx, db
}

func createTestUser(t *testing.T, db database.Service, email string) string {
	t.Helper()
	record, err := db.Create(context.Background(), usersCollection, map[string]any{
		"email":    email,
		"password": "hashed_password",
	})
	require.NoError(t, err)
	return record.ID
}

// --- Users tests ---

func TestAdminBlock_ListUsers(t *testing.T) {
	block, ctx, db := setupAdmin(t)

	createTestUser(t, db, "alice@test.com")
	createTestUser(t, db, "bob@test.com")

	msg := waffletest.Retrieve("/admin/users")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)

	data, ok := resp["data"].([]any)
	require.True(t, ok, "expected data array")
	assert.Equal(t, 2, len(data))
}

func TestAdminBlock_GetUser(t *testing.T) {
	block, ctx, db := setupAdmin(t)

	userID := createTestUser(t, db, "alice@test.com")

	msg := waffletest.Retrieve("/admin/users/" + userID)
	waffletest.WithVar(msg, "id", userID)

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "alice@test.com", resp["email"])
	_, hasPassword := resp["password"]
	assert.False(t, hasPassword, "password should be sanitized")
}

func TestAdminBlock_DeleteUser(t *testing.T) {
	block, ctx, db := setupAdmin(t)

	userID := createTestUser(t, db, "alice@test.com")

	msg := waffletest.Delete("/admin/users/" + userID)
	waffletest.WithVar(msg, "id", userID)

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	record, err := db.Get(context.Background(), usersCollection, userID)
	require.NoError(t, err)
	assert.NotNil(t, record.Data["deleted_at"], "user should be soft-deleted")
}

// --- Database tests ---

func TestAdminBlock_DatabaseInfo(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/admin/database/info")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var resp map[string]any
	waffletest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "SQLite", resp["type"])
	assert.Equal(t, "connected", resp["status"])
}

func TestAdminBlock_DatabaseTables(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/admin/database/tables")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))
}

// --- IAM tests ---

func TestAdminBlock_GetRoles(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/admin/iam/roles")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var roles []any
	waffletest.DecodeResponse(t, result, &roles)
	assert.GreaterOrEqual(t, len(roles), 2, "should have at least admin and user roles")
}

// --- Settings tests ---

func TestAdminBlock_GetSettings(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/settings")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))
}

// --- Waffle tests ---

func TestAdminBlock_ListBlocks(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/admin/waffle/blocks")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))
}

func TestAdminBlock_ListChains(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := waffletest.Retrieve("/admin/waffle/chains")
	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))
}
