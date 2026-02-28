// +build ignore

// Integration test: starts a mock node + solobase-cloud and exercises the full API flow.
// Run with: go run test_integration.go
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/auth"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/billing"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/node"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/tenant"
)

func main() {
	fmt.Println("=== Solobase Cloud Integration Test ===")
	fmt.Println()

	passed := 0
	failed := 0
	check := func(name string, ok bool, detail string) {
		if ok {
			passed++
			fmt.Printf("  PASS  %s\n", name)
		} else {
			failed++
			fmt.Printf("  FAIL  %s: %s\n", name, detail)
		}
	}

	// --- 1. Start mock node API ---
	fmt.Println("[1/6] Starting mock node API...")
	mockNode := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case r.URL.Path == "/api/health":
			json.NewEncoder(w).Encode(map[string]interface{}{
				"status": "ok", "max_vms": 20, "running": 0, "free_slots": 20,
			})
		case r.URL.Path == "/api/tenants" && r.Method == "POST":
			var req map[string]interface{}
			json.NewDecoder(r.Body).Decode(&req)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"id": req["tenant_id"], "subdomain": req["subdomain"],
				"state": "running", "vm_ip": "10.0.0.5",
			})
		case strings.HasPrefix(r.URL.Path, "/api/tenants/") && r.Method == "DELETE":
			w.WriteHeader(http.StatusOK)
		default:
			w.WriteHeader(http.StatusOK)
		}
	}))
	defer mockNode.Close()
	fmt.Printf("  Mock node running at %s\n\n", mockNode.URL)

	// --- 2. Initialize solobase-cloud services ---
	fmt.Println("[2/6] Initializing solobase-cloud services...")
	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "test-node-1", BaseURL: mockNode.URL, Secret: "test-secret",
		Region: "us-east-1", IP: "127.0.0.1",
	})

	tenantSvc := tenant.NewService(scheduler)
	sessions := auth.NewSessionStore(1 * time.Hour)
	users := auth.NewUserStore()

	fmt.Printf("  Registered node: test-node-1 (%s)\n", mockNode.URL)
	fmt.Println()

	// --- 3. Test billing plans ---
	fmt.Println("[3/6] Testing billing plans...")
	plans := billing.DefaultPlans()
	check("5 plans returned", len(plans) == 5, fmt.Sprintf("got %d", len(plans)))

	expectedPlans := []struct {
		id         string
		name       string
		priceCents int
		maxVMs     int
	}{
		{"free", "Free", 0, 1},
		{"hobby", "Hobby", 500, 1},
		{"starter", "Starter", 1500, 1},
		{"professional", "Professional", 7900, 3},
		{"business", "Business", 19900, 10},
	}

	for _, ep := range expectedPlans {
		p := billing.GetPlan(ep.id)
		check(
			fmt.Sprintf("plan '%s' exists with correct price", ep.id),
			p != nil && p.PriceCents == ep.priceCents && p.MaxVMs == ep.maxVMs,
			fmt.Sprintf("plan=%v", p),
		)
	}

	// Verify plan features
	freePlan := billing.GetPlan("free")
	hobbyPlan := billing.GetPlan("hobby")
	check("free plan has fewer features than hobby",
		len(freePlan.Features) < len(hobbyPlan.Features),
		fmt.Sprintf("free=%d, hobby=%d", len(freePlan.Features), len(hobbyPlan.Features)))

	check("free plan auto-sleeps", freePlan.AutoSleep && !freePlan.AlwaysOn, "")
	check("starter plan is always-on", billing.GetPlan("starter").AlwaysOn, "")
	check("professional plan has custom domains", billing.GetPlan("professional").CustomDomain, "")
	check("business plan has custom domains", billing.GetPlan("business").CustomDomain, "")
	check("hobby plan has no custom domain", !hobbyPlan.CustomDomain, "")

	check("nonexistent plan returns nil", billing.GetPlan("enterprise") == nil, "")
	check("old 'pro' plan returns nil", billing.GetPlan("pro") == nil, "")
	fmt.Println()

	// --- 4. Test user creation and sessions ---
	fmt.Println("[4/6] Testing auth flow (simulated OAuth)...")
	userInfo := &auth.UserInfo{
		ProviderID: "gh-12345",
		Provider:   "github",
		Email:      "dev@example.com",
		Name:       "Test Developer",
	}

	user, isNew, err := users.FindOrCreate(userInfo)
	check("user created", err == nil && isNew && user != nil, fmt.Sprintf("err=%v", err))
	check("user email correct", user.Email == "dev@example.com", user.Email)

	// Create again — should find existing
	user2, isNew2, _ := users.FindOrCreate(userInfo)
	check("user found (not recreated)", !isNew2 && user2.ID == user.ID, "")

	session, err := sessions.Create(user.ID)
	check("session created", err == nil && session != nil, fmt.Sprintf("err=%v", err))

	// Validate session
	got, ok := sessions.Get(session.Token)
	check("session valid", ok && got.UserID == user.ID, "")

	// Bad session
	_, ok = sessions.Get("bogus-token")
	check("invalid session rejected", !ok, "")
	fmt.Println()

	// --- 5. Test tenant lifecycle ---
	fmt.Println("[5/6] Testing tenant lifecycle...")

	// Create tenant with free plan
	t1, err := tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: user.ID, Subdomain: "myapp", Plan: "free",
		Features: freePlan.Features,
	})
	check("tenant created (free plan)", err == nil && t1 != nil, fmt.Sprintf("err=%v", err))
	if t1 != nil {
		check("tenant subdomain correct", t1.Subdomain == "myapp", t1.Subdomain)
		check("tenant assigned to node", t1.NodeID != "", "empty node ID")
	}

	// Create tenant with hobby plan
	t2, err := tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: user.ID, Subdomain: "sideproject", Plan: "hobby",
		Features: hobbyPlan.Features,
	})
	check("tenant created (hobby plan)", err == nil && t2 != nil, fmt.Sprintf("err=%v", err))

	// Create tenant with professional plan
	proPlan := billing.GetPlan("professional")
	t3, err := tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: user.ID, Subdomain: "prodapp", Plan: "professional",
		Features: proPlan.Features,
	})
	check("tenant created (professional plan)", err == nil && t3 != nil, fmt.Sprintf("err=%v", err))

	// List tenants by user
	userTenants := tenantSvc.ListByUser(user.ID)
	check("3 tenants listed for user", len(userTenants) == 3, fmt.Sprintf("got %d", len(userTenants)))

	// Another user sees nothing
	otherTenants := tenantSvc.ListByUser("other-user-id")
	check("other user sees 0 tenants", len(otherTenants) == 0, fmt.Sprintf("got %d", len(otherTenants)))

	// Get tenant by ID
	if t1 != nil {
		fetched, ok := tenantSvc.Get(t1.ID)
		check("get tenant by ID", ok && fetched.Subdomain == "myapp", "")
	}

	// Pause & resume
	if t1 != nil {
		err = tenantSvc.Pause(context.Background(), t1.ID)
		check("tenant paused", err == nil, fmt.Sprintf("err=%v", err))

		err = tenantSvc.Resume(context.Background(), t1.ID)
		check("tenant resumed", err == nil, fmt.Sprintf("err=%v", err))
	}

	// Delete tenant
	if t2 != nil {
		err = tenantSvc.Delete(context.Background(), t2.ID)
		check("tenant deleted", err == nil, fmt.Sprintf("err=%v", err))

		_, ok := tenantSvc.Get(t2.ID)
		check("deleted tenant not found", !ok, "")

		remaining := tenantSvc.ListByUser(user.ID)
		check("2 tenants remain after delete", len(remaining) == 2, fmt.Sprintf("got %d", len(remaining)))
	}
	fmt.Println()

	// --- 6. Test HTTP API endpoints ---
	fmt.Println("[6/6] Testing HTTP API endpoints...")

	// Build the same mux as serve.go
	mux := http.NewServeMux()

	type ctxUserID struct{}

	// Simplified handlers for testing
	mux.HandleFunc("/api/plans", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(billing.DefaultPlans())
	})

	mux.HandleFunc("/api/tenants", func(w http.ResponseWriter, r *http.Request) {
		// Check session
		cookie, err := r.Cookie("session")
		if err != nil {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
		sess, ok := sessions.Get(cookie.Value)
		if !ok {
			http.Error(w, "session expired", http.StatusUnauthorized)
			return
		}

		switch r.Method {
		case http.MethodGet:
			tenants := tenantSvc.ListByUser(sess.UserID)
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(tenants)
		case http.MethodPost:
			var req struct {
				Subdomain string `json:"subdomain"`
				Plan      string `json:"plan"`
			}
			json.NewDecoder(r.Body).Decode(&req)
			plan := billing.GetPlan(req.Plan)
			if plan == nil {
				http.Error(w, "invalid plan", http.StatusBadRequest)
				return
			}
			t, err := tenantSvc.Create(r.Context(), &tenant.CreateRequest{
				UserID:    sess.UserID,
				Subdomain: req.Subdomain,
				Plan:      req.Plan,
				Features:  plan.Features,
			})
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusCreated)
			json.NewEncoder(w).Encode(t)
		}
	})

	ts := httptest.NewServer(mux)
	defer ts.Close()

	// GET /api/plans
	resp, _ := http.Get(ts.URL + "/api/plans")
	body, _ := io.ReadAll(resp.Body)
	resp.Body.Close()
	var apiPlans []map[string]interface{}
	json.Unmarshal(body, &apiPlans)
	check("GET /api/plans returns 200", resp.StatusCode == 200, fmt.Sprintf("status=%d", resp.StatusCode))
	check("GET /api/plans returns 5 plans", len(apiPlans) == 5, fmt.Sprintf("got %d", len(apiPlans)))

	// Verify plan IDs in response
	planIDs := make([]string, len(apiPlans))
	for i, p := range apiPlans {
		planIDs[i] = p["id"].(string)
	}
	check("plans ordered: free,hobby,starter,professional,business",
		strings.Join(planIDs, ",") == "free,hobby,starter,professional,business",
		strings.Join(planIDs, ","))

	// GET /api/tenants without auth
	resp, _ = http.Get(ts.URL + "/api/tenants")
	check("GET /api/tenants unauthenticated returns 401",
		resp.StatusCode == http.StatusUnauthorized,
		fmt.Sprintf("status=%d", resp.StatusCode))
	resp.Body.Close()

	// GET /api/tenants with auth
	client := &http.Client{}
	req, _ := http.NewRequest("GET", ts.URL+"/api/tenants", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	resp, _ = client.Do(req)
	body, _ = io.ReadAll(resp.Body)
	resp.Body.Close()
	check("GET /api/tenants authenticated returns 200",
		resp.StatusCode == 200, fmt.Sprintf("status=%d", resp.StatusCode))

	var tenantList []map[string]interface{}
	json.Unmarshal(body, &tenantList)
	check("authenticated user sees their tenants", len(tenantList) == 2,
		fmt.Sprintf("got %d", len(tenantList)))

	// POST /api/tenants — create with hobby plan
	createBody := `{"subdomain":"apitest","plan":"hobby"}`
	req, _ = http.NewRequest("POST", ts.URL+"/api/tenants", bytes.NewBufferString(createBody))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	resp, _ = client.Do(req)
	body, _ = io.ReadAll(resp.Body)
	resp.Body.Close()
	check("POST /api/tenants (hobby) returns 201",
		resp.StatusCode == http.StatusCreated, fmt.Sprintf("status=%d body=%s", resp.StatusCode, string(body)))

	// POST /api/tenants — create with invalid plan
	createBody = `{"subdomain":"badplan","plan":"enterprise"}`
	req, _ = http.NewRequest("POST", ts.URL+"/api/tenants", bytes.NewBufferString(createBody))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	resp, _ = client.Do(req)
	resp.Body.Close()
	check("POST /api/tenants (invalid plan) returns 400",
		resp.StatusCode == http.StatusBadRequest, fmt.Sprintf("status=%d", resp.StatusCode))

	// POST /api/tenants — create with old 'pro' plan (should fail)
	createBody = `{"subdomain":"oldpro","plan":"pro"}`
	req, _ = http.NewRequest("POST", ts.URL+"/api/tenants", bytes.NewBufferString(createBody))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	resp, _ = client.Do(req)
	resp.Body.Close()
	check("POST /api/tenants (old 'pro' plan) returns 400",
		resp.StatusCode == http.StatusBadRequest, fmt.Sprintf("status=%d", resp.StatusCode))

	// Verify final count
	req, _ = http.NewRequest("GET", ts.URL+"/api/tenants", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	resp, _ = client.Do(req)
	body, _ = io.ReadAll(resp.Body)
	resp.Body.Close()
	json.Unmarshal(body, &tenantList)
	check("final tenant count is 3", len(tenantList) == 3,
		fmt.Sprintf("got %d", len(tenantList)))

	fmt.Println()

	// --- Summary ---
	fmt.Println("========================================")
	fmt.Printf("Results: %d passed, %d failed\n", passed, failed)
	fmt.Println("========================================")

	if failed > 0 {
		os.Exit(1)
	}
}
