package dns

import (
	"testing"
)

func TestNewCloudflareClient(t *testing.T) {
	client := NewCloudflareClient("token-123", "zone-abc", "solobase.app")
	if client == nil {
		t.Fatal("expected non-nil client")
	}
	if client.apiToken != "token-123" {
		t.Errorf("apiToken = %q, want %q", client.apiToken, "token-123")
	}
	if client.zoneID != "zone-abc" {
		t.Errorf("zoneID = %q, want %q", client.zoneID, "zone-abc")
	}
	if client.domain != "solobase.app" {
		t.Errorf("domain = %q, want %q", client.domain, "solobase.app")
	}
}
