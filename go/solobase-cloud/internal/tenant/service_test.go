package tenant

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/node"
)

// mockNodeServer creates a test HTTP server simulating a solobase-node API.
func mockNodeServer(t *testing.T) *httptest.Server {
	t.Helper()
	mux := http.NewServeMux()

	mux.HandleFunc("/api/health", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]interface{}{
			"status":     "ok",
			"max_vms":    10,
			"running":    2,
			"paused":     1,
			"total":      3,
			"free_slots": 7,
		})
	})

	mux.HandleFunc("/api/tenants", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == "POST" {
			var req map[string]interface{}
			json.NewDecoder(r.Body).Decode(&req)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"id":        req["tenant_id"],
				"subdomain": req["subdomain"],
				"state":     "running",
				"vm_ip":     "10.0.0.5",
			})
			return
		}
		http.Error(w, "method not allowed", 405)
	})

	mux.HandleFunc("/api/tenants/", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	return httptest.NewServer(mux)
}

func TestCreateTenant(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID:      "node-1",
		BaseURL: server.URL,
		Secret:  "test-secret",
		Region:  "us-east",
		IP:      "1.2.3.4",
	})

	svc := NewService(scheduler)

	tenant, err := svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-1",
		Subdomain: "myapp",
		Plan:      "free",
		Features:  []string{"auth", "admin"},
	})
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	if tenant.ID == "" {
		t.Error("expected non-empty tenant ID")
	}
	if tenant.Subdomain != "myapp" {
		t.Errorf("Subdomain = %q, want %q", tenant.Subdomain, "myapp")
	}
	if tenant.UserID != "user-1" {
		t.Errorf("UserID = %q, want %q", tenant.UserID, "user-1")
	}
	if tenant.State != "running" {
		t.Errorf("State = %q, want %q", tenant.State, "running")
	}
	if tenant.NodeID != "node-1" {
		t.Errorf("NodeID = %q, want %q", tenant.NodeID, "node-1")
	}
	if tenant.NodeIP != "1.2.3.4" {
		t.Errorf("NodeIP = %q, want %q", tenant.NodeIP, "1.2.3.4")
	}
}

func TestCreateTenant_DuplicateSubdomain(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID:      "node-1",
		BaseURL: server.URL,
		Secret:  "test-secret",
		Region:  "us-east",
		IP:      "1.2.3.4",
	})

	svc := NewService(scheduler)

	_, err := svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-1",
		Subdomain: "taken",
		Plan:      "free",
	})
	if err != nil {
		t.Fatalf("Create first: %v", err)
	}

	_, err = svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-2",
		Subdomain: "taken",
		Plan:      "free",
	})
	if err == nil {
		t.Fatal("expected error for duplicate subdomain")
	}
}

func TestCreateTenant_NoNodes(t *testing.T) {
	scheduler := node.NewScheduler()
	svc := NewService(scheduler)

	_, err := svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-1",
		Subdomain: "test",
		Plan:      "free",
	})
	if err == nil {
		t.Fatal("expected error when no nodes registered")
	}
}

func TestGetTenant(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID:      "node-1",
		BaseURL: server.URL,
		Secret:  "s",
		Region:  "us",
		IP:      "1.1.1.1",
	})

	svc := NewService(scheduler)

	created, _ := svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-1",
		Subdomain: "gettest",
		Plan:      "free",
	})

	got, ok := svc.Get(created.ID)
	if !ok {
		t.Fatal("tenant not found")
	}
	if got.Subdomain != "gettest" {
		t.Errorf("Subdomain = %q, want %q", got.Subdomain, "gettest")
	}

	_, ok = svc.Get("nonexistent")
	if ok {
		t.Error("expected not found for bogus ID")
	}
}

func TestGetTenant_ReturnsCopy(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "node-1", BaseURL: server.URL, Secret: "s", Region: "us", IP: "1.1.1.1",
	})

	svc := NewService(scheduler)
	created, _ := svc.Create(context.Background(), &CreateRequest{
		UserID: "user-1", Subdomain: "copytest", Plan: "free",
	})

	got1, _ := svc.Get(created.ID)
	got1.Subdomain = "modified"

	got2, _ := svc.Get(created.ID)
	if got2.Subdomain != "copytest" {
		t.Error("Get should return a copy, not a reference to internal state")
	}
}

func TestListByUser(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "node-1", BaseURL: server.URL, Secret: "s", Region: "us", IP: "1.1.1.1",
	})

	svc := NewService(scheduler)

	// Create tenants for two users
	for i := 0; i < 3; i++ {
		svc.Create(context.Background(), &CreateRequest{
			UserID:    "user-1",
			Subdomain: fmt.Sprintf("app-%d", i),
			Plan:      "free",
		})
	}
	svc.Create(context.Background(), &CreateRequest{
		UserID:    "user-2",
		Subdomain: "other-app",
		Plan:      "starter",
	})

	user1 := svc.ListByUser("user-1")
	if len(user1) != 3 {
		t.Errorf("user-1 tenants = %d, want 3", len(user1))
	}

	user2 := svc.ListByUser("user-2")
	if len(user2) != 1 {
		t.Errorf("user-2 tenants = %d, want 1", len(user2))
	}

	empty := svc.ListByUser("nobody")
	if len(empty) != 0 {
		t.Errorf("nobody tenants = %d, want 0", len(empty))
	}
}

func TestDeleteTenant(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "node-1", BaseURL: server.URL, Secret: "s", Region: "us", IP: "1.1.1.1",
	})

	svc := NewService(scheduler)

	created, _ := svc.Create(context.Background(), &CreateRequest{
		UserID: "user-1", Subdomain: "todelete", Plan: "free",
	})

	err := svc.Delete(context.Background(), created.ID)
	if err != nil {
		t.Fatalf("Delete: %v", err)
	}

	_, ok := svc.Get(created.ID)
	if ok {
		t.Error("expected tenant to be deleted")
	}
}

func TestDeleteTenant_NotFound(t *testing.T) {
	scheduler := node.NewScheduler()
	svc := NewService(scheduler)

	err := svc.Delete(context.Background(), "nonexistent")
	if err == nil {
		t.Fatal("expected error for nonexistent tenant")
	}
}

func TestPauseTenant(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "node-1", BaseURL: server.URL, Secret: "s", Region: "us", IP: "1.1.1.1",
	})

	svc := NewService(scheduler)
	created, err := svc.Create(context.Background(), &CreateRequest{
		UserID: "user-1", Subdomain: "pausetest", Plan: "free",
	})
	if err != nil {
		t.Fatalf("Create: %v", err)
	}
	if created.State != "running" {
		t.Fatalf("initial State = %q, want %q", created.State, "running")
	}

	if err := svc.Pause(context.Background(), created.ID); err != nil {
		t.Fatalf("Pause: %v", err)
	}

	got, ok := svc.Get(created.ID)
	if !ok {
		t.Fatal("tenant not found after pause")
	}
	if got.State != "paused" {
		t.Errorf("State after Pause = %q, want %q", got.State, "paused")
	}
}

func TestResumeTenant(t *testing.T) {
	server := mockNodeServer(t)
	defer server.Close()

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "node-1", BaseURL: server.URL, Secret: "s", Region: "us", IP: "1.1.1.1",
	})

	svc := NewService(scheduler)
	created, err := svc.Create(context.Background(), &CreateRequest{
		UserID: "user-1", Subdomain: "resumetest", Plan: "free",
	})
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	// Pause first
	if err := svc.Pause(context.Background(), created.ID); err != nil {
		t.Fatalf("Pause: %v", err)
	}

	got, _ := svc.Get(created.ID)
	if got.State != "paused" {
		t.Fatalf("State after Pause = %q, want %q", got.State, "paused")
	}

	// Resume
	if err := svc.Resume(context.Background(), created.ID); err != nil {
		t.Fatalf("Resume: %v", err)
	}

	got, ok := svc.Get(created.ID)
	if !ok {
		t.Fatal("tenant not found after resume")
	}
	if got.State != "running" {
		t.Errorf("State after Resume = %q, want %q", got.State, "running")
	}
}
