package routing

import (
	"context"
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
)

// VMResumer is the interface for resuming paused VMs (implemented by vm.Manager).
type VMResumer interface {
	ResumeVM(ctx context.Context, tenantID string) error
}

// WakeManager handles wake-on-request for paused VMs.
// It ensures only one wake is in progress per tenant to avoid races.
type WakeManager struct {
	resumer VMResumer
	store   *tenant.Store
	mu      sync.Mutex
	waking  map[string]chan struct{} // tenantID → done channel
}

// NewWakeManager creates a new WakeManager.
func NewWakeManager(resumer VMResumer, store *tenant.Store) *WakeManager {
	return &WakeManager{
		resumer: resumer,
		store:   store,
		waking:  make(map[string]chan struct{}),
	}
}

// Wake resumes a paused VM. If a wake is already in progress, it waits
// for it to complete. Returns nil once the VM is running.
func (w *WakeManager) Wake(tenantID string) error {
	w.mu.Lock()

	// Check if already waking
	if ch, ok := w.waking[tenantID]; ok {
		w.mu.Unlock()
		// Wait for the in-progress wake to finish
		select {
		case <-ch:
			return nil
		case <-time.After(30 * time.Second):
			return fmt.Errorf("timeout waiting for VM %s to wake", tenantID)
		}
	}

	// Start waking
	ch := make(chan struct{})
	w.waking[tenantID] = ch
	w.mu.Unlock()

	log.Printf("waking tenant %s", tenantID)
	start := time.Now()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	err := w.resumer.ResumeVM(ctx, tenantID)

	w.mu.Lock()
	delete(w.waking, tenantID)
	close(ch) // Signal all waiters
	w.mu.Unlock()

	if err != nil {
		return fmt.Errorf("resume VM %s: %w", tenantID, err)
	}

	log.Printf("tenant %s woke in %v", tenantID, time.Since(start))
	return nil
}

// IdleChecker periodically checks for idle running VMs and pauses them.
type IdleChecker struct {
	store     *tenant.Store
	pauser    VMPauser
	threshold time.Duration
	interval  time.Duration
	stop      chan struct{}
}

// VMPauser is the interface for pausing VMs (implemented by vm.Manager).
type VMPauser interface {
	PauseVM(tenantID string) error
}

// NewIdleChecker creates an idle checker that pauses VMs after the threshold.
func NewIdleChecker(store *tenant.Store, pauser VMPauser, threshold, interval time.Duration) *IdleChecker {
	return &IdleChecker{
		store:     store,
		pauser:    pauser,
		threshold: threshold,
		interval:  interval,
		stop:      make(chan struct{}),
	}
}

// Start begins the idle checking loop in a goroutine.
func (ic *IdleChecker) Start() {
	go func() {
		ticker := time.NewTicker(ic.interval)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				ic.check()
			case <-ic.stop:
				return
			}
		}
	}()
}

// Stop halts the idle checker.
func (ic *IdleChecker) Stop() {
	close(ic.stop)
}

func (ic *IdleChecker) check() {
	cutoff := time.Now().Add(-ic.threshold)
	for _, t := range ic.store.List() {
		if t.State == tenant.StateRunning && t.LastActivity.Before(cutoff) {
			log.Printf("pausing idle tenant %s (last activity: %v)", t.ID, t.LastActivity)
			if err := ic.pauser.PauseVM(t.ID); err != nil {
				log.Printf("failed to pause tenant %s: %v", t.ID, err)
			}
		}
	}
}
