-- Custom table definitions and migrations

CREATE TABLE IF NOT EXISTS custom_table_definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    description TEXT,
    fields TEXT,
    indexes TEXT,
    options TEXT,
    created_by TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS custom_table_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER,
    version INTEGER,
    migration_type TEXT,
    old_schema TEXT,
    new_schema TEXT,
    executed_by TEXT,
    executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    rollback_at DATETIME,
    status TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_custom_table_migrations_table_id ON custom_table_migrations(table_id);
