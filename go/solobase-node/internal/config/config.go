// Package config holds node-level configuration for the solobase-node orchestrator.
package config

import (
	"encoding/json"
	"os"
)

// Config contains all settings for a solobase-node instance.
type Config struct {
	// ListenAddr is the address for the management HTTP API (e.g. ":9090").
	ListenAddr string `json:"listen_addr"`

	// KernelPath is the path to the uncompressed Linux kernel image for Firecracker.
	KernelPath string `json:"kernel_path"`

	// RootFSPath is the path to the shared base rootfs ext4 image.
	RootFSPath string `json:"rootfs_path"`

	// DataDir is the base directory for per-tenant overlay filesystems and snapshots.
	DataDir string `json:"data_dir"`

	// SubnetCIDR defines the IP range for TAP device allocation (e.g. "10.0.0.0/16").
	SubnetCIDR string `json:"subnet_cidr"`

	// MaxVMs is the maximum number of concurrent VMs this node can run.
	MaxVMs int `json:"max_vms"`

	// DefaultVCPUs is the vCPU count per VM.
	DefaultVCPUs int `json:"default_vcpus"`

	// DefaultMemMB is the memory allocation per VM in megabytes.
	DefaultMemMB int `json:"default_mem_mb"`

	// IdleTimeoutSec is how long a VM can be idle before being paused (scale-to-zero).
	IdleTimeoutSec int `json:"idle_timeout_sec"`

	// APISecret is a shared secret for authenticating control-plane API calls.
	APISecret string `json:"api_secret"`

	// FirecrackerBin is the path to the firecracker binary.
	FirecrackerBin string `json:"firecracker_bin"`
}

// DefaultConfig returns a Config with sensible defaults.
func DefaultConfig() *Config {
	return &Config{
		ListenAddr:     ":9090",
		KernelPath:     "/opt/solobase/vmlinux",
		RootFSPath:     "/opt/solobase/rootfs.ext4",
		DataDir:        "/var/lib/solobase-node",
		SubnetCIDR:     "10.0.0.0/16",
		MaxVMs:         500,
		DefaultVCPUs:   1,
		DefaultMemMB:   128,
		IdleTimeoutSec: 300,
		FirecrackerBin: "firecracker",
	}
}

// Load reads config from a JSON file, falling back to defaults for missing fields.
func Load(path string) (*Config, error) {
	cfg := DefaultConfig()
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, nil
		}
		return nil, err
	}
	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, err
	}
	// Override with env vars
	if v := os.Getenv("SOLOBASE_NODE_LISTEN"); v != "" {
		cfg.ListenAddr = v
	}
	if v := os.Getenv("SOLOBASE_NODE_SECRET"); v != "" {
		cfg.APISecret = v
	}
	if v := os.Getenv("SOLOBASE_NODE_DATA_DIR"); v != "" {
		cfg.DataDir = v
	}
	return cfg, nil
}
