package core

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"sort"
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// MetricsCollector collects metrics for extensions
type MetricsCollector struct {
	mu               sync.RWMutex
	extensionMetrics map[string]*ExtensionMetrics

	// Prometheus metrics
	requestsTotal    *prometheus.CounterVec
	requestDuration  *prometheus.HistogramVec
	errorsTotal      *prometheus.CounterVec
	activeExtensions prometheus.Gauge
	healthStatus     *prometheus.GaugeVec
	resourceUsage    *prometheus.GaugeVec
	hooksExecuted    *prometheus.CounterVec
	hookDuration     *prometheus.HistogramVec
}

// NewMetricsCollector creates a new metrics collector
func NewMetricsCollector() *MetricsCollector {
	return &MetricsCollector{
		extensionMetrics: make(map[string]*ExtensionMetrics),

		// Initialize Prometheus metrics
		requestsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Name: "extension_requests_total",
				Help: "Total number of requests handled by extensions",
			},
			[]string{"extension", "method", "path"},
		),

		requestDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Name:    "extension_request_duration_seconds",
				Help:    "Duration of extension requests in seconds",
				Buckets: prometheus.DefBuckets,
			},
			[]string{"extension", "method", "path"},
		),

		errorsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Name: "extension_errors_total",
				Help: "Total number of errors in extensions",
			},
			[]string{"extension", "error_type"},
		),

		activeExtensions: promauto.NewGauge(
			prometheus.GaugeOpts{
				Name: "extensions_active_total",
				Help: "Number of active extensions",
			},
		),

		healthStatus: promauto.NewGaugeVec(
			prometheus.GaugeOpts{
				Name: "extension_health_status",
				Help: "Health status of extensions (0=unhealthy, 1=healthy)",
			},
			[]string{"extension"},
		),

		resourceUsage: promauto.NewGaugeVec(
			prometheus.GaugeOpts{
				Name: "extension_resource_usage",
				Help: "Resource usage by extensions",
			},
			[]string{"extension", "resource"},
		),

		hooksExecuted: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Name: "extension_hooks_executed_total",
				Help: "Total number of hooks executed",
			},
			[]string{"extension", "hook", "type"},
		),

		hookDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Name:    "extension_hook_duration_seconds",
				Help:    "Duration of hook execution in seconds",
				Buckets: prometheus.DefBuckets,
			},
			[]string{"extension", "hook", "type"},
		),
	}
}

// RecordRequest records a request for an extension
func (mc *MetricsCollector) RecordRequest(extension, method, path string, duration time.Duration, err error) {
	mc.requestsTotal.WithLabelValues(extension, method, path).Inc()
	mc.requestDuration.WithLabelValues(extension, method, path).Observe(duration.Seconds())

	if err != nil {
		mc.errorsTotal.WithLabelValues(extension, "request").Inc()
	}

	// Update internal metrics
	mc.mu.Lock()
	defer mc.mu.Unlock()

	metrics := mc.getOrCreateMetrics(extension)
	metrics.RequestCount++
	metrics.TotalRequestTime += duration
	if err != nil {
		metrics.ErrorCount++
	}
	metrics.LastActive = time.Now()
}

// RecordHook records a hook execution
func (mc *MetricsCollector) RecordHook(extension, hook string, hookType HookType, duration time.Duration, err error) {
	typeStr := hookTypeToString(hookType)
	mc.hooksExecuted.WithLabelValues(extension, hook, typeStr).Inc()
	mc.hookDuration.WithLabelValues(extension, hook, typeStr).Observe(duration.Seconds())

	if err != nil {
		mc.errorsTotal.WithLabelValues(extension, "hook").Inc()
	}

	// Update internal metrics
	mc.mu.Lock()
	defer mc.mu.Unlock()

	metrics := mc.getOrCreateMetrics(extension)
	metrics.HooksExecuted++
	if err != nil {
		metrics.HookErrors++
	}
	metrics.LastActive = time.Now()
}

// RecordHealth records health status for an extension
func (mc *MetricsCollector) RecordHealth(extension string, healthy bool) {
	healthValue := 0.0
	if healthy {
		healthValue = 1.0
	}
	mc.healthStatus.WithLabelValues(extension).Set(healthValue)

	// Update internal metrics
	mc.mu.Lock()
	defer mc.mu.Unlock()

	metrics := mc.getOrCreateMetrics(extension)
	metrics.Healthy = healthy
	metrics.LastHealthCheck = time.Now()
}

// RecordResourceUsage records resource usage for an extension
func (mc *MetricsCollector) RecordResourceUsage(extension string, resource string, value float64) {
	mc.resourceUsage.WithLabelValues(extension, resource).Set(value)

	// Update internal metrics
	mc.mu.Lock()
	defer mc.mu.Unlock()

	metrics := mc.getOrCreateMetrics(extension)
	switch resource {
	case "memory_mb":
		metrics.MemoryUsageMB = int64(value)
	case "goroutines":
		metrics.GoroutineCount = int(value)
	case "db_connections":
		metrics.DatabaseConnections = int(value)
	}
}

// UpdateActiveExtensions updates the count of active extensions
func (mc *MetricsCollector) UpdateActiveExtensions(count int) {
	mc.activeExtensions.Set(float64(count))
}

// RecordError records an error for an extension
func (mc *MetricsCollector) RecordError(extension string, errorType string) {
	mc.errorsTotal.WithLabelValues(extension, errorType).Inc()

	// Update internal metrics
	mc.mu.Lock()
	defer mc.mu.Unlock()

	metrics := mc.getOrCreateMetrics(extension)
	metrics.ErrorCount++
	metrics.LastError = errorType
	metrics.LastErrorTime = time.Now()
}

// GetMetrics returns metrics for an extension
func (mc *MetricsCollector) GetMetrics(extension string) (*ExtensionMetrics, error) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	metrics, exists := mc.extensionMetrics[extension]
	if !exists {
		return nil, fmt.Errorf("metrics not found for extension: %s", extension)
	}

	// Return a copy to prevent concurrent modification
	copy := *metrics
	return &copy, nil
}

// GetAllMetrics returns metrics for all extensions
func (mc *MetricsCollector) GetAllMetrics() map[string]*ExtensionMetrics {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	result := make(map[string]*ExtensionMetrics)
	for name, metrics := range mc.extensionMetrics {
		copy := *metrics
		result[name] = &copy
	}
	return result
}

// ResetMetrics resets metrics for an extension
func (mc *MetricsCollector) ResetMetrics(extension string) {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	if metrics, exists := mc.extensionMetrics[extension]; exists {
		// Keep some fields
		startTime := metrics.StartTime

		// Reset to new instance
		mc.extensionMetrics[extension] = &ExtensionMetrics{
			StartTime: startTime,
		}
	}
}

// getOrCreateMetrics gets or creates metrics for an extension
func (mc *MetricsCollector) getOrCreateMetrics(extension string) *ExtensionMetrics {
	metrics, exists := mc.extensionMetrics[extension]
	if !exists {
		metrics = &ExtensionMetrics{
			StartTime: time.Now(),
		}
		mc.extensionMetrics[extension] = metrics
	}
	return metrics
}

// hookTypeToString converts HookType to string for metrics
func hookTypeToString(hookType HookType) string {
	// HookType is already a string, just convert it
	return string(hookType)
}

// MetricsMiddleware creates middleware for tracking extension metrics
func (mc *MetricsCollector) MetricsMiddleware(extension string, handler http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		// Wrap response writer to capture status
		wrapped := &responseWrapper{
			ResponseWriter: w,
			statusCode:     http.StatusOK,
		}

		// Execute handler
		handler(wrapped, r)

		// Record metrics
		duration := time.Since(start)
		var err error
		if wrapped.statusCode >= 400 {
			err = fmt.Errorf("http %d", wrapped.statusCode)
		}

		mc.RecordRequest(extension, r.Method, r.URL.Path, duration, err)
	}
}

// responseWrapper wraps http.ResponseWriter to capture status code
type responseWrapper struct {
	http.ResponseWriter
	statusCode int
	written    bool
}

func (w *responseWrapper) WriteHeader(statusCode int) {
	if !w.written {
		w.statusCode = statusCode
		w.written = true
	}
	w.ResponseWriter.WriteHeader(statusCode)
}

func (w *responseWrapper) Write(b []byte) (int, error) {
	if !w.written {
		w.written = true
	}
	return w.ResponseWriter.Write(b)
}

// MetricsExporter exports metrics in various formats
type MetricsExporter struct {
	collector *MetricsCollector
}

// NewMetricsExporter creates a new metrics exporter
func NewMetricsExporter(collector *MetricsCollector) *MetricsExporter {
	return &MetricsExporter{
		collector: collector,
	}
}

// ExportJSON exports metrics as JSON
func (e *MetricsExporter) ExportJSON() ([]byte, error) {
	metrics := e.collector.GetAllMetrics()
	return json.Marshal(metrics)
}

// ExportPrometheus returns Prometheus metrics handler
func (e *MetricsExporter) ExportPrometheus() http.Handler {
	return promhttp.Handler()
}

// MetricsAggregator aggregates metrics across extensions
type MetricsAggregator struct {
	collector *MetricsCollector
}

// NewMetricsAggregator creates a new metrics aggregator
func NewMetricsAggregator(collector *MetricsCollector) *MetricsAggregator {
	return &MetricsAggregator{
		collector: collector,
	}
}

// GetSummary returns aggregated metrics summary
func (a *MetricsAggregator) GetSummary(ctx context.Context) map[string]interface{} {
	allMetrics := a.collector.GetAllMetrics()

	totalRequests := int64(0)
	totalErrors := int64(0)
	totalHooks := int64(0)
	healthyCount := 0
	totalMemoryMB := int64(0)
	totalGoroutines := 0

	for _, metrics := range allMetrics {
		totalRequests += metrics.RequestCount
		totalErrors += metrics.ErrorCount
		totalHooks += metrics.HooksExecuted
		if metrics.Healthy {
			healthyCount++
		}
		totalMemoryMB += metrics.MemoryUsageMB
		totalGoroutines += metrics.GoroutineCount
	}

	return map[string]interface{}{
		"extensions_total":      len(allMetrics),
		"extensions_healthy":    healthyCount,
		"requests_total":        totalRequests,
		"errors_total":          totalErrors,
		"hooks_executed_total":  totalHooks,
		"memory_usage_mb_total": totalMemoryMB,
		"goroutines_total":      totalGoroutines,
		"timestamp":             time.Now(),
	}
}

// GetTopExtensions returns top extensions by request count
func (a *MetricsAggregator) GetTopExtensions(limit int) []map[string]interface{} {
	allMetrics := a.collector.GetAllMetrics()

	// Create slice for sorting
	type extMetric struct {
		name    string
		metrics *ExtensionMetrics
	}

	extensions := make([]extMetric, 0, len(allMetrics))
	for name, metrics := range allMetrics {
		extensions = append(extensions, extMetric{name: name, metrics: metrics})
	}

	// Sort by request count
	sort.Slice(extensions, func(i, j int) bool {
		return extensions[i].metrics.RequestCount > extensions[j].metrics.RequestCount
	})

	// Return top N
	result := []map[string]interface{}{}
	for i := 0; i < limit && i < len(extensions); i++ {
		ext := extensions[i]
		avgDuration := time.Duration(0)
		if ext.metrics.RequestCount > 0 {
			avgDuration = ext.metrics.TotalRequestTime / time.Duration(ext.metrics.RequestCount)
		}

		result = append(result, map[string]interface{}{
			"name":             ext.name,
			"requests":         ext.metrics.RequestCount,
			"errors":           ext.metrics.ErrorCount,
			"average_duration": avgDuration.Seconds(),
			"healthy":          ext.metrics.Healthy,
		})
	}

	return result
}
