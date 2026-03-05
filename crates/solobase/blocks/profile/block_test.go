package profile

import (
	"testing"

	"github.com/stretchr/testify/assert"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/wafertest"
)

func setupProfile(t *testing.T) (*ProfileBlock, wafer.Context) {
	t.Helper()
	block := NewProfileBlock()
	db := wafertest.SetupDB(t)
	ctx := wafertest.NewContext(db)
	wafertest.InitBlock(t, block, ctx)
	return block, ctx
}

func TestProfileSectionsReturnsEmptyArray(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := wafertest.Retrieve("/profile/sections")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var sections []any
	wafertest.DecodeResponse(t, result, &sections)
	assert.Empty(t, sections)
	assert.NotNil(t, sections, "should be an empty array, not null")
}

func TestProfileSectionsWithAuth(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := wafertest.Retrieve("/profile/sections")
	msg = wafertest.WithAuth(msg, "user-123", "test@example.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var sections []any
	wafertest.DecodeResponse(t, result, &sections)
	assert.Empty(t, sections)
}

func TestBlockInfo(t *testing.T) {
	block := NewProfileBlock()
	info := block.Info()
	assert.Equal(t, BlockName, info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, wafer.Singleton, info.InstanceMode)
}

func TestUnmatchedRouteReturns404(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := wafertest.Retrieve("/profile/nonexistent")

	result := block.Handle(ctx, msg)

	// Router returns 404 error for unmatched routes
	assert.Equal(t, wafer.ActionError, result.Action)
	assert.Equal(t, 404, wafertest.Status(result))
}
