package network

import (
	"fmt"
	"os/exec"
)

// CreateTAP creates a TAP device and configures its IP address.
func CreateTAP(tapName string, hostIP string) error {
	cmds := [][]string{
		{"ip", "tuntap", "add", "dev", tapName, "mode", "tap"},
		{"ip", "addr", "add", hostIP + "/30", "dev", tapName},
		{"ip", "link", "set", tapName, "up"},
	}
	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		if out, err := cmd.CombinedOutput(); err != nil {
			return fmt.Errorf("running %v: %w\n%s", args, err, out)
		}
	}
	return nil
}

// DestroyTAP removes a TAP device.
func DestroyTAP(tapName string) error {
	cmd := exec.Command("ip", "link", "del", tapName)
	if out, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("deleting TAP %s: %w\n%s", tapName, err, out)
	}
	return nil
}
