-- Storage Bucket queries

-- name: CreateBucket :one
INSERT INTO storage_buckets (id, name, public)
VALUES (?, ?, ?)
RETURNING *;

-- name: GetBucketByID :one
SELECT * FROM storage_buckets WHERE id = ? LIMIT 1;

-- name: GetBucketByName :one
SELECT * FROM storage_buckets WHERE name = ? LIMIT 1;

-- name: ListBuckets :many
SELECT * FROM storage_buckets ORDER BY name;

-- name: UpdateBucket :exec
UPDATE storage_buckets SET name = ?, public = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?;

-- name: DeleteBucket :exec
DELETE FROM storage_buckets WHERE id = ?;

-- Storage Object queries

-- name: CreateObject :one
INSERT INTO storage_objects (
    id, bucket_name, object_name, parent_folder_id, size, content_type,
    checksum, metadata, user_id, app_id
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetObjectByID :one
SELECT * FROM storage_objects WHERE id = ? LIMIT 1;

-- name: GetObjectByPath :one
SELECT * FROM storage_objects WHERE bucket_name = ? AND object_name = ? LIMIT 1;

-- name: GetObjectByChecksum :one
SELECT * FROM storage_objects WHERE checksum = ? LIMIT 1;

-- name: ListObjectsByBucket :many
SELECT * FROM storage_objects WHERE bucket_name = ? ORDER BY object_name;

-- name: ListObjectsByParentFolder :many
SELECT * FROM storage_objects WHERE parent_folder_id = ? ORDER BY object_name;

-- name: ListObjectsByUser :many
SELECT * FROM storage_objects WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?;

-- name: ListRootObjectsByBucket :many
SELECT * FROM storage_objects WHERE bucket_name = ? AND parent_folder_id IS NULL ORDER BY object_name;

-- name: UpdateObject :exec
UPDATE storage_objects SET
    object_name = ?,
    parent_folder_id = ?,
    size = ?,
    content_type = ?,
    checksum = ?,
    metadata = ?,
    updated_at = CURRENT_TIMESTAMP
WHERE id = ?;

-- name: UpdateObjectLastViewed :exec
UPDATE storage_objects SET last_viewed = ? WHERE id = ?;

-- name: DeleteObject :exec
DELETE FROM storage_objects WHERE id = ?;

-- name: DeleteObjectsByBucket :exec
DELETE FROM storage_objects WHERE bucket_name = ?;

-- name: DeleteObjectsByParentFolder :exec
DELETE FROM storage_objects WHERE parent_folder_id = ?;

-- name: CountObjectsByBucket :one
SELECT COUNT(*) FROM storage_objects WHERE bucket_name = ?;

-- name: CountObjectsByUser :one
SELECT COUNT(*) FROM storage_objects WHERE user_id = ?;

-- name: SumSizeByBucket :one
SELECT COALESCE(SUM(size), 0) FROM storage_objects WHERE bucket_name = ?;

-- name: SumSizeByUser :one
SELECT COALESCE(SUM(size), 0) FROM storage_objects WHERE user_id = ?;

-- Upload Token queries

-- name: CreateUploadToken :one
INSERT INTO storage_upload_tokens (
    id, token, bucket, parent_folder_id, object_name, user_id,
    max_size, content_type, expires_at, created_at, client_ip
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetUploadTokenByID :one
SELECT * FROM storage_upload_tokens WHERE id = ? LIMIT 1;

-- name: GetUploadTokenByToken :one
SELECT * FROM storage_upload_tokens WHERE token = ? LIMIT 1;

-- name: UpdateUploadTokenProgress :exec
UPDATE storage_upload_tokens SET bytes_uploaded = ? WHERE id = ?;

-- name: CompleteUploadToken :exec
UPDATE storage_upload_tokens SET completed = 1, object_id = ?, completed_at = ? WHERE id = ?;

-- name: DeleteUploadToken :exec
DELETE FROM storage_upload_tokens WHERE id = ?;

-- name: DeleteExpiredUploadTokens :exec
DELETE FROM storage_upload_tokens WHERE expires_at < ? AND completed = 0;

-- Download Token queries

-- name: CreateDownloadToken :one
INSERT INTO storage_download_tokens (
    id, token, file_id, bucket, parent_folder_id, object_name,
    user_id, file_size, expires_at, created_at, client_ip
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetDownloadTokenByID :one
SELECT * FROM storage_download_tokens WHERE id = ? LIMIT 1;

-- name: GetDownloadTokenByToken :one
SELECT * FROM storage_download_tokens WHERE token = ? LIMIT 1;

-- name: UpdateDownloadTokenProgress :exec
UPDATE storage_download_tokens SET bytes_served = ? WHERE id = ?;

-- name: CompleteDownloadToken :exec
UPDATE storage_download_tokens SET completed = 1, callback_at = ? WHERE id = ?;

-- name: DeleteDownloadToken :exec
DELETE FROM storage_download_tokens WHERE id = ?;

-- name: DeleteExpiredDownloadTokens :exec
DELETE FROM storage_download_tokens WHERE expires_at < ? AND completed = 0;
