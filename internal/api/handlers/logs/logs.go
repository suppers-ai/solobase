package logs

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"time"

	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

// HandleGetLogs returns paginated logs
func HandleGetLogs(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse query parameters
		page, _ := strconv.Atoi(r.URL.Query().Get("page"))
		if page < 1 {
			page = 1
		}

		size, _ := strconv.Atoi(r.URL.Query().Get("size"))
		if size < 1 || size > 1000 {
			size = 100
		}

		level := r.URL.Query().Get("level")
		search := r.URL.Query().Get("search")
		timeRange := r.URL.Query().Get("range")
		if timeRange == "" {
			timeRange = "24h"
		}

		// Get logs
		logs, total, err := logsService.GetLogs(page, size, level, search, timeRange)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch logs")
			return
		}

		// Transform logs to include formatted data
		var responseLogs []map[string]interface{}
		for _, log := range logs {
			logMap := map[string]interface{}{
				"id":        log.ID.String(),
				"level":     log.Level,
				"message":   log.Message,
				"createdAt": log.CreatedAt.Format(time.RFC3339),
				"userID":    log.UserID,
				"traceID":   log.TraceID,
			}

			// Parse fields JSON if present
			if len(log.Fields) > 0 {
				var fields map[string]interface{}
				if err := json.Unmarshal(log.Fields, &fields); err == nil {
					logMap["fields"] = fields
				}
			}

			responseLogs = append(responseLogs, logMap)
		}

		result := map[string]interface{}{
			"logs":  responseLogs,
			"total": total,
			"page":  page,
			"size":  size,
		}

		utils.JSONResponse(w, http.StatusOK, result)
	}
}

// HandleGetRequestLogs returns paginated request logs
func HandleGetRequestLogs(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse query parameters
		page, _ := strconv.Atoi(r.URL.Query().Get("page"))
		if page < 1 {
			page = 1
		}

		size, _ := strconv.Atoi(r.URL.Query().Get("size"))
		if size < 1 || size > 1000 {
			size = 100
		}

		method := r.URL.Query().Get("method")
		path := r.URL.Query().Get("path")
		timeRange := r.URL.Query().Get("range")
		if timeRange == "" {
			timeRange = "24h"
		}

		minStatus, _ := strconv.Atoi(r.URL.Query().Get("minStatus"))
		maxStatus, _ := strconv.Atoi(r.URL.Query().Get("maxStatus"))

		// Get request logs
		logs, total, err := logsService.GetRequestLogs(page, size, method, path, timeRange, minStatus, maxStatus)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch request logs")
			return
		}

		// Transform logs for response
		var responseLogs []map[string]interface{}
		for _, log := range logs {
			duration := fmt.Sprintf("%dms", log.ExecTimeMs)

			// Determine level based on status code
			level := "info"
			if log.StatusCode >= 400 && log.StatusCode < 500 {
				level = "warning"
			} else if log.StatusCode >= 500 {
				level = "error"
			}

			logMap := map[string]interface{}{
				"id":        log.ID.String(),
				"level":     level,
				"method":    log.Method,
				"path":      log.Path,
				"status":    log.StatusCode,
				"duration":  duration,
				"userIP":    log.UserIP,
				"userID":    log.UserID,
				"message":   fmt.Sprintf("%s %s", log.Method, log.Path),
				"createdAt": log.CreatedAt.Format(time.RFC3339),
				"error":     log.Error,
				"userAgent": log.UserAgent,
				"traceID":   log.TraceID,
			}

			responseLogs = append(responseLogs, logMap)
		}

		result := map[string]interface{}{
			"logs":  responseLogs,
			"total": total,
			"page":  page,
			"size":  size,
		}

		utils.JSONResponse(w, http.StatusOK, result)
	}
}

// HandleGetLogStats returns log statistics for charts
func HandleGetLogStats(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		timeRange := r.URL.Query().Get("range")
		if timeRange == "" {
			timeRange = "24h"
		}

		stats, err := logsService.GetLogStats(timeRange)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch log statistics")
			return
		}

		utils.JSONResponse(w, http.StatusOK, stats)
	}
}

// HandleGetLogDetails returns details for a specific log
func HandleGetLogDetails(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		logID := r.URL.Query().Get("id")
		if logID == "" {
			utils.JSONError(w, http.StatusBadRequest, "Log ID is required")
			return
		}

		// Try to get regular log first
		log, err := logsService.GetLogByID(logID)
		if err != nil {
			// Try request log
			reqLog, err := logsService.GetRequestLogByID(logID)
			if err != nil {
				utils.JSONError(w, http.StatusNotFound, "Log not found")
				return
			}

			// Format request log as response
			duration := fmt.Sprintf("%dms", reqLog.ExecTimeMs)
			result := map[string]interface{}{
				"id":        reqLog.ID.String(),
				"createdAt": reqLog.CreatedAt.Format(time.RFC3339),
				"level":     getLogLevelFromStatus(reqLog.StatusCode),
				"method":    reqLog.Method,
				"path":      reqLog.Path,
				"status":    reqLog.StatusCode,
				"duration":  duration,
				"userID":    reqLog.UserID,
				"userIP":    reqLog.UserIP,
				"message":   fmt.Sprintf("%s %s - %d", reqLog.Method, reqLog.Path, reqLog.StatusCode),
				"error":     reqLog.Error,
				"userAgent": reqLog.UserAgent,
				"traceID":   reqLog.TraceID,
				"query":     reqLog.Query,
			}

			// Add request/response bodies if present
			if reqLog.RequestBody != nil {
				result["requestBody"] = *reqLog.RequestBody
			}
			if reqLog.ResponseBody != nil {
				result["responseBody"] = *reqLog.ResponseBody
			}
			if reqLog.Headers != nil {
				result["headers"] = *reqLog.Headers
			}

			utils.JSONResponse(w, http.StatusOK, result)
			return
		}

		// Format regular log as response
		result := map[string]interface{}{
			"id":        log.ID.String(),
			"createdAt": log.CreatedAt.Format(time.RFC3339),
			"level":     log.Level,
			"message":   log.Message,
			"userID":    log.UserID,
			"traceID":   log.TraceID,
		}

		// Parse and add fields if present
		if len(log.Fields) > 0 {
			var fields map[string]interface{}
			if err := json.Unmarshal(log.Fields, &fields); err == nil {
				result["details"] = fields
			}
		}

		utils.JSONResponse(w, http.StatusOK, result)
	}
}

// HandleClearLogs deletes old logs
func HandleClearLogs(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var request struct {
			OlderThan string `json:"olderThan"`
		}

		if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		if request.OlderThan == "" {
			request.OlderThan = "7d"
		}

		deleted, err := logsService.ClearLogs(request.OlderThan)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to clear logs")
			return
		}

		result := map[string]interface{}{
			"message": fmt.Sprintf("Successfully deleted %d log entries", deleted),
			"deleted": deleted,
		}

		utils.JSONResponse(w, http.StatusOK, result)
	}
}

// HandleExportLogs exports logs as CSV
func HandleExportLogs(logsService *services.LogsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse query parameters
		page := 1
		size := 10000 // Max for export
		level := r.URL.Query().Get("level")
		search := r.URL.Query().Get("search")
		timeRange := r.URL.Query().Get("range")
		if timeRange == "" {
			timeRange = "24h"
		}

		// Get logs
		logs, _, err := logsService.GetLogs(page, size, level, search, timeRange)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch logs")
			return
		}

		// Set CSV headers
		w.Header().Set("Content-Type", "text/csv")
		w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=logs_%s.csv", time.Now().Format("20060102_150405")))

		// Create CSV writer
		csvWriter := csv.NewWriter(w)
		defer csvWriter.Flush()

		// Write header
		header := []string{"ID", "Time", "Level", "Message", "User ID", "Trace ID"}
		if err := csvWriter.Write(header); err != nil {
			return
		}

		// Write log entries
		for _, log := range logs {
			record := []string{
				log.ID.String(),
				log.CreatedAt.Format(time.RFC3339),
				log.Level,
				log.Message,
				"",
				"",
			}

			if log.UserID != nil {
				record[4] = *log.UserID
			}
			if log.TraceID != nil {
				record[5] = *log.TraceID
			}

			if err := csvWriter.Write(record); err != nil {
				return
			}
		}
	}
}

// Helper function to get log level from HTTP status code
func getLogLevelFromStatus(status int) string {
	if status >= 200 && status < 300 {
		return "info"
	} else if status >= 300 && status < 400 {
		return "info"
	} else if status >= 400 && status < 500 {
		return "warning"
	} else {
		return "error"
	}
}
