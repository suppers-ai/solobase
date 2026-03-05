package monitoring

import (
	"encoding/json"
	"sync"
)

// BlockStats holds per-block metrics.
type BlockStats struct {
	Count   int64   `json:"count"`
	AvgMs   float64 `json:"avgMs"`
	Errors  int64   `json:"errors"`
	totalMs int64
}

// FlowStats holds per-flow metrics.
type FlowStats struct {
	Count   int64   `json:"count"`
	AvgMs   float64 `json:"avgMs"`
	Errors  int64   `json:"errors"`
	totalMs int64
}

// Collector is a thread-safe in-memory stats collector for WAFER message processing.
type Collector struct {
	mu            sync.RWMutex
	totalMessages int64
	totalErrors   int64
	perBlock      map[string]*BlockStats
	perFlow       map[string]*FlowStats
	perKind       map[string]int64
}

// NewCollector creates a new Collector.
func NewCollector() *Collector {
	return &Collector{
		perBlock: make(map[string]*BlockStats),
		perFlow:  make(map[string]*FlowStats),
		perKind:  make(map[string]int64),
	}
}

// RecordBlock records a block execution.
func (c *Collector) RecordBlock(name string, durationMs int64, isError bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.totalMessages++
	if isError {
		c.totalErrors++
	}

	bs, ok := c.perBlock[name]
	if !ok {
		bs = &BlockStats{}
		c.perBlock[name] = bs
	}
	bs.Count++
	bs.totalMs += durationMs
	bs.AvgMs = float64(bs.totalMs) / float64(bs.Count)
	if isError {
		bs.Errors++
	}
}

// RecordFlow records a flow execution.
func (c *Collector) RecordFlow(flowID string, durationMs int64, isError bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	fs, ok := c.perFlow[flowID]
	if !ok {
		fs = &FlowStats{}
		c.perFlow[flowID] = fs
	}
	fs.Count++
	fs.totalMs += durationMs
	fs.AvgMs = float64(fs.totalMs) / float64(fs.Count)
	if isError {
		fs.Errors++
	}
}

// RecordKind records a message kind.
func (c *Collector) RecordKind(kind string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.perKind[kind]++
}

// LiveStats returns the current stats without resetting (for API).
type LiveStats struct {
	TotalMessages int64                `json:"totalMessages"`
	TotalErrors   int64                `json:"totalErrors"`
	PerBlock      map[string]*BlockStats `json:"perBlock"`
	PerFlow       map[string]*FlowStats  `json:"perFlow"`
	PerKind       map[string]int64       `json:"perKind"`
}

// ReadStats returns the current stats without resetting.
func (c *Collector) ReadStats() LiveStats {
	c.mu.RLock()
	defer c.mu.RUnlock()

	// Deep copy maps
	perBlock := make(map[string]*BlockStats, len(c.perBlock))
	for k, v := range c.perBlock {
		cp := *v
		perBlock[k] = &cp
	}
	perFlow := make(map[string]*FlowStats, len(c.perFlow))
	for k, v := range c.perFlow {
		cp := *v
		perFlow[k] = &cp
	}
	perKind := make(map[string]int64, len(c.perKind))
	for k, v := range c.perKind {
		perKind[k] = v
	}

	return LiveStats{
		TotalMessages: c.totalMessages,
		TotalErrors:   c.totalErrors,
		PerBlock:      perBlock,
		PerFlow:       perFlow,
		PerKind:       perKind,
	}
}

// Snapshot returns the current stats and resets the collector for the next period.
// Returns JSON-encoded per-block, per-flow, and per-kind data.
func (c *Collector) Snapshot() (totalMessages, totalErrors int64, perBlockJSON, perFlowJSON, perKindJSON string) {
	c.mu.Lock()
	defer c.mu.Unlock()

	totalMessages = c.totalMessages
	totalErrors = c.totalErrors

	if data, err := json.Marshal(c.perBlock); err == nil {
		perBlockJSON = string(data)
	}
	if data, err := json.Marshal(c.perFlow); err == nil {
		perFlowJSON = string(data)
	}
	if data, err := json.Marshal(c.perKind); err == nil {
		perKindJSON = string(data)
	}

	// Reset
	c.totalMessages = 0
	c.totalErrors = 0
	c.perBlock = make(map[string]*BlockStats)
	c.perFlow = make(map[string]*FlowStats)
	c.perKind = make(map[string]int64)

	return
}
