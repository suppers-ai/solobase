package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/auth"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/node"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/tenant"
)

func mockNodeAPI(t *testing.T) *httptest.Server {
	t.Helper()
	mux := http.NewServeMux()
	mux.HandleFunc("/api/health", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]interface{}{
			"status": "ok", "max_vms": 10, "running": 1, "free_slots": 9,
		})
	})
	mux.HandleFunc("/api/tenants", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == "POST" {
			var req map[string]interface{}
			json.NewDecoder(r.Body).Decode(&req)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"id": req["tenant_id"], "subdomain": req["subdomain"],
				"state": "running", "vm_ip": "10.0.0.5",
			})
			return
		}
	})
	mux.HandleFunc("/api/tenants/", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})
	return httptest.NewServer(mux)
}

func setupApp(t *testing.T) (*App, func()) {
	t.Helper()
	nodeServer := mockNodeAPI(t)

	scheduler := node.NewScheduler()
	scheduler.RegisterNode(&node.NodeInfo{
		ID: "test-node", BaseURL: nodeServer.URL, Secret: "secret",
		Region: "us-east", IP: "1.2.3.4",
	})

	app := &App{
		tenantSvc: tenant.NewService(scheduler),
		scheduler: scheduler,
		sessions:  auth.NewSessionStore(1 * time.Hour),
		users:     auth.NewUserStore(),
		providers: make(map[string]*auth.OAuthProvider),
		apiSecret: "test-secret",
	}

	return app, func() { nodeServer.Close() }
}

func createTestSession(t *testing.T, app *App) (*auth.User, *auth.Session) {
	t.Helper()
	user, _, _ := app.users.FindOrCreate(&auth.UserInfo{
		ProviderID: "test-id", Provider: "github",
		Email: "test@example.com", Name: "Test",
	})
	session, err := app.sessions.Create(user.ID)
	if err != nil {
		t.Fatalf("create session: %v", err)
	}
	return user, session
}

func TestHandlePlans(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	req := httptest.NewRequest("GET", "/api/plans", nil)
	w := httptest.NewRecorder()

	app.handlePlans(w, req)

	if w.Code != 200 {
		t.Fatalf("status = %d, want 200", w.Code)
	}

	var plans []map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &plans)
	if len(plans) != 5 {
		t.Errorf("got %d plans, want 5", len(plans))
	}
}

func TestHandleTenants_Unauthenticated(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	req := httptest.NewRequest("GET", "/api/tenants", nil)
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("status = %d, want 401", w.Code)
	}
}

func TestHandleTenants_ListEmpty(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	req := httptest.NewRequest("GET", "/api/tenants", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != 200 {
		t.Fatalf("status = %d, want 200", w.Code)
	}
}

func TestHandleTenants_CreateAndList(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	// Create
	body := `{"subdomain":"testapp","plan":"free"}`
	req := httptest.NewRequest("POST", "/api/tenants", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != http.StatusCreated {
		t.Fatalf("create status = %d, want 201, body: %s", w.Code, w.Body.String())
	}

	var created map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &created)
	if created["subdomain"] != "testapp" {
		t.Errorf("subdomain = %v, want testapp", created["subdomain"])
	}

	// List
	req = httptest.NewRequest("GET", "/api/tenants", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w = httptest.NewRecorder()
	handler(w, req)

	if w.Code != 200 {
		t.Fatalf("list status = %d, want 200", w.Code)
	}

	var tenants []map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &tenants)
	if len(tenants) != 1 {
		t.Errorf("got %d tenants, want 1", len(tenants))
	}
}

func TestHandleTenants_InvalidPlan(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	body := `{"subdomain":"testapp","plan":"enterprise"}`
	req := httptest.NewRequest("POST", "/api/tenants", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("status = %d, want 400", w.Code)
	}
}

func TestHandleTenants_MissingSubdomain(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	body := `{"plan":"free"}`
	req := httptest.NewRequest("POST", "/api/tenants", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("status = %d, want 400", w.Code)
	}
}

func TestHandleTenantAction_GetByID(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	user, session := createTestSession(t, app)

	// Create a tenant first
	created, err := app.tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: user.ID, Subdomain: "getme", Plan: "free",
	})
	if err != nil {
		t.Fatalf("create tenant: %v", err)
	}

	req := httptest.NewRequest("GET", "/api/tenants/"+created.ID, nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenantAction)
	handler(w, req)

	if w.Code != 200 {
		t.Fatalf("status = %d, want 200, body: %s", w.Code, w.Body.String())
	}

	var got map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &got)
	if got["subdomain"] != "getme" {
		t.Errorf("subdomain = %v, want getme", got["subdomain"])
	}
}

func TestHandleTenantAction_NotFound(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	req := httptest.NewRequest("GET", "/api/tenants/nonexistent", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenantAction)
	handler(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("status = %d, want 404", w.Code)
	}
}

func TestHandleTenantAction_ForbiddenOtherUser(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	// Create tenant as user-1
	created, err := app.tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: "other-user", Subdomain: "forbidden", Plan: "free",
	})
	if err != nil {
		t.Fatalf("create: %v", err)
	}

	// Try to access as a different user
	_, session := createTestSession(t, app)

	req := httptest.NewRequest("GET", "/api/tenants/"+created.ID, nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenantAction)
	handler(w, req)

	if w.Code != http.StatusForbidden {
		t.Errorf("status = %d, want 403", w.Code)
	}
}

func TestHandleTenantAction_Delete(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	user, session := createTestSession(t, app)

	created, _ := app.tenantSvc.Create(context.Background(), &tenant.CreateRequest{
		UserID: user.ID, Subdomain: "deleteme", Plan: "free",
	})

	req := httptest.NewRequest("DELETE", "/api/tenants/"+created.ID, nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenantAction)
	handler(w, req)

	if w.Code != http.StatusNoContent {
		t.Errorf("status = %d, want 204", w.Code)
	}

	_, ok := app.tenantSvc.Get(created.ID)
	if ok {
		t.Error("expected tenant to be deleted")
	}
}

func TestHandleMe(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	req := httptest.NewRequest("GET", "/api/me", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleMe)
	handler(w, req)

	if w.Code != 200 {
		t.Fatalf("status = %d, want 200", w.Code)
	}

	var user map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &user)
	if user["email"] != "test@example.com" {
		t.Errorf("email = %v, want test@example.com", user["email"])
	}
}

func TestAdminMiddleware_ValidSecret(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	req := httptest.NewRequest("GET", "/api/admin/nodes", nil)
	req.Header.Set("Authorization", "Bearer test-secret")
	w := httptest.NewRecorder()

	handler := app.adminMiddleware(app.handleNodes)
	handler(w, req)

	if w.Code != 200 {
		t.Fatalf("status = %d, want 200", w.Code)
	}
}

func TestAdminMiddleware_InvalidSecret(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	req := httptest.NewRequest("GET", "/api/admin/nodes", nil)
	req.Header.Set("Authorization", "Bearer wrong")
	w := httptest.NewRecorder()

	handler := app.adminMiddleware(app.handleNodes)
	handler(w, req)

	if w.Code != http.StatusForbidden {
		t.Errorf("status = %d, want 403", w.Code)
	}
}

func TestHandleNodes_CRUD(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	// List
	req := httptest.NewRequest("GET", "/api/admin/nodes", nil)
	req.Header.Set("Authorization", "Bearer test-secret")
	w := httptest.NewRecorder()
	app.adminMiddleware(app.handleNodes)(w, req)

	if w.Code != 200 {
		t.Fatalf("list status = %d, want 200", w.Code)
	}

	// Register a new node
	body := `{"id":"n2","base_url":"http://n2:9090","secret":"s2","region":"eu-west","ip":"5.5.5.5"}`
	req = httptest.NewRequest("POST", "/api/admin/nodes", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer test-secret")
	req.Header.Set("Content-Type", "application/json")
	w = httptest.NewRecorder()
	app.adminMiddleware(app.handleNodes)(w, req)

	if w.Code != http.StatusCreated {
		t.Fatalf("create status = %d, want 201", w.Code)
	}

	// Verify it's registered
	n, ok := app.scheduler.GetNode("n2")
	if !ok {
		t.Fatal("node n2 not found")
	}
	if n.Region != "eu-west" {
		t.Errorf("Region = %q, want %q", n.Region, "eu-west")
	}
}

func TestHandleLogout(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	req := httptest.NewRequest("POST", "/auth/logout", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	app.handleLogout(w, req)

	// Session should be invalidated
	_, ok := app.sessions.Get(session.Token)
	if ok {
		t.Error("expected session to be deleted after logout")
	}
}

func TestHandleLogin_PlanParam(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	// Register a mock GitHub provider so login proceeds past provider lookup
	app.providers["github"] = auth.GitHubProvider("fake-id", "fake-secret", "http://localhost/callback")

	req := httptest.NewRequest("GET", "/auth/login/github?plan=hobby", nil)
	w := httptest.NewRecorder()

	app.handleLogin(w, req)

	// Should redirect to the provider's auth URL
	if w.Code != http.StatusTemporaryRedirect {
		t.Fatalf("status = %d, want 307", w.Code)
	}

	// Verify the oauth_state cookie contains the plan suffix
	var stateCookie *http.Cookie
	for _, c := range w.Result().Cookies() {
		if c.Name == "oauth_state" {
			stateCookie = c
			break
		}
	}
	if stateCookie == nil {
		t.Fatal("oauth_state cookie not set")
	}
	if !strings.Contains(stateCookie.Value, ":hobby") {
		t.Errorf("oauth_state cookie = %q, want to contain ':hobby'", stateCookie.Value)
	}
}

func TestHandleLogin_NoPlanParam(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	app.providers["github"] = auth.GitHubProvider("fake-id", "fake-secret", "http://localhost/callback")

	req := httptest.NewRequest("GET", "/auth/login/github", nil)
	w := httptest.NewRecorder()

	app.handleLogin(w, req)

	if w.Code != http.StatusTemporaryRedirect {
		t.Fatalf("status = %d, want 307", w.Code)
	}

	// Without a plan param, the cookie should just be the raw state (no colon)
	for _, c := range w.Result().Cookies() {
		if c.Name == "oauth_state" {
			if strings.Contains(c.Value, ":") {
				t.Errorf("oauth_state cookie = %q, should not contain ':'", c.Value)
			}
			return
		}
	}
	t.Fatal("oauth_state cookie not set")
}

func TestHandleTenants_CreateWithHobbyPlan(t *testing.T) {
	app, cleanup := setupApp(t)
	defer cleanup()

	_, session := createTestSession(t, app)

	body := `{"subdomain":"hobbyapp","plan":"hobby"}`
	req := httptest.NewRequest("POST", "/api/tenants", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleTenants)
	handler(w, req)

	if w.Code != http.StatusCreated {
		t.Fatalf("create status = %d, want 201, body: %s", w.Code, w.Body.String())
	}

	var created map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &created)
	if created["subdomain"] != "hobbyapp" {
		t.Errorf("subdomain = %v, want hobbyapp", created["subdomain"])
	}
}

func TestExpiredSession(t *testing.T) {
	scheduler := node.NewScheduler()
	app := &App{
		tenantSvc: tenant.NewService(scheduler),
		scheduler: scheduler,
		sessions:  auth.NewSessionStore(1 * time.Millisecond),
		users:     auth.NewUserStore(),
		providers: make(map[string]*auth.OAuthProvider),
		apiSecret: "test-secret",
	}

	user, _, _ := app.users.FindOrCreate(&auth.UserInfo{
		ProviderID: "1", Provider: "github", Email: "x@x.com", Name: "X",
	})
	session, _ := app.sessions.Create(user.ID)

	time.Sleep(5 * time.Millisecond)

	req := httptest.NewRequest("GET", "/api/me", nil)
	req.AddCookie(&http.Cookie{Name: "session", Value: session.Token})
	w := httptest.NewRecorder()

	handler := app.authMiddleware(app.handleMe)
	handler(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("status = %d, want 401 (expired session)", w.Code)
	}
}
