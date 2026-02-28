// Package network manages TAP devices, IP allocation, and iptables rules
// for Firecracker microVM networking.
package network

import (
	"encoding/binary"
	"fmt"
	"net"
	"sync"
)

// IPPool allocates /30 subnets from a larger CIDR block.
// Each VM gets a /30 (4 IPs): network, host gateway, guest, broadcast.
type IPPool struct {
	mu       sync.Mutex
	base     net.IP
	mask     net.IPMask
	nextIdx  uint32
	released []uint32
}

// Allocation represents one /30 allocation for a single VM.
type Allocation struct {
	// Index within the pool (used for release).
	Index uint32
	// HostIP is the gateway IP on the host TAP (e.g. 10.0.0.1).
	HostIP net.IP
	// GuestIP is the IP assigned to the VM guest (e.g. 10.0.0.2).
	GuestIP net.IP
	// Subnet is the /30 in CIDR notation.
	Subnet string
	// TapName is the suggested TAP device name.
	TapName string
}

// NewIPPool creates an IP pool from a CIDR string (e.g. "10.0.0.0/16").
func NewIPPool(cidr string) (*IPPool, error) {
	ip, ipNet, err := net.ParseCIDR(cidr)
	if err != nil {
		return nil, fmt.Errorf("parse CIDR %q: %w", cidr, err)
	}
	base := ip.Mask(ipNet.Mask).To4()
	if base == nil {
		return nil, fmt.Errorf("only IPv4 CIDRs supported")
	}
	return &IPPool{
		base: base,
		mask: ipNet.Mask,
	}, nil
}

// Allocate returns the next available /30 subnet.
func (p *IPPool) Allocate() (*Allocation, error) {
	p.mu.Lock()
	defer p.mu.Unlock()

	var idx uint32
	if len(p.released) > 0 {
		idx = p.released[len(p.released)-1]
		p.released = p.released[:len(p.released)-1]
	} else {
		idx = p.nextIdx
		p.nextIdx++
	}

	// Each /30 block is 4 IPs. Skip block 0 (often the network itself).
	blockStart := (idx + 1) * 4
	baseNum := binary.BigEndian.Uint32(p.base)

	// Verify still in range
	maxIPs := ^binary.BigEndian.Uint32(p.mask)
	if blockStart+3 > maxIPs {
		return nil, fmt.Errorf("IP pool exhausted")
	}

	hostIP := make(net.IP, 4)
	guestIP := make(net.IP, 4)
	binary.BigEndian.PutUint32(hostIP, baseNum+blockStart+1)
	binary.BigEndian.PutUint32(guestIP, baseNum+blockStart+2)

	return &Allocation{
		Index:   idx,
		HostIP:  hostIP,
		GuestIP: guestIP,
		Subnet:  fmt.Sprintf("%s/30", net.IP(append(net.IP{}, hostIP...)).Mask(net.CIDRMask(30, 32))),
		TapName: fmt.Sprintf("sb%d", idx),
	}, nil
}

// Release returns an allocation back to the pool.
func (p *IPPool) Release(idx uint32) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.released = append(p.released, idx)
}
