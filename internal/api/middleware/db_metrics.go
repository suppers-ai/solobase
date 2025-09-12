package middleware

import (
	"github.com/prometheus/client_golang/prometheus"
)

var (
	dbQueryDuration = prometheus.NewHistogramVec(prometheus.HistogramOpts{
		Name: "db_query_duration_seconds",
		Help: "Duration of database queries.",
	}, []string{"operation", "table"})

	dbQueryTotal = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "db_queries_total",
		Help: "Total number of database queries.",
	}, []string{"operation", "table"})

	dbErrorTotal = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "db_errors_total",
		Help: "Total number of database errors.",
	}, []string{"operation", "table"})
)

func init() {
	// Register metrics with prometheus
	prometheus.MustRegister(dbQueryDuration)
	prometheus.MustRegister(dbQueryTotal)
	prometheus.MustRegister(dbErrorTotal)
}

// RecordDBQuery records database query metrics
func RecordDBQuery(operation string, duration float64, isError bool) {
	// Extract table name from operation if possible
	table := "unknown"
	
	dbQueryDuration.WithLabelValues(operation, table).Observe(duration)
	dbQueryTotal.WithLabelValues(operation, table).Inc()
	
	if isError {
		dbErrorTotal.WithLabelValues(operation, table).Inc()
	}
}