// Package vm manages Firecracker microVM lifecycle operations.
package vm

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"time"

	"github.com/suppers-ai/solobase/go/solobase-node/internal/config"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/network"
	"github.com/suppers-ai/solobase/go/solobase-node/internal/tenant"
)

// Manager handles Firecracker VM creation, start, stop, and destruction.
type Manager struct {
	cfg    *config.Config
	store  *tenant.Store
	ipPool *network.IPPool
}

// NewManager creates a VM manager.
func NewManager(cfg *config.Config, store *tenant.Store, ipPool *network.IPPool) *Manager {
	return &Manager{
		cfg:    cfg,
		store:  store,
		ipPool: ipPool,
	}
}

// CreateRequest contains parameters for creating a new tenant VM.
type CreateRequest struct {
	TenantID    string            `json:"tenant_id"`
	Subdomain   string            `json:"subdomain"`
	VCPUs       int               `json:"vcpus,omitempty"`
	MemMB       int               `json:"mem_mb,omitempty"`
	Config      map[string]string `json:"config,omitempty"`
	SolobaseCfg *tenant.SolobaseConfig `json:"solobase_config,omitempty"`
}

// CreateVM provisions and starts a new Firecracker microVM for a tenant.
func (m *Manager) CreateVM(ctx context.Context, req *CreateRequest) (*tenant.Tenant, error) {
	// Check capacity
	running := m.store.CountByState(tenant.StateRunning)
	if running >= m.cfg.MaxVMs {
		return nil, fmt.Errorf("node at capacity: %d/%d VMs running", running, m.cfg.MaxVMs)
	}

	// Allocate network
	alloc, err := m.ipPool.Allocate()
	if err != nil {
		return nil, fmt.Errorf("allocate IP: %w", err)
	}

	vcpus := req.VCPUs
	if vcpus <= 0 {
		vcpus = m.cfg.DefaultVCPUs
	}
	memMB := req.MemMB
	if memMB <= 0 {
		memMB = m.cfg.DefaultMemMB
	}

	t := &tenant.Tenant{
		ID:           req.TenantID,
		Subdomain:    req.Subdomain,
		State:        tenant.StateProvisioning,
		VMIP:         alloc.GuestIP.String(),
		HostIP:       alloc.HostIP.String(),
		TapName:      alloc.TapName,
		IPIndex:      alloc.Index,
		VCPUs:        vcpus,
		MemMB:        memMB,
		Config:       req.Config,
		CreatedAt:    time.Now(),
		LastActivity: time.Now(),
	}

	if err := m.store.Put(t); err != nil {
		m.ipPool.Release(alloc.Index)
		return nil, fmt.Errorf("persist tenant: %w", err)
	}

	// Provision overlay filesystem
	solobaseCfg := req.SolobaseCfg
	if solobaseCfg == nil {
		solobaseCfg = &tenant.SolobaseConfig{
			DatabaseType: "sqlite",
			DatabasePath: "/data/solobase.db",
			StorageType:  "local",
			StorageRoot:  "/data/storage",
			BindAddr:     "0.0.0.0:8090",
			Features: map[string]bool{
				"auth": true, "admin": true, "files": true,
				"products": true, "monitoring": true, "legalpages": true,
				"profile": true, "system": true, "userportal": true, "web": true,
			},
		}
	}
	if solobaseCfg.JWTSecret == "" {
		solobaseCfg.JWTSecret = fmt.Sprintf("tenant-%s-%d", req.TenantID, time.Now().UnixNano())
	}

	overlayDir, err := tenant.ProvisionOverlay(m.cfg.DataDir, req.TenantID, solobaseCfg)
	if err != nil {
		m.cleanup(t)
		return nil, fmt.Errorf("provision overlay: %w", err)
	}

	// Create TAP device
	if err := network.CreateTAP(alloc.TapName, alloc.HostIP.String()); err != nil {
		m.cleanup(t)
		return nil, fmt.Errorf("create TAP: %w", err)
	}

	// Setup NAT rules
	if err := network.SetupNAT(alloc.TapName, alloc.GuestIP.String()); err != nil {
		m.cleanup(t)
		return nil, fmt.Errorf("setup NAT: %w", err)
	}

	// Start Firecracker
	socketPath := filepath.Join(overlayDir, "firecracker.sock")
	t.SocketPath = socketPath

	if err := m.startFirecracker(ctx, t, overlayDir); err != nil {
		m.cleanup(t)
		return nil, fmt.Errorf("start firecracker: %w", err)
	}

	// Wait for health check
	if err := m.waitForHealth(ctx, t.VMIP, 30*time.Second); err != nil {
		m.cleanup(t)
		return nil, fmt.Errorf("health check failed: %w", err)
	}

	t.State = tenant.StateRunning
	if err := m.store.Put(t); err != nil {
		return nil, fmt.Errorf("update tenant state: %w", err)
	}

	return t, nil
}

// StopVM gracefully stops a tenant's Firecracker VM.
func (m *Manager) StopVM(tenantID string) error {
	t, ok := m.store.Get(tenantID)
	if !ok {
		return fmt.Errorf("tenant %s not found", tenantID)
	}

	// Send InstanceHalt via Firecracker API socket
	if t.SocketPath != "" {
		_ = m.firecrackerAPI(t.SocketPath, "PUT", "/actions", map[string]string{
			"action_type": "InstanceHalt",
		})
	}

	// Kill process if still running
	if t.PID > 0 {
		if proc, err := os.FindProcess(t.PID); err == nil {
			_ = proc.Signal(syscall.SIGTERM)
			// Give it 5 seconds, then SIGKILL
			time.AfterFunc(5*time.Second, func() {
				_ = proc.Signal(syscall.SIGKILL)
			})
		}
	}

	return m.store.UpdateState(tenantID, tenant.StateStopped, "")
}

// DestroyVM stops the VM and removes all tenant resources.
func (m *Manager) DestroyVM(tenantID string) error {
	t, ok := m.store.Get(tenantID)
	if !ok {
		return fmt.Errorf("tenant %s not found", tenantID)
	}

	// Stop first
	if t.State == tenant.StateRunning || t.State == tenant.StatePaused {
		_ = m.StopVM(tenantID)
	}

	m.cleanup(t)
	return m.store.Delete(tenantID)
}

// Status returns the current status of a tenant VM.
func (m *Manager) Status(tenantID string) (*tenant.Tenant, error) {
	t, ok := m.store.Get(tenantID)
	if !ok {
		return nil, fmt.Errorf("tenant %s not found", tenantID)
	}
	return t, nil
}

func (m *Manager) cleanup(t *tenant.Tenant) {
	// Teardown network
	if t.TapName != "" {
		_ = network.TeardownNAT(t.TapName, t.VMIP)
		_ = network.DestroyTAP(t.TapName)
	}
	// Release IP
	m.ipPool.Release(t.IPIndex)
	// Remove overlay
	_ = tenant.CleanupOverlay(m.cfg.DataDir, t.ID)
	// Update state
	_ = m.store.UpdateState(t.ID, tenant.StateError, "cleanup")
}

func (m *Manager) startFirecracker(ctx context.Context, t *tenant.Tenant, overlayDir string) error {
	// Remove old socket if exists
	os.Remove(t.SocketPath)

	// Build Firecracker config
	fcConfig := map[string]interface{}{
		"boot-source": map[string]interface{}{
			"kernel_image_path": m.cfg.KernelPath,
			"boot_args":        "console=ttyS0 reboot=k panic=1 pci=off init=/sbin/init",
		},
		"drives": []map[string]interface{}{
			{
				"drive_id":       "rootfs",
				"path_on_host":   m.cfg.RootFSPath,
				"is_root_device": true,
				"is_read_only":   false,
			},
		},
		"machine-config": map[string]interface{}{
			"vcpu_count":  t.VCPUs,
			"mem_size_mib": t.MemMB,
		},
		"network-interfaces": []map[string]interface{}{
			{
				"iface_id":    "eth0",
				"guest_mac":   fmt.Sprintf("AA:FC:00:00:%02X:%02X", t.IPIndex/256, t.IPIndex%256),
				"host_dev_name": t.TapName,
			},
		},
	}

	configPath := filepath.Join(overlayDir, "fc-config.json")
	configData, err := json.MarshalIndent(fcConfig, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal FC config: %w", err)
	}
	if err := os.WriteFile(configPath, configData, 0644); err != nil {
		return fmt.Errorf("write FC config: %w", err)
	}

	cmd := exec.CommandContext(ctx, m.cfg.FirecrackerBin,
		"--api-sock", t.SocketPath,
		"--config-file", configPath,
	)

	// Redirect logs
	logPath := filepath.Join(overlayDir, "firecracker.log")
	logFile, err := os.Create(logPath)
	if err != nil {
		return fmt.Errorf("create log file: %w", err)
	}
	cmd.Stdout = logFile
	cmd.Stderr = logFile

	if err := cmd.Start(); err != nil {
		logFile.Close()
		return fmt.Errorf("start firecracker: %w", err)
	}

	t.PID = cmd.Process.Pid

	// Don't wait — let it run in the background.
	// The caller will check health.
	go func() {
		_ = cmd.Wait()
		logFile.Close()
	}()

	return nil
}

func (m *Manager) waitForHealth(ctx context.Context, vmIP string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	healthURL := fmt.Sprintf("http://%s:8090/api/health", vmIP)

	for time.Now().Before(deadline) {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		client := &http.Client{Timeout: 2 * time.Second}
		resp, err := client.Get(healthURL)
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			return nil
		}
		if resp != nil {
			resp.Body.Close()
		}
		time.Sleep(500 * time.Millisecond)
	}
	return fmt.Errorf("VM %s did not become healthy within %v", vmIP, timeout)
}

func (m *Manager) firecrackerAPI(socketPath, method, path string, body interface{}) error {
	data, err := json.Marshal(body)
	if err != nil {
		return err
	}

	client := &http.Client{
		Transport: &http.Transport{
			DialContext: func(ctx context.Context, _, _ string) (net.Conn, error) {
				return net.Dial("unix", socketPath)
			},
		},
		Timeout: 5 * time.Second,
	}

	url := fmt.Sprintf("http://localhost%s", path)
	req, err := http.NewRequest(method, url, nil)
	if err != nil {
		return err
	}
	if body != nil {
		req.Body = io.NopCloser(bytes.NewReader(data))
		req.ContentLength = int64(len(data))
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		return fmt.Errorf("firecracker API %s %s returned %d", method, path, resp.StatusCode)
	}
	return nil
}
