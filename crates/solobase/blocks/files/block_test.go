package files

import (
	"os"
	"testing"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/wafertest"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func setupFiles(t *testing.T) (*FilesBlock, wafer.Context) {
	t.Helper()

	manifest, err := os.ReadFile("block.json")
	require.NoError(t, err)

	// Settings block manifest needed for initializeExtensionSettings
	settingsManifest := []byte(`{
		"name": "solobase/settings",
		"version": "0.0.1",
		"services": {
			"database": {
				"collections": {
					"sys_settings": {
						"fields": {
							"id": { "type": "string", "primary": true },
							"key": { "type": "string", "unique": true },
							"value": { "type": "text" },
							"type": { "type": "string", "default": "string" }
						}
					}
				}
			}
		}
	}`)

	db := wafertest.SetupDBFromManifest(t, manifest, settingsManifest)
	ctx := wafertest.NewContext(db)

	block := NewFilesBlock()
	wafertest.InitBlock(t, block, ctx)

	return block, ctx
}

func TestFilesBlockInfo(t *testing.T) {
	block := NewFilesBlock()
	info := block.Info()

	assert.Equal(t, BlockName, info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, "http.handler", info.Interface)
	assert.Equal(t, wafer.Singleton, info.InstanceMode)
	assert.NotNil(t, info.AdminUI)
	assert.Equal(t, "/admin/storage", info.AdminUI.Path)
}

func TestFilesBlockInit(t *testing.T) {
	block, _ := setupFiles(t)

	assert.NotNil(t, block.db)
	assert.NotNil(t, block.storageService)
	assert.NotNil(t, block.shareService)
	assert.NotNil(t, block.quotaService)
	assert.NotNil(t, block.accessLogService)
	assert.NotNil(t, block.cloudConfig)
}

func TestGetBucketsEmpty(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(wafertest.Retrieve("/storage/buckets"), "user1", "user@example.com")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestCreateBucketMissingName(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(
		wafertest.Create("/storage/buckets", map[string]any{"name": ""}),
		"user1", "user@example.com",
	)
	result := block.Handle(ctx, msg)
	assert.Equal(t, 400, wafertest.Status(result))
}

func TestGetSharesUnauthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.Retrieve("/ext/cloudstorage/shares")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 401, wafertest.Status(result))
}

func TestGetSharesAuthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(wafertest.Retrieve("/ext/cloudstorage/shares"), "user1", "user@example.com")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestGetCloudQuotaUnauthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.Retrieve("/ext/cloudstorage/quota")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 401, wafertest.Status(result))
}

func TestGetCloudStatsUnauthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.Retrieve("/ext/cloudstorage/stats")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 401, wafertest.Status(result))
}

func TestGetCloudStatsAuthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(wafertest.Retrieve("/ext/cloudstorage/stats"), "user1", "user@example.com")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestGetRoleQuotasEmpty(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(
		wafertest.WithRoles(wafertest.Retrieve("/ext/cloudstorage/quotas/roles"), "admin"),
		"admin1", "admin@example.com",
	)
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestSearchObjectsUnauthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.Retrieve("/storage/search")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 401, wafertest.Status(result))
}

func TestSearchObjectsEmptyQuery(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(wafertest.Retrieve("/storage/search"), "user1", "user@example.com")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}

func TestGetQuotaUnauthenticated(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.Retrieve("/storage/quota")
	result := block.Handle(ctx, msg)
	assert.Equal(t, 401, wafertest.Status(result))
}

func TestDefaultQuotasGet(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(
		wafertest.WithRoles(wafertest.Retrieve("/ext/cloudstorage/default-quotas"), "admin"),
		"admin1", "admin@example.com",
	)
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))

	var resp map[string]any
	wafertest.DecodeResponse(t, result, &resp)
	assert.Equal(t, float64(5368709120), resp["defaultStorage"])
	assert.Equal(t, float64(10737418240), resp["defaultBandwidth"])
}

func TestUserSearchShortQuery(t *testing.T) {
	block, ctx := setupFiles(t)

	msg := wafertest.WithAuth(
		wafertest.WithQuery(wafertest.Retrieve("/ext/cloudstorage/users/search"), "q", "a"),
		"admin1", "admin@example.com",
	)
	result := block.Handle(ctx, msg)
	assert.Equal(t, 200, wafertest.Status(result))
}
