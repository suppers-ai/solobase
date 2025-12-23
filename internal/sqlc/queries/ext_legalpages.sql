-- Legal Document queries

-- name: CreateLegalDocument :one
INSERT INTO ext_legalpages_legal_documents (
    id, document_type, title, content, version, status, created_by
) VALUES (?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetLegalDocumentByID :one
SELECT * FROM ext_legalpages_legal_documents WHERE id = ? LIMIT 1;

-- name: GetPublishedDocumentByType :one
SELECT * FROM ext_legalpages_legal_documents
WHERE document_type = ? AND status = 'published'
ORDER BY version DESC
LIMIT 1;

-- name: GetLatestDocumentByType :one
SELECT * FROM ext_legalpages_legal_documents
WHERE document_type = ?
ORDER BY version DESC
LIMIT 1;

-- name: GetDocumentByTypeAndVersion :one
SELECT * FROM ext_legalpages_legal_documents
WHERE document_type = ? AND version = ?
LIMIT 1;

-- name: GetMaxVersionByType :one
SELECT CAST(COALESCE(MAX(version), 0) AS INTEGER) FROM ext_legalpages_legal_documents WHERE document_type = ?;

-- name: ListLegalDocuments :many
SELECT * FROM ext_legalpages_legal_documents ORDER BY document_type, version DESC;

-- name: ListLegalDocumentsByType :many
SELECT * FROM ext_legalpages_legal_documents WHERE document_type = ? ORDER BY version DESC;

-- name: ListLegalDocumentsByStatus :many
SELECT * FROM ext_legalpages_legal_documents WHERE status = ? ORDER BY document_type, version DESC;

-- name: ListPublishedDocuments :many
SELECT * FROM ext_legalpages_legal_documents WHERE status = 'published' ORDER BY document_type;

-- name: CountLegalDocuments :one
SELECT COUNT(*) FROM ext_legalpages_legal_documents;

-- name: CountLegalDocumentsByType :one
SELECT COUNT(*) FROM ext_legalpages_legal_documents WHERE document_type = ?;

-- name: UpdateLegalDocument :exec
UPDATE ext_legalpages_legal_documents SET
    title = ?,
    content = ?,
    status = ?,
    updated_at = CURRENT_TIMESTAMP
WHERE id = ?;

-- name: UpdateLegalDocumentStatus :exec
UPDATE ext_legalpages_legal_documents SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?;

-- name: UpdateDocumentStatusByTypeAndVersion :exec
UPDATE ext_legalpages_legal_documents SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE document_type = ? AND version = ?;

-- name: ArchivePublishedDocumentsByType :exec
UPDATE ext_legalpages_legal_documents SET status = 'archived', updated_at = CURRENT_TIMESTAMP WHERE document_type = ? AND status = 'published';

-- name: DeleteLegalDocument :exec
DELETE FROM ext_legalpages_legal_documents WHERE id = ?;
