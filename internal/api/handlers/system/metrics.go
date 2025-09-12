package system

import (
	"fmt"
	"net/http"
	"sync"
	"time"
)

// MetricsCollector collects application metrics
type MetricsCollector struct {
	mu sync.RWMutex

	// HTTP metrics
	httpRequests      map[string]map[string]int64 // method -> status -> count
	httpDurations     []float64
	httpDurationSum   float64
	httpDurationCount int64

	// Database metrics
	dbQueries       map[string]int64 // operation -> count
	dbErrors        int64
	dbDuration      float64
	dbDurationCount int64

	// Business metrics
	userRegistrations int64
	userLogins        int64
	apiCalls          map[string]int64 // endpoint -> count

	startTime time.Time
}

var metricsCollector = &MetricsCollector{
	httpRequests: make(map[string]map[string]int64),
	dbQueries:    make(map[string]int64),
	apiCalls:     make(map[string]int64),
	startTime:    time.Now(),
}

// RecordHTTPRequest records an HTTP request
func RecordHTTPRequest(method, status string, duration float64) {
	metricsCollector.mu.Lock()
	defer metricsCollector.mu.Unlock()

	if metricsCollector.httpRequests[method] == nil {
		metricsCollector.httpRequests[method] = make(map[string]int64)
	}
	metricsCollector.httpRequests[method][status]++

	metricsCollector.httpDurationSum += duration
	metricsCollector.httpDurationCount++
	metricsCollector.httpDurations = append(metricsCollector.httpDurations, duration)
}

// RecordDBQuery records a database query
func RecordDBQuery(operation string, duration float64, isError bool) {
	metricsCollector.mu.Lock()
	defer metricsCollector.mu.Unlock()

	metricsCollector.dbQueries[operation]++
	if isError {
		metricsCollector.dbErrors++
	}
	metricsCollector.dbDuration += duration
	metricsCollector.dbDurationCount++
}

// RecordUserRegistration records a user registration
func RecordUserRegistration() {
	metricsCollector.mu.Lock()
	defer metricsCollector.mu.Unlock()
	metricsCollector.userRegistrations++
}

// RecordUserLogin records a user login
func RecordUserLogin() {
	metricsCollector.mu.Lock()
	defer metricsCollector.mu.Unlock()
	metricsCollector.userLogins++
}

// RecordAPICall records an API call
func RecordAPICall(endpoint string) {
	metricsCollector.mu.Lock()
	defer metricsCollector.mu.Unlock()
	metricsCollector.apiCalls[endpoint]++
}

// HandleGetMetrics returns Prometheus-formatted metrics
func HandleGetMetrics() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		metricsCollector.mu.RLock()
		defer metricsCollector.mu.RUnlock()

		w.Header().Set("Content-Type", "text/plain; version=0.0.4")

		// Write metrics in Prometheus format
		fmt.Fprintln(w, "# HELP http_requests_total Total number of HTTP requests")
		fmt.Fprintln(w, "# TYPE http_requests_total counter")
		for method, statuses := range metricsCollector.httpRequests {
			for status, count := range statuses {
				fmt.Fprintf(w, "http_requests_total{method=\"%s\",status=\"%s\"} %d\n", method, status, count)
			}
		}

		fmt.Fprintln(w, "# HELP http_request_duration_seconds HTTP request latencies in seconds")
		fmt.Fprintln(w, "# TYPE http_request_duration_seconds summary")
		if metricsCollector.httpDurationCount > 0 {
			fmt.Fprintf(w, "http_request_duration_seconds_sum{} %f\n", metricsCollector.httpDurationSum)
			fmt.Fprintf(w, "http_request_duration_seconds_count{} %d\n", metricsCollector.httpDurationCount)
		}

		fmt.Fprintln(w, "# HELP database_queries_total Total number of database queries")
		fmt.Fprintln(w, "# TYPE database_queries_total counter")
		for operation, count := range metricsCollector.dbQueries {
			fmt.Fprintf(w, "database_queries_total{operation=\"%s\"} %d\n", operation, count)
		}

		fmt.Fprintln(w, "# HELP database_errors_total Total number of database errors")
		fmt.Fprintln(w, "# TYPE database_errors_total counter")
		fmt.Fprintf(w, "database_errors_total{} %d\n", metricsCollector.dbErrors)

		if metricsCollector.dbDurationCount > 0 {
			fmt.Fprintln(w, "# HELP database_query_duration_seconds Database query latencies in seconds")
			fmt.Fprintln(w, "# TYPE database_query_duration_seconds summary")
			fmt.Fprintf(w, "database_query_duration_seconds_sum{} %f\n", metricsCollector.dbDuration)
			fmt.Fprintf(w, "database_query_duration_seconds_count{} %d\n", metricsCollector.dbDurationCount)
		}

		fmt.Fprintln(w, "# HELP user_registrations_total Total number of user registrations")
		fmt.Fprintln(w, "# TYPE user_registrations_total counter")
		fmt.Fprintf(w, "user_registrations_total{} %d\n", metricsCollector.userRegistrations)

		fmt.Fprintln(w, "# HELP user_logins_total Total number of user logins")
		fmt.Fprintln(w, "# TYPE user_logins_total counter")
		fmt.Fprintf(w, "user_logins_total{} %d\n", metricsCollector.userLogins)

		fmt.Fprintln(w, "# HELP api_calls_total Total number of API calls by endpoint")
		fmt.Fprintln(w, "# TYPE api_calls_total counter")
		for endpoint, count := range metricsCollector.apiCalls {
			fmt.Fprintf(w, "api_calls_total{endpoint=\"%s\"} %d\n", endpoint, count)
		}

		// Uptime metric
		uptime := time.Since(metricsCollector.startTime).Seconds()
		fmt.Fprintln(w, "# HELP uptime_seconds Number of seconds since the application started")
		fmt.Fprintln(w, "# TYPE uptime_seconds gauge")
		fmt.Fprintf(w, "uptime_seconds{} %f\n", uptime)
	}
}

// MetricsMiddleware is middleware to track HTTP metrics
func MetricsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		// Wrap ResponseWriter to capture status code
		wrapped := &responseWriter{ResponseWriter: w, statusCode: http.StatusOK}

		// Call the next handler
		next.ServeHTTP(wrapped, r)

		// Record metrics
		duration := time.Since(start).Seconds()
		status := fmt.Sprintf("%d", wrapped.statusCode)
		RecordHTTPRequest(r.Method, status, duration)
		RecordAPICall(r.URL.Path)
	})
}

type responseWriter struct {
	http.ResponseWriter
	statusCode int
}

func (rw *responseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}
