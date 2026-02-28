// Package main implements a mock solobase-node server for local development.
// It simulates the node API with in-memory state, supporting tenant lifecycle
// operations (create, pause, resume, destroy) and health reporting.
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"
)

type tenant struct {
	ID           string    `json:"id"`
	Subdomain    string    `json:"subdomain"`
	State        string    `json:"state"`
	VMIP         string    `json:"vm_ip"`
	CreatedAt    time.Time `json:"created_at"`
	LastActivity time.Time `json:"last_activity"`
}

type mockNode struct {
	mu      sync.RWMutex
	tenants map[string]*tenant
	secret  string
	maxVMs  int
}

func main() {
	addr := os.Getenv("LISTEN_ADDR")
	if addr == "" {
		addr = ":9090"
	}
	secret := os.Getenv("NODE_SECRET")
	if secret == "" {
		secret = "dev-secret"
	}

	mn := &mockNode{
		tenants: make(map[string]*tenant),
		secret:  secret,
		maxVMs:  20,
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/api/health", mn.handleHealth)
	mux.HandleFunc("/api/tenants", mn.authCheck(mn.handleTenants))
	mux.HandleFunc("/api/tenants/", mn.authCheck(mn.handleTenantAction))

	log.Printf("Mock node listening on %s (secret=%s)", addr, secret)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatalf("mock-node: %v", err)
	}
}

func (mn *mockNode) authCheck(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth != "Bearer "+mn.secret {
			http.Error(w, `{"error":"unauthorized"}`, http.StatusUnauthorized)
			return
		}
		next(w, r)
	}
}

func (mn *mockNode) handleHealth(w http.ResponseWriter, r *http.Request) {
	mn.mu.RLock()
	running, paused := 0, 0
	for _, t := range mn.tenants {
		switch t.State {
		case "running":
			running++
		case "paused":
			paused++
		}
	}
	mn.mu.RUnlock()

	total := running + paused
	writeJSON(w, 200, map[string]interface{}{
		"status":     "ok",
		"max_vms":    mn.maxVMs,
		"running":    running,
		"paused":     paused,
		"total":      total,
		"free_slots": mn.maxVMs - total,
	})
}

func (mn *mockNode) handleTenants(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req struct {
		TenantID  string            `json:"tenant_id"`
		Subdomain string            `json:"subdomain"`
		VCPUs     int               `json:"vcpus"`
		MemMB     int               `json:"mem_mb"`
		Config    map[string]string `json:"config"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error":"invalid request"}`, http.StatusBadRequest)
		return
	}

	now := time.Now()
	t := &tenant{
		ID:           req.TenantID,
		Subdomain:    req.Subdomain,
		State:        "running",
		VMIP:         "10.0.0." + fmt.Sprintf("%d", len(mn.tenants)+2),
		CreatedAt:    now,
		LastActivity: now,
	}

	mn.mu.Lock()
	mn.tenants[req.TenantID] = t
	mn.mu.Unlock()

	log.Printf("Created tenant %s (%s)", req.TenantID, req.Subdomain)
	writeJSON(w, 200, t)
}

func (mn *mockNode) handleTenantAction(w http.ResponseWriter, r *http.Request) {
	path := strings.TrimPrefix(r.URL.Path, "/api/tenants/")
	parts := strings.SplitN(path, "/", 2)
	tenantID := parts[0]

	action := ""
	if len(parts) > 1 {
		action = parts[1]
	}

	mn.mu.Lock()
	defer mn.mu.Unlock()

	t, ok := mn.tenants[tenantID]
	if !ok && action != "" {
		http.Error(w, `{"error":"tenant not found"}`, http.StatusNotFound)
		return
	}

	switch {
	case action == "status" && r.Method == http.MethodGet:
		if !ok {
			http.Error(w, `{"error":"tenant not found"}`, http.StatusNotFound)
			return
		}
		writeJSON(w, 200, t)

	case action == "pause" && r.Method == http.MethodPost:
		t.State = "paused"
		t.LastActivity = time.Now()
		log.Printf("Paused tenant %s", tenantID)
		w.WriteHeader(200)

	case action == "resume" && r.Method == http.MethodPost:
		t.State = "running"
		t.LastActivity = time.Now()
		log.Printf("Resumed tenant %s", tenantID)
		w.WriteHeader(200)

	case action == "" && r.Method == http.MethodDelete:
		delete(mn.tenants, tenantID)
		log.Printf("Deleted tenant %s", tenantID)
		w.WriteHeader(200)

	default:
		http.Error(w, `{"error":"not found"}`, http.StatusNotFound)
	}
}

func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}
