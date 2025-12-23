package system

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"net/http"

	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

type DashboardStats struct {
	TotalUsers       int        `json:"totalUsers"`
	TotalRows        int64      `json:"totalRows"`
	TotalStorageUsed int64      `json:"totalStorageUsed"`
	TotalAPICalls    int        `json:"totalApiCalls"`
	UsersGrowth      float64    `json:"usersGrowth"`
	StorageGrowth    float64    `json:"storageGrowth"`
	RecentActivities []Activity `json:"recentActivities"`
}

type Activity struct {
	ID          string       `json:"id"`
	Type        string       `json:"type"`
	Description string       `json:"description"`
	UserID      string       `json:"userId,omitempty"`
	UserEmail   string       `json:"userEmail,omitempty"`
	CreatedAt   apptime.Time `json:"createdAt"`
}

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

		utils.JSONResponse(w, http.StatusOK, stats)
	}
}
