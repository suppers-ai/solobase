package metrics

import (
	"context"
	"runtime"
	"sync"
	"time"

	"github.com/shirou/gopsutil/v3/cpu"
	"github.com/shirou/gopsutil/v3/disk"
	"github.com/shirou/gopsutil/v3/host"
	"github.com/shirou/gopsutil/v3/load"
	"github.com/shirou/gopsutil/v3/mem"
	"github.com/shirou/gopsutil/v3/net"
	"github.com/shirou/gopsutil/v3/process"
)

// SystemMetricsCollector collects system metrics
type SystemMetricsCollector struct {
	mu         sync.RWMutex
	config     *Config
	latest     *SystemMetrics
	history    []*SystemMetrics
	maxHistory int
	ticker     *time.Ticker
	stopCh     chan struct{}
	startTime  time.Time

	// Counters for application metrics
	requestsTotal     int64
	errorCount        int64
	dbQueriesTotal    int64
	activeConnections int32

	// For calculating rates
	lastRequestCount int64
	lastRequestTime  time.Time
	lastNetworkStats *net.IOCountersStat
	lastDiskStats    *disk.IOCountersStat
}

// NewSystemMetricsCollector creates a new system metrics collector
func NewSystemMetricsCollector(config *Config) *SystemMetricsCollector {
	return &SystemMetricsCollector{
		config:     config,
		maxHistory: 120, // Keep 1 hour of 30-second samples
		startTime:  time.Now(),
		history:    make([]*SystemMetrics, 0, 120),
	}
}

// Collect collects current system metrics
func (s *SystemMetricsCollector) Collect(ctx context.Context) (*SystemMetrics, error) {
	metrics := &SystemMetrics{
		Timestamp: time.Now(),
		Uptime:    time.Since(s.startTime),
	}

	// Collect CPU metrics
	if s.config.CollectCPU {
		if err := s.collectCPU(metrics); err != nil {
			// Log error but continue
		}
	}

	// Collect Memory metrics
	if s.config.CollectMemory {
		if err := s.collectMemory(metrics); err != nil {
			// Log error but continue
		}
	}

	// Collect Disk metrics
	if s.config.CollectDisk {
		if err := s.collectDisk(metrics); err != nil {
			// Log error but continue
		}
	}

	// Collect Network metrics
	if s.config.CollectNetwork {
		if err := s.collectNetwork(metrics); err != nil {
			// Log error but continue
		}
	}

	// Collect Process metrics
	if s.config.CollectProcess {
		if err := s.collectProcess(metrics); err != nil {
			// Log error but continue
		}
	}

	// Calculate application metrics
	s.collectApplicationMetrics(metrics)

	// Store in history
	s.mu.Lock()
	s.latest = metrics
	s.history = append(s.history, metrics)
	if len(s.history) > s.maxHistory {
		s.history = s.history[1:]
	}
	s.mu.Unlock()

	return metrics, nil
}

// Start starts periodic collection
func (s *SystemMetricsCollector) Start(ctx context.Context, interval time.Duration) error {
	if s.ticker != nil {
		return nil // Already running
	}

	s.ticker = time.NewTicker(interval)
	s.stopCh = make(chan struct{})

	// Collect initial metrics
	s.Collect(ctx)

	go func() {
		for {
			select {
			case <-s.ticker.C:
				s.Collect(ctx)
			case <-s.stopCh:
				return
			case <-ctx.Done():
				return
			}
		}
	}()

	return nil
}

// Stop stops periodic collection
func (s *SystemMetricsCollector) Stop() error {
	if s.ticker != nil {
		s.ticker.Stop()
		s.ticker = nil
	}
	if s.stopCh != nil {
		close(s.stopCh)
		s.stopCh = nil
	}
	return nil
}

// GetLatest returns the latest metrics
func (s *SystemMetricsCollector) GetLatest() *SystemMetrics {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.latest
}

// GetHistory returns historical metrics
func (s *SystemMetricsCollector) GetHistory(duration time.Duration) []*SystemMetrics {
	s.mu.RLock()
	defer s.mu.RUnlock()

	cutoff := time.Now().Add(-duration)
	var result []*SystemMetrics

	for _, m := range s.history {
		if m.Timestamp.After(cutoff) {
			result = append(result, m)
		}
	}

	return result
}

// Collection methods

func (s *SystemMetricsCollector) collectCPU(m *SystemMetrics) error {
	// CPU usage percent
	percent, err := cpu.Percent(100*time.Millisecond, false)
	if err == nil && len(percent) > 0 {
		m.CPUUsagePercent = percent[0]
	}

	// CPU cores
	cores, err := cpu.Counts(true)
	if err == nil {
		m.CPUCores = cores
	}

	// Load average
	avg, err := load.Avg()
	if err == nil {
		m.LoadAverage1 = avg.Load1
		m.LoadAverage5 = avg.Load5
		m.LoadAverage15 = avg.Load15
	}

	return nil
}

func (s *SystemMetricsCollector) collectMemory(m *SystemMetrics) error {
	// Virtual memory
	vm, err := mem.VirtualMemory()
	if err == nil {
		m.MemoryTotal = vm.Total
		m.MemoryUsed = vm.Used
		m.MemoryFree = vm.Free
		m.MemoryUsedPercent = vm.UsedPercent
	}

	// Swap memory
	swap, err := mem.SwapMemory()
	if err == nil {
		m.SwapTotal = swap.Total
		m.SwapUsed = swap.Used
		m.SwapUsedPercent = swap.UsedPercent
	}

	// Go runtime memory
	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)
	m.HeapAlloc = ms.HeapAlloc
	m.HeapInuse = ms.HeapInuse
	m.StackInuse = ms.StackInuse
	m.GCPauseNs = ms.PauseTotalNs
	m.GCRuns = ms.NumGC

	return nil
}

func (s *SystemMetricsCollector) collectDisk(m *SystemMetrics) error {
	// Disk usage for root partition
	usage, err := disk.Usage("/")
	if err == nil {
		m.DiskTotal = usage.Total
		m.DiskUsed = usage.Used
		m.DiskFree = usage.Free
		m.DiskUsedPercent = usage.UsedPercent
	}

	// Disk I/O stats
	ioCounters, err := disk.IOCounters()
	if err == nil {
		var totalReadBytes, totalWriteBytes, totalReadOps, totalWriteOps uint64
		for _, counter := range ioCounters {
			totalReadBytes += counter.ReadBytes
			totalWriteBytes += counter.WriteBytes
			totalReadOps += counter.ReadCount
			totalWriteOps += counter.WriteCount
		}

		// Calculate delta if we have previous stats
		if s.lastDiskStats != nil {
			m.DiskReadBytes = totalReadBytes - s.lastDiskStats.ReadBytes
			m.DiskWriteBytes = totalWriteBytes - s.lastDiskStats.WriteBytes
			m.DiskReadOps = totalReadOps - s.lastDiskStats.ReadCount
			m.DiskWriteOps = totalWriteOps - s.lastDiskStats.WriteCount
		}

		// Store for next calculation
		s.lastDiskStats = &disk.IOCountersStat{
			ReadBytes:  totalReadBytes,
			WriteBytes: totalWriteBytes,
			ReadCount:  totalReadOps,
			WriteCount: totalWriteOps,
		}
	}

	return nil
}

func (s *SystemMetricsCollector) collectNetwork(m *SystemMetrics) error {
	// Network I/O stats
	ioCounters, err := net.IOCounters(false)
	if err == nil && len(ioCounters) > 0 {
		counter := ioCounters[0]

		// Calculate delta if we have previous stats
		if s.lastNetworkStats != nil {
			m.NetworkBytesReceived = counter.BytesRecv - s.lastNetworkStats.BytesRecv
			m.NetworkBytesSent = counter.BytesSent - s.lastNetworkStats.BytesSent
			m.NetworkPacketsRecv = counter.PacketsRecv - s.lastNetworkStats.PacketsRecv
			m.NetworkPacketsSent = counter.PacketsSent - s.lastNetworkStats.PacketsSent
			m.NetworkErrorsIn = counter.Errin - s.lastNetworkStats.Errin
			m.NetworkErrorsOut = counter.Errout - s.lastNetworkStats.Errout
		}

		// Store for next calculation
		s.lastNetworkStats = &counter
	}

	return nil
}

func (s *SystemMetricsCollector) collectProcess(m *SystemMetrics) error {
	// Process count
	procs, err := process.Processes()
	if err == nil {
		m.ProcessCount = len(procs)
	}

	// Current process info
	pid := int32(runtime.NumGoroutine())
	if proc, err := process.NewProcess(int32(runtime.NumGoroutine())); err == nil {
		// Get thread count
		if threads, err := proc.NumThreads(); err == nil {
			m.ThreadCount = int(threads)
		}

		// Get file descriptors
		if fds, err := proc.NumFDs(); err == nil {
			m.FileDescriptors = int(fds)
		}
	}

	// Goroutines
	m.GoRoutines = runtime.NumGoroutine()

	return nil
}

func (s *SystemMetricsCollector) collectApplicationMetrics(m *SystemMetrics) {
	s.mu.RLock()
	m.RequestsTotal = s.requestsTotal
	m.DBQueries = s.dbQueriesTotal
	m.ActiveConnections = int(s.activeConnections)

	// Calculate requests per second
	now := time.Now()
	if !s.lastRequestTime.IsZero() {
		duration := now.Sub(s.lastRequestTime).Seconds()
		if duration > 0 {
			requestDelta := s.requestsTotal - s.lastRequestCount
			m.RequestsPerSecond = float64(requestDelta) / duration
		}
	}

	// Calculate error rate
	if s.requestsTotal > 0 {
		m.ErrorRate = float64(s.errorCount) / float64(s.requestsTotal) * 100
	}

	s.lastRequestCount = s.requestsTotal
	s.lastRequestTime = now
	s.mu.RUnlock()
}

// IncrementRequests increments request counter
func (s *SystemMetricsCollector) IncrementRequests() {
	s.mu.Lock()
	s.requestsTotal++
	s.mu.Unlock()
}

// IncrementErrors increments error counter
func (s *SystemMetricsCollector) IncrementErrors() {
	s.mu.Lock()
	s.errorCount++
	s.mu.Unlock()
}

// IncrementDBQueries increments database query counter
func (s *SystemMetricsCollector) IncrementDBQueries() {
	s.mu.Lock()
	s.dbQueriesTotal++
	s.mu.Unlock()
}

// SetActiveConnections sets active connection count
func (s *SystemMetricsCollector) SetActiveConnections(count int32) {
	s.mu.Lock()
	s.activeConnections = count
	s.mu.Unlock()
}

// GetUptime returns the uptime duration
func (s *SystemMetricsCollector) GetUptime() time.Duration {
	return time.Since(s.startTime)
}

// GetHistory returns the last n metrics points
func (s *SystemMetricsCollector) GetHistory(n int) []*SystemMetrics {
	s.mu.RLock()
	defer s.mu.RUnlock()

	historyLen := len(s.history)
	if n > historyLen {
		n = historyLen
	}

	if n <= 0 {
		return []*SystemMetrics{}
	}

	// Return last n points
	result := make([]*SystemMetrics, n)
	copy(result, s.history[historyLen-n:])
	return result
}
