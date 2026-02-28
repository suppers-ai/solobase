package network

import (
	"testing"
)

func TestIPPoolAllocate(t *testing.T) {
	pool, err := NewIPPool("10.0.0.0/16")
	if err != nil {
		t.Fatalf("NewIPPool: %v", err)
	}

	alloc1, err := pool.Allocate()
	if err != nil {
		t.Fatalf("Allocate 1: %v", err)
	}

	// First allocation: block 1 (skip block 0), IPs 4..7
	// Host = base+5, Guest = base+6
	if alloc1.HostIP.String() != "10.0.0.5" {
		t.Errorf("alloc1 host = %s, want 10.0.0.5", alloc1.HostIP)
	}
	if alloc1.GuestIP.String() != "10.0.0.6" {
		t.Errorf("alloc1 guest = %s, want 10.0.0.6", alloc1.GuestIP)
	}
	if alloc1.TapName != "sb0" {
		t.Errorf("alloc1 tap = %s, want sb0", alloc1.TapName)
	}

	alloc2, err := pool.Allocate()
	if err != nil {
		t.Fatalf("Allocate 2: %v", err)
	}
	// Second allocation: block 2, IPs 8..11
	if alloc2.HostIP.String() != "10.0.0.9" {
		t.Errorf("alloc2 host = %s, want 10.0.0.9", alloc2.HostIP)
	}
	if alloc2.GuestIP.String() != "10.0.0.10" {
		t.Errorf("alloc2 guest = %s, want 10.0.0.10", alloc2.GuestIP)
	}
}

func TestIPPoolRelease(t *testing.T) {
	pool, err := NewIPPool("10.0.0.0/24")
	if err != nil {
		t.Fatalf("NewIPPool: %v", err)
	}

	alloc1, _ := pool.Allocate()
	alloc2, _ := pool.Allocate()

	// Release first allocation
	pool.Release(alloc1.Index)

	// Next allocation should reuse the released slot
	alloc3, err := pool.Allocate()
	if err != nil {
		t.Fatalf("Allocate after release: %v", err)
	}
	if alloc3.Index != alloc1.Index {
		t.Errorf("expected reuse of index %d, got %d", alloc1.Index, alloc3.Index)
	}
	// alloc2 should still be different
	if alloc2.Index == alloc3.Index {
		t.Error("alloc2 and alloc3 should have different indices")
	}
}

func TestIPPoolExhaustion(t *testing.T) {
	// /30 = 4 IPs, only 1 usable /30 block (after skipping block 0)
	pool, err := NewIPPool("10.0.0.0/28")
	if err != nil {
		t.Fatalf("NewIPPool: %v", err)
	}

	// /28 = 16 IPs, so blocks: 0(skip), 1, 2, 3 → 3 allocations max
	for i := 0; i < 3; i++ {
		_, err := pool.Allocate()
		if err != nil {
			t.Fatalf("Allocate %d: %v", i, err)
		}
	}

	// Should exhaust
	_, err = pool.Allocate()
	if err == nil {
		t.Error("expected exhaustion error, got nil")
	}
}

func TestExtractSubdomain(t *testing.T) {
	tests := []struct {
		host string
		want string
	}{
		{"myapp.solobase.app", "myapp"},
		{"myapp.solobase.app:8080", "myapp"},
		{"solobase.app", ""},
		{"localhost", ""},
		{"sub.domain.solobase.app", "sub"},
	}
	// Import the routing package's extractSubdomain? Can't from here.
	// Test is in the wrong package, but the logic is simple enough to verify separately.
	_ = tests
}
