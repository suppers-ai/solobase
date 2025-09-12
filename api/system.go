package api

import (
	"bufio"
	"fmt"
	"net/http"
	"os"
	"runtime"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
)

type SystemMetrics struct {
	CPUUsage      float64 `json:"cpu_usage"`
	MemoryUsage   float64 `json:"memory_usage"`
	MemoryTotal   uint64  `json:"memory_total"`
	MemoryUsed    uint64  `json:"memory_used"`
	DiskUsage     float64 `json:"disk_usage"`
	DiskTotal     uint64  `json:"disk_total"`
	DiskUsed      uint64  `json:"disk_used"`
	Uptime        string  `json:"uptime"`
	Goroutines    int     `json:"goroutines"`
	DBQueries     int64   `json:"db_queries"`
	APICallsTotal int64   `json:"api_calls_total"`
}

var (
	startTime = time.Now()

	// Prometheus metrics
	httpRequestsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "http_requests_total",
			Help: "Total number of HTTP requests",
		},
		[]string{"method", "path", "status"},
	)

	httpRequestDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "http_request_duration_seconds",
			Help:    "HTTP request duration in seconds",
			Buckets: prometheus.DefBuckets,
		},
		[]string{"method", "path"},
	)

	dbQueriesTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "database_queries_total",
			Help: "Total number of database queries",
		},
		[]string{"operation", "table"},
	)

	dbQueryDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "database_query_duration_seconds",
			Help:    "Database query duration in seconds",
			Buckets: prometheus.DefBuckets,
		},
		[]string{"operation", "table"},
	)

	activeUsers = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "active_users_total",
			Help: "Number of active users",
		},
	)

	storageUsageBytes = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "storage_usage_bytes",
			Help: "Total storage usage in bytes",
		},
	)
)

// getSystemMemory reads actual system memory from /proc/meminfo on Linux
func getSystemMemory() (total, available uint64) {
	// Default values in case we can't read the file
	total = 8 * 1024 * 1024 * 1024     // 8 GB default
	available = 4 * 1024 * 1024 * 1024 // 4 GB default

	// Try to read from /proc/meminfo on Linux
	file, err := os.Open("/proc/meminfo")
	if err != nil {
		// If not Linux or can't read, return defaults
		return
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}

		switch fields[0] {
		case "MemTotal:":
			if val, err := strconv.ParseUint(fields[1], 10, 64); err == nil {
				total = val * 1024 // Convert from KB to bytes
			}
		case "MemAvailable:":
			if val, err := strconv.ParseUint(fields[1], 10, 64); err == nil {
				available = val * 1024 // Convert from KB to bytes
			}
		}
	}

	return total, available
}

// getCPUUsage gets a rough CPU usage percentage
func getCPUUsage() float64 {
	// Try to read from /proc/stat on Linux
	file, err := os.Open("/proc/stat")
	if err != nil {
		// Fallback to a simple estimation based on goroutines
		numGoroutines := runtime.NumGoroutine()
		usage := float64(numGoroutines) * 2.0
		if usage > 100 {
			usage = 85.0
		}
		return usage
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	if scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "cpu ") {
			// Simple estimation: count non-idle time
			// This is a simplified approach; real CPU usage calculation requires
			// reading the values twice with a time interval
			fields := strings.Fields(line)
			if len(fields) >= 5 {
				var total, idle uint64
				for i := 1; i < len(fields) && i <= 8; i++ {
					if val, err := strconv.ParseUint(fields[i], 10, 64); err == nil {
						total += val
						if i == 4 || i == 5 { // idle and iowait
							idle += val
						}
					}
				}
				if total > 0 {
					return float64(total-idle) / float64(total) * 100
				}
			}
		}
	}

	// Fallback
	return 10.0
}

func HandleGetSystemMetrics() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get actual system memory
		memTotal, memAvailable := getSystemMemory()
		memUsed := memTotal - memAvailable
		memoryUsage := float64(memUsed) / float64(memTotal) * 100

		// Get disk stats
		var stat syscall.Statfs_t
		syscall.Statfs("/", &stat)

		diskTotal := stat.Blocks * uint64(stat.Bsize)
		diskFree := stat.Bavail * uint64(stat.Bsize)
		diskUsed := diskTotal - diskFree
		diskUsage := float64(diskUsed) / float64(diskTotal) * 100

		// Get CPU usage
		cpuUsage := getCPUUsage()

		// Calculate uptime
		uptime := time.Since(startTime)
		uptimeStr := formatUptime(uptime)

		// Get API call count
		metricsCollector.mu.RLock()
		totalAPICalls := int64(0)
		for _, count := range metricsCollector.apiCalls {
			totalAPICalls += count
		}
		dbQueriesCount := int64(0)
		for _, count := range metricsCollector.dbQueries {
			dbQueriesCount += count
		}
		metricsCollector.mu.RUnlock()

		metrics := SystemMetrics{
			CPUUsage:      cpuUsage,
			MemoryUsage:   memoryUsage,
			MemoryTotal:   memTotal,
			MemoryUsed:    memUsed,
			DiskUsage:     diskUsage,
			DiskTotal:     diskTotal,
			DiskUsed:      diskUsed,
			Uptime:        uptimeStr,
			Goroutines:    runtime.NumGoroutine(),
			DBQueries:     dbQueriesCount,
			APICallsTotal: totalAPICalls,
		}

		utils.JSONResponse(w, http.StatusOK, metrics)
	}
}

func formatUptime(d time.Duration) string {
	days := int(d.Hours()) / 24
	hours := int(d.Hours()) % 24
	minutes := int(d.Minutes()) % 60

	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, minutes)
	} else if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}

// HandlePrometheusMetrics returns Prometheus metrics endpoint
func HandlePrometheusMetrics() http.HandlerFunc {
	return promhttp.Handler().ServeHTTP
}

// PrometheusMiddleware tracks HTTP metrics
func PrometheusMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		// Wrap response writer to capture status
		wrapped := &metricsResponseWrapper{
			ResponseWriter: w,
			statusCode:     http.StatusOK,
		}

		// Execute handler
		next.ServeHTTP(wrapped, r)

		// Record metrics
		duration := time.Since(start)
		httpRequestsTotal.WithLabelValues(r.Method, r.URL.Path, fmt.Sprintf("%d", wrapped.statusCode)).Inc()
		httpRequestDuration.WithLabelValues(r.Method, r.URL.Path).Observe(duration.Seconds())
	})
}

type metricsResponseWrapper struct {
	http.ResponseWriter
	statusCode int
	written    bool
}

func (w *metricsResponseWrapper) WriteHeader(statusCode int) {
	if !w.written {
		w.statusCode = statusCode
		w.written = true
	}
	w.ResponseWriter.WriteHeader(statusCode)
}

func (w *metricsResponseWrapper) Write(b []byte) (int, error) {
	if !w.written {
		w.written = true
	}
	return w.ResponseWriter.Write(b)
}

// UpdateMetrics updates various Prometheus metrics
func UpdateMetrics(users int, storage int64) {
	activeUsers.Set(float64(users))
	storageUsageBytes.Set(float64(storage))
}

// RecordDatabaseQuery records database query metrics
func RecordDatabaseQuery(operation, table string, duration time.Duration) {
	dbQueriesTotal.WithLabelValues(operation, table).Inc()
	dbQueryDuration.WithLabelValues(operation, table).Observe(duration.Seconds())
}

// HandleGetDatabaseInfo returns database configuration info
func HandleGetDatabaseInfo(databaseService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get the actual database type from the service
		dbType, dbVersion := databaseService.GetDatabaseInfo()

		info := map[string]interface{}{
			"type":    dbType,
			"version": dbVersion,
		}

		utils.JSONResponse(w, http.StatusOK, info)
	}
}
