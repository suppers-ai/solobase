// Package main is the entry point for the solobase-cloud control plane.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"os"
	"os/signal"
	"regexp"
	"strings"
	"syscall"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/auth"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/billing"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/dashboard"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/dns"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/node"
	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/tenant"
)

func main() {
	// Configuration from environment
	listenAddr := envDefault("LISTEN_ADDR", ":8080")
	apiSecret := envDefault("API_SECRET", "dev-secret")
	cfToken := os.Getenv("CLOUDFLARE_API_TOKEN")
	cfZoneID := os.Getenv("CLOUDFLARE_ZONE_ID")
	cfDomain := envDefault("DOMAIN", "solobase.app")
	stripeKey := os.Getenv("STRIPE_API_KEY")
	ghClientID := os.Getenv("GITHUB_CLIENT_ID")
	ghClientSecret := os.Getenv("GITHUB_CLIENT_SECRET")
	googleClientID := os.Getenv("GOOGLE_CLIENT_ID")
	googleClientSecret := os.Getenv("GOOGLE_CLIENT_SECRET")
	baseURL := envDefault("BASE_URL", "http://localhost:8080")

	// Services
	scheduler := node.NewScheduler()
	tenantSvc := tenant.NewService(scheduler)
	sessions := auth.NewSessionStore(24 * time.Hour)
	users := auth.NewUserStore()

	var dnsClient *dns.CloudflareClient
	if cfToken != "" && cfZoneID != "" {
		dnsClient = dns.NewCloudflareClient(cfToken, cfZoneID, cfDomain)
	}

	var stripeClient *billing.StripeClient
	if stripeKey != "" {
		stripeClient = billing.NewStripeClient(stripeKey)
	}

	// OAuth providers
	providers := make(map[string]*auth.OAuthProvider)
	if ghClientID != "" {
		providers["github"] = auth.GitHubProvider(ghClientID, ghClientSecret, baseURL+"/auth/callback/github")
	}
	if googleClientID != "" {
		providers["google"] = auth.GoogleProvider(googleClientID, googleClientSecret, baseURL+"/auth/callback/google")
	}

	// Register initial nodes from environment
	// Format: NODE_0=id,base_url,secret,region,ip
	for i := 0; ; i++ {
		val := os.Getenv(fmt.Sprintf("NODE_%d", i))
		if val == "" {
			break
		}
		parts := strings.SplitN(val, ",", 5)
		if len(parts) < 5 {
			log.Printf("WARN: invalid NODE_%d format, expected id,base_url,secret,region,ip", i)
			continue
		}
		scheduler.RegisterNode(&node.NodeInfo{
			ID:      parts[0],
			BaseURL: parts[1],
			Secret:  parts[2],
			Region:  parts[3],
			IP:      parts[4],
		})
		log.Printf("Registered node %s (%s)", parts[0], parts[3])
	}

	// Periodic session cleanup
	go func() {
		for {
			time.Sleep(15 * time.Minute)
			count := sessions.Cleanup()
			if count > 0 {
				log.Printf("Cleaned up %d expired sessions", count)
			}
		}
	}()

	app := &App{
		tenantSvc:   tenantSvc,
		scheduler:   scheduler,
		sessions:    sessions,
		users:       users,
		dnsClient:   dnsClient,
		stripe:      stripeClient,
		providers:   providers,
		apiSecret:   apiSecret,
	}

	mux := http.NewServeMux()

	// API routes (authenticated)
	mux.HandleFunc("/api/tenants", app.authMiddleware(app.handleTenants))
	mux.HandleFunc("/api/tenants/", app.authMiddleware(app.handleTenantAction))
	mux.HandleFunc("/api/me", app.authMiddleware(app.handleMe))
	mux.HandleFunc("/api/plans", app.handlePlans)

	// Node management (API secret)
	mux.HandleFunc("/api/admin/nodes", app.adminMiddleware(app.handleNodes))
	mux.HandleFunc("/api/admin/nodes/", app.adminMiddleware(app.handleNodeAction))

	// Dev session endpoint (only when DEV_MODE is set)
	if os.Getenv("DEV_MODE") != "" {
		mux.HandleFunc("/api/dev/session", app.handleDevSession)
		log.Println("DEV_MODE enabled: /api/dev/session endpoint active")
	}

	// Auth routes
	mux.HandleFunc("/auth/login/", app.handleLogin)
	mux.HandleFunc("/auth/callback/", app.handleCallback)
	mux.HandleFunc("/auth/logout", app.handleLogout)

	// Dashboard (SPA)
	mux.Handle("/", dashboard.Handler())

	server := &http.Server{
		Addr:         listenAddr,
		Handler:      mux,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Graceful shutdown
	go func() {
		sigCh := make(chan os.Signal, 1)
		signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
		<-sigCh
		log.Println("Shutting down...")
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		server.Shutdown(ctx)
	}()

	log.Printf("Control plane listening on %s", listenAddr)
	if err := server.ListenAndServe(); err != http.ErrServerClosed {
		log.Fatalf("Server error: %v", err)
	}
}

// App holds the application dependencies.
type App struct {
	tenantSvc *tenant.Service
	scheduler *node.Scheduler
	sessions  *auth.SessionStore
	users     *auth.UserStore
	dnsClient *dns.CloudflareClient
	stripe    *billing.StripeClient
	providers map[string]*auth.OAuthProvider
	apiSecret string
}

// authMiddleware requires a valid session cookie.
func (a *App) authMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		cookie, err := r.Cookie("session")
		if err != nil {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
		session, ok := a.sessions.Get(cookie.Value)
		if !ok {
			http.Error(w, "session expired", http.StatusUnauthorized)
			return
		}
		// Store user ID in context
		ctx := context.WithValue(r.Context(), ctxUserID{}, session.UserID)
		next(w, r.WithContext(ctx))
	}
}

// adminMiddleware requires the API secret via Bearer token.
func (a *App) adminMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth != "Bearer "+a.apiSecret {
			http.Error(w, "forbidden", http.StatusForbidden)
			return
		}
		next(w, r)
	}
}

type ctxUserID struct{}

func getUserID(r *http.Request) string {
	if v, ok := r.Context().Value(ctxUserID{}).(string); ok {
		return v
	}
	return ""
}

// handleTenants handles GET (list) and POST (create) for /api/tenants.
func (a *App) handleTenants(w http.ResponseWriter, r *http.Request) {
	userID := getUserID(r)

	switch r.Method {
	case http.MethodGet:
		tenants := a.tenantSvc.ListByUser(userID)
		writeJSON(w, http.StatusOK, tenants)

	case http.MethodPost:
		var req struct {
			Subdomain string `json:"subdomain"`
			Plan      string `json:"plan"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}
		if req.Subdomain == "" {
			http.Error(w, "subdomain is required", http.StatusBadRequest)
			return
		}
		if req.Plan == "" {
			req.Plan = "free"
		}
		plan := billing.GetPlan(req.Plan)
		if plan == nil {
			http.Error(w, "invalid plan", http.StatusBadRequest)
			return
		}

		t, err := a.tenantSvc.Create(r.Context(), &tenant.CreateRequest{
			UserID:    userID,
			Subdomain: req.Subdomain,
			Plan:      req.Plan,
			Features:  plan.Features,
		})
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// Create DNS record if configured
		if a.dnsClient != nil {
			if _, err := a.dnsClient.CreateRecord(r.Context(), req.Subdomain, t.NodeIP); err != nil {
				log.Printf("WARN: DNS record creation failed for %s: %v", req.Subdomain, err)
			}
		}

		writeJSON(w, http.StatusCreated, t)

	default:
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleTenantAction handles /api/tenants/{id}[/action].
func (a *App) handleTenantAction(w http.ResponseWriter, r *http.Request) {
	userID := getUserID(r)
	path := strings.TrimPrefix(r.URL.Path, "/api/tenants/")
	parts := strings.SplitN(path, "/", 2)
	tenantID := parts[0]

	// Verify ownership
	t, ok := a.tenantSvc.Get(tenantID)
	if !ok {
		http.Error(w, "tenant not found", http.StatusNotFound)
		return
	}
	if t.UserID != userID {
		http.Error(w, "forbidden", http.StatusForbidden)
		return
	}

	// Determine action
	action := ""
	if len(parts) > 1 {
		action = parts[1]
	}

	switch {
	case action == "" && r.Method == http.MethodGet:
		writeJSON(w, http.StatusOK, t)

	case action == "" && r.Method == http.MethodDelete:
		if err := a.tenantSvc.Delete(r.Context(), tenantID); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		// Clean up DNS
		if a.dnsClient != nil {
			rec, err := a.dnsClient.FindRecord(r.Context(), t.Subdomain)
			if err == nil && rec != nil {
				_ = a.dnsClient.DeleteRecord(r.Context(), rec.ID)
			}
		}
		w.WriteHeader(http.StatusNoContent)

	case action == "pause" && r.Method == http.MethodPost:
		if err := a.tenantSvc.Pause(r.Context(), tenantID); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.WriteHeader(http.StatusOK)

	case action == "resume" && r.Method == http.MethodPost:
		if err := a.tenantSvc.Resume(r.Context(), tenantID); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.WriteHeader(http.StatusOK)

	default:
		http.Error(w, "not found", http.StatusNotFound)
	}
}

// handleMe returns the current user's info.
func (a *App) handleMe(w http.ResponseWriter, r *http.Request) {
	userID := getUserID(r)
	user, ok := a.users.Get(userID)
	if !ok {
		http.Error(w, "user not found", http.StatusNotFound)
		return
	}
	writeJSON(w, http.StatusOK, user)
}

// handlePlans returns the available billing plans.
func (a *App) handlePlans(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, billing.DefaultPlans())
}

// handleNodes handles admin node management.
func (a *App) handleNodes(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		writeJSON(w, http.StatusOK, a.scheduler.Nodes())

	case http.MethodPost:
		var nodeInfo node.NodeInfo
		if err := json.NewDecoder(r.Body).Decode(&nodeInfo); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}
		a.scheduler.RegisterNode(&nodeInfo)
		w.WriteHeader(http.StatusCreated)

	default:
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleNodeAction handles /api/admin/nodes/{id}.
func (a *App) handleNodeAction(w http.ResponseWriter, r *http.Request) {
	nodeID := strings.TrimPrefix(r.URL.Path, "/api/admin/nodes/")

	switch r.Method {
	case http.MethodGet:
		n, ok := a.scheduler.GetNode(nodeID)
		if !ok {
			http.Error(w, "node not found", http.StatusNotFound)
			return
		}
		writeJSON(w, http.StatusOK, n)

	case http.MethodDelete:
		a.scheduler.RemoveNode(nodeID)
		w.WriteHeader(http.StatusNoContent)

	default:
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleLogin initiates OAuth flow.
func (a *App) handleLogin(w http.ResponseWriter, r *http.Request) {
	providerName := strings.TrimPrefix(r.URL.Path, "/auth/login/")
	provider, ok := a.providers[providerName]
	if !ok {
		http.Error(w, "unknown provider", http.StatusBadRequest)
		return
	}

	state, err := auth.GenerateState()
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	// Encode plan into state so we can read it back after OAuth callback
	plan := r.URL.Query().Get("plan")
	stateValue := state
	if plan != "" {
		stateValue = state + ":" + plan
	}

	// Store state in cookie for CSRF verification
	http.SetCookie(w, &http.Cookie{
		Name:     "oauth_state",
		Value:    stateValue,
		Path:     "/",
		MaxAge:   600,
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
	})

	http.Redirect(w, r, provider.AuthorizeURL(state), http.StatusTemporaryRedirect)
}

// handleCallback processes the OAuth callback.
func (a *App) handleCallback(w http.ResponseWriter, r *http.Request) {
	providerName := strings.TrimPrefix(r.URL.Path, "/auth/callback/")
	provider, ok := a.providers[providerName]
	if !ok {
		http.Error(w, "unknown provider", http.StatusBadRequest)
		return
	}

	// Verify CSRF state (cookie may contain "state:plan")
	stateCookie, err := r.Cookie("oauth_state")
	if err != nil {
		http.Error(w, "invalid state parameter", http.StatusBadRequest)
		return
	}
	cookieState := stateCookie.Value
	var selectedPlan string
	if idx := strings.Index(cookieState, ":"); idx != -1 {
		selectedPlan = cookieState[idx+1:]
		cookieState = cookieState[:idx]
	}
	if cookieState != r.URL.Query().Get("state") {
		http.Error(w, "invalid state parameter", http.StatusBadRequest)
		return
	}

	// Exchange code for token
	code := r.URL.Query().Get("code")
	if code == "" {
		http.Error(w, "missing authorization code", http.StatusBadRequest)
		return
	}

	token, err := provider.ExchangeCode(r.Context(), code)
	if err != nil {
		log.Printf("OAuth exchange error: %v", err)
		http.Error(w, "authentication failed", http.StatusInternalServerError)
		return
	}

	// Fetch user info
	userInfo, err := provider.FetchUserInfo(r.Context(), token)
	if err != nil {
		log.Printf("OAuth userinfo error: %v", err)
		http.Error(w, "failed to get user info", http.StatusInternalServerError)
		return
	}

	// Find or create user
	user, isNew, err := a.users.FindOrCreate(userInfo)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	// Create Stripe customer for new users
	if isNew && a.stripe != nil {
		customer, err := a.stripe.CreateCustomer(r.Context(), user.Email, user.ID)
		if err != nil {
			log.Printf("WARN: Stripe customer creation failed: %v", err)
		} else {
			_ = a.users.UpdateStripeID(user.ID, customer.ID)
		}
	}

	// Create session
	session, err := a.sessions.Create(user.ID)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	// Set session cookie
	http.SetCookie(w, &http.Cookie{
		Name:     "session",
		Value:    session.Token,
		Path:     "/",
		MaxAge:   86400,
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
	})

	// Clear oauth state cookie
	http.SetCookie(w, &http.Cookie{
		Name:   "oauth_state",
		Value:  "",
		Path:   "/",
		MaxAge: -1,
	})

	redirectURL := "/"
	if selectedPlan != "" {
		redirectURL = "/?plan=" + url.QueryEscape(selectedPlan)
	}
	http.Redirect(w, r, redirectURL, http.StatusTemporaryRedirect)
}

// handleLogout destroys the session.
func (a *App) handleLogout(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("session")
	if err == nil {
		a.sessions.Delete(cookie.Value)
	}
	http.SetCookie(w, &http.Cookie{
		Name:   "session",
		Value:  "",
		Path:   "/",
		MaxAge: -1,
	})
	http.Redirect(w, r, "/", http.StatusTemporaryRedirect)
}

// validDevUser matches allowed dev user identifiers (lowercase alphanumeric and hyphens).
var validDevUser = regexp.MustCompile(`^[a-z0-9-]+$`)

// handleDevSession creates a dev user and session without OAuth.
// Only registered when DEV_MODE env var is set.
// Accepts an optional ?user= query parameter to create distinct users for testing.
func (a *App) handleDevSession(w http.ResponseWriter, r *http.Request) {
	providerID := "dev-local-1"
	email := "dev@localhost"
	name := "Local Dev"
	if u := r.URL.Query().Get("user"); u != "" {
		if !validDevUser.MatchString(u) {
			http.Error(w, "invalid user parameter", http.StatusBadRequest)
			return
		}
		providerID = u
		email = u + "@localhost"
		name = "Dev " + u
	}

	user, _, err := a.users.FindOrCreate(&auth.UserInfo{
		ProviderID: providerID,
		Provider:   "dev",
		Email:      email,
		Name:       name,
	})
	if err != nil {
		http.Error(w, "failed to create dev user", http.StatusInternalServerError)
		return
	}

	session, err := a.sessions.Create(user.ID)
	if err != nil {
		http.Error(w, "failed to create session", http.StatusInternalServerError)
		return
	}

	http.SetCookie(w, &http.Cookie{
		Name:     "session",
		Value:    session.Token,
		Path:     "/",
		MaxAge:   86400,
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
	})

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"message":   "dev session created",
		"user_id":   user.ID,
		"user_email": user.Email,
		"token":     session.Token,
	})
}

func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

func envDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
