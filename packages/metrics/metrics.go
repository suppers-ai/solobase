package metrics

import (
	"context"
	"time"
)

// Collector is the main interface for collecting metrics
type Collector interface {
	// Core metric operations
	Counter(name string, labels ...Label) Counter
	Gauge(name string, labels ...Label) Gauge
	Histogram(name string, buckets []float64, labels ...Label) Histogram
	Summary(name string, objectives map[float64]float64, labels ...Label) Summary

	// Batch operations
	Collect(ctx context.Context) error

	// Management
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
	Reset()

	// Export metrics
	Export() ([]byte, error)
	Handler() interface{} // Returns HTTP handler for metrics endpoint
}

// Counter is a metric that only increases
type Counter interface {
	Inc()
	Add(float64)
	Get() float64
}

// Gauge is a metric that can go up and down
type Gauge interface {
	Set(float64)
	Inc()
	Dec()
	Add(float64)
	Sub(float64)
	Get() float64
	SetToCurrentTime()
}

// Histogram tracks distributions of values
type Histogram interface {
	Observe(float64)
	ObserveDuration(start time.Time)
}

// Summary provides quantiles of observations
type Summary interface {
	Observe(float64)
	ObserveDuration(start time.Time)
}

// Label represents a metric label
type Label struct {
	Name  string
	Value string
}

// SystemMetrics represents system-level metrics
type SystemMetrics struct {
	Timestamp time.Time `json:"timestamp"`

	// CPU metrics
	CPUUsagePercent float64 `json:"cpu_usage_percent"`
	CPUCores        int     `json:"cpu_cores"`
	LoadAverage1    float64 `json:"load_avg_1"`
	LoadAverage5    float64 `json:"load_avg_5"`
	LoadAverage15   float64 `json:"load_avg_15"`

	// Memory metrics
	MemoryTotal       uint64  `json:"memory_total_bytes"`
	MemoryUsed        uint64  `json:"memory_used_bytes"`
	MemoryFree        uint64  `json:"memory_free_bytes"`
	MemoryUsedPercent float64 `json:"memory_used_percent"`
	SwapTotal         uint64  `json:"swap_total_bytes"`
	SwapUsed          uint64  `json:"swap_used_bytes"`
	SwapUsedPercent   float64 `json:"swap_used_percent"`

	// Disk metrics
	DiskTotal       uint64  `json:"disk_total_bytes"`
	DiskUsed        uint64  `json:"disk_used_bytes"`
	DiskFree        uint64  `json:"disk_free_bytes"`
	DiskUsedPercent float64 `json:"disk_used_percent"`
	DiskReadBytes   uint64  `json:"disk_read_bytes"`
	DiskWriteBytes  uint64  `json:"disk_write_bytes"`
	DiskReadOps     uint64  `json:"disk_read_ops"`
	DiskWriteOps    uint64  `json:"disk_write_ops"`

	// Network metrics
	NetworkBytesReceived uint64 `json:"network_bytes_received"`
	NetworkBytesSent     uint64 `json:"network_bytes_sent"`
	NetworkPacketsRecv   uint64 `json:"network_packets_recv"`
	NetworkPacketsSent   uint64 `json:"network_packets_sent"`
	NetworkErrorsIn      uint64 `json:"network_errors_in"`
	NetworkErrorsOut     uint64 `json:"network_errors_out"`

	// Process metrics
	ProcessCount    int    `json:"process_count"`
	ThreadCount     int    `json:"thread_count"`
	FileDescriptors int    `json:"file_descriptors"`
	GoRoutines      int    `json:"goroutines"`
	HeapAlloc       uint64 `json:"heap_alloc_bytes"`
	HeapInuse       uint64 `json:"heap_inuse_bytes"`
	StackInuse      uint64 `json:"stack_inuse_bytes"`
	GCPauseNs       uint64 `json:"gc_pause_ns"`
	GCRuns          uint32 `json:"gc_runs"`

	// Application metrics
	Uptime            time.Duration `json:"uptime"`
	RequestsTotal     int64         `json:"requests_total"`
	RequestsPerSecond float64       `json:"requests_per_second"`
	ResponseTime      float64       `json:"response_time_ms"`
	ErrorRate         float64       `json:"error_rate"`
	ActiveConnections int           `json:"active_connections"`
	DBQueries         int64         `json:"db_queries_total"`
}

// SystemCollector collects system metrics
type SystemCollector interface {
	Collect(ctx context.Context) (*SystemMetrics, error)
	Start(ctx context.Context, interval time.Duration) error
	Stop() error
	GetLatest() *SystemMetrics
	GetHistory(duration time.Duration) []*SystemMetrics
}

// MetricsService is the main service for metrics collection
type MetricsService interface {
	// Collectors
	GetCollector() Collector
	GetSystemCollector() SystemCollector

	// HTTP metrics
	RecordHTTPRequest(method, path string, statusCode int, duration time.Duration)
	RecordHTTPError(method, path string, err error)

	// Database metrics
	RecordDBQuery(query string, duration time.Duration, err error)
	RecordDBConnection(connected bool)

	// Business metrics
	RecordUserAction(action string, userID string, success bool)
	RecordEvent(event string, metadata map[string]string)

	// Get metrics data
	GetSystemMetrics() *SystemMetrics
	GetApplicationMetrics() map[string]interface{}

	// Management
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
}

// Config represents metrics configuration
type Config struct {
	Enabled         bool          `json:"enabled"`
	Type            string        `json:"type"` // "prometheus", "datadog", "newrelic", etc.
	Endpoint        string        `json:"endpoint"`
	CollectInterval time.Duration `json:"collect_interval"`
	RetentionPeriod time.Duration `json:"retention_period"`
	Labels          []Label       `json:"labels"`

	// Prometheus specific
	PrometheusPort int    `json:"prometheus_port"`
	PrometheusPath string `json:"prometheus_path"`

	// System metrics
	CollectCPU     bool `json:"collect_cpu"`
	CollectMemory  bool `json:"collect_memory"`
	CollectDisk    bool `json:"collect_disk"`
	CollectNetwork bool `json:"collect_network"`
	CollectProcess bool `json:"collect_process"`
}

// DefaultConfig returns default metrics configuration
func DefaultConfig() *Config {
	return &Config{
		Enabled:         true,
		Type:            "prometheus",
		CollectInterval: 30 * time.Second,
		RetentionPeriod: 24 * time.Hour,
		PrometheusPort:  9090,
		PrometheusPath:  "/metrics",
		CollectCPU:      true,
		CollectMemory:   true,
		CollectDisk:     true,
		CollectNetwork:  true,
		CollectProcess:  true,
	}
}
