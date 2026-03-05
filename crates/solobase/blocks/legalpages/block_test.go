package legalpages

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
	"github.com/wafer-run/wafer-go/wafertest"
)

func setupLegalPages(t *testing.T) (*LegalPagesBlock, wafer.Context, database.Service) {
	t.Helper()
	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	db := wafertest.SetupDBFromManifest(t, manifest)
	ctx := wafertest.NewContext(db)
	block := NewLegalPagesBlock()
	// InitBlock triggers seedDefaults which inserts default terms/privacy docs
	wafertest.InitBlock(t, block, ctx)
	return block, ctx, db
}

func setupLegalPagesNoSeed(t *testing.T) (*LegalPagesBlock, wafer.Context, database.Service) {
	t.Helper()
	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	db := wafertest.SetupDBFromManifest(t, manifest)
	ctx := wafertest.NewContext(db)
	block := NewLegalPagesBlock()
	// Do NOT call InitBlock -- skip seeding so we can test empty state
	return block, ctx, db
}

func TestGetPublicTerms_Empty(t *testing.T) {
	block, ctx, _ := setupLegalPagesNoSeed(t)

	msg := wafertest.Retrieve("/ext/legalpages/terms")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	// Should return HTML with "not yet available" message since no docs exist
	body := wafertest.ResponseBody(result)
	assert.Contains(t, string(body), "not yet available")
}

func TestGetPublicTerms_WithSeededData(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Retrieve("/ext/legalpages/terms")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	body := wafertest.ResponseBody(result)
	// The seeded default content contains "Acceptance of Terms"
	assert.Contains(t, string(body), "Acceptance of Terms")
}

func TestGetPublicPrivacy_WithSeededData(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Retrieve("/ext/legalpages/privacy")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	body := wafertest.ResponseBody(result)
	assert.Contains(t, string(body), "Information We Collect")
}

func TestSaveDocument(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	// Save a new version of terms -- use actual path so router extracts {type}=terms
	msg := wafertest.Create("/ext/legalpages/api/documents/terms", map[string]any{
		"title":   "Updated Terms",
		"content": "<p>New terms content</p>",
	})
	wafertest.WithAuth(msg, "admin-1", "admin@test.com")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 201, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "Updated Terms", resp["title"])
	assert.Equal(t, "<p>New terms content</p>", resp["content"])
	// Version should be 2 since seed created version 1
	assert.Equal(t, float64(2), resp["version"])
}

func TestGetDocumentByType(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Retrieve("/ext/legalpages/api/documents/terms")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var doc map[string]any
	wafertest.DecodeResponse(t, result, &doc)
	assert.Equal(t, "Terms and Conditions", doc["title"])
	assert.Equal(t, "terms", doc["document_type"])
}

func TestGetDocumentByType_InvalidType(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Retrieve("/ext/legalpages/api/documents/invalid")
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionError, result.Action)
	assert.Equal(t, 400, wafertest.Status(result))
}

func TestGetDocumentsList(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Retrieve("/ext/legalpages/api/documents")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var docs []map[string]any
	wafertest.DecodeResponse(t, result, &docs)
	// Should have both terms and privacy entries
	require.Len(t, docs, 2)

	types := map[string]bool{}
	for _, d := range docs {
		types[d["type"].(string)] = true
	}
	assert.True(t, types["terms"])
	assert.True(t, types["privacy"])
}

func TestPublishDocument(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	// The seeded documents are already published at version 1,
	// so publish version 1 again (idempotent operation)
	msg := wafertest.Create("/ext/legalpages/api/documents/terms/publish", map[string]any{
		"version": 1,
	})
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)
	assert.Equal(t, "published", resp["status"])
}

func TestPublishDocument_VersionNotFound(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	msg := wafertest.Create("/ext/legalpages/api/documents/terms/publish", map[string]any{
		"version": 999,
	})
	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionError, result.Action)
	assert.Equal(t, 404, wafertest.Status(result))
}

func TestGetDocumentHistory(t *testing.T) {
	block, ctx, _ := setupLegalPages(t)

	// Create an additional version
	msg := wafertest.Create("/ext/legalpages/api/documents/terms", map[string]any{
		"title":   "Terms v2",
		"content": "<p>Version 2</p>",
	})
	wafertest.WithAuth(msg, "admin-1", "admin@test.com")
	result := block.Handle(ctx, msg)
	require.Equal(t, 201, wafertest.Status(result))

	// Get history
	historyMsg := wafertest.Retrieve("/ext/legalpages/api/documents/terms/history")
	histResult := block.Handle(ctx, historyMsg)

	assert.Equal(t, 200, wafertest.Status(histResult))

	var history []map[string]any
	wafertest.DecodeResponse(t, histResult, &history)
	// Should have 2 versions: seed (v1) + our new one (v2)
	require.Len(t, history, 2)
	// Sorted by version desc, so v2 first
	assert.Equal(t, float64(2), history[0]["version"])
	assert.Equal(t, float64(1), history[1]["version"])
}
