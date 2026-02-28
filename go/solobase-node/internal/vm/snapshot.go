package vm

import (
	"context"
	"fmt"
	"path/filepath"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
)

// PauseVM pauses a running VM and creates a snapshot for scale-to-zero.
func (m *Manager) PauseVM(tenantID string) error {
	t, ok := m.store.Get(tenantID)
	if !ok {
		return fmt.Errorf("tenant %s not found", tenantID)
	}
	if t.State != tenant.StateRunning {
		return fmt.Errorf("tenant %s is not running (state=%s)", tenantID, t.State)
	}

	overlayDir := filepath.Join(m.cfg.DataDir, "tenants", tenantID)
	snapshotDir := filepath.Join(overlayDir, "snapshot")

	// 1. Pause the VM
	err := m.firecrackerAPI(t.SocketPath, "PATCH", "/vm", map[string]string{
		"state": "Paused",
	})
	if err != nil {
		return fmt.Errorf("pause VM: %w", err)
	}

	// 2. Create snapshot
	err = m.firecrackerAPI(t.SocketPath, "PUT", "/snapshot/create", map[string]string{
		"snapshot_type": "Full",
		"snapshot_path": filepath.Join(snapshotDir, "snapshot.bin"),
		"mem_file_path": filepath.Join(snapshotDir, "mem.bin"),
	})
	if err != nil {
		// Try to resume on failure
		_ = m.firecrackerAPI(t.SocketPath, "PATCH", "/vm", map[string]string{
			"state": "Resumed",
		})
		return fmt.Errorf("create snapshot: %w", err)
	}

	// 3. Stop the Firecracker process (frees RAM)
	_ = m.firecrackerAPI(t.SocketPath, "PUT", "/actions", map[string]string{
		"action_type": "InstanceHalt",
	})

	return m.store.UpdateState(tenantID, tenant.StatePaused, "")
}

// ResumeVM restores a paused VM from its snapshot.
func (m *Manager) ResumeVM(ctx context.Context, tenantID string) error {
	t, ok := m.store.Get(tenantID)
	if !ok {
		return fmt.Errorf("tenant %s not found", tenantID)
	}
	if t.State != tenant.StatePaused {
		return fmt.Errorf("tenant %s is not paused (state=%s)", tenantID, t.State)
	}

	overlayDir := filepath.Join(m.cfg.DataDir, "tenants", tenantID)

	// Ensure TAP device exists (may have been cleaned up)
	// Re-create networking if needed
	// (in practice, TAP devices persist across pause/resume on the same host)

	// Start a new Firecracker process that loads the snapshot
	if err := m.startFirecrackerFromSnapshot(ctx, t, overlayDir); err != nil {
		return fmt.Errorf("start from snapshot: %w", err)
	}

	// Wait for the VM to be responsive
	if err := m.waitForHealth(ctx, t.VMIP, 10*time.Second); err != nil {
		return fmt.Errorf("resumed VM health check: %w", err)
	}

	return m.store.UpdateState(tenantID, tenant.StateRunning, "")
}

func (m *Manager) startFirecrackerFromSnapshot(ctx context.Context, t *tenant.Tenant, overlayDir string) error {
	snapshotDir := filepath.Join(overlayDir, "snapshot")

	// Firecracker snapshot restore uses the --no-api flag with snapshot config.
	// We PUT /snapshot/load on a fresh Firecracker instance.

	// Start fresh Firecracker with just the API socket
	if err := m.startFirecracker(ctx, t, overlayDir); err != nil {
		return fmt.Errorf("start fresh fc: %w", err)
	}

	// Brief wait for socket to be ready
	time.Sleep(200 * time.Millisecond)

	// Load snapshot
	err := m.firecrackerAPI(t.SocketPath, "PUT", "/snapshot/load", map[string]interface{}{
		"snapshot_path":     filepath.Join(snapshotDir, "snapshot.bin"),
		"mem_backend": map[string]string{
			"backend_type":  "File",
			"backend_path":  filepath.Join(snapshotDir, "mem.bin"),
		},
		"enable_diff_snapshots": false,
		"resume_vm":             true,
	})
	if err != nil {
		return fmt.Errorf("load snapshot: %w", err)
	}

	return nil
}
