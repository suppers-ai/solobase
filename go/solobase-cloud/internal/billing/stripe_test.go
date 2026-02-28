package billing

import (
	"testing"
)

func TestDefaultPlans(t *testing.T) {
	plans := DefaultPlans()
	if len(plans) != 5 {
		t.Fatalf("expected 5 plans, got %d", len(plans))
	}

	ids := map[string]bool{}
	for _, p := range plans {
		ids[p.ID] = true
	}
	for _, expected := range []string{"free", "hobby", "starter", "professional", "business"} {
		if !ids[expected] {
			t.Errorf("missing plan %q", expected)
		}
	}
}

func TestGetPlan(t *testing.T) {
	tests := []struct {
		id    string
		found bool
	}{
		{"free", true},
		{"hobby", true},
		{"starter", true},
		{"professional", true},
		{"business", true},
		{"pro", false},
		{"enterprise", false},
		{"", false},
	}
	for _, tt := range tests {
		p := GetPlan(tt.id)
		if (p != nil) != tt.found {
			t.Errorf("GetPlan(%q): got %v, want found=%v", tt.id, p, tt.found)
		}
	}
}

func TestFreePlanConfig(t *testing.T) {
	p := GetPlan("free")
	if p == nil {
		t.Fatal("free plan not found")
	}
	if p.MaxVMs != 1 {
		t.Errorf("free plan MaxVMs = %d, want 1", p.MaxVMs)
	}
	if p.PriceID != "" {
		t.Errorf("free plan PriceID = %q, want empty", p.PriceID)
	}
	if p.VCPUs != 1 {
		t.Errorf("free plan VCPUs = %d, want 1", p.VCPUs)
	}
	if p.PriceCents != 0 {
		t.Errorf("free plan PriceCents = %d, want 0", p.PriceCents)
	}
	if !p.AutoSleep {
		t.Error("free plan should have AutoSleep enabled")
	}
	if p.AlwaysOn {
		t.Error("free plan should not be always-on")
	}
}

func TestHobbyPlanConfig(t *testing.T) {
	p := GetPlan("hobby")
	if p == nil {
		t.Fatal("hobby plan not found")
	}
	if p.MaxVMs != 1 {
		t.Errorf("hobby plan MaxVMs = %d, want 1", p.MaxVMs)
	}
	if p.PriceCents != 500 {
		t.Errorf("hobby plan PriceCents = %d, want 500", p.PriceCents)
	}
	if p.MemMB != 512 {
		t.Errorf("hobby plan MemMB = %d, want 512", p.MemMB)
	}
	if len(p.Features) < 5 {
		t.Errorf("hobby plan should have many features, got %d", len(p.Features))
	}
}

func TestStarterPlanConfig(t *testing.T) {
	p := GetPlan("starter")
	if p == nil {
		t.Fatal("starter plan not found")
	}
	if p.PriceCents != 1500 {
		t.Errorf("starter plan PriceCents = %d, want 1500", p.PriceCents)
	}
	if !p.AlwaysOn {
		t.Error("starter plan should be always-on")
	}
	if p.AutoSleep {
		t.Error("starter plan should not auto-sleep")
	}
}

func TestProfessionalPlanConfig(t *testing.T) {
	p := GetPlan("professional")
	if p == nil {
		t.Fatal("professional plan not found")
	}
	if p.MaxVMs != 3 {
		t.Errorf("professional plan MaxVMs = %d, want 3", p.MaxVMs)
	}
	if p.PriceCents != 7900 {
		t.Errorf("professional plan PriceCents = %d, want 7900", p.PriceCents)
	}
	if !p.CustomDomain {
		t.Error("professional plan should support custom domains")
	}
}

func TestBusinessPlanConfig(t *testing.T) {
	p := GetPlan("business")
	if p == nil {
		t.Fatal("business plan not found")
	}
	if p.MaxVMs != 10 {
		t.Errorf("business plan MaxVMs = %d, want 10", p.MaxVMs)
	}
	if p.MemMB != 4096 {
		t.Errorf("business plan MemMB = %d, want 4096", p.MemMB)
	}
	if p.PriceCents != 19900 {
		t.Errorf("business plan PriceCents = %d, want 19900", p.PriceCents)
	}
	if len(p.Features) < 5 {
		t.Errorf("business plan should have many features, got %d", len(p.Features))
	}
}

func TestNewStripeClient(t *testing.T) {
	client := NewStripeClient("sk_test_123")
	if client == nil {
		t.Fatal("expected non-nil client")
	}
	if client.apiKey != "sk_test_123" {
		t.Errorf("apiKey = %q, want %q", client.apiKey, "sk_test_123")
	}
}

func TestNewUsageCollector(t *testing.T) {
	client := NewStripeClient("sk_test_123")
	collector := NewUsageCollector(client, 0)
	if collector == nil {
		t.Fatal("expected non-nil collector")
	}
}
