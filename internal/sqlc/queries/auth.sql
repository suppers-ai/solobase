-- name: CreateUser :one
INSERT INTO auth_users (
    id, email, password, username, confirmed, first_name, last_name,
    display_name, phone, location, metadata, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetUserByID :one
SELECT * FROM auth_users WHERE id = ? AND deleted_at IS NULL LIMIT 1;

-- name: GetUserByEmail :one
SELECT * FROM auth_users WHERE email = ? AND deleted_at IS NULL LIMIT 1;

-- name: GetUserByUsername :one
SELECT * FROM auth_users WHERE username = ? AND deleted_at IS NULL LIMIT 1;

-- name: GetUserByConfirmSelector :one
SELECT * FROM auth_users WHERE confirm_selector = ? AND deleted_at IS NULL LIMIT 1;

-- name: GetUserByRecoverSelector :one
SELECT * FROM auth_users WHERE recover_selector = ? AND deleted_at IS NULL LIMIT 1;

-- name: ListUsers :many
SELECT * FROM auth_users WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: CountUsers :one
SELECT COUNT(*) FROM auth_users WHERE deleted_at IS NULL;

-- name: UpdateUser :exec
UPDATE auth_users SET
    email = ?,
    password = ?,
    username = ?,
    confirmed = ?,
    first_name = ?,
    last_name = ?,
    display_name = ?,
    phone = ?,
    location = ?,
    metadata = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: UpdateUserPassword :exec
UPDATE auth_users SET password = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL;

-- name: UpdateUserConfirmation :exec
UPDATE auth_users SET
    confirmed = ?,
    confirm_token = NULL,
    confirm_selector = NULL,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: SetUserConfirmToken :exec
UPDATE auth_users SET
    confirm_token = ?,
    confirm_selector = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: SetUserRecoverToken :exec
UPDATE auth_users SET
    recover_token = ?,
    recover_selector = ?,
    recover_token_exp = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: ClearUserRecoverToken :exec
UPDATE auth_users SET
    recover_token = NULL,
    recover_selector = NULL,
    recover_token_exp = NULL,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: UpdateUserLoginAttempt :exec
UPDATE auth_users SET
    attempt_count = ?,
    last_attempt = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: UpdateUserLastLogin :exec
UPDATE auth_users SET
    last_login = ?,
    attempt_count = 0,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: SetUserTOTP :exec
UPDATE auth_users SET
    totp_secret = ?,
    totp_secret_backup = ?,
    recovery_codes = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: ClearUserTOTP :exec
UPDATE auth_users SET
    totp_secret = NULL,
    totp_secret_backup = NULL,
    recovery_codes = NULL,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: SetUserSMSPhone :exec
UPDATE auth_users SET
    sms_phone_number = ?,
    updated_at = ?
WHERE id = ? AND deleted_at IS NULL;

-- name: SoftDeleteUser :exec
UPDATE auth_users SET deleted_at = ?, updated_at = ? WHERE id = ?;

-- name: HardDeleteUser :exec
DELETE FROM auth_users WHERE id = ?;

-- Token queries

-- name: CreateToken :one
INSERT INTO auth_tokens (
    id, user_id, token_hash, token, type, family_id, provider, provider_uid,
    access_token, oauth_expiry, expires_at, device_info, ip_address, created_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetTokenByID :one
SELECT * FROM auth_tokens WHERE id = ? LIMIT 1;

-- name: GetTokenByHash :one
SELECT * FROM auth_tokens WHERE token_hash = ? AND revoked_at IS NULL LIMIT 1;

-- name: GetTokenByToken :one
SELECT * FROM auth_tokens WHERE token = ? AND revoked_at IS NULL LIMIT 1;

-- name: GetTokenByProviderUID :one
SELECT * FROM auth_tokens WHERE provider_uid = ? AND provider = ? AND revoked_at IS NULL LIMIT 1;

-- name: ListTokensByUserID :many
SELECT * FROM auth_tokens WHERE user_id = ? AND revoked_at IS NULL ORDER BY created_at DESC;

-- name: ListTokensByFamily :many
SELECT * FROM auth_tokens WHERE family_id = ? ORDER BY created_at DESC;

-- name: UpdateTokenUsed :exec
UPDATE auth_tokens SET used_at = ? WHERE id = ?;

-- name: RevokeToken :exec
UPDATE auth_tokens SET revoked_at = ? WHERE id = ?;

-- name: RevokeTokensByUserID :exec
UPDATE auth_tokens SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL;

-- name: RevokeTokensByFamily :exec
UPDATE auth_tokens SET revoked_at = ? WHERE family_id = ? AND revoked_at IS NULL;

-- name: RevokeTokensByType :exec
UPDATE auth_tokens SET revoked_at = ? WHERE user_id = ? AND type = ? AND revoked_at IS NULL;

-- name: DeleteExpiredTokens :exec
DELETE FROM auth_tokens WHERE expires_at < ? OR revoked_at IS NOT NULL;

-- name: DeleteToken :exec
DELETE FROM auth_tokens WHERE id = ?;

-- API Key queries

-- name: CreateAPIKey :one
INSERT INTO api_keys (
    id, user_id, name, key_prefix, key_hash, scopes, expires_at, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetAPIKeyByID :one
SELECT * FROM api_keys WHERE id = ? AND revoked_at IS NULL LIMIT 1;

-- name: GetAPIKeyByHash :one
SELECT * FROM api_keys WHERE key_hash = ? AND revoked_at IS NULL LIMIT 1;

-- name: GetAPIKeyByPrefix :one
SELECT * FROM api_keys WHERE key_prefix = ? AND revoked_at IS NULL LIMIT 1;

-- name: ListAPIKeysByUserID :many
SELECT * FROM api_keys WHERE user_id = ? AND revoked_at IS NULL ORDER BY created_at DESC;

-- name: UpdateAPIKeyLastUsed :exec
UPDATE api_keys SET last_used_at = ?, last_used_ip = ?, updated_at = ? WHERE id = ?;

-- name: UpdateAPIKey :exec
UPDATE api_keys SET name = ?, scopes = ?, expires_at = ?, updated_at = ? WHERE id = ?;

-- name: RevokeAPIKey :exec
UPDATE api_keys SET revoked_at = ?, updated_at = ? WHERE id = ?;

-- name: RevokeAPIKeysByUserID :exec
UPDATE api_keys SET revoked_at = ?, updated_at = ? WHERE user_id = ? AND revoked_at IS NULL;

-- name: DeleteAPIKey :exec
DELETE FROM api_keys WHERE id = ?;
