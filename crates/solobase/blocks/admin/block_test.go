package admin

import (
	"context"
	"os"
	"testing"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
	"github.com/wafer-run/wafer-go/wafertest"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func setupAdmin(t *testing.T) (*AdminBlock, wafer.Context, database.Service) {
	t.Helper()

	authManifest, err := os.ReadFile("../auth/block.json")
	require.NoError(t, err)

	adminManifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	db := wafertest.SetupDBFromManifest(t, authManifest, adminManifest)
	ctx := wafertest.NewContext(db)

	w := wafer.New()
	block := NewAdminBlock()
	ctx.SetService("wafer.runtime", w)
	wafertest.InitBlock(t, block, ctx)

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

	msg := wafertest.Retrieve("/admin/users")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)

	data, ok := resp["data"].([]any)
	require.True(t, ok, "expected data array")
	assert.Equal(t, 2, len(data))
}

func TestAdminBlock_GetUser(t *testing.T) {
	block, ctx, db := setupAdmin(t)

	userID := createTestUser(t, db, "alice@test.com")

	msg := wafertest.Retrieve("/admin/users/" + userID)
	wafertest.WithVar(msg, "id", userID)

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "alice@test.com", resp["email"])
	_, hasPassword := resp["password"]
	assert.False(t, hasPassword, "password should be sanitized")
}

func TestAdminBlock_DeleteUser(t *testing.T) {
	block, ctx, db := setupAdmin(t)

	userID := createTestUser(t, db, "alice@test.com")

	msg := wafertest.Delete("/admin/users/" + userID)
	wafertest.WithVar(msg, "id", userID)

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	record, err := db.Get(context.Background(), usersCollection, userID)
	require.NoError(t, err)
	assert.NotNil(t, record.Data["deleted_at"], "user should be soft-deleted")
}

// --- Database tests ---

func TestAdminBlock_DatabaseInfo(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/admin/database/info")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "SQLite", resp["type"])
	assert.Equal(t, "connected", resp["status"])
}

func TestAdminBlock_DatabaseTables(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/admin/database/tables")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))
}

// --- IAM tests ---

func TestAdminBlock_GetRoles(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/admin/iam/roles")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var roles []any
	wafertest.DecodeResponse(t, result, &roles)
	assert.GreaterOrEqual(t, len(roles), 2, "should have at least admin and user roles")
}

// --- Settings tests ---

func TestAdminBlock_GetSettings(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/settings")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))
}

// --- Wafer tests ---

func TestAdminBlock_ListBlocks(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/admin/wafer/blocks")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestAdminBlock_ListFlows(t *testing.T) {
	block, ctx, _ := setupAdmin(t)

	msg := wafertest.Retrieve("/admin/wafer/flows")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))
}
