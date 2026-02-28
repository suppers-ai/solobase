package auth

import (
	"github.com/suppers-ai/solobase/adapters/auth/oauth"
	"github.com/suppers-ai/solobase/core/env"
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *AuthBlock) handleOAuthProviders(_ waffle.Context, msg *waffle.Message) waffle.Result {
	providers := b.oauthProvider.GetAvailableProviders()
	return waffle.JSONRespond(msg, 200, map[string]any{
		"providers": providers,
	})
}

func (b *AuthBlock) handleOAuthLogin(_ waffle.Context, msg *waffle.Message) waffle.Result {
	providerName := msg.Query("provider")
	if providerName == "" {
		return waffle.Error(msg, 400, "bad_request", "provider query parameter is required")
	}

	authURL, _, stateCookie, err := b.oauthProvider.GetLoginURL(providerName)
	if err != nil {
		return waffle.Error(msg, 400, "oauth_error", err.Error())
	}

	return waffle.NewResponse(msg, 302).
		SetCookie(stateCookie).
		SetHeader("Location", authURL).
		Body(nil, "")
}

func (b *AuthBlock) handleOAuthCallback(_ waffle.Context, msg *waffle.Message) waffle.Result {
	providerName := msg.Var("provider")
	code := msg.Query("code")
	state := msg.Query("state")

	if code == "" {
		return waffle.Error(msg, 400, "bad_request", "missing authorization code")
	}

	// Verify state matches cookie
	savedState := msg.Cookie("oauth_state")
	if savedState == "" || savedState != state {
		return waffle.Error(msg, 400, "bad_request", "invalid OAuth state")
	}

	result, err := b.oauthProvider.ProcessCallback(providerName, code)
	if err != nil {
		return waffle.Error(msg, 500, "oauth_error", err.Error())
	}

	return waffle.NewResponse(msg, 302).
		SetCookie(result.AuthCookie).
		SetCookie(result.ClearCookie).
		SetHeader("Location", "/").
		Body(nil, "")
}

func (b *AuthBlock) handleOAuthSyncUser(_ waffle.Context, msg *waffle.Message) waffle.Result {
	// Verify internal call via secret header
	internalSecret := msg.Header("X-Internal-Secret")
	expectedSecret := env.GetEnv("INTERNAL_SECRET")
	if expectedSecret != "" && internalSecret != expectedSecret {
		return waffle.Error(msg, 403, "forbidden", "Forbidden")
	}

	var syncReq oauth.SyncRequest
	if err := msg.Decode(&syncReq); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid request body")
	}

	result, err := b.oauthProvider.SyncUser(&syncReq)
	if err != nil {
		return waffle.Error(msg, 500, "sync_error", err.Error())
	}

	return waffle.JSONRespond(msg, 200, result)
}
