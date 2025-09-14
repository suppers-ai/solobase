package services

import (
	"fmt"
	"time"

	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/database"
)

type LogsService struct {
	db *database.DB
}

func NewLogsService(db *database.DB) *LogsService {
	return &LogsService{db: db}
}

// GetLogs retrieves logs with filters
func (s *LogsService) GetLogs(page, size int, level, search, timeRange string) ([]logger.LogModel, int64, error) {
	var logs []logger.LogModel
	var total int64

	query := s.db.Model(&logger.LogModel{})

	// Apply filters
	if level != "" {
		query = query.Where("level = ?", level)
	}

	if search != "" {
		query = query.Where("message LIKE ?", "%"+search+"%")
	}

	// Apply time range filter
	if timeRange != "" {
		var startTime time.Time
		now := time.Now()

		switch timeRange {
		case "1h":
			startTime = now.Add(-1 * time.Hour)
		case "24h":
			startTime = now.Add(-24 * time.Hour)
		case "7d":
			startTime = now.Add(-7 * 24 * time.Hour)
		case "30d":
			startTime = now.Add(-30 * 24 * time.Hour)
		default:
			startTime = now.Add(-24 * time.Hour)
		}

		query = query.Where("created_at >= ?", startTime)
	}

	// Get total count
	query.Count(&total)

	// Apply pagination
	offset := (page - 1) * size
	query = query.Order("created_at DESC").Limit(size).Offset(offset)

	// Execute query
	if err := query.Find(&logs).Error; err != nil {
		return nil, 0, err
	}

	return logs, total, nil
}

// GetRequestLogs retrieves request logs with filters
func (s *LogsService) GetRequestLogs(page, size int, method, path, timeRange string, minStatus, maxStatus int) ([]logger.RequestLogModel, int64, error) {
	var logs []logger.RequestLogModel
	var total int64

	query := s.db.Model(&logger.RequestLogModel{})

	// Apply filters
	if method != "" {
		query = query.Where("method = ?", method)
	}

	if path != "" {
		query = query.Where("path LIKE ?", "%"+path+"%")
	}

	if minStatus > 0 && maxStatus > 0 {
		query = query.Where("status_code >= ? AND status_code <= ?", minStatus, maxStatus)
	}

	// Apply time range filter
	if timeRange != "" {
		var startTime time.Time
		now := time.Now()

		switch timeRange {
		case "1h":
			startTime = now.Add(-1 * time.Hour)
		case "24h":
			startTime = now.Add(-24 * time.Hour)
		case "7d":
			startTime = now.Add(-7 * 24 * time.Hour)
		case "30d":
			startTime = now.Add(-30 * 24 * time.Hour)
		default:
			startTime = now.Add(-24 * time.Hour)
		}

		query = query.Where("created_at >= ?", startTime)
	}

	// Get total count
	query.Count(&total)

	// Apply pagination
	offset := (page - 1) * size
	query = query.Order("created_at DESC").Limit(size).Offset(offset)

	// Execute query
	if err := query.Find(&logs).Error; err != nil {
		return nil, 0, err
	}

	return logs, total, nil
}

// GetLogStats returns aggregated log statistics for charts
func (s *LogsService) GetLogStats(timeRange string) (map[string]interface{}, error) {
	var startTime time.Time
	now := time.Now()

	switch timeRange {
	case "1h":
		startTime = now.Add(-1 * time.Hour)
	case "24h":
		startTime = now.Add(-24 * time.Hour)
	case "7d":
		startTime = now.Add(-7 * 24 * time.Hour)
	case "30d":
		startTime = now.Add(-30 * 24 * time.Hour)
	default:
		startTime = now.Add(-24 * time.Hour)
	}

	// Get counts by level
	var results []struct {
		Level string
		Count int
		Hour  int
	}

	query := `
		SELECT 
			level,
			COUNT(*) as count,
			EXTRACT(HOUR FROM created_at) as hour
		FROM logs
		WHERE created_at >= ?
		GROUP BY level, EXTRACT(HOUR FROM created_at)
		ORDER BY hour
	`

	if s.db.Config.Type == "sqlite" {
		query = `
			SELECT 
				level,
				COUNT(*) as count,
				strftime('%H', created_at) as hour
			FROM logs
			WHERE created_at >= ?
			GROUP BY level, strftime('%H', created_at)
			ORDER BY hour
		`
	}

	s.db.Raw(query, startTime).Scan(&results)

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
		labels = append(labels, fmt.Sprintf("%02d:00", hour))

		// Initialize counters
		success := 0
		warning := 0
		error := 0

		// Count logs for this hour
		for _, r := range results {
			hourInt := 0
			fmt.Sscanf(fmt.Sprintf("%v", r.Hour), "%d", &hourInt)
			if hourInt == hour {
				switch r.Level {
				case "info", "debug":
					success += r.Count
				case "warn", "warning":
					warning += r.Count
				case "error", "fatal", "panic":
					error += r.Count
				}
			}
		}

		successData = append(successData, success)
		warningData = append(warningData, warning)
		errorData = append(errorData, error)
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
	var cutoffTime time.Time
	now := time.Now()

	switch olderThan {
	case "1h":
		cutoffTime = now.Add(-1 * time.Hour)
	case "24h":
		cutoffTime = now.Add(-24 * time.Hour)
	case "7d":
		cutoffTime = now.Add(-7 * 24 * time.Hour)
	case "30d":
		cutoffTime = now.Add(-30 * 24 * time.Hour)
	case "all":
		cutoffTime = now.Add(1 * time.Hour) // Future time to delete all
	default:
		return 0, fmt.Errorf("invalid duration: %s", olderThan)
	}

	// Delete logs
	result := s.db.Where("created_at < ?", cutoffTime).Delete(&logger.LogModel{})
	if result.Error != nil {
		return 0, result.Error
	}

	// Also delete request logs
	result2 := s.db.Where("created_at < ?", cutoffTime).Delete(&logger.RequestLogModel{})
	if result2.Error != nil {
		return result.RowsAffected, result2.Error
	}

	return result.RowsAffected + result2.RowsAffected, nil
}

// GetLogByID retrieves a single log by ID
func (s *LogsService) GetLogByID(id string) (*logger.LogModel, error) {
	var log logger.LogModel
	if err := s.db.Where("id = ?", id).First(&log).Error; err != nil {
		return nil, err
	}
	return &log, nil
}

// GetRequestLogByID retrieves a single request log by ID
func (s *LogsService) GetRequestLogByID(id string) (*logger.RequestLogModel, error) {
	var log logger.RequestLogModel
	if err := s.db.Where("id = ?", id).First(&log).Error; err != nil {
		return nil, err
	}
	return &log, nil
}
