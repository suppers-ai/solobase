// Package auth handles user authentication via OAuth and sessions.
package auth

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// OAuthProvider represents a configured OAuth provider (GitHub, Google).
type OAuthProvider struct {
	Name         string
	ClientID     string
	ClientSecret string
	AuthURL      string
	TokenURL     string
	UserInfoURL  string
	Scopes       []string
	RedirectURL  string
}

// GitHubProvider creates a GitHub OAuth provider.
func GitHubProvider(clientID, clientSecret, redirectURL string) *OAuthProvider {
	return &OAuthProvider{
		Name:         "github",
		ClientID:     clientID,
		ClientSecret: clientSecret,
		AuthURL:      "https://github.com/login/oauth/authorize",
		TokenURL:     "https://github.com/login/oauth/access_token",
		UserInfoURL:  "https://api.github.com/user",
		Scopes:       []string{"user:email"},
		RedirectURL:  redirectURL,
	}
}

// GoogleProvider creates a Google OAuth provider.
func GoogleProvider(clientID, clientSecret, redirectURL string) *OAuthProvider {
	return &OAuthProvider{
		Name:         "google",
		ClientID:     clientID,
		ClientSecret: clientSecret,
		AuthURL:      "https://accounts.google.com/o/oauth2/v2/auth",
		TokenURL:     "https://oauth2.googleapis.com/token",
		UserInfoURL:  "https://www.googleapis.com/oauth2/v2/userinfo",
		Scopes:       []string{"openid", "email", "profile"},
		RedirectURL:  redirectURL,
	}
}

// OAuthToken contains the token response from the provider.
type OAuthToken struct {
	AccessToken  string `json:"access_token"`
	TokenType    string `json:"token_type"`
	Scope        string `json:"scope"`
	RefreshToken string `json:"refresh_token,omitempty"`
	ExpiresIn    int    `json:"expires_in,omitempty"`
}

// UserInfo contains normalized user information from OAuth provider.
type UserInfo struct {
	ProviderID   string `json:"provider_id"`
	Provider     string `json:"provider"`
	Email        string `json:"email"`
	Name         string `json:"name"`
	AvatarURL    string `json:"avatar_url"`
}

// AuthorizeURL returns the URL to redirect the user to for OAuth.
func (p *OAuthProvider) AuthorizeURL(state string) string {
	params := url.Values{}
	params.Set("client_id", p.ClientID)
	params.Set("redirect_uri", p.RedirectURL)
	params.Set("scope", strings.Join(p.Scopes, " "))
	params.Set("state", state)
	params.Set("response_type", "code")
	return p.AuthURL + "?" + params.Encode()
}

// ExchangeCode exchanges an authorization code for tokens.
func (p *OAuthProvider) ExchangeCode(ctx context.Context, code string) (*OAuthToken, error) {
	form := url.Values{}
	form.Set("client_id", p.ClientID)
	form.Set("client_secret", p.ClientSecret)
	form.Set("code", code)
	form.Set("redirect_uri", p.RedirectURL)
	form.Set("grant_type", "authorization_code")

	req, err := http.NewRequestWithContext(ctx, "POST", p.TokenURL, strings.NewReader(form.Encode()))
	if err != nil {
		return nil, fmt.Errorf("create token request: %w", err)
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Set("Accept", "application/json")

	client := &http.Client{Timeout: 15 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("token exchange: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read token response: %w", err)
	}

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("token exchange failed (status %d): %s", resp.StatusCode, string(body))
	}

	var token OAuthToken
	if err := json.Unmarshal(body, &token); err != nil {
		return nil, fmt.Errorf("unmarshal token: %w", err)
	}
	return &token, nil
}

// FetchUserInfo retrieves the user's profile using the access token.
func (p *OAuthProvider) FetchUserInfo(ctx context.Context, token *OAuthToken) (*UserInfo, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", p.UserInfoURL, nil)
	if err != nil {
		return nil, fmt.Errorf("create userinfo request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+token.AccessToken)
	req.Header.Set("Accept", "application/json")

	client := &http.Client{Timeout: 15 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("fetch user info: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read userinfo response: %w", err)
	}

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("userinfo failed (status %d): %s", resp.StatusCode, string(body))
	}

	// Parse provider-specific response
	switch p.Name {
	case "github":
		return parseGitHubUser(body)
	case "google":
		return parseGoogleUser(body)
	default:
		return nil, fmt.Errorf("unknown provider: %s", p.Name)
	}
}

func parseGitHubUser(data []byte) (*UserInfo, error) {
	var gh struct {
		ID        int    `json:"id"`
		Login     string `json:"login"`
		Name      string `json:"name"`
		Email     string `json:"email"`
		AvatarURL string `json:"avatar_url"`
	}
	if err := json.Unmarshal(data, &gh); err != nil {
		return nil, fmt.Errorf("parse github user: %w", err)
	}
	name := gh.Name
	if name == "" {
		name = gh.Login
	}
	return &UserInfo{
		ProviderID: fmt.Sprintf("%d", gh.ID),
		Provider:   "github",
		Email:      gh.Email,
		Name:       name,
		AvatarURL:  gh.AvatarURL,
	}, nil
}

func parseGoogleUser(data []byte) (*UserInfo, error) {
	var g struct {
		ID      string `json:"id"`
		Email   string `json:"email"`
		Name    string `json:"name"`
		Picture string `json:"picture"`
	}
	if err := json.Unmarshal(data, &g); err != nil {
		return nil, fmt.Errorf("parse google user: %w", err)
	}
	return &UserInfo{
		ProviderID: g.ID,
		Provider:   "google",
		Email:      g.Email,
		Name:       g.Name,
		AvatarURL:  g.Picture,
	}, nil
}

// GenerateState creates a random state token for CSRF protection.
func GenerateState() (string, error) {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}
