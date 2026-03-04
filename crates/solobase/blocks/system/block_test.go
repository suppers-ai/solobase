package system

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/waffletest"
)

func setupSystem(t *testing.T) (*SystemBlock, waffle.Context) {
	t.Helper()
	w := waffle.New()
	block := NewSystemBlock()
	db := waffletest.SetupDB(t)
	ctx := waffletest.NewContext(db)
	ctx.SetService("waffle.runtime", w)
	waffletest.InitBlock(t, block, ctx)
	return block, ctx
}

func TestHealthCheck(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := waffletest.Retrieve("/health")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var body map[string]any
	waffletest.DecodeResponse(t, result, &body)
	assert.Equal(t, "ok", body["status"])
	assert.Equal(t, "API is running", body["message"])
}

func TestDebugTime(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := waffletest.Retrieve("/debug/time")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var body map[string]any
	waffletest.DecodeResponse(t, result, &body)
	assert.Contains(t, body, "now")
	assert.Contains(t, body, "unix")
	assert.Contains(t, body, "rfc3339")
	assert.Contains(t, body, "year")
	assert.Contains(t, body, "startTime")
}

func TestGetNavItems(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := waffletest.Retrieve("/nav")

	result := block.Handle(ctx, msg)

	assert.Equal(t, waffle.ActionRespond, result.Action)
	assert.Equal(t, 200, waffletest.Status(result))

	var items []NavItem
	waffletest.DecodeResponse(t, result, &items)
	// Should always include Blocks and Flows
	require.GreaterOrEqual(t, len(items), 2)

	titles := make([]string, len(items))
	for i, item := range items {
		titles[i] = item.Title
	}
	assert.Contains(t, titles, "Blocks")
	assert.Contains(t, titles, "Flows")
}

func TestGetNavItemsSortOrder(t *testing.T) {
	// Create a runtime with a block that has AdminUI
	w := waffle.New()
	w.RegisterBlock("test-block", &stubAdminBlock{})
	block := NewSystemBlock()
	db := waffletest.SetupDB(t)
	ctx := waffletest.NewContext(db)
	ctx.SetService("waffle.runtime", w)
	waffletest.InitBlock(t, block, ctx)

	msg := waffletest.Retrieve("/nav")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, waffletest.Status(result))

	var items []NavItem
	waffletest.DecodeResponse(t, result, &items)
	require.GreaterOrEqual(t, len(items), 3)

	// Blocks and Flows (order group 1) should come before block admin UIs (order group 2)
	blocksIdx := -1
	testIdx := -1
	for i, item := range items {
		if item.Title == "Blocks" {
			blocksIdx = i
		}
		if item.Title == "Test Admin" {
			testIdx = i
		}
	}
	require.NotEqual(t, -1, blocksIdx, "Blocks nav item not found")
	require.NotEqual(t, -1, testIdx, "Test Admin nav item not found")
	assert.Less(t, blocksIdx, testIdx, "Blocks should appear before block admin UIs")
}

func TestBlockInfo(t *testing.T) {
	block := NewSystemBlock()
	info := block.Info()
	assert.Equal(t, BlockName, info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, waffle.Singleton, info.InstanceMode)
}

// stubAdminBlock is a test block that exposes an AdminUI entry.
type stubAdminBlock struct{}

func (b *stubAdminBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         "test-admin-block",
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Test block with admin UI",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
		AdminUI:      &waffle.AdminUIInfo{Path: "/admin/test", Icon: "test", Title: "Test Admin"},
	}
}

func (b *stubAdminBlock) Handle(_ waffle.Context, msg *waffle.Message) waffle.Result {
	return msg.Continue()
}

func (b *stubAdminBlock) Lifecycle(_ waffle.Context, _ waffle.LifecycleEvent) error {
	return nil
}
