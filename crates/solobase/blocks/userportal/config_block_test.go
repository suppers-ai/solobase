package userportal

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/wafertest"
)

func setupConfigBlock(t *testing.T) (*UserPortalConfigBlock, wafer.Context) {
	t.Helper()
	portal := NewUserPortalBlock(nil) // nil config -> defaults
	block := NewUserPortalConfigBlock(portal)
	db := wafertest.SetupDB(t)
	ctx := wafertest.NewContext(db)
	wafertest.InitBlock(t, block, ctx)
	return block, ctx
}

func TestGetConfigDefaults(t *testing.T) {
	block, ctx := setupConfigBlock(t)
	msg := wafertest.Retrieve("/ext/userportal/config")

	result := block.Handle(ctx, msg)

	assert.Equal(t, wafer.ActionRespond, result.Action)
	assert.Equal(t, 200, wafertest.Status(result))

	var config UserPortalConfig
	wafertest.DecodeResponse(t, result, &config)
	assert.Equal(t, "Solobase", config.AppName)
	assert.Equal(t, "#189AB4", config.PrimaryColor)
	assert.Equal(t, "/logo_long.png", config.LogoURL)
	assert.Equal(t, "/logo.png", config.LogoCollapsed)
	assert.True(t, config.EnableOAuth)
	assert.True(t, config.AllowSignup)
	assert.Equal(t, "/profile", config.RedirectAfter)
	assert.Contains(t, config.OAuthProviders, "google")
}

func TestGetConfigCustom(t *testing.T) {
	portal := NewUserPortalBlock(&UserPortalConfig{
		LogoURL:        "/custom-logo.png",
		LogoCollapsed:  "/custom-icon.png",
		PrimaryColor:   "#FF0000",
		AppName:        "CustomApp",
		EnableOAuth:    false,
		OAuthProviders: []string{},
		RedirectAfter:  "/dashboard",
		AllowSignup:    false,
	})
	block := NewUserPortalConfigBlock(portal)
	db := wafertest.SetupDB(t)
	ctx := wafertest.NewContext(db)
	wafertest.InitBlock(t, block, ctx)

	msg := wafertest.Retrieve("/ext/userportal/config")
	result := block.Handle(ctx, msg)

	assert.Equal(t, 200, wafertest.Status(result))

	var config UserPortalConfig
	wafertest.DecodeResponse(t, result, &config)
	assert.Equal(t, "CustomApp", config.AppName)
	assert.Equal(t, "#FF0000", config.PrimaryColor)
	assert.Equal(t, "/custom-logo.png", config.LogoURL)
	assert.False(t, config.EnableOAuth)
	assert.False(t, config.AllowSignup)
	assert.Equal(t, "/dashboard", config.RedirectAfter)
}

func TestGetConfigResponseIsValidJSON(t *testing.T) {
	block, ctx := setupConfigBlock(t)
	msg := wafertest.Retrieve("/ext/userportal/config")

	result := block.Handle(ctx, msg)

	body := wafertest.ResponseBody(result)
	require.NotNil(t, body)

	// Verify it's valid JSON
	var raw json.RawMessage
	err := json.Unmarshal(body, &raw)
	assert.NoError(t, err, "response should be valid JSON")
}

func TestBlockInfo(t *testing.T) {
	portal := NewUserPortalBlock(nil)
	block := NewUserPortalConfigBlock(portal)
	info := block.Info()
	assert.Equal(t, BlockName, info.Name)
	assert.Equal(t, "1.0.0", info.Version)
	assert.Equal(t, wafer.Singleton, info.InstanceMode)
}
