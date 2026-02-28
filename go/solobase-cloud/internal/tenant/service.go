// Package tenant manages tenant lifecycle in the control plane.
package tenant

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"sync"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-cloud/internal/node"
)

// Tenant represents a customer's solobase instance.
type Tenant struct {
	ID        string    `json:"id"`
	UserID    string    `json:"user_id"`
	Subdomain string    `json:"subdomain"`
	Plan      string    `json:"plan"` // "free", "starter", "pro"
	NodeID    string    `json:"node_id"`
	State     string    `json:"state"` // "provisioning", "running", "paused", "stopped", "error"
	NodeIP    string    `json:"node_ip"`
	CreatedAt time.Time `json:"created_at"`
	Features  []string  `json:"features"`
}

// Service manages tenant CRUD and lifecycle operations.
type Service struct {
	mu        sync.RWMutex
	tenants   map[string]*Tenant
	scheduler *node.Scheduler
}

// NewService creates a tenant service.
func NewService(scheduler *node.Scheduler) *Service {
	return &Service{
		tenants:   make(map[string]*Tenant),
		scheduler: scheduler,
	}
}

// CreateRequest describes a new tenant to provision.
type CreateRequest struct {
	UserID    string   `json:"user_id"`
	Subdomain string   `json:"subdomain"`
	Plan      string   `json:"plan"`
	Features  []string `json:"features,omitempty"`
}

// Create provisions a new tenant: selects a node, creates the VM, sets up DNS.
func (s *Service) Create(ctx context.Context, req *CreateRequest) (*Tenant, error) {
	// Check subdomain uniqueness
	s.mu.RLock()
	for _, t := range s.tenants {
		if t.Subdomain == req.Subdomain {
			s.mu.RUnlock()
			return nil, fmt.Errorf("subdomain %q already taken", req.Subdomain)
		}
	}
	s.mu.RUnlock()

	// Generate tenant ID
	id, err := generateID()
	if err != nil {
		return nil, fmt.Errorf("generate ID: %w", err)
	}

	// Select best node
	selectedNode, _, err := s.scheduler.SelectNode(ctx)
	if err != nil {
		return nil, fmt.Errorf("select node: %w", err)
	}

	tenant := &Tenant{
		ID:        id,
		UserID:    req.UserID,
		Subdomain: req.Subdomain,
		Plan:      req.Plan,
		NodeID:    selectedNode.ID,
		State:     "provisioning",
		NodeIP:    selectedNode.IP,
		CreatedAt: time.Now(),
		Features:  req.Features,
	}

	s.mu.Lock()
	s.tenants[id] = tenant
	s.mu.Unlock()

	// Provision VM on selected node
	client := node.NewClient(selectedNode.BaseURL, selectedNode.Secret)
	_, err = client.CreateTenant(ctx, &node.CreateTenantRequest{
		TenantID:  id,
		Subdomain: req.Subdomain,
	})
	if err != nil {
		tenant.State = "error"
		return tenant, fmt.Errorf("provision VM: %w", err)
	}

	tenant.State = "running"
	return tenant, nil
}

// Get returns a tenant by ID.
func (s *Service) Get(id string) (*Tenant, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	t, ok := s.tenants[id]
	if !ok {
		return nil, false
	}
	copy := *t
	return &copy, true
}

// ListByUser returns all tenants for a user.
func (s *Service) ListByUser(userID string) []*Tenant {
	s.mu.RLock()
	defer s.mu.RUnlock()
	var result []*Tenant
	for _, t := range s.tenants {
		if t.UserID == userID {
			copy := *t
			result = append(result, &copy)
		}
	}
	return result
}

// Delete destroys a tenant's VM and removes it.
func (s *Service) Delete(ctx context.Context, id string) error {
	s.mu.RLock()
	t, ok := s.tenants[id]
	if !ok {
		s.mu.RUnlock()
		return fmt.Errorf("tenant %s not found", id)
	}
	s.mu.RUnlock()

	// Destroy VM on node
	client, err := s.scheduler.ClientForNode(t.NodeID)
	if err != nil {
		return fmt.Errorf("get node client: %w", err)
	}
	if err := client.DestroyTenant(ctx, id); err != nil {
		return fmt.Errorf("destroy VM: %w", err)
	}

	s.mu.Lock()
	delete(s.tenants, id)
	s.mu.Unlock()

	return nil
}

// Pause pauses a tenant's VM.
func (s *Service) Pause(ctx context.Context, id string) error {
	t, ok := s.Get(id)
	if !ok {
		return fmt.Errorf("tenant %s not found", id)
	}

	client, err := s.scheduler.ClientForNode(t.NodeID)
	if err != nil {
		return err
	}
	if err := client.PauseTenant(ctx, id); err != nil {
		return err
	}

	s.mu.Lock()
	if tenant, ok := s.tenants[id]; ok {
		tenant.State = "paused"
	}
	s.mu.Unlock()
	return nil
}

// Resume resumes a paused tenant's VM.
func (s *Service) Resume(ctx context.Context, id string) error {
	t, ok := s.Get(id)
	if !ok {
		return fmt.Errorf("tenant %s not found", id)
	}

	client, err := s.scheduler.ClientForNode(t.NodeID)
	if err != nil {
		return err
	}
	if err := client.ResumeTenant(ctx, id); err != nil {
		return err
	}

	s.mu.Lock()
	if tenant, ok := s.tenants[id]; ok {
		tenant.State = "running"
	}
	s.mu.Unlock()
	return nil
}

func generateID() (string, error) {
	b := make([]byte, 12)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}
