-- Role queries

-- name: CreateRole :exec
INSERT INTO iam_roles (id, name, display_name, description, type, metadata)
VALUES (?, ?, ?, ?, ?, ?);

-- name: GetRoleByID :one
SELECT * FROM iam_roles WHERE id = ? LIMIT 1;

-- name: GetRoleByName :one
SELECT * FROM iam_roles WHERE name = ? LIMIT 1;

-- name: ListRoles :many
SELECT * FROM iam_roles ORDER BY name;

-- name: ListRolesByType :many
SELECT * FROM iam_roles WHERE type = ? ORDER BY name;

-- name: UpdateRole :exec
UPDATE iam_roles SET
    name = ?,
    display_name = ?,
    description = ?,
    type = ?,
    metadata = ?,
    updated_at = ?
WHERE id = ?;

-- name: DeleteRole :exec
DELETE FROM iam_roles WHERE id = ?;

-- User Role queries

-- name: CreateUserRole :exec
INSERT INTO iam_user_roles (id, user_id, role_id, granted_by, expires_at)
VALUES (?, ?, ?, ?, ?);

-- name: GetUserRole :one
SELECT * FROM iam_user_roles WHERE user_id = ? AND role_id = ? LIMIT 1;

-- name: ListUserRolesByUserID :many
SELECT ur.*, r.name as role_name, r.display_name as role_display_name
FROM iam_user_roles ur
JOIN iam_roles r ON ur.role_id = r.id
WHERE ur.user_id = ?
ORDER BY ur.granted_at DESC;

-- name: ListUserRolesByRoleID :many
SELECT * FROM iam_user_roles WHERE role_id = ? ORDER BY granted_at DESC;

-- name: ListUserIDsWithRole :many
SELECT user_id FROM iam_user_roles WHERE role_id = ?;

-- name: DeleteUserRole :exec
DELETE FROM iam_user_roles WHERE user_id = ? AND role_id = ?;

-- name: DeleteUserRolesByUserID :exec
DELETE FROM iam_user_roles WHERE user_id = ?;

-- name: DeleteUserRolesByRoleID :exec
DELETE FROM iam_user_roles WHERE role_id = ?;

-- name: DeleteExpiredUserRoles :exec
DELETE FROM iam_user_roles WHERE expires_at IS NOT NULL AND expires_at < ?;

-- Policy queries

-- name: CreatePolicy :exec
INSERT INTO iam_policies (id, ptype, v0, v1, v2, v3, v4, v5)
VALUES (?, ?, ?, ?, ?, ?, ?, ?);

-- name: GetPolicyByID :one
SELECT * FROM iam_policies WHERE id = ? LIMIT 1;

-- name: ListPolicies :many
SELECT * FROM iam_policies ORDER BY created_at;

-- name: ListPoliciesByType :many
SELECT * FROM iam_policies WHERE ptype = ? ORDER BY created_at;

-- name: ListPoliciesBySubject :many
SELECT * FROM iam_policies WHERE ptype = 'p' AND v0 = ? ORDER BY created_at;

-- name: ListGroupingPolicies :many
SELECT * FROM iam_policies WHERE ptype = 'g' ORDER BY created_at;

-- name: ListGroupingPoliciesByUser :many
SELECT * FROM iam_policies WHERE ptype = 'g' AND v0 = ? ORDER BY created_at;

-- name: GetPolicy :one
SELECT * FROM iam_policies WHERE ptype = ? AND v0 = ? AND v1 = ? AND v2 = ? AND v3 = ? LIMIT 1;

-- name: DeletePolicy :exec
DELETE FROM iam_policies WHERE id = ?;

-- name: DeletePolicyByValues :exec
DELETE FROM iam_policies WHERE ptype = ? AND v0 = ? AND v1 = ? AND v2 = ? AND v3 = ?;

-- name: DeletePoliciesBySubject :exec
DELETE FROM iam_policies WHERE ptype = 'p' AND v0 = ?;

-- name: DeleteGroupingPoliciesByUser :exec
DELETE FROM iam_policies WHERE ptype = 'g' AND v0 = ?;

-- Audit Log queries

-- name: CreateAuditLog :exec
INSERT INTO iam_audit_logs (id, user_id, action, resource, result, reason, ip_address, user_agent, metadata)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);

-- name: ListAuditLogs :many
SELECT * FROM iam_audit_logs ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListAuditLogsByUserID :many
SELECT * FROM iam_audit_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListAuditLogsByAction :many
SELECT * FROM iam_audit_logs WHERE action = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: CountAuditLogs :one
SELECT COUNT(*) FROM iam_audit_logs;

-- name: CountAuditLogsByUserID :one
SELECT COUNT(*) FROM iam_audit_logs WHERE user_id = ?;

-- name: DeleteAuditLogsOlderThan :exec
DELETE FROM iam_audit_logs WHERE created_at < ?;
