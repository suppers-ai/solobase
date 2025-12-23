//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type logsRepository struct{}

func (r *logsRepository) CreateLog(ctx context.Context, log *logger.LogModel) error {
	return ErrNotImplemented
}

func (r *logsRepository) CreateLogs(ctx context.Context, logs []*logger.LogModel) error {
	return ErrNotImplemented
}

func (r *logsRepository) GetLog(ctx context.Context, id string) (*logger.LogModel, error) {
	return nil, ErrNotImplemented
}

func (r *logsRepository) ListLogs(ctx context.Context, opts repos.LogQueryOptions) (*repos.PaginatedResult[*logger.LogModel], error) {
	return nil, ErrNotImplemented
}

func (r *logsRepository) CountLogs(ctx context.Context) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *logsRepository) CountLogsByLevel(ctx context.Context, level string) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *logsRepository) DeleteLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return ErrNotImplemented
}

func (r *logsRepository) CreateRequestLog(ctx context.Context, log *logger.RequestLogModel) error {
	return ErrNotImplemented
}

func (r *logsRepository) CreateRequestLogs(ctx context.Context, logs []*logger.RequestLogModel) error {
	return ErrNotImplemented
}

func (r *logsRepository) GetRequestLog(ctx context.Context, id string) (*logger.RequestLogModel, error) {
	return nil, ErrNotImplemented
}

func (r *logsRepository) ListRequestLogs(ctx context.Context, opts repos.RequestLogQueryOptions) (*repos.PaginatedResult[*logger.RequestLogModel], error) {
	return nil, ErrNotImplemented
}

func (r *logsRepository) CountRequestLogs(ctx context.Context) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *logsRepository) CountRequestLogsByStatusCode(ctx context.Context, statusCode int) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *logsRepository) DeleteRequestLogsOlderThan(ctx context.Context, cutoff apptime.Time) error {
	return ErrNotImplemented
}

func (r *logsRepository) GetLogStats(ctx context.Context, startTime apptime.Time) (*repos.LogStats, error) {
	return nil, ErrNotImplemented
}

// Ensure logsRepository implements LogsRepository
var _ repos.LogsRepository = (*logsRepository)(nil)
