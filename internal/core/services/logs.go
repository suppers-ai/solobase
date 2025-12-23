package services

import (
	"context"
	"errors"
	"fmt"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type LogsService struct {
	repo repos.LogsRepository
}

func NewLogsService(repo repos.LogsRepository) *LogsService {
	return &LogsService{repo: repo}
}

// GetLogs retrieves logs with filters
func (s *LogsService) GetLogs(page, size int, level, search, timeRange string) ([]logger.LogModel, int64, error) {
	ctx := context.Background()

	// Calculate start time based on range
	startTime := calculateStartTime(timeRange)
	offset := (page - 1) * size

	var levelPtr *string
	if level != "" {
		levelPtr = &level
	}
	var searchPtr *string
	if search != "" {
		searchPtr = &search
	}

	result, err := s.repo.ListLogs(ctx, repos.LogQueryOptions{
		Level:     levelPtr,
		Search:    searchPtr,
		StartTime: startTime,
		Pagination: repos.Pagination{
			Limit:  size,
			Offset: offset,
		},
	})
	if err != nil {
		return nil, 0, err
	}

	// Convert pointers to values
	logs := make([]logger.LogModel, len(result.Items))
	for i, log := range result.Items {
		logs[i] = *log
	}

	return logs, result.Total, nil
}

// GetRequestLogs retrieves request logs with filters
func (s *LogsService) GetRequestLogs(page, size int, method, path, timeRange string, minStatus, maxStatus int) ([]logger.RequestLogModel, int64, error) {
	ctx := context.Background()

	// Calculate start time based on range
	startTime := calculateStartTime(timeRange)
	offset := (page - 1) * size

	var methodPtr, pathPtr *string
	if method != "" {
		methodPtr = &method
	}
	if path != "" {
		pathPtr = &path
	}

	var minStatusPtr, maxStatusPtr *int
	if minStatus > 0 {
		minStatusPtr = &minStatus
	}
	if maxStatus > 0 {
		maxStatusPtr = &maxStatus
	}

	result, err := s.repo.ListRequestLogs(ctx, repos.RequestLogQueryOptions{
		Method:    methodPtr,
		Path:      pathPtr,
		MinStatus: minStatusPtr,
		MaxStatus: maxStatusPtr,
		StartTime: startTime,
		Pagination: repos.Pagination{
			Limit:  size,
			Offset: offset,
		},
	})
	if err != nil {
		return nil, 0, err
	}

	// Convert pointers to values
	logs := make([]logger.RequestLogModel, len(result.Items))
	for i, log := range result.Items {
		logs[i] = *log
	}

	return logs, result.Total, nil
}

// GetLogStats returns aggregated log statistics for charts
func (s *LogsService) GetLogStats(timeRange string) (map[string]interface{}, error) {
	ctx := context.Background()
	startTime := calculateStartTime(timeRange)
	now := apptime.NowTime()

	stats, err := s.repo.GetLogStats(ctx, startTime)
	if err != nil {
		return nil, err
	}

	// Process results into chart data
	labels := []string{}
	successData := []int{}
	warningData := []int{}
	errorData := []int{}

	// Generate hour labels based on range
	hoursToShow := 24
	if timeRange == "1h" {
		hoursToShow = 1
	}

	for i := 0; i < hoursToShow; i++ {
		hour := (now.Hour() - hoursToShow + i + 1) % 24
		if hour < 0 {
			hour += 24
		}
		hourStr := fmt.Sprintf("%02d", hour)
		labels = append(labels, hourStr+":00")

		// Initialize counters
		success := 0
		warning := 0
		errorCount := 0

		// Count logs for this hour from stats
		for level, hourCounts := range stats.ByLevelAndHour {
			if count, ok := hourCounts[hourStr]; ok {
				switch level {
				case "info", "debug":
					success += int(count)
				case "warn", "warning":
					warning += int(count)
				case "error", "fatal", "panic":
					errorCount += int(count)
				}
			}
		}

		successData = append(successData, success)
		warningData = append(warningData, warning)
		errorData = append(errorData, errorCount)
	}

	return map[string]interface{}{
		"labels":  labels,
		"success": successData,
		"warning": warningData,
		"error":   errorData,
	}, nil
}

// ClearLogs deletes logs older than specified duration
func (s *LogsService) ClearLogs(olderThan string) (int64, error) {
	ctx := context.Background()
	now := apptime.NowTime()

	var cutoffTime apptime.Time
	switch olderThan {
	case "1h":
		cutoffTime = now.Add(-1 * apptime.Hour)
	case "24h":
		cutoffTime = now.Add(-24 * apptime.Hour)
	case "7d":
		cutoffTime = now.Add(-7 * 24 * apptime.Hour)
	case "30d":
		cutoffTime = now.Add(-30 * 24 * apptime.Hour)
	case "all":
		cutoffTime = now.Add(1 * apptime.Hour) // Future time to delete all
	default:
		return 0, fmt.Errorf("invalid duration: %s", olderThan)
	}

	// Delete logs
	if err := s.repo.DeleteLogsOlderThan(ctx, cutoffTime); err != nil {
		return 0, err
	}

	// Delete request logs
	if err := s.repo.DeleteRequestLogsOlderThan(ctx, cutoffTime); err != nil {
		return 0, err
	}

	return 0, nil
}

// GetLogByID retrieves a single log by ID
func (s *LogsService) GetLogByID(id string) (*logger.LogModel, error) {
	ctx := context.Background()
	log, err := s.repo.GetLog(ctx, id)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, errors.New("log not found")
		}
		return nil, err
	}
	return log, nil
}

// GetRequestLogByID retrieves a single request log by ID
func (s *LogsService) GetRequestLogByID(id string) (*logger.RequestLogModel, error) {
	ctx := context.Background()
	log, err := s.repo.GetRequestLog(ctx, id)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, errors.New("request log not found")
		}
		return nil, err
	}
	return log, nil
}

// Helper function to calculate start time based on time range
func calculateStartTime(timeRange string) apptime.Time {
	now := apptime.NowTime()
	switch timeRange {
	case "1h":
		return now.Add(-1 * apptime.Hour)
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

// CleanupOldLogs deletes logs older than the specified retention period
// Default retention is 7 days
func (s *LogsService) CleanupOldLogs(retentionDays int) (int64, error) {
	if retentionDays <= 0 {
		retentionDays = 7
	}

	ctx := context.Background()
	cutoffTime := apptime.NowTime().Add(-apptime.Duration(retentionDays) * 24 * apptime.Hour)

	// Delete old logs
	if err := s.repo.DeleteLogsOlderThan(ctx, cutoffTime); err != nil {
		return 0, err
	}

	// Delete old request logs
	if err := s.repo.DeleteRequestLogsOlderThan(ctx, cutoffTime); err != nil {
		return 0, err
	}

	return 0, nil
}
