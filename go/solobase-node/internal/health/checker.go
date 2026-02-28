// Package health provides periodic VM liveness checks and auto-restart.
package health

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
)

// VMRestarter can restart a failed VM.
type VMRestarter interface {
	StopVM(tenantID string) error
	CreateVM(ctx context.Context, req interface{}) error
}

// Checker periodically verifies that running VMs are responsive.
type Checker struct {
	store    *tenant.Store
	interval time.Duration
	stop     chan struct{}
}

// NewChecker creates a health checker.
func NewChecker(store *tenant.Store, interval time.Duration) *Checker {
	return &Checker{
		store:    store,
		interval: interval,
		stop:     make(chan struct{}),
	}
}

// Start begins the health check loop.
func (c *Checker) Start() {
	go func() {
		ticker := time.NewTicker(c.interval)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				c.checkAll()
			case <-c.stop:
				return
			}
		}
	}()
}

// Stop halts the checker.
func (c *Checker) Stop() {
	close(c.stop)
}

func (c *Checker) checkAll() {
	for _, t := range c.store.List() {
		if t.State != tenant.StateRunning {
			continue
		}
		if err := ping(t.VMIP); err != nil {
			log.Printf("health check failed for tenant %s (%s): %v", t.ID, t.VMIP, err)
			_ = c.store.UpdateState(t.ID, tenant.StateError,
				fmt.Sprintf("health check failed: %v", err))
		}
	}
}

func ping(vmIP string) error {
	client := &http.Client{Timeout: 5 * time.Second}
	resp, err := client.Get(fmt.Sprintf("http://%s:8090/api/health", vmIP))
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return fmt.Errorf("unexpected status: %d", resp.StatusCode)
	}
	return nil
}
