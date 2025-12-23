package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
)

// LogQueryOptions configures log queries
type LogQueryOptions struct {
	Level     *string
	UserID    *string
	TraceID   *string
	Search    *string
	StartTime apptime.Time
	Pagination
}

// RequestLogQueryOptions configures request log queries
type RequestLogQueryOptions struct {
	Method    *string
	Path      *string
	UserID    *string
	MinStatus *int
	MaxStatus *int
	StartTime apptime.Time
	Pagination
}

// LogStats contains aggregated log statistics
type LogStats struct {
	ByLevelAndHour map[string]map[string]int64 // level -> hour -> count
}

// LogsRepository provides log operations
type LogsRepository interface {
	// System logs - write
	CreateLog(ctx context.Context, log *logger.LogModel) error
	CreateLogs(ctx context.Context, logs []*logger.LogModel) error

	// System logs - read
	GetLog(ctx context.Context, id string) (*logger.LogModel, error)
	ListLogs(ctx context.Context, opts LogQueryOptions) (*PaginatedResult[*logger.LogModel], error)
	CountLogs(ctx context.Context) (int64, error)
	CountLogsByLevel(ctx context.Context, level string) (int64, error)

	// System logs - delete
	DeleteLogsOlderThan(ctx context.Context, cutoff apptime.Time) error

	// Request logs - write
	CreateRequestLog(ctx context.Context, log *logger.RequestLogModel) error
	CreateRequestLogs(ctx context.Context, logs []*logger.RequestLogModel) error

	// Request logs - read
	GetRequestLog(ctx context.Context, id string) (*logger.RequestLogModel, error)
	ListRequestLogs(ctx context.Context, opts RequestLogQueryOptions) (*PaginatedResult[*logger.RequestLogModel], error)
	CountRequestLogs(ctx context.Context) (int64, error)
	CountRequestLogsByStatusCode(ctx context.Context, statusCode int) (int64, error)

	// Request logs - delete
	DeleteRequestLogsOlderThan(ctx context.Context, cutoff apptime.Time) error

	// Stats
	GetLogStats(ctx context.Context, startTime apptime.Time) (*LogStats, error)
}
