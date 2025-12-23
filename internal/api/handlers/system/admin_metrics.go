package system

import (
	"context"
	"database/sql"
	"net/http"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/suppers-ai/solobase/utils"
)

// AdminMetrics represents aggregated metrics from request logs
type AdminMetrics struct {
	// Time range info
	TimeRange string    `json:"timeRange"`
	StartTime apptime.Time `json:"startTime"`
	EndTime   apptime.Time `json:"endTime"`

	// Request metrics
	TotalRequests     int64            `json:"totalRequests"`
	RequestsByMethod  map[string]int64 `json:"requestsByMethod"`
	RequestsByStatus  map[string]int64 `json:"requestsByStatus"`
	RequestsPerMinute float64          `json:"requestsPerMinute"`

	// Performance metrics
	AvgResponseTimeMs float64 `json:"avgResponseTimeMs"`
	MaxResponseTimeMs int64   `json:"maxResponseTimeMs"`
	MinResponseTimeMs int64   `json:"minResponseTimeMs"`
	P95ResponseTimeMs int64   `json:"p95ResponseTimeMs"`
	P99ResponseTimeMs int64   `json:"p99ResponseTimeMs"`

	// Error metrics
	ErrorCount int64   `json:"errorCount"`
	ErrorRate  float64 `json:"errorRate"`

	// Top endpoints
	TopEndpoints []EndpointStats `json:"topEndpoints"`

	// Top errors
	TopErrors []ErrorStats `json:"topErrors"`

	// Hourly breakdown
	HourlyStats []HourlyStats `json:"hourlyStats"`
}

// EndpointStats represents stats for a single endpoint
type EndpointStats struct {
	Path          string  `json:"path"`
	Method        string  `json:"method"`
	RequestCount  int64   `json:"requestCount"`
	AvgTimeMs     float64 `json:"avgTimeMs"`
	ErrorCount    int64   `json:"errorCount"`
	ErrorRate     float64 `json:"errorRate"`
}

// ErrorStats represents error statistics
type ErrorStats struct {
	Path       string `json:"path"`
	Method     string `json:"method"`
	StatusCode int    `json:"statusCode"`
	Count      int64  `json:"count"`
	LastError  string `json:"lastError,omitempty"`
}

// HourlyStats represents hourly aggregated stats
type HourlyStats struct {
	Hour          string  `json:"hour"`
	RequestCount  int64   `json:"requestCount"`
	AvgTimeMs     float64 `json:"avgTimeMs"`
	ErrorCount    int64   `json:"errorCount"`
}

// HandleGetAdminMetrics returns aggregated metrics from request logs
func HandleGetAdminMetrics(sqlDB *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get time range from query params (default: 24h)
		timeRange := r.URL.Query().Get("range")
		if timeRange == "" {
			timeRange = "24h"
		}

		startTime := calculateMetricsStartTime(timeRange)
		endTime := apptime.NowTime()

		metrics := AdminMetrics{
			TimeRange:        timeRange,
			StartTime:        startTime,
			EndTime:          endTime,
			RequestsByMethod: make(map[string]int64),
			RequestsByStatus: make(map[string]int64),
			TopEndpoints:     []EndpointStats{},
			TopErrors:        []ErrorStats{},
			HourlyStats:      []HourlyStats{},
		}

		// In WASM mode, sqlDB may be nil - return empty metrics
		if sqlDB == nil {
			utils.JSONResponse(w, http.StatusOK, metrics)
			return
		}

		ctx := r.Context()

		// Get basic request stats
		getBasicStats(ctx, sqlDB, startTime, &metrics)

		// Get requests by method
		getRequestsByMethod(ctx, sqlDB, startTime, &metrics)

		// Get requests by status code group
		getRequestsByStatus(ctx, sqlDB, startTime, &metrics)

		// Get response time percentiles
		getResponseTimeStats(ctx, sqlDB, startTime, &metrics)

		// Get top endpoints
		getTopEndpoints(ctx, sqlDB, startTime, &metrics)

		// Get top errors
		getTopErrors(ctx, sqlDB, startTime, &metrics)

		// Get hourly breakdown
		getHourlyStats(ctx, sqlDB, startTime, &metrics)

		// Calculate derived metrics
		if metrics.TotalRequests > 0 {
			durationMinutes := endTime.Sub(startTime).Minutes()
			if durationMinutes > 0 {
				metrics.RequestsPerMinute = float64(metrics.TotalRequests) / durationMinutes
			}
			metrics.ErrorRate = float64(metrics.ErrorCount) / float64(metrics.TotalRequests) * 100
		}

		utils.JSONResponse(w, http.StatusOK, metrics)
	}
}

func calculateMetricsStartTime(timeRange string) apptime.Time {
	now := apptime.NowTime()
	switch timeRange {
	case "1h":
		return now.Add(-1 * apptime.Hour)
	case "6h":
		return now.Add(-6 * apptime.Hour)
	case "24h":
		return now.Add(-24 * apptime.Hour)
	case "7d":
		return now.Add(-7 * 24 * apptime.Hour)
	case "30d":
		return now.Add(-30 * 24 * apptime.Hour)
	default:
		return now.Add(-24 * apptime.Hour)
	}
}

func getBasicStats(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT
			COUNT(*) as total_requests,
			COALESCE(AVG(exec_time_ms), 0) as avg_time,
			COALESCE(MAX(exec_time_ms), 0) as max_time,
			COALESCE(MIN(exec_time_ms), 0) as min_time,
			COALESCE(SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END), 0) as error_count
		FROM sys_request_logs
		WHERE created_at >= ?
	`

	row := db.QueryRowContext(ctx, query, startTime)
	var avgTime float64
	row.Scan(&metrics.TotalRequests, &avgTime, &metrics.MaxResponseTimeMs, &metrics.MinResponseTimeMs, &metrics.ErrorCount)
	metrics.AvgResponseTimeMs = avgTime
}

func getRequestsByMethod(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT method, COUNT(*) as count
		FROM sys_request_logs
		WHERE created_at >= ?
		GROUP BY method
		ORDER BY count DESC
	`

	rows, err := db.QueryContext(ctx, query, startTime)
	if err != nil {
		return
	}
	defer rows.Close()

	for rows.Next() {
		var method string
		var count int64
		if err := rows.Scan(&method, &count); err == nil {
			metrics.RequestsByMethod[method] = count
		}
	}
}

func getRequestsByStatus(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT
			CASE
				WHEN status_code >= 200 AND status_code < 300 THEN '2xx'
				WHEN status_code >= 300 AND status_code < 400 THEN '3xx'
				WHEN status_code >= 400 AND status_code < 500 THEN '4xx'
				WHEN status_code >= 500 THEN '5xx'
				ELSE 'other'
			END as status_group,
			COUNT(*) as count
		FROM sys_request_logs
		WHERE created_at >= ?
		GROUP BY status_group
		ORDER BY count DESC
	`

	rows, err := db.QueryContext(ctx, query, startTime)
	if err != nil {
		return
	}
	defer rows.Close()

	for rows.Next() {
		var statusGroup string
		var count int64
		if err := rows.Scan(&statusGroup, &count); err == nil {
			metrics.RequestsByStatus[statusGroup] = count
		}
	}
}

func getResponseTimeStats(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	// Get P95 and P99 response times using percentile approximation
	// SQLite doesn't have built-in percentile functions, so we use a subquery approach
	query := `
		WITH ordered_times AS (
			SELECT exec_time_ms,
				ROW_NUMBER() OVER (ORDER BY exec_time_ms) as rn,
				COUNT(*) OVER () as total
			FROM sys_request_logs
			WHERE created_at >= ?
		)
		SELECT
			COALESCE(MAX(CASE WHEN rn = CAST(total * 0.95 AS INTEGER) THEN exec_time_ms END), 0) as p95,
			COALESCE(MAX(CASE WHEN rn = CAST(total * 0.99 AS INTEGER) THEN exec_time_ms END), 0) as p99
		FROM ordered_times
	`

	row := db.QueryRowContext(ctx, query, startTime)
	row.Scan(&metrics.P95ResponseTimeMs, &metrics.P99ResponseTimeMs)
}

func getTopEndpoints(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT
			path,
			method,
			COUNT(*) as request_count,
			COALESCE(AVG(exec_time_ms), 0) as avg_time,
			COALESCE(SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END), 0) as error_count
		FROM sys_request_logs
		WHERE created_at >= ?
		GROUP BY path, method
		ORDER BY request_count DESC
		LIMIT 10
	`

	rows, err := db.QueryContext(ctx, query, startTime)
	if err != nil {
		return
	}
	defer rows.Close()

	for rows.Next() {
		var stat EndpointStats
		if err := rows.Scan(&stat.Path, &stat.Method, &stat.RequestCount, &stat.AvgTimeMs, &stat.ErrorCount); err == nil {
			if stat.RequestCount > 0 {
				stat.ErrorRate = float64(stat.ErrorCount) / float64(stat.RequestCount) * 100
			}
			metrics.TopEndpoints = append(metrics.TopEndpoints, stat)
		}
	}
}

func getTopErrors(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT
			path,
			method,
			status_code,
			COUNT(*) as count,
			COALESCE(MAX(error), '') as last_error
		FROM sys_request_logs
		WHERE created_at >= ? AND status_code >= 400
		GROUP BY path, method, status_code
		ORDER BY count DESC
		LIMIT 10
	`

	rows, err := db.QueryContext(ctx, query, startTime)
	if err != nil {
		return
	}
	defer rows.Close()

	for rows.Next() {
		var stat ErrorStats
		var lastError sql.NullString
		if err := rows.Scan(&stat.Path, &stat.Method, &stat.StatusCode, &stat.Count, &lastError); err == nil {
			if lastError.Valid {
				stat.LastError = lastError.String
			}
			metrics.TopErrors = append(metrics.TopErrors, stat)
		}
	}
}

func getHourlyStats(ctx context.Context, db *sql.DB, startTime apptime.Time, metrics *AdminMetrics) {
	query := `
		SELECT
			strftime('%Y-%m-%d %H:00', created_at) as hour,
			COUNT(*) as request_count,
			COALESCE(AVG(exec_time_ms), 0) as avg_time,
			COALESCE(SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END), 0) as error_count
		FROM sys_request_logs
		WHERE created_at >= ?
		GROUP BY strftime('%Y-%m-%d %H:00', created_at)
		ORDER BY hour DESC
		LIMIT 24
	`

	rows, err := db.QueryContext(ctx, query, startTime)
	if err != nil {
		return
	}
	defer rows.Close()

	for rows.Next() {
		var stat HourlyStats
		if err := rows.Scan(&stat.Hour, &stat.RequestCount, &stat.AvgTimeMs, &stat.ErrorCount); err == nil {
			metrics.HourlyStats = append(metrics.HourlyStats, stat)
		}
	}
}
