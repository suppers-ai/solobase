package userportal

import (
	"embed"
	"io/fs"
)

//go:embed all:frontend/build/*
var frontendFiles embed.FS

// UserPortalConfig holds the block configuration
type UserPortalConfig struct {
	LogoURL        string   `json:"logoUrl"`
	LogoCollapsed  string   `json:"logoCollapsed"`
	PrimaryColor   string   `json:"primaryColor"`
	AppName        string   `json:"appName"`
	EnableOAuth    bool     `json:"enableOAuth"`
	OAuthProviders []string `json:"oauthProviders"`
	RedirectAfter  string   `json:"redirectAfterLogin"`
	AllowSignup    bool     `json:"allowSignup"`
}

// UserPortalBlock provides user-facing authentication and profile pages
type UserPortalBlock struct {
	config *UserPortalConfig
}

// NewUserPortalBlock creates a new user portal block
func NewUserPortalBlock(config *UserPortalConfig) *UserPortalBlock {
	if config == nil {
		config = &UserPortalConfig{
			LogoURL:        "/logo_long.png",
			LogoCollapsed:  "/logo.png",
			PrimaryColor:   "#189AB4",
			AppName:        "Solobase",
			EnableOAuth:    true,
			OAuthProviders: []string{"google"},
			RedirectAfter:  "/profile",
			AllowSignup:    true,
		}
	}
	return &UserPortalBlock{config: config}
}

// GetFrontendFS returns the embedded frontend filesystem
func (b *UserPortalBlock) GetFrontendFS() fs.FS {
	buildFS, err := fs.Sub(frontendFiles, "frontend/build")
	if err != nil {
		return nil
	}
	return buildFS
}

// GetRoutes returns the SPA routes this block handles
func (b *UserPortalBlock) GetRoutes() []string {
	return []string{
		"/login",
		"/signup",
		"/logout",
		"/profile",
		"/profile/",
		"/oauth/",
		"/products",
		"/products/checkout",
		"/products/success",
	}
}

// GetAssetBasePath returns the base path for serving frontend assets
func (b *UserPortalBlock) GetAssetBasePath() string {
	return "/_ext/userportal/"
}
