package files

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	"github.com/wafer-run/wafer-go/services/database"
)

// AccessLogService manages access logging for storage operations
type AccessLogService struct {
	db database.Service
}

// NewAccessLogService creates a new access log service
func NewAccessLogService(db database.Service) *AccessLogService {
	return &AccessLogService{
		db: db,
	}
}

// LogAccess logs an access event for a storage object
func (a *AccessLogService) LogAccess(ctx context.Context, objectID string, action StorageAction, opts LogOptions) error {
	metadata := make(map[string]interface{})
	if opts.ShareID != "" {
		metadata["shareId"] = opts.ShareID
	}
	if opts.Success != nil {
		metadata["success"] = *opts.Success
	}
	if opts.ErrorMsg != "" {
		metadata["errorMsg"] = opts.ErrorMsg
	}
	if opts.BytesSize > 0 {
		metadata["bytesSize"] = opts.BytesSize
	}
	if opts.Duration > 0 {
		metadata["durationMs"] = opts.Duration.Milliseconds()
	}

	metadataJSON, _ := json.Marshal(metadata)

	id := uuid.New().String()

	var userIDPtr, ipAddrPtr, userAgentPtr *string
	if opts.UserID != "" {
		userIDPtr = &opts.UserID
	}
	if opts.IPAddress != "" {
		ipAddrPtr = &opts.IPAddress
	}
	if opts.UserAgent != "" {
		userAgentPtr = &opts.UserAgent
	}

	_, err := a.db.Create(ctx, "ext_cloudstorage_storage_access_logs", map[string]any{
		"id":         id,
		"object_id":  objectID,
		"user_id":    userIDPtr,
		"ip_address": ipAddrPtr,
		"action":     string(action),
		"user_agent": userAgentPtr,
		"metadata":   string(metadataJSON),
	})

	return err
}

// GetAccessLogs retrieves access logs with filters
func (a *AccessLogService) GetAccessLogs(ctx context.Context, filters AccessLogFilters) ([]StorageAccessLog, error) {
	query := `
		SELECT id, object_id, user_id, ip_address, action, user_agent, metadata, created_at
		FROM ext_cloudstorage_storage_access_logs
		WHERE 1=1
	`
	var args []any

	if filters.ObjectID != "" {
		query += " AND object_id = ?"
		args = append(args, filters.ObjectID)
	}
	if filters.UserID != "" {
		query += " AND user_id = ?"
		args = append(args, filters.UserID)
	}
	if filters.Action != "" {
		query += " AND action = ?"
		args = append(args, filters.Action)
	}
	if filters.StartDate != nil {
		query += " AND created_at >= ?"
		args = append(args, apptime.Format(*filters.StartDate))
	}
	if filters.EndDate != nil {
		query += " AND created_at <= ?"
		args = append(args, apptime.Format(*filters.EndDate))
	}

	limit := filters.Limit
	if limit <= 0 {
		limit = 100
	}
	query += " ORDER BY created_at DESC LIMIT ?"
	args = append(args, limit)

	records, err := a.db.QueryRaw(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to get access logs: %w", err)
	}

	var logs []StorageAccessLog
	for _, rec := range records {
		logs = append(logs, recordToAccessLog(rec))
	}

	return logs, nil
}

// GetAccessStats retrieves access statistics for an object or user
func (a *AccessLogService) GetAccessStats(ctx context.Context, filters StatsFilters) (*AccessStats, error) {
	baseQuery := `FROM ext_cloudstorage_storage_access_logs WHERE 1=1`
	var args []any

	if filters.ObjectID != "" {
		baseQuery += " AND object_id = ?"
		args = append(args, filters.ObjectID)
	}
	if filters.UserID != "" {
		baseQuery += " AND user_id = ?"
		args = append(args, filters.UserID)
	}
	if filters.StartDate != nil {
		baseQuery += " AND created_at >= ?"
		args = append(args, apptime.Format(*filters.StartDate))
	}
	if filters.EndDate != nil {
		baseQuery += " AND created_at <= ?"
		args = append(args, apptime.Format(*filters.EndDate))
	}

	var stats AccessStats

	// Get total access count
	countRecords, err := a.db.QueryRaw(ctx, "SELECT COUNT(*) as count "+baseQuery, args...)
	if err != nil {
		return nil, err
	}
	if len(countRecords) > 0 {
		stats.TotalAccess = toInt64Val(countRecords[0].Data["count"])
	}

	// Get action breakdown
	breakdownRecords, err := a.db.QueryRaw(ctx, "SELECT action, COUNT(*) as count "+baseQuery+" GROUP BY action", args...)
	if err != nil {
		return nil, err
	}

	stats.ActionBreakdown = make(map[string]int64)
	for _, rec := range breakdownRecords {
		action := stringVal(rec.Data["action"])
		count := toInt64Val(rec.Data["count"])
		stats.ActionBreakdown[action] = count
	}

	// Get unique users count
	uniqueRecords, err := a.db.QueryRaw(ctx, "SELECT COUNT(DISTINCT user_id) as count "+baseQuery, args...)
	if err != nil {
		return nil, err
	}
	if len(uniqueRecords) > 0 {
		stats.UniqueUsers = toInt64Val(uniqueRecords[0].Data["count"])
	}

	return &stats, nil
}
