package system

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/wafertest"
)

func setupSystem(t *testing.T) (*SystemBlock, wafer.Context) {
	t.Helper()
	w := wafer.New()
	block := NewSystemBlock()
	db := wafertest.SetupDB(t)
	ctx := wafertest.NewContext(db)
	ctx.SetService("wafer.runtime", w)
	wafertest.InitBlock(t, block, ctx)
	return block, ctx
}

func TestHealthCheck(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := wafertest.Retrieve("/health")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var body map[string]any
	wafertest.DecodeResponse(t, result, &body)
	assert.Equal(t, "ok", body["status"])
	assert.Equal(t, "API is running", body["message"])
}

func TestDebugTime(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := wafertest.Retrieve("/debug/time")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var body map[string]any
	wafertest.DecodeResponse(t, result, &body)
	assert.Contains(t, body, "now")
	assert.Contains(t, body, "unix")
	assert.Contains(t, body, "rfc3339")
	assert.Contains(t, body, "year")
	assert.Contains(t, body, "startTime")
}

func TestGetNavItems(t *testing.T) {
	block, ctx := setupSystem(t)
	msg := wafertest.Retrieve("/nav")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var items []NavItem
	wafertest.DecodeResponse(t, result, &items)
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
	w := wafer.New()
	w.RegisterBlock("test-block", &stubAdminBlock{})
	block := NewSystemBlock()
	db := wafertest.SetupDB(t)
	ctx := wafertest.NewContext(db)
	ctx.SetService("wafer.runtime", w)
	wafertest.InitBlock(t, block, ctx)

	msg := wafertest.Retrieve("/nav")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var items []NavItem
	wafertest.DecodeResponse(t, result, &items)
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
	assert.Equal(t, wafer.Singleton, info.InstanceMode)
}

// stubAdminBlock is a test block that exposes an AdminUI entry.
type stubAdminBlock struct{}

func (b *stubAdminBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         "test-admin-block",
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Test block with admin UI",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
		AdminUI:      &wafer.AdminUIInfo{Path: "/admin/test", Icon: "test", Title: "Test Admin"},
	}
}

func (b *stubAdminBlock) Handle(_ wafer.Context, msg *wafer.Message) wafer.Result {
	return msg.Continue()
}

func (b *stubAdminBlock) Lifecycle(_ wafer.Context, _ wafer.LifecycleEvent) error {
	return nil
}
