package middleware

import (
	"net/http"
	"strconv"
	"time"

	"github.com/gorilla/mux"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

var (
	httpDuration = prometheus.NewHistogramVec(prometheus.HistogramOpts{
		Name: "http_duration_seconds",
		Help: "Duration of HTTP requests.",
	}, []string{"path", "method", "status"})

	httpRequests = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "http_requests_total",
		Help: "Total number of HTTP requests.",
	}, []string{"path", "method", "status"})

	httpRequestSize = prometheus.NewHistogramVec(prometheus.HistogramOpts{
		Name: "http_request_size_bytes",
		Help: "Size of HTTP requests.",
	}, []string{"path", "method"})

	httpResponseSize = prometheus.NewHistogramVec(prometheus.HistogramOpts{
		Name: "http_response_size_bytes",
		Help: "Size of HTTP responses.",
	}, []string{"path", "method", "status"})
)

func init() {
	// Register metrics with prometheus
	// NOTE: These may already be registered by the system
	// Only register if not already registered
	prometheus.Register(httpDuration)
	prometheus.Register(httpRequests)
	prometheus.Register(httpRequestSize)
	prometheus.Register(httpResponseSize)
}

// metricsResponseWriter wraps http.ResponseWriter to capture status code and size
type metricsResponseWriter struct {
	http.ResponseWriter
	statusCode int
	size       int
}

func (rw *metricsResponseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}

func (rw *metricsResponseWriter) Write(b []byte) (int, error) {
	size, err := rw.ResponseWriter.Write(b)
	rw.size += size
	return size, err
}

// MetricsMiddleware collects HTTP metrics for Prometheus
func MetricsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		// Get the route pattern from mux
		route := mux.CurrentRoute(r)
		path := "unknown"
		if route != nil {
			pathTemplate, err := route.GetPathTemplate()
			if err == nil {
				path = pathTemplate
			}
		}

		// Wrap the ResponseWriter to capture status code and size
		wrapped := &metricsResponseWriter{
			ResponseWriter: w,
			statusCode:     http.StatusOK,
		}

		// Record request size
		if r.ContentLength > 0 {
			httpRequestSize.WithLabelValues(path, r.Method).Observe(float64(r.ContentLength))
		}

		// Process request
		next.ServeHTTP(wrapped, r)

		// Record metrics
		duration := time.Since(start).Seconds()
		status := strconv.Itoa(wrapped.statusCode)

		httpDuration.WithLabelValues(path, r.Method, status).Observe(duration)
		httpRequests.WithLabelValues(path, r.Method, status).Inc()
		httpResponseSize.WithLabelValues(path, r.Method, status).Observe(float64(wrapped.size))
	})
}

// PrometheusHandler returns the Prometheus metrics handler
func PrometheusHandler() http.Handler {
	return promhttp.Handler()
}