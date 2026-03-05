package monitoring

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
	"github.com/wafer-run/wafer-go/wafertest"
)

func setupMonitoring(t *testing.T) (*MonitoringBlock, wafer.Context, database.Service) {
	t.Helper()
	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	db := wafertest.SetupDBFromManifest(t, manifest)
	ctx := wafertest.NewContext(db)
	block := NewMonitoringBlock()
	// Note: we call InitBlock which triggers persister start; the persister
	// is safe in tests since it only flushes on tick or stop.
	wafertest.InitBlock(t, block, ctx)
	t.Cleanup(func() {
		if block.persister != nil {
			block.persister.Stop()
		}
	})
	return block, ctx, db
}

func TestLiveStats_Empty(t *testing.T) {
	block, ctx, _ := setupMonitoring(t)

	msg := wafertest.Retrieve("/admin/monitoring/live")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var stats LiveStats
	wafertest.DecodeResponse(t, result, &stats)
	assert.Equal(t, int64(0), stats.TotalMessages)
	assert.Equal(t, int64(0), stats.TotalErrors)
	assert.Empty(t, stats.PerBlock)
	assert.Empty(t, stats.PerFlow)
	assert.Empty(t, stats.PerKind)
}

func TestLiveStats_AfterRecording(t *testing.T) {
	block, ctx, _ := setupMonitoring(t)

	// Record some stats via the collector
	block.Collector.RecordBlock("auth-block", 15, false)
	block.Collector.RecordBlock("auth-block", 25, true)
	block.Collector.RecordFlow("admin-pipe", 40, false)
	block.Collector.RecordKind("http.request")
	block.Collector.RecordKind("http.request")

	msg := wafertest.Retrieve("/admin/monitoring/live")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var stats LiveStats
	wafertest.DecodeResponse(t, result, &stats)
	assert.Equal(t, int64(2), stats.TotalMessages)
	assert.Equal(t, int64(1), stats.TotalErrors)

	// Per-block checks
	require.Contains(t, stats.PerBlock, "auth-block")
	assert.Equal(t, int64(2), stats.PerBlock["auth-block"].Count)
	assert.Equal(t, int64(1), stats.PerBlock["auth-block"].Errors)
	assert.Equal(t, float64(20), stats.PerBlock["auth-block"].AvgMs) // (15+25)/2

	// Per-flow checks
	require.Contains(t, stats.PerFlow, "admin-pipe")
	assert.Equal(t, int64(1), stats.PerFlow["admin-pipe"].Count)

	// Per-kind checks
	require.Contains(t, stats.PerKind, "http.request")
	assert.Equal(t, int64(2), stats.PerKind["http.request"])
}

func TestCollectorSnapshot_Resets(t *testing.T) {
	block, _, _ := setupMonitoring(t)

	block.Collector.RecordBlock("users-block", 10, false)
	block.Collector.RecordBlock("users-block", 30, true)

	totalMsg, totalErr, perBlock, perFlow, perKind := block.Collector.Snapshot()
	assert.Equal(t, int64(2), totalMsg)
	assert.Equal(t, int64(1), totalErr)
	assert.NotEmpty(t, perBlock)
	assert.NotEmpty(t, perFlow) // may be empty if no flow recorded, but perBlock is not
	_ = perFlow
	_ = perKind

	// After snapshot, collector should be reset
	stats := block.Collector.ReadStats()
	assert.Equal(t, int64(0), stats.TotalMessages)
	assert.Equal(t, int64(0), stats.TotalErrors)
	assert.Empty(t, stats.PerBlock)
}

func TestHistory_Empty(t *testing.T) {
	block, ctx, _ := setupMonitoring(t)

	msg := wafertest.Retrieve("/admin/monitoring/history")
	wafertest.WithQuery(msg, "range", "24h")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var snapshots []map[string]any
	wafertest.DecodeResponse(t, result, &snapshots)
	assert.Empty(t, snapshots)
}

func TestHistory_WithPersistedData(t *testing.T) {
	block, ctx, db := setupMonitoring(t)

	// Manually persist a snapshot record to the DB
	_, err := db.Create(ctx.Ctx(), monitoringCollection, map[string]any{
		"id":             "snap-1",
		"period_start":   "2026-02-20T00:00:00Z",
		"period_end":     "2026-02-20T01:00:00Z",
		"total_messages": 100,
		"total_errors":   5,
		"per_block_json": `{"auth":{"count":50}}`,
		"per_flow_json":  `{"admin-pipe":{"count":100}}`,
		"per_kind_json":  `{"http.request":80}`,
		"created_at":     "2026-02-20T01:00:00Z",
	})
	require.NoError(t, err)

	msg := wafertest.Retrieve("/admin/monitoring/history")
	wafertest.WithQuery(msg, "range", "30d")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var snapshots []map[string]any
	wafertest.DecodeResponse(t, result, &snapshots)
	require.Len(t, snapshots, 1)
	assert.Equal(t, "snap-1", snapshots[0]["id"])
	assert.Equal(t, float64(100), snapshots[0]["totalMessages"])
	assert.Equal(t, float64(5), snapshots[0]["totalErrors"])
}
