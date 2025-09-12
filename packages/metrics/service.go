package metrics

import (
	"context"
	"fmt"
	"sync"
	"time"
)

// Service implements MetricsService
type Service struct {
	mu              sync.RWMutex
	config          *Config
	collector       Collector
	systemCollector SystemCollector

	// HTTP metrics
	httpRequests Counter
	httpErrors   Counter
	httpDuration Histogram

	// Database metrics
	dbQueries     Counter
	dbErrors      Counter
	dbDuration    Histogram
	dbConnections Gauge

	// Business metrics
	userActions Counter
	events      Counter

	// Status
	running bool
	stopCh  chan struct{}
}

// NewService creates a new metrics service
func NewService(config *Config) (*Service, error) {
	if config == nil {
		config = DefaultConfig()
	}

	var collector Collector
	switch config.Type {
	case "prometheus":
		collector = NewPrometheusCollector(config)
	default:
		return nil, fmt.Errorf("unsupported metrics type: %s", config.Type)
	}

	systemCollector := NewSystemMetricsCollector(config)

	s := &Service{
		config:          config,
		collector:       collector,
		systemCollector: systemCollector.(*SystemMetricsCollector),
		stopCh:          make(chan struct{}),
	}

	// Initialize common metrics
	s.initializeMetrics()

	return s, nil
}

// initializeMetrics initializes common metrics
func (s *Service) initializeMetrics() {
	// HTTP metrics
	s.httpRequests = s.collector.Counter("http_requests_total",
		Label{Name: "service", Value: "solobase"})
	s.httpErrors = s.collector.Counter("http_errors_total",
		Label{Name: "service", Value: "solobase"})
	s.httpDuration = s.collector.Histogram("http_request_duration_seconds",
		[]float64{0.001, 0.01, 0.1, 0.5, 1, 2.5, 5, 10},
		Label{Name: "service", Value: "solobase"})

	// Database metrics
	s.dbQueries = s.collector.Counter("db_queries_total",
		Label{Name: "service", Value: "solobase"})
	s.dbErrors = s.collector.Counter("db_errors_total",
		Label{Name: "service", Value: "solobase"})
	s.dbDuration = s.collector.Histogram("db_query_duration_seconds",
		[]float64{0.001, 0.01, 0.1, 0.5, 1, 2.5, 5},
		Label{Name: "service", Value: "solobase"})
	s.dbConnections = s.collector.Gauge("db_connections_active",
		Label{Name: "service", Value: "solobase"})

	// Business metrics
	s.userActions = s.collector.Counter("user_actions_total",
		Label{Name: "service", Value: "solobase"})
	s.events = s.collector.Counter("events_total",
		Label{Name: "service", Value: "solobase"})
}

// GetCollector returns the metrics collector
func (s *Service) GetCollector() Collector {
	return s.collector
}

// GetSystemCollector returns the system metrics collector
func (s *Service) GetSystemCollector() SystemCollector {
	return s.systemCollector
}

// RecordHTTPRequest records an HTTP request
func (s *Service) RecordHTTPRequest(method, path string, statusCode int, duration time.Duration) {
	// Increment request counter with labels
	counter := s.collector.Counter("http_requests_total",
		Label{Name: "method", Value: method},
		Label{Name: "path", Value: path},
		Label{Name: "status", Value: fmt.Sprintf("%d", statusCode)})
	counter.Inc()

	// Record duration
	histogram := s.collector.Histogram("http_request_duration_seconds",
		[]float64{0.001, 0.01, 0.1, 0.5, 1, 2.5, 5, 10},
		Label{Name: "method", Value: method},
		Label{Name: "path", Value: path})
	histogram.Observe(duration.Seconds())

	// Update system collector
	if sc, ok := s.systemCollector.(*SystemMetricsCollector); ok {
		sc.IncrementRequests()
		if statusCode >= 400 {
			sc.IncrementErrors()
		}
	}
}

// RecordHTTPError records an HTTP error
func (s *Service) RecordHTTPError(method, path string, err error) {
	counter := s.collector.Counter("http_errors_total",
		Label{Name: "method", Value: method},
		Label{Name: "path", Value: path},
		Label{Name: "error", Value: err.Error()})
	counter.Inc()

	// Update system collector
	if sc, ok := s.systemCollector.(*SystemMetricsCollector); ok {
		sc.IncrementErrors()
	}
}

// RecordDBQuery records a database query
func (s *Service) RecordDBQuery(query string, duration time.Duration, err error) {
	// Count query
	s.dbQueries.Inc()

	// Record duration
	s.dbDuration.Observe(duration.Seconds())

	// Count errors
	if err != nil {
		s.dbErrors.Inc()
	}

	// Update system collector
	if sc, ok := s.systemCollector.(*SystemMetricsCollector); ok {
		sc.IncrementDBQueries()
	}
}

// RecordDBConnection records database connection state
func (s *Service) RecordDBConnection(connected bool) {
	if connected {
		s.dbConnections.Inc()
	} else {
		s.dbConnections.Dec()
	}
}

// RecordUserAction records a user action
func (s *Service) RecordUserAction(action string, userID string, success bool) {
	counter := s.collector.Counter("user_actions_total",
		Label{Name: "action", Value: action},
		Label{Name: "success", Value: fmt.Sprintf("%t", success)})
	counter.Inc()
}

// RecordEvent records an event
func (s *Service) RecordEvent(event string, metadata map[string]string) {
	labels := []Label{
		{Name: "event", Value: event},
	}
	for k, v := range metadata {
		labels = append(labels, Label{Name: k, Value: v})
	}

	counter := s.collector.Counter("events_total", labels...)
	counter.Inc()
}

// GetSystemMetrics returns current system metrics
func (s *Service) GetSystemMetrics() *SystemMetrics {
	return s.systemCollector.GetLatest()
}

// GetApplicationMetrics returns application metrics
func (s *Service) GetApplicationMetrics() map[string]interface{} {
	systemMetrics := s.systemCollector.GetLatest()
	if systemMetrics == nil {
		return map[string]interface{}{}
	}

	return map[string]interface{}{
		"uptime":              systemMetrics.Uptime.String(),
		"requests_total":      systemMetrics.RequestsTotal,
		"requests_per_second": systemMetrics.RequestsPerSecond,
		"error_rate":          systemMetrics.ErrorRate,
		"active_connections":  systemMetrics.ActiveConnections,
		"goroutines":          systemMetrics.GoRoutines,
		"memory_used":         systemMetrics.HeapAlloc,
	}
}

// Start starts the metrics service
func (s *Service) Start(ctx context.Context) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.running {
		return nil
	}

	// Start collector
	if err := s.collector.Start(ctx); err != nil {
		return fmt.Errorf("failed to start collector: %w", err)
	}

	// Start system metrics collection
	if err := s.systemCollector.Start(ctx, s.config.CollectInterval); err != nil {
		return fmt.Errorf("failed to start system collector: %w", err)
	}

	s.running = true

	// Start periodic collection if needed
	go s.runCollectionLoop(ctx)

	return nil
}

// Stop stops the metrics service
func (s *Service) Stop(ctx context.Context) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if !s.running {
		return nil
	}

	// Signal stop
	close(s.stopCh)

	// Stop collectors
	s.collector.Stop(ctx)
	s.systemCollector.Stop()

	s.running = false

	return nil
}

// runCollectionLoop runs periodic metric collection
func (s *Service) runCollectionLoop(ctx context.Context) {
	ticker := time.NewTicker(s.config.CollectInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			if err := s.collector.Collect(ctx); err != nil {
				// Log error
			}
		case <-s.stopCh:
			return
		case <-ctx.Done():
			return
		}
	}
}

// GetHandler returns the HTTP handler for metrics endpoint
func (s *Service) GetHandler() interface{} {
	return s.collector.Handler()
}

// MetricsHistoryPoint represents a point in metrics history
type MetricsHistoryPoint struct {
	Timestamp    time.Time
	RequestRate  float64
	ResponseTime float64
	CPUUsage     float64
	MemoryUsage  float64
	ErrorRate    float64
	DBQueries    int64
}

// GetMetricsHistory returns historical metrics data
func (s *Service) GetMetricsHistory(points int) []MetricsHistoryPoint {
	// Get system metrics history
	systemHistory := s.systemCollector.(*SystemMetricsCollector).GetHistory(points)

	result := make([]MetricsHistoryPoint, len(systemHistory))
	for i, m := range systemHistory {
		result[i] = MetricsHistoryPoint{
			Timestamp:    m.Timestamp,
			RequestRate:  m.RequestsPerSecond * 60,                            // Convert to per minute
			ResponseTime: float64(m.ResponseTime) / float64(time.Millisecond), // Convert to ms
			CPUUsage:     m.CPUUsage,
			MemoryUsage:  float64(m.HeapAlloc) / float64(m.MemTotal) * 100,
			ErrorRate:    m.ErrorRate,
			DBQueries:    m.DBQueries,
		}
	}

	return result
}
