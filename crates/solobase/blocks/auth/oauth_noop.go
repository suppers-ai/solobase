package auth

import (
	"fmt"

	"github.com/suppers-ai/solobase/adapters/auth/oauth"
)

// noopOAuthProvider is a stub OAuth provider used when the network service is not available.
type noopOAuthProvider struct{}

func (p *noopOAuthProvider) RegisterProvider(_, _, _ string, _ []string) error { return nil }
func (p *noopOAuthProvider) GetAvailableProviders() []string                   { return nil }
func (p *noopOAuthProvider) GetLoginURL(_ string) (string, string, string, error) {
	return "", "", "", fmt.Errorf("OAuth is not available")
}
func (p *noopOAuthProvider) ProcessCallback(_, _ string) (*oauth.CallbackResult, error) {
	return nil, fmt.Errorf("OAuth is not available")
}
func (p *noopOAuthProvider) SyncUser(_ *oauth.SyncRequest) (*oauth.SyncResult, error) {
	return nil, fmt.Errorf("OAuth is not available")
}
