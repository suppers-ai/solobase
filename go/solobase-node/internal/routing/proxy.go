// Package routing implements HTTP reverse proxy and wake-on-request
// for routing tenant subdomain traffic to the correct Firecracker VM.
package routing

import (
	"fmt"
	"log"
	"net/http"
	"net/http/httputil"
	"net/url"
	"strings"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
)

// Proxy is an HTTP reverse proxy that routes requests by subdomain
// to the appropriate tenant VM.
type Proxy struct {
	store  *tenant.Store
	waker  Waker
}

// Waker is an interface for waking paused VMs.
type Waker interface {
	Wake(tenantID string) error
}

// NewProxy creates a new reverse proxy.
func NewProxy(store *tenant.Store, waker Waker) *Proxy {
	return &Proxy{store: store, waker: waker}
}

// ServeHTTP implements http.Handler. It extracts the tenant subdomain
// from the Host header, looks up the VM, wakes it if paused, and proxies.
func (p *Proxy) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	subdomain := extractSubdomain(r.Host)
	if subdomain == "" {
		http.Error(w, "no tenant subdomain in host header", http.StatusBadRequest)
		return
	}

	t, ok := p.store.GetBySubdomain(subdomain)
	if !ok {
		http.Error(w, fmt.Sprintf("tenant %q not found", subdomain), http.StatusNotFound)
		return
	}

	switch t.State {
	case tenant.StateRunning:
		// Direct proxy
	case tenant.StatePaused:
		// Wake the VM, then proxy
		if err := p.waker.Wake(t.ID); err != nil {
			log.Printf("failed to wake tenant %s: %v", t.ID, err)
			http.Error(w, "tenant VM is waking up, please retry", http.StatusServiceUnavailable)
			return
		}
	default:
		http.Error(w, fmt.Sprintf("tenant VM is %s", t.State), http.StatusServiceUnavailable)
		return
	}

	// Update last activity
	p.store.TouchActivity(t.ID)

	// Proxy request to VM
	target, err := url.Parse(fmt.Sprintf("http://%s:8090", t.VMIP))
	if err != nil {
		http.Error(w, "internal routing error", http.StatusInternalServerError)
		return
	}

	proxy := httputil.NewSingleHostReverseProxy(target)
	proxy.ErrorHandler = func(w http.ResponseWriter, r *http.Request, err error) {
		log.Printf("proxy error for tenant %s: %v", t.ID, err)
		http.Error(w, "upstream VM unavailable", http.StatusBadGateway)
	}

	proxy.ServeHTTP(w, r)
}

// extractSubdomain extracts the first subdomain component from a host.
// e.g. "myapp.solobase.app" → "myapp"
// e.g. "myapp.solobase.app:8080" → "myapp"
func extractSubdomain(host string) string {
	// Strip port
	if idx := strings.LastIndex(host, ":"); idx != -1 {
		host = host[:idx]
	}
	parts := strings.Split(host, ".")
	if len(parts) < 3 {
		return "" // no subdomain (e.g. "solobase.app")
	}
	return parts[0]
}
