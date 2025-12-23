-- Settings queries

-- name: CreateSetting :one
INSERT INTO sys_settings (id, key, value, type, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetSettingByID :one
SELECT * FROM sys_settings WHERE id = ? AND deleted_at IS NULL LIMIT 1;

-- name: GetSettingByKey :one
SELECT * FROM sys_settings WHERE key = ? AND deleted_at IS NULL LIMIT 1;

-- name: ListSettings :many
SELECT * FROM sys_settings WHERE deleted_at IS NULL ORDER BY key;

-- name: ListSettingsByType :many
SELECT * FROM sys_settings WHERE type = ? AND deleted_at IS NULL ORDER BY key;

-- name: UpdateSetting :exec
UPDATE sys_settings SET value = ?, type = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL;

-- name: UpdateSettingByKey :exec
UPDATE sys_settings SET value = ?, type = ?, updated_at = ? WHERE key = ? AND deleted_at IS NULL;

-- name: UpsertSetting :exec
INSERT INTO sys_settings (id, key, value, type, created_at, updated_at)
VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT(key) DO UPDATE SET value = excluded.value, type = excluded.type, updated_at = excluded.updated_at;

-- name: SoftDeleteSetting :exec
UPDATE sys_settings SET deleted_at = ?, updated_at = ? WHERE id = ?;

-- name: SoftDeleteSettingByKey :exec
UPDATE sys_settings SET deleted_at = ?, updated_at = ? WHERE key = ?;

-- name: HardDeleteSetting :exec
DELETE FROM sys_settings WHERE id = ?;
