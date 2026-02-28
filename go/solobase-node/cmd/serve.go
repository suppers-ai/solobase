// Package main is the entry point for the solobase-node Firecracker orchestrator.
//
// solobase-node runs on bare metal servers (e.g. Hetzner AX41) and manages
// Firecracker microVMs — one per tenant — providing full process-level isolation.
//
// It exposes two interfaces:
//  1. Management API (e.g. :9090) — called by the control plane to provision,
//     pause, resume, and destroy tenant VMs.
//  2. Tenant proxy (e.g. :80/:443) — routes subdomain-based HTTP traffic to
//     the correct VM, waking paused VMs on demand (scale-to-zero).
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/config"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/health"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/network"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/routing"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/vm"
)

func main() {
	// Load config
	configPath := os.Getenv("SOLOBASE_NODE_CONFIG")
	if configPath == "" {
		configPath = "solobase-node.json"
	}
	cfg, err := config.Load(configPath)
	if err != nil {
		log.Fatalf("load config: %v", err)
	}

	log.Printf("solobase-node starting (listen=%s, data=%s)", cfg.ListenAddr, cfg.DataDir)

	// Enable IP forwarding
	if err := network.EnableIPForwarding(); err != nil {
		log.Printf("warning: could not enable IP forwarding: %v", err)
	}

	// Initialize IP pool
	ipPool, err := network.NewIPPool(cfg.SubnetCIDR)
	if err != nil {
		log.Fatalf("create IP pool: %v", err)
	}

	// Initialize tenant store
	storePath := filepath.Join(cfg.DataDir, "tenants.json")
	store, err := tenant.NewStore(storePath)
	if err != nil {
		log.Fatalf("open tenant store: %v", err)
	}

	// Initialize VM manager
	vmMgr := vm.NewManager(cfg, store, ipPool)

	// Initialize wake manager (for scale-to-zero)
	wakeMgr := routing.NewWakeManager(vmMgr, store)

	// Initialize idle checker (pause inactive VMs)
	idleThreshold := time.Duration(cfg.IdleTimeoutSec) * time.Second
	idleChecker := routing.NewIdleChecker(store, vmMgr, idleThreshold, 30*time.Second)
	idleChecker.Start()
	defer idleChecker.Stop()

	// Initialize health checker
	healthChecker := health.NewChecker(store, 60*time.Second)
	healthChecker.Start()
	defer healthChecker.Stop()

	// --- Management API ---
	apiMux := http.NewServeMux()
	registerAPIRoutes(apiMux, cfg, vmMgr, store)

	apiServer := &http.Server{
		Addr:    cfg.ListenAddr,
		Handler: authMiddleware(cfg.APISecret, apiMux),
	}

	// --- Tenant Proxy ---
	proxy := routing.NewProxy(store, wakeMgr)
	proxyServer := &http.Server{
		Addr:    ":8080",
		Handler: proxy,
	}

	// Start servers
	go func() {
		log.Printf("management API listening on %s", cfg.ListenAddr)
		if err := apiServer.ListenAndServe(); err != http.ErrServerClosed {
			log.Fatalf("API server: %v", err)
		}
	}()

	go func() {
		log.Printf("tenant proxy listening on :8080")
		if err := proxyServer.ListenAndServe(); err != http.ErrServerClosed {
			log.Fatalf("proxy server: %v", err)
		}
	}()

	// Graceful shutdown
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	<-sigCh

	log.Println("shutting down...")
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	_ = apiServer.Shutdown(ctx)
	_ = proxyServer.Shutdown(ctx)
	log.Println("solobase-node stopped")
}

// registerAPIRoutes sets up the management HTTP API routes.
func registerAPIRoutes(mux *http.ServeMux, cfg *config.Config, vmMgr *vm.Manager, store *tenant.Store) {
	// POST /api/tenants — create new tenant VM
	mux.HandleFunc("POST /api/tenants", func(w http.ResponseWriter, r *http.Request) {
		var req vm.CreateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			jsonError(w, http.StatusBadRequest, "invalid request: %v", err)
			return
		}
		if req.TenantID == "" {
			jsonError(w, http.StatusBadRequest, "tenant_id is required")
			return
		}

		t, err := vmMgr.CreateVM(r.Context(), &req)
		if err != nil {
			jsonError(w, http.StatusInternalServerError, "create VM: %v", err)
			return
		}
		jsonResponse(w, http.StatusCreated, t)
	})

	// DELETE /api/tenants/{id} — destroy tenant VM
	mux.HandleFunc("DELETE /api/tenants/{id}", func(w http.ResponseWriter, r *http.Request) {
		tenantID := r.PathValue("id")
		if err := vmMgr.DestroyVM(tenantID); err != nil {
			jsonError(w, http.StatusInternalServerError, "destroy VM: %v", err)
			return
		}
		jsonResponse(w, http.StatusOK, map[string]string{"status": "destroyed"})
	})

	// POST /api/tenants/{id}/pause — pause tenant VM
	mux.HandleFunc("POST /api/tenants/{id}/pause", func(w http.ResponseWriter, r *http.Request) {
		tenantID := r.PathValue("id")
		if err := vmMgr.PauseVM(tenantID); err != nil {
			jsonError(w, http.StatusInternalServerError, "pause VM: %v", err)
			return
		}
		jsonResponse(w, http.StatusOK, map[string]string{"status": "paused"})
	})

	// POST /api/tenants/{id}/resume — resume tenant VM
	mux.HandleFunc("POST /api/tenants/{id}/resume", func(w http.ResponseWriter, r *http.Request) {
		tenantID := r.PathValue("id")
		if err := vmMgr.ResumeVM(r.Context(), tenantID); err != nil {
			jsonError(w, http.StatusInternalServerError, "resume VM: %v", err)
			return
		}
		jsonResponse(w, http.StatusOK, map[string]string{"status": "resumed"})
	})

	// GET /api/tenants/{id}/status — tenant status
	mux.HandleFunc("GET /api/tenants/{id}/status", func(w http.ResponseWriter, r *http.Request) {
		tenantID := r.PathValue("id")
		t, err := vmMgr.Status(tenantID)
		if err != nil {
			jsonError(w, http.StatusNotFound, "%v", err)
			return
		}
		jsonResponse(w, http.StatusOK, t)
	})

	// GET /api/tenants — list all tenants
	mux.HandleFunc("GET /api/tenants", func(w http.ResponseWriter, r *http.Request) {
		tenants := store.List()
		jsonResponse(w, http.StatusOK, tenants)
	})

	// GET /api/health — node health + capacity
	mux.HandleFunc("GET /api/health", func(w http.ResponseWriter, r *http.Request) {
		running := store.CountByState(tenant.StateRunning)
		paused := store.CountByState(tenant.StatePaused)
		total := len(store.List())
		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"status":      "ok",
			"max_vms":     cfg.MaxVMs,
			"running":     running,
			"paused":      paused,
			"total":       total,
			"free_slots":  cfg.MaxVMs - running,
		})
	})
}

// authMiddleware checks the API secret in the Authorization header.
func authMiddleware(secret string, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip auth for health endpoint
		if r.URL.Path == "/api/health" {
			next.ServeHTTP(w, r)
			return
		}
		if secret == "" {
			next.ServeHTTP(w, r)
			return
		}
		auth := r.Header.Get("Authorization")
		expected := "Bearer " + secret
		if !strings.EqualFold(auth, expected) {
			jsonError(w, http.StatusUnauthorized, "unauthorized")
			return
		}
		next.ServeHTTP(w, r)
	})
}

func jsonResponse(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func jsonError(w http.ResponseWriter, status int, format string, args ...interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]string{
		"error": fmt.Sprintf(format, args...),
	})
}
