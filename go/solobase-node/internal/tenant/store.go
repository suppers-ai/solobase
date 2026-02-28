// Package tenant manages per-tenant state and provisioning.
package tenant

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"
)

// State represents the lifecycle state of a tenant VM.
type State string

const (
	StateProvisioning State = "provisioning"
	StateRunning      State = "running"
	StatePaused       State = "paused"
	StateStopped      State = "stopped"
	StateError        State = "error"
)

// Tenant holds all metadata for a single tenant.
type Tenant struct {
	ID           string            `json:"id"`
	Subdomain    string            `json:"subdomain"`
	State        State             `json:"state"`
	VMIP         string            `json:"vm_ip"`
	HostIP       string            `json:"host_ip"`
	TapName      string            `json:"tap_name"`
	IPIndex      uint32            `json:"ip_index"`
	VCPUs        int               `json:"vcpus"`
	MemMB        int               `json:"mem_mb"`
	SocketPath   string            `json:"socket_path"`
	PID          int               `json:"pid"`
	Config       map[string]string `json:"config"`
	CreatedAt    time.Time         `json:"created_at"`
	LastActivity time.Time         `json:"last_activity"`
	ErrorMsg     string            `json:"error_msg,omitempty"`
}

// Store persists tenant metadata to a JSON file on disk.
// In production this could be backed by SQLite, but JSON is simpler
// for an initial implementation and sufficient for hundreds of tenants.
type Store struct {
	mu      sync.RWMutex
	tenants map[string]*Tenant
	path    string
}

// NewStore creates or loads a tenant store from the given file path.
func NewStore(path string) (*Store, error) {
	s := &Store{
		tenants: make(map[string]*Tenant),
		path:    path,
	}
	// Load existing state if file exists
	data, err := os.ReadFile(path)
	if err == nil {
		var tenants []*Tenant
		if err := json.Unmarshal(data, &tenants); err != nil {
			return nil, fmt.Errorf("corrupt tenant store %s: %w", path, err)
		}
		for _, t := range tenants {
			s.tenants[t.ID] = t
		}
	} else if !os.IsNotExist(err) {
		return nil, fmt.Errorf("read tenant store: %w", err)
	}
	return s, nil
}

// Get returns a tenant by ID.
func (s *Store) Get(id string) (*Tenant, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	t, ok := s.tenants[id]
	if !ok {
		return nil, false
	}
	copy := *t
	return &copy, true
}

// GetBySubdomain returns a tenant by subdomain.
func (s *Store) GetBySubdomain(subdomain string) (*Tenant, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	for _, t := range s.tenants {
		if t.Subdomain == subdomain {
			copy := *t
			return &copy, true
		}
	}
	return nil, false
}

// List returns all tenants.
func (s *Store) List() []*Tenant {
	s.mu.RLock()
	defer s.mu.RUnlock()
	result := make([]*Tenant, 0, len(s.tenants))
	for _, t := range s.tenants {
		copy := *t
		result = append(result, &copy)
	}
	return result
}

// Put creates or updates a tenant.
func (s *Store) Put(t *Tenant) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.tenants[t.ID] = t
	return s.persistLocked()
}

// Delete removes a tenant by ID.
func (s *Store) Delete(id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.tenants, id)
	return s.persistLocked()
}

// UpdateState updates just the state (and optional error) of a tenant.
func (s *Store) UpdateState(id string, state State, errMsg string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	t, ok := s.tenants[id]
	if !ok {
		return fmt.Errorf("tenant %s not found", id)
	}
	t.State = state
	t.ErrorMsg = errMsg
	return s.persistLocked()
}

// TouchActivity updates the last activity timestamp.
func (s *Store) TouchActivity(id string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if t, ok := s.tenants[id]; ok {
		t.LastActivity = time.Now()
		_ = s.persistLocked() // best-effort
	}
}

// CountByState counts tenants in a given state.
func (s *Store) CountByState(state State) int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	count := 0
	for _, t := range s.tenants {
		if t.State == state {
			count++
		}
	}
	return count
}

func (s *Store) persistLocked() error {
	tenants := make([]*Tenant, 0, len(s.tenants))
	for _, t := range s.tenants {
		tenants = append(tenants, t)
	}
	data, err := json.MarshalIndent(tenants, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal tenants: %w", err)
	}
	dir := filepath.Dir(s.path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create store dir: %w", err)
	}
	return os.WriteFile(s.path, data, 0644)
}
