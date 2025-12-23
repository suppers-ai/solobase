-- Custom Table Definition queries

-- name: CreateCustomTableDefinition :one
INSERT INTO custom_table_definitions (
    name, display_name, description, fields, indexes, options,
    created_by, status, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetCustomTableDefinitionByID :one
SELECT * FROM custom_table_definitions WHERE id = ? LIMIT 1;

-- name: GetCustomTableDefinitionByName :one
SELECT * FROM custom_table_definitions WHERE name = ? LIMIT 1;

-- name: ListCustomTableDefinitions :many
SELECT * FROM custom_table_definitions ORDER BY name;

-- name: ListActiveCustomTableDefinitions :many
SELECT * FROM custom_table_definitions WHERE status = 'active' ORDER BY name;

-- name: UpdateCustomTableDefinition :exec
UPDATE custom_table_definitions SET
    display_name = ?,
    description = ?,
    fields = ?,
    indexes = ?,
    options = ?,
    status = ?,
    updated_at = ?
WHERE id = ?;

-- name: DeleteCustomTableDefinition :exec
DELETE FROM custom_table_definitions WHERE id = ?;

-- Custom Table Migration queries

-- name: CreateCustomTableMigration :one
INSERT INTO custom_table_migrations (
    table_id, version, migration_type, old_schema, new_schema,
    executed_by, executed_at, status
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
RETURNING *;

-- name: GetCustomTableMigrationByID :one
SELECT * FROM custom_table_migrations WHERE id = ? LIMIT 1;

-- name: ListCustomTableMigrationsByTableID :many
SELECT * FROM custom_table_migrations WHERE table_id = ? ORDER BY version DESC;

-- name: GetLatestCustomTableMigration :one
SELECT * FROM custom_table_migrations WHERE table_id = ? ORDER BY version DESC LIMIT 1;

-- name: UpdateCustomTableMigrationStatus :exec
UPDATE custom_table_migrations SET status = ?, error_message = ? WHERE id = ?;

-- name: RollbackCustomTableMigration :exec
UPDATE custom_table_migrations SET rollback_at = ? WHERE id = ?;
