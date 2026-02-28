package tenant

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestStoreRoundTrip(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "tenants.json")

	store, err := NewStore(path)
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}

	tenant1 := &Tenant{
		ID:           "t1",
		Subdomain:    "myapp",
		State:        StateRunning,
		VMIP:         "10.0.0.6",
		CreatedAt:    time.Now(),
		LastActivity: time.Now(),
	}
	if err := store.Put(tenant1); err != nil {
		t.Fatalf("Put: %v", err)
	}

	got, ok := store.Get("t1")
	if !ok {
		t.Fatal("Get: not found")
	}
	if got.Subdomain != "myapp" {
		t.Errorf("got subdomain = %q, want %q", got.Subdomain, "myapp")
	}

	got2, ok := store.GetBySubdomain("myapp")
	if !ok {
		t.Fatal("GetBySubdomain: not found")
	}
	if got2.ID != "t1" {
		t.Errorf("got ID = %q, want %q", got2.ID, "t1")
	}

	list := store.List()
	if len(list) != 1 {
		t.Errorf("List: got %d, want 1", len(list))
	}

	// Reload from disk
	store2, err := NewStore(path)
	if err != nil {
		t.Fatalf("NewStore reload: %v", err)
	}
	got3, ok := store2.Get("t1")
	if !ok {
		t.Fatal("reloaded store: not found")
	}
	if got3.VMIP != "10.0.0.6" {
		t.Errorf("reloaded VMIP = %q, want %q", got3.VMIP, "10.0.0.6")
	}

	if err := store.Delete("t1"); err != nil {
		t.Fatalf("Delete: %v", err)
	}
	_, ok = store.Get("t1")
	if ok {
		t.Error("Get after delete: still found")
	}
}

func TestStoreCountByState(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "tenants.json")

	store, err := NewStore(path)
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}

	for i, state := range []State{StateRunning, StateRunning, StatePaused, StateStopped} {
		store.Put(&Tenant{
			ID:    fmt.Sprintf("t%d", i),
			State: state,
		})
	}

	if n := store.CountByState(StateRunning); n != 2 {
		t.Errorf("running = %d, want 2", n)
	}
	if n := store.CountByState(StatePaused); n != 1 {
		t.Errorf("paused = %d, want 1", n)
	}
}

func TestStoreUpdateState(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "tenants.json")
	store, _ := NewStore(path)

	store.Put(&Tenant{ID: "t1", State: StateRunning})
	if err := store.UpdateState("t1", StatePaused, ""); err != nil {
		t.Fatalf("UpdateState: %v", err)
	}

	got, _ := store.Get("t1")
	if got.State != StatePaused {
		t.Errorf("state = %s, want %s", got.State, StatePaused)
	}
}

func TestProvisionOverlay(t *testing.T) {
	dir := t.TempDir()
	cfg := &SolobaseConfig{
		DatabaseType: "sqlite",
		DatabasePath: "/data/solobase.db",
		StorageType:  "local",
		StorageRoot:  "/data/storage",
		BindAddr:     "0.0.0.0:8090",
		JWTSecret:    "test-secret",
		Features:     map[string]bool{"auth": true, "admin": true},
	}

	overlayDir, err := ProvisionOverlay(dir, "test-tenant", cfg)
	if err != nil {
		t.Fatalf("ProvisionOverlay: %v", err)
	}

	configPath := filepath.Join(overlayDir, "upper", "etc", "solobase", "solobase.toml")
	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("read config: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "sqlite") {
		t.Error("config missing sqlite")
	}
	if !strings.Contains(content, "test-secret") {
		t.Error("config missing jwt_secret")
	}
}
