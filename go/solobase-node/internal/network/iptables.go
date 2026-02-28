package network

import (
	"fmt"
	"os/exec"
)

// SetupNAT configures iptables rules so a VM can reach the internet
// via masquerade, and optionally forwards a host port to the VM.
func SetupNAT(tapName, guestIP string) error {
	cmds := [][]string{
		// Allow forwarding to/from the TAP
		{"iptables", "-A", "FORWARD", "-i", tapName, "-o", "eth0", "-j", "ACCEPT"},
		{"iptables", "-A", "FORWARD", "-i", "eth0", "-o", tapName, "-j", "ACCEPT"},
		// Masquerade outbound traffic from the VM
		{"iptables", "-t", "nat", "-A", "POSTROUTING", "-o", "eth0",
			"-s", guestIP + "/30", "-j", "MASQUERADE"},
	}
	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		if out, err := cmd.CombinedOutput(); err != nil {
			return fmt.Errorf("running %v: %w\n%s", args, err, out)
		}
	}
	return nil
}

// TeardownNAT removes the iptables rules added by SetupNAT.
func TeardownNAT(tapName, guestIP string) error {
	cmds := [][]string{
		{"iptables", "-D", "FORWARD", "-i", tapName, "-o", "eth0", "-j", "ACCEPT"},
		{"iptables", "-D", "FORWARD", "-i", "eth0", "-o", tapName, "-j", "ACCEPT"},
		{"iptables", "-t", "nat", "-D", "POSTROUTING", "-o", "eth0",
			"-s", guestIP + "/30", "-j", "MASQUERADE"},
	}
	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		// Best-effort cleanup; don't fail if rules are already gone.
		_ = cmd.Run()
	}
	return nil
}

// EnableIPForwarding enables IP forwarding on the host.
func EnableIPForwarding() error {
	cmd := exec.Command("sysctl", "-w", "net.ipv4.ip_forward=1")
	if out, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("enable ip forwarding: %w\n%s", err, out)
	}
	return nil
}
