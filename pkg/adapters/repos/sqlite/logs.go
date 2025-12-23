//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type logsRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewLogsRepository creates a new SQLite logs repository
func NewLogsRepository(sqlDB *sql.DB, queries *db.Queries) repos.LogsRepository {
	return &logsRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *logsRepository) CreateLog(ctx context.Context, log *logger.LogModel) error {
	log.PrepareForCreate()
	_, err := r.queries.CreateLog(ctx, db.CreateLogParams{
		ID:        log.ID.String(),
		Level:     log.Level,
		Message:   log.Message,
		Fields:    log.Fields,
		UserID:    log.UserID,
		TraceID:   log.TraceID,
		CreatedAt: apptime.Format(log.CreatedAt),
	})
	return err
}

func (r *logsRepository) CreateLogs(ctx context.Context, logs []*logger.LogModel) error {
	// Batch insert using raw SQL for efficiency
	if len(logs) == 0 {
		return nil
	}

	valueStrings := make([]string, 0, len(logs))
	valueArgs := make([]interface{}, 0, len(logs)*7)

	for _, log := range logs {
		log.PrepareForCreate()
		valueStrings = append(valueStrings, "(?, ?, ?, ?, ?, ?, ?)")
		valueArgs = append(valueArgs,
			log.ID.String(),
			log.Level,
			log.Message,
			log.Fields,
			log.UserID,
			log.TraceID,
			apptime.Format(log.CreatedAt),
		)
	}

	query := fmt.Sprintf(
		"INSERT INTO sys_logs (id, level, message, fields, user_id, trace_id, created_at) VALUES %s",
		strings.Join(valueStrings, ","),
	)

	_, err := r.sqlDB.ExecContext(ctx, query, valueArgs...)
	return err
}

func (r *logsRepository) GetLog(ctx context.Context, id string) (*logger.LogModel, error) {
	row := r.sqlDB.QueryRowContext(ctx,
		"SELECT id, level, message, fields, user_id, trace_id, created_at FROM sys_logs WHERE id = ?",
		id,
	)

	var log logger.LogModel
	var idStr, createdAt string
	err := row.Scan(&idStr, &log.Level, &log.Message, &log.Fields, &log.UserID, &log.TraceID, &createdAt)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	log.ID = uuid.MustParse(idStr)
	log.CreatedAt = apptime.MustParse(createdAt)
	return &log, nil
}

func (r *logsRepository) ListLogs(ctx context.Context, opts repos.LogQueryOptions) (*repos.PaginatedResult[*logger.LogModel], error) {
	// Build dynamic query
	whereClause := "WHERE created_at >= ?"
	args := []interface{}{apptime.Format(opts.StartTime)}

	if opts.Level != nil && *opts.Level != "" {
		whereClause += " AND level = ?"
		args = append(args, *opts.Level)
	}
	if opts.UserID != nil && *opts.UserID != "" {
		whereClause += " AND user_id = ?"
		args = append(args, *opts.UserID)
	}
	if opts.TraceID != nil && *opts.TraceID != "" {
		whereClause += " AND trace_id = ?"
		args = append(args, *opts.TraceID)
	}
	if opts.Search != nil && *opts.Search != "" {
		whereClause += " AND message LIKE ?"
		args = append(args, "%"+*opts.Search+"%")
	}

	// Count total
	countQuery := "SELECT COUNT(*) FROM sys_logs " + whereClause
	var total int64
	if err := r.sqlDB.QueryRowContext(ctx, countQuery, args...).Scan(&total); err != nil {
		return nil, err
	}

	// Get paginated results
	query := fmt.Sprintf(
		"SELECT id, level, message, fields, user_id, trace_id, created_at FROM sys_logs %s ORDER BY created_at DESC LIMIT ? OFFSET ?",
		whereClause,
	)
	args = append(args, opts.Limit, opts.Offset)

	rows, err := r.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var logs []*logger.LogModel
	for rows.Next() {
		var log logger.LogModel
		var idStr, createdAt string
		if err := rows.Scan(&idStr, &log.Level, &log.Message, &log.Fields, &log.UserID, &log.TraceID, &createdAt); err != nil {
			return nil, err
		}
		log.ID = uuid.MustParse(idStr)
		log.CreatedAt = apptime.MustParse(createdAt)
		logs = append(logs, &log)
	}

	return &repos.PaginatedResult[*logger.LogModel]{
		Items: logs,
		Total: total,
	}, nil
}

func (r *logsRepository) CountLogs(ctx context.Context) (int64, error) {
	var count int64
	err := r.sqlDB.QueryRowContext(ctx, "SELECT COUNT(*) FROM sys_logs").Scan(&count)
	return count, err
}

func (r *logsRepository) CountLogsByLevel(ctx context.Context, level string) (int64, error) {
	var count int64
	err := r.sqlDB.QueryRowContext(ctx, "SELECT COUNT(*) FROM sys_logs WHERE level = ?", level).Scan(&count)
	return count, err
}

func (r *logsRepository) DeleteLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return r.queries.DeleteLogsOlderThan(ctx, apptime.Format(cutoff))
}

func (r *logsRepository) CreateRequestLog(ctx context.Context, log *logger.RequestLogModel) error {
	log.PrepareForCreate()
	_, err := r.queries.CreateRequestLog(ctx, db.CreateRequestLogParams{
		ID:           log.ID.String(),
		Level:        log.Level,
		Method:       log.Method,
		Path:         log.Path,
		Query:        log.Query,
		StatusCode:   int64(log.StatusCode),
		ExecTimeMs:   log.ExecTimeMs,
		UserIp:       log.UserIP,
		UserAgent:    log.UserAgent,
		UserID:       log.UserID,
		TraceID:      log.TraceID,
		Error:        log.Error,
		RequestBody:  log.RequestBody,
		ResponseBody: log.ResponseBody,
		Headers:      log.Headers,
		CreatedAt:    apptime.Format(log.CreatedAt),
	})
	return err
}

func (r *logsRepository) CreateRequestLogs(ctx context.Context, logs []*logger.RequestLogModel) error {
	if len(logs) == 0 {
		return nil
	}

	valueStrings := make([]string, 0, len(logs))
	valueArgs := make([]interface{}, 0, len(logs)*16)

	for _, log := range logs {
		log.PrepareForCreate()
		valueStrings = append(valueStrings, "(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
		valueArgs = append(valueArgs,
			log.ID.String(),
			log.Level,
			log.Method,
			log.Path,
			log.Query,
			log.StatusCode,
			log.ExecTimeMs,
			log.UserIP,
			log.UserAgent,
			log.UserID,
			log.TraceID,
			log.Error,
			log.RequestBody,
			log.ResponseBody,
			log.Headers,
			apptime.Format(log.CreatedAt),
		)
	}

	query := fmt.Sprintf(
		"INSERT INTO sys_request_logs (id, level, method, path, query, status_code, exec_time_ms, user_ip, user_agent, user_id, trace_id, error, request_body, response_body, headers, created_at) VALUES %s",
		strings.Join(valueStrings, ","),
	)

	_, err := r.sqlDB.ExecContext(ctx, query, valueArgs...)
	return err
}

func (r *logsRepository) GetRequestLog(ctx context.Context, id string) (*logger.RequestLogModel, error) {
	row := r.sqlDB.QueryRowContext(ctx,
		"SELECT id, level, method, path, query, status_code, exec_time_ms, user_ip, user_agent, user_id, trace_id, error, request_body, response_body, headers, created_at FROM sys_request_logs WHERE id = ?",
		id,
	)

	var log logger.RequestLogModel
	var idStr, createdAt string
	err := row.Scan(&idStr, &log.Level, &log.Method, &log.Path, &log.Query, &log.StatusCode, &log.ExecTimeMs, &log.UserIP, &log.UserAgent, &log.UserID, &log.TraceID, &log.Error, &log.RequestBody, &log.ResponseBody, &log.Headers, &createdAt)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	log.ID = uuid.MustParse(idStr)
	log.CreatedAt = apptime.MustParse(createdAt)
	return &log, nil
}

func (r *logsRepository) ListRequestLogs(ctx context.Context, opts repos.RequestLogQueryOptions) (*repos.PaginatedResult[*logger.RequestLogModel], error) {
	whereClause := "WHERE created_at >= ?"
	args := []interface{}{apptime.Format(opts.StartTime)}

	if opts.Method != nil && *opts.Method != "" {
		whereClause += " AND method = ?"
		args = append(args, *opts.Method)
	}
	if opts.Path != nil && *opts.Path != "" {
		whereClause += " AND path LIKE ?"
		args = append(args, "%"+*opts.Path+"%")
	}
	if opts.UserID != nil && *opts.UserID != "" {
		whereClause += " AND user_id = ?"
		args = append(args, *opts.UserID)
	}
	if opts.MinStatus != nil {
		whereClause += " AND status_code >= ?"
		args = append(args, *opts.MinStatus)
	}
	if opts.MaxStatus != nil {
		whereClause += " AND status_code <= ?"
		args = append(args, *opts.MaxStatus)
	}

	countQuery := "SELECT COUNT(*) FROM sys_request_logs " + whereClause
	var total int64
	if err := r.sqlDB.QueryRowContext(ctx, countQuery, args...).Scan(&total); err != nil {
		return nil, err
	}

	query := fmt.Sprintf(
		"SELECT id, level, method, path, query, status_code, exec_time_ms, user_ip, user_agent, user_id, trace_id, error, request_body, response_body, headers, created_at FROM sys_request_logs %s ORDER BY created_at DESC LIMIT ? OFFSET ?",
		whereClause,
	)
	args = append(args, opts.Limit, opts.Offset)

	rows, err := r.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var logs []*logger.RequestLogModel
	for rows.Next() {
		var log logger.RequestLogModel
		var idStr, createdAt string
		if err := rows.Scan(&idStr, &log.Level, &log.Method, &log.Path, &log.Query, &log.StatusCode, &log.ExecTimeMs, &log.UserIP, &log.UserAgent, &log.UserID, &log.TraceID, &log.Error, &log.RequestBody, &log.ResponseBody, &log.Headers, &createdAt); err != nil {
			return nil, err
		}
		log.ID = uuid.MustParse(idStr)
		log.CreatedAt = apptime.MustParse(createdAt)
		logs = append(logs, &log)
	}

	return &repos.PaginatedResult[*logger.RequestLogModel]{
		Items: logs,
		Total: total,
	}, nil
}

func (r *logsRepository) CountRequestLogs(ctx context.Context) (int64, error) {
	var count int64
	err := r.sqlDB.QueryRowContext(ctx, "SELECT COUNT(*) FROM sys_request_logs").Scan(&count)
	return count, err
}

func (r *logsRepository) CountRequestLogsByStatusCode(ctx context.Context, statusCode int) (int64, error) {
	var count int64
	err := r.sqlDB.QueryRowContext(ctx, "SELECT COUNT(*) FROM sys_request_logs WHERE status_code = ?", statusCode).Scan(&count)
	return count, err
}

func (r *logsRepository) DeleteRequestLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return r.queries.DeleteRequestLogsOlderThan(ctx, apptime.Format(cutoff))
}

func (r *logsRepository) GetLogStats(ctx context.Context, startTime apptime.Time) (*repos.LogStats, error) {
	query := `
		SELECT level, strftime('%H', created_at) as hour, COUNT(*) as count
		FROM sys_logs
		WHERE created_at >= ?
		GROUP BY level, strftime('%H', created_at)
		ORDER BY level, hour
	`

	rows, err := r.sqlDB.QueryContext(ctx, query, apptime.Format(startTime))
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	stats := &repos.LogStats{
		ByLevelAndHour: make(map[string]map[string]int64),
	}

	for rows.Next() {
		var level, hour string
		var count int64
		if err := rows.Scan(&level, &hour, &count); err != nil {
			return nil, err
		}
		if stats.ByLevelAndHour[level] == nil {
			stats.ByLevelAndHour[level] = make(map[string]int64)
		}
		stats.ByLevelAndHour[level][hour] = count
	}

	return stats, nil
}

// Ensure logsRepository implements LogsRepository
var _ repos.LogsRepository = (*logsRepository)(nil)
