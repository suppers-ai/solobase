package system

import (
	"fmt"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"net/http"

	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

// SystemMetrics represents basic system metrics
type SystemMetrics struct {
	CPUUsage      float64 `json:"cpuUsage"`
	MemoryUsage   float64 `json:"memoryUsage"`
	MemoryTotal   uint64  `json:"memoryTotal"`
	MemoryUsed    uint64  `json:"memoryUsed"`
	DiskUsage     float64 `json:"diskUsage"`
	DiskTotal     uint64  `json:"diskTotal"`
	DiskUsed      uint64  `json:"diskUsed"`
	Uptime        string  `json:"uptime"`
	Goroutines    int     `json:"goroutines"`
	DBQueries     int64   `json:"dbQueries"`
	APICallsTotal int64   `json:"apiCallsTotal"`
}

var startTime = apptime.NowTime()

// HandleDebugTime returns time debugging info for WASM testing
func HandleDebugTime() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		now := apptime.NowTime()
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"now":         now.String(),
			"unix":        now.Unix(),
			"unixNano":    now.UnixNano(),
			"rfc3339":     now.Format(apptime.TimeFormat),
			"isZero":      now.IsZero(),
			"year":        now.Year(),
			"startTime":   startTime.String(),
			"startIsZero": startTime.IsZero(),
		})
	}
}

// HandleGetSystemMetrics returns basic system metrics
// For detailed request metrics, use /api/admin/metrics
func HandleGetSystemMetrics() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		uptime := apptime.Since(startTime)

		metrics := SystemMetrics{
			CPUUsage:      0, // Not tracked without external dependencies
			MemoryUsage:   0,
			MemoryTotal:   0,
			MemoryUsed:    0,
			DiskUsage:     0,
			DiskTotal:     0,
			DiskUsed:      0,
			Uptime:        formatUptime(uptime),
			Goroutines:    0,
			DBQueries:     0,
			APICallsTotal: 0,
		}

		utils.JSONResponse(w, http.StatusOK, metrics)
	}
}

func formatUptime(d apptime.Duration) string {
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

// HandleGetDatabaseInfo returns database configuration info
func HandleGetDatabaseInfo(databaseService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		dbType, dbVersion := databaseService.GetDatabaseInfo()

		info := map[string]interface{}{
			"type":    dbType,
			"version": dbVersion,
		}

		utils.JSONResponse(w, http.StatusOK, info)
	}
}
