-- Storage Share queries

-- name: CreateStorageShare :one
INSERT INTO ext_cloudstorage_storage_shares (
    id, object_id, shared_with_user_id, shared_with_email, permission_level,
    inherit_to_children, share_token, is_public, expires_at, created_by, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetStorageShareByID :one
SELECT * FROM ext_cloudstorage_storage_shares WHERE id = ? LIMIT 1;

-- name: GetStorageShareByToken :one
SELECT * FROM ext_cloudstorage_storage_shares WHERE share_token = ? LIMIT 1;

-- name: ListStorageSharesByObjectID :many
SELECT * FROM ext_cloudstorage_storage_shares WHERE object_id = ? ORDER BY created_at DESC;

-- name: ListStorageSharesByUserID :many
SELECT * FROM ext_cloudstorage_storage_shares WHERE shared_with_user_id = ? ORDER BY created_at DESC;

-- name: ListStorageSharesByCreatedBy :many
SELECT * FROM ext_cloudstorage_storage_shares WHERE created_by = ? ORDER BY created_at DESC;

-- name: GetStorageShareByIDAndCreator :one
SELECT * FROM ext_cloudstorage_storage_shares WHERE id = ? AND created_by = ? LIMIT 1;

-- name: DeleteStorageShareByIDAndCreator :exec
DELETE FROM ext_cloudstorage_storage_shares WHERE id = ? AND created_by = ?;

-- name: ListPublicStorageShares :many
SELECT * FROM ext_cloudstorage_storage_shares WHERE is_public = 1 ORDER BY created_at DESC;

-- name: UpdateStorageShare :exec
UPDATE ext_cloudstorage_storage_shares SET
    permission_level = ?,
    inherit_to_children = ?,
    is_public = ?,
    expires_at = ?,
    updated_at = ?
WHERE id = ?;

-- name: DeleteStorageShare :exec
DELETE FROM ext_cloudstorage_storage_shares WHERE id = ?;

-- name: DeleteStorageSharesByObjectID :exec
DELETE FROM ext_cloudstorage_storage_shares WHERE object_id = ?;

-- name: DeleteExpiredStorageShares :exec
DELETE FROM ext_cloudstorage_storage_shares WHERE expires_at IS NOT NULL AND expires_at < ?;

-- Storage Access Log queries

-- name: CreateStorageAccessLog :one
INSERT INTO ext_cloudstorage_storage_access_logs (
    id, object_id, user_id, ip_address, action, user_agent, metadata, created_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: ListStorageAccessLogsByObjectID :many
SELECT * FROM ext_cloudstorage_storage_access_logs WHERE object_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListStorageAccessLogsByUserID :many
SELECT * FROM ext_cloudstorage_storage_access_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: CountStorageAccessLogsByObjectID :one
SELECT COUNT(*) FROM ext_cloudstorage_storage_access_logs WHERE object_id = ?;

-- name: DeleteStorageAccessLogsOlderThan :exec
DELETE FROM ext_cloudstorage_storage_access_logs WHERE created_at < ?;

-- Storage Quota queries

-- name: CreateStorageQuota :one
INSERT INTO ext_cloudstorage_storage_quotas (
    id, user_id, max_storage_bytes, max_bandwidth_bytes, storage_used,
    bandwidth_used, reset_bandwidth_at, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetStorageQuotaByID :one
SELECT * FROM ext_cloudstorage_storage_quotas WHERE id = ? LIMIT 1;

-- name: GetStorageQuotaByUserID :one
SELECT * FROM ext_cloudstorage_storage_quotas WHERE user_id = ? LIMIT 1;

-- name: UpsertStorageQuota :exec
INSERT INTO ext_cloudstorage_storage_quotas (
    id, user_id, max_storage_bytes, max_bandwidth_bytes, storage_used,
    bandwidth_used, reset_bandwidth_at, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(user_id) DO UPDATE SET
    max_storage_bytes = excluded.max_storage_bytes,
    max_bandwidth_bytes = excluded.max_bandwidth_bytes,
    updated_at = excluded.updated_at;

-- name: UpdateStorageQuotaUsage :exec
UPDATE ext_cloudstorage_storage_quotas SET
    storage_used = ?,
    bandwidth_used = ?,
    updated_at = ?
WHERE user_id = ?;

-- name: IncrementStorageUsed :exec
UPDATE ext_cloudstorage_storage_quotas SET
    storage_used = storage_used + ?,
    updated_at = ?
WHERE user_id = ?;

-- name: DecrementStorageUsed :exec
UPDATE ext_cloudstorage_storage_quotas SET
    storage_used = MAX(0, storage_used - ?),
    updated_at = ?
WHERE user_id = ?;

-- name: IncrementBandwidthUsed :exec
UPDATE ext_cloudstorage_storage_quotas SET
    bandwidth_used = bandwidth_used + ?,
    updated_at = ?
WHERE user_id = ?;

-- name: ResetBandwidthUsage :exec
UPDATE ext_cloudstorage_storage_quotas SET
    bandwidth_used = 0,
    reset_bandwidth_at = ?,
    updated_at = ?
WHERE user_id = ?;

-- name: ResetAllBandwidthUsage :exec
UPDATE ext_cloudstorage_storage_quotas SET
    bandwidth_used = 0,
    reset_bandwidth_at = ?,
    updated_at = ?
WHERE reset_bandwidth_at IS NULL OR reset_bandwidth_at < ?;

-- name: DeleteStorageQuota :exec
DELETE FROM ext_cloudstorage_storage_quotas WHERE id = ?;

-- Role Quota queries

-- name: CreateRoleQuota :one
INSERT INTO ext_cloudstorage_role_quotas (
    id, role_id, role_name, max_storage_bytes, max_bandwidth_bytes,
    max_upload_size, max_files_count, allowed_extensions, blocked_extensions,
    created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetRoleQuotaByID :one
SELECT * FROM ext_cloudstorage_role_quotas WHERE id = ? LIMIT 1;

-- name: GetRoleQuotaByRoleID :one
SELECT * FROM ext_cloudstorage_role_quotas WHERE role_id = ? LIMIT 1;

-- name: GetRoleQuotaByRoleName :one
SELECT * FROM ext_cloudstorage_role_quotas WHERE role_name = ? LIMIT 1;

-- name: ListRoleQuotas :many
SELECT * FROM ext_cloudstorage_role_quotas ORDER BY role_name;

-- name: UpdateRoleQuota :exec
UPDATE ext_cloudstorage_role_quotas SET
    role_name = ?,
    max_storage_bytes = ?,
    max_bandwidth_bytes = ?,
    max_upload_size = ?,
    max_files_count = ?,
    allowed_extensions = ?,
    blocked_extensions = ?,
    updated_at = ?
WHERE id = ?;

-- name: DeleteRoleQuota :exec
DELETE FROM ext_cloudstorage_role_quotas WHERE id = ?;

-- User Quota Override queries

-- name: CreateUserQuotaOverride :one
INSERT INTO ext_cloudstorage_user_quota_overrides (
    id, user_id, max_storage_bytes, max_bandwidth_bytes, max_upload_size,
    max_files_count, allowed_extensions, blocked_extensions, reason, expires_at,
    created_by, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetUserQuotaOverrideByID :one
SELECT * FROM ext_cloudstorage_user_quota_overrides WHERE id = ? LIMIT 1;

-- name: GetUserQuotaOverrideByUserID :one
SELECT * FROM ext_cloudstorage_user_quota_overrides WHERE user_id = ? LIMIT 1;

-- name: GetActiveUserQuotaOverride :one
SELECT * FROM ext_cloudstorage_user_quota_overrides
WHERE user_id = ? AND (expires_at IS NULL OR expires_at > ?)
LIMIT 1;

-- name: ListUserQuotaOverrides :many
SELECT * FROM ext_cloudstorage_user_quota_overrides ORDER BY created_at DESC;

-- name: ListActiveUserQuotaOverrides :many
SELECT * FROM ext_cloudstorage_user_quota_overrides
WHERE expires_at IS NULL OR expires_at > ?
ORDER BY created_at DESC;

-- name: UpdateUserQuotaOverride :exec
UPDATE ext_cloudstorage_user_quota_overrides SET
    max_storage_bytes = ?,
    max_bandwidth_bytes = ?,
    max_upload_size = ?,
    max_files_count = ?,
    allowed_extensions = ?,
    blocked_extensions = ?,
    reason = ?,
    expires_at = ?,
    updated_at = ?
WHERE id = ?;

-- name: DeleteUserQuotaOverride :exec
DELETE FROM ext_cloudstorage_user_quota_overrides WHERE id = ?;

-- name: DeleteExpiredUserQuotaOverrides :exec
DELETE FROM ext_cloudstorage_user_quota_overrides WHERE expires_at IS NOT NULL AND expires_at < ?;
