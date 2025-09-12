package api

import (
	"net/http"
	"time"

	"github.com/suppers-ai/solobase/services"
)

type DashboardStats struct {
	TotalUsers       int        `json:"total_users"`
	TotalRows        int64      `json:"total_rows"`
	TotalStorageUsed int64      `json:"total_storage_used"`
	TotalAPICalls    int        `json:"total_api_calls"`
	UsersGrowth      float64    `json:"users_growth"`
	StorageGrowth    float64    `json:"storage_growth"`
	RecentActivities []Activity `json:"recent_activities"`
}

type Activity struct {
	ID          string    `json:"id"`
	Type        string    `json:"type"`
	Description string    `json:"description"`
	UserID      string    `json:"user_id,omitempty"`
	UserEmail   string    `json:"user_email,omitempty"`
	CreatedAt   time.Time `json:"created_at"`
}

// TODO: Re-enable when metrics service is properly integrated
// type MetricsHistory struct {
// 	Timestamps      []string  `json:"timestamps"`
// 	RequestsPerMin  []float64 `json:"requests_per_min"`
// 	ResponseTimeMs  []float64 `json:"response_time_ms"`
// 	CPUUsage        []float64 `json:"cpu_usage"`
// 	MemoryUsage     []float64 `json:"memory_usage"`
// 	ErrorRate       []float64 `json:"error_rate"`
// 	DBQueries       []int64   `json:"db_queries"`
// }

func HandleGetDashboardStats(
	userService *services.UserService,
	storageService *services.StorageService,
	databaseService *services.DatabaseService,
) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get total users
		totalUsers, err := userService.GetUserCount()
		if err != nil {
			totalUsers = 0
		}

		// Get total rows across all tables
		totalRows, err := databaseService.GetTotalRowCount()
		if err != nil {
			totalRows = 0
		}

		// Get storage usage
		totalStorage, err := storageService.GetTotalStorageUsed()
		if err != nil {
			totalStorage = 0
		}

		// Get total API calls from metrics collector
		metricsCollector.mu.RLock()
		totalAPICalls := int64(0)
		for _, count := range metricsCollector.apiCalls {
			totalAPICalls += count
		}
		metricsCollector.mu.RUnlock()

		// Calculate growth - set to 0 for now until we implement historical tracking
		usersGrowth := 0.0
		storageGrowth := 0.0

		// Get recent activities - empty for now until we implement activity logging
		activities := []Activity{}

		stats := DashboardStats{
			TotalUsers:       totalUsers,
			TotalRows:        totalRows,
			TotalStorageUsed: totalStorage,
			TotalAPICalls:    int(totalAPICalls),
			UsersGrowth:      usersGrowth,
			StorageGrowth:    storageGrowth,
			RecentActivities: activities,
		}

		RespondWithJSON(w, http.StatusOK, stats)
	}
}

// TODO: Re-enable when metrics service is properly integrated
// func HandleGetMetricsHistory(metricsService *services.MetricsService) http.HandlerFunc {
// 	return func(w http.ResponseWriter, r *http.Request) {
// 		// Get last 20 data points (10 minutes at 30 second intervals)
// 		history := metricsService.GetMetricsHistory(20)
//
// 		// Format timestamps for display
// 		timestamps := make([]string, len(history))
// 		requestsPerMin := make([]float64, len(history))
// 		responseTimeMs := make([]float64, len(history))
// 		cpuUsage := make([]float64, len(history))
// 		memoryUsage := make([]float64, len(history))
// 		errorRate := make([]float64, len(history))
// 		dbQueries := make([]int64, len(history))
//
// 		for i, point := range history {
// 			timestamps[i] = point.Timestamp.Format("15:04:05")
// 			requestsPerMin[i] = point.RequestRate
// 			responseTimeMs[i] = point.ResponseTime
// 			cpuUsage[i] = point.CPUUsage
// 			memoryUsage[i] = point.MemoryUsage
// 			errorRate[i] = point.ErrorRate
// 			dbQueries[i] = point.DBQueries
// 		}
//
// 		response := MetricsHistory{
// 			Timestamps:      timestamps,
// 			RequestsPerMin:  requestsPerMin,
// 			ResponseTimeMs:  responseTimeMs,
// 			CPUUsage:        cpuUsage,
// 			MemoryUsage:     memoryUsage,
// 			ErrorRate:       errorRate,
// 			DBQueries:       dbQueries,
// 		}
//
// 		RespondWithJSON(w, http.StatusOK, response)
// 	}
// }
