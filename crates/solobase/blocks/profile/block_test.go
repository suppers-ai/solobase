package profile

import (
	"testing"

	"github.com/stretchr/testify/assert"

	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/waffletest"
)

func setupProfile(t *testing.T) (*ProfileBlock, waffle.Context) {
	t.Helper()
	block := NewProfileBlock()
	db := waffletest.SetupDB(t)
	ctx := waffletest.NewContext(db)
	waffletest.InitBlock(t, block, ctx)
	return block, ctx
}

func TestProfileSectionsReturnsEmptyArray(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := waffletest.Retrieve("/profile/sections")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var sections []any
	waffletest.DecodeResponse(t, result, &sections)
	assert.Empty(t, sections)
	assert.NotNil(t, sections, "should be an empty array, not null")
}

func TestProfileSectionsWithAuth(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := waffletest.Retrieve("/profile/sections")
	msg = waffletest.WithAuth(msg, "user-123", "test@example.com")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var sections []any
	waffletest.DecodeResponse(t, result, &sections)
	assert.Empty(t, sections)
}

func TestBlockInfo(t *testing.T) {
	block := NewProfileBlock()
	info := block.Info()
	assert.Equal(t, BlockName, info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, waffle.Singleton, info.InstanceMode)
}

func TestUnmatchedRouteReturns404(t *testing.T) {
	block, ctx := setupProfile(t)
	msg := waffletest.Retrieve("/profile/nonexistent")

	result := block.Handle(ctx, msg)

	// Router returns 404 error for unmatched routes
	assert.Equal(t, waffle.ActionError, result.Action)
	assert.Equal(t, 404, waffletest.Status(result))
}
