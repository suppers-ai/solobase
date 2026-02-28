package node

import (
	"context"
	"fmt"
	"log"
	"sync"
)

// NodeInfo describes a registered solobase-node instance.
type NodeInfo struct {
	ID      string `json:"id"`
	BaseURL string `json:"base_url"`
	Secret  string `json:"secret"`
	Region  string `json:"region"`
	IP      string `json:"ip"` // Public IP for DNS records
}

// Scheduler picks the best node for a new tenant based on available capacity.
type Scheduler struct {
	mu    sync.RWMutex
	nodes map[string]*NodeInfo
}

// NewScheduler creates a node scheduler.
func NewScheduler() *Scheduler {
	return &Scheduler{
		nodes: make(map[string]*NodeInfo),
	}
}

// RegisterNode adds a node to the scheduler.
func (s *Scheduler) RegisterNode(node *NodeInfo) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.nodes[node.ID] = node
}

// RemoveNode removes a node from the scheduler.
func (s *Scheduler) RemoveNode(id string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.nodes, id)
}

// GetNode returns a node by ID.
func (s *Scheduler) GetNode(id string) (*NodeInfo, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	n, ok := s.nodes[id]
	return n, ok
}

// Nodes returns all registered nodes.
func (s *Scheduler) Nodes() []*NodeInfo {
	s.mu.RLock()
	defer s.mu.RUnlock()
	result := make([]*NodeInfo, 0, len(s.nodes))
	for _, n := range s.nodes {
		result = append(result, n)
	}
	return result
}

// SelectNode picks the node with the most free capacity.
// Returns the selected node and its health info.
func (s *Scheduler) SelectNode(ctx context.Context) (*NodeInfo, *NodeHealth, error) {
	s.mu.RLock()
	nodes := make([]*NodeInfo, 0, len(s.nodes))
	for _, n := range s.nodes {
		nodes = append(nodes, n)
	}
	s.mu.RUnlock()

	if len(nodes) == 0 {
		return nil, nil, fmt.Errorf("no nodes registered")
	}

	var bestNode *NodeInfo
	var bestHealth *NodeHealth
	bestFree := -1

	for _, node := range nodes {
		client := NewClient(node.BaseURL, node.Secret)
		health, err := client.Health(ctx)
		if err != nil {
			log.Printf("node %s health check failed: %v", node.ID, err)
			continue
		}
		if health.FreeSlots > bestFree {
			bestNode = node
			bestHealth = health
			bestFree = health.FreeSlots
		}
	}

	if bestNode == nil {
		return nil, nil, fmt.Errorf("all nodes unreachable or at capacity")
	}
	if bestFree <= 0 {
		return nil, nil, fmt.Errorf("all nodes at capacity")
	}

	return bestNode, bestHealth, nil
}

// ClientForNode returns an API client for the specified node.
func (s *Scheduler) ClientForNode(nodeID string) (*Client, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	node, ok := s.nodes[nodeID]
	if !ok {
		return nil, fmt.Errorf("node %s not found", nodeID)
	}
	return NewClient(node.BaseURL, node.Secret), nil
}
