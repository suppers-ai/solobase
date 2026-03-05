package auth

import (
	"github.com/suppers-ai/solobase/adapters/auth/oauth"
	"github.com/suppers-ai/solobase/core/env"
	wafer "github.com/wafer-run/wafer-go"
)

func (b *AuthBlock) handleOAuthProviders(_ wafer.Context, msg *wafer.Message) wafer.Result {
	providers := b.oauthProvider.GetAvailableProviders()
	return wafer.JSONRespond(msg, 200, map[string]any{
		"providers": providers,
	})
}

func (b *AuthBlock) handleOAuthLogin(_ wafer.Context, msg *wafer.Message) wafer.Result {
	providerName := msg.Query("provider")
	if providerName == "" {
		return wafer.Error(msg, 400, "bad_request", "provider query parameter is required")
	}

	authURL, _, stateCookie, err := b.oauthProvider.GetLoginURL(providerName)
	if err != nil {
		return wafer.Error(msg, 400, "oauth_error", err.Error())
	}

	return wafer.NewResponse(msg, 302).
		SetCookie(stateCookie).
		SetHeader("Location", authURL).
		Body(nil, "")
}

func (b *AuthBlock) handleOAuthCallback(_ wafer.Context, msg *wafer.Message) wafer.Result {
	providerName := msg.Var("provider")
	code := msg.Query("code")
	state := msg.Query("state")

	if code == "" {
		return wafer.Error(msg, 400, "bad_request", "missing authorization code")
	}

	// Verify state matches cookie
	savedState := msg.Cookie("oauth_state")
	if savedState == "" || savedState != state {
		return wafer.Error(msg, 400, "bad_request", "invalid OAuth state")
	}

	result, err := b.oauthProvider.ProcessCallback(providerName, code)
	if err != nil {
		return wafer.Error(msg, 500, "oauth_error", err.Error())
	}

	return wafer.NewResponse(msg, 302).
		SetCookie(result.AuthCookie).
		SetCookie(result.ClearCookie).
		SetHeader("Location", "/").
		Body(nil, "")
}

func (b *AuthBlock) handleOAuthSyncUser(_ wafer.Context, msg *wafer.Message) wafer.Result {
	// Verify internal call via secret header
	internalSecret := msg.Header("X-Internal-Secret")
	expectedSecret := env.GetEnv("INTERNAL_SECRET")
	if expectedSecret != "" && internalSecret != expectedSecret {
		return wafer.Error(msg, 403, "forbidden", "Forbidden")
	}

	var syncReq oauth.SyncRequest
	if err := msg.Decode(&syncReq); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid request body")
	}

	result, err := b.oauthProvider.SyncUser(&syncReq)
	if err != nil {
		return wafer.Error(msg, 500, "sync_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, result)
}
