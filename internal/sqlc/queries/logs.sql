-- System Log queries

-- name: CreateLog :one
INSERT INTO sys_logs (id, level, message, fields, user_id, trace_id, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: ListLogs :many
SELECT * FROM sys_logs ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListLogsByLevel :many
SELECT * FROM sys_logs WHERE level = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListLogsByUserID :many
SELECT * FROM sys_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListLogsByTraceID :many
SELECT * FROM sys_logs WHERE trace_id = ? ORDER BY created_at DESC;

-- name: CountLogs :one
SELECT COUNT(*) FROM sys_logs;

-- name: CountLogsByLevel :one
SELECT COUNT(*) FROM sys_logs WHERE level = ?;

-- name: DeleteLogsOlderThan :exec
DELETE FROM sys_logs WHERE created_at < ?;

-- Request Log queries

-- name: CreateRequestLog :one
INSERT INTO sys_request_logs (
    id, level, method, path, query, status_code, exec_time_ms,
    user_ip, user_agent, user_id, trace_id, error, request_body,
    response_body, headers, created_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: ListRequestLogs :many
SELECT * FROM sys_request_logs ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListRequestLogsByMethod :many
SELECT * FROM sys_request_logs WHERE method = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListRequestLogsByPath :many
SELECT * FROM sys_request_logs WHERE path LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListRequestLogsByStatusCode :many
SELECT * FROM sys_request_logs WHERE status_code = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListRequestLogsByUserID :many
SELECT * FROM sys_request_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListErrorRequestLogs :many
SELECT * FROM sys_request_logs WHERE status_code >= 400 ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: CountRequestLogs :one
SELECT COUNT(*) FROM sys_request_logs;

-- name: CountRequestLogsByStatusCode :one
SELECT COUNT(*) FROM sys_request_logs WHERE status_code = ?;

-- name: DeleteRequestLogsOlderThan :exec
DELETE FROM sys_request_logs WHERE created_at < ?;
